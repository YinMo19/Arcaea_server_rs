use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::config::RequestChecksumCalculation;
use aws_sdk_s3::primitives::ByteStream;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Parser)]
#[command(name = "sync_s3_manifest")]
#[command(about = "Sync local Arcaea assets to S3-compatible storage and publish manifest.json")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    #[arg(long, env = "S3_ENDPOINT")]
    endpoint: Option<String>,

    #[arg(long, env = "S3_REGION", default_value = "us-east-1")]
    region: String,

    #[arg(long, env = "S3_BUCKET")]
    bucket: String,

    #[arg(
        long,
        env = "S3_ACCESS_KEY_ID",
        alias = "access-key-id",
        hide_env_values = true
    )]
    access_key_id: Option<String>,

    #[arg(
        long,
        env = "S3_SECRET_ACCESS_KEY",
        alias = "secret-access-key",
        hide_env_values = true
    )]
    secret_access_key: Option<String>,

    #[arg(long, env = "AWS_ACCESS_KEY_ID", hide_env_values = true)]
    aws_access_key_id: Option<String>,

    #[arg(long, env = "AWS_SECRET_ACCESS_KEY", hide_env_values = true)]
    aws_secret_access_key: Option<String>,

    #[arg(long, env = "S3_FORCE_PATH_STYLE", default_value_t = false)]
    force_path_style: bool,

    #[arg(long, env = "S3_MANIFEST_KEY", default_value = "manifest.json")]
    manifest_key: String,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Upload songs/bundles and publish a manifest.
    Sync(SyncArgs),
}

#[derive(Debug, Parser)]
struct SyncArgs {
    #[arg(long, env = "S3_SYNC_SONGS_DIR", default_value = "songs")]
    songs_dir: PathBuf,

    #[arg(long, env = "S3_SYNC_BUNDLES_DIR", default_value = "bundles")]
    bundles_dir: PathBuf,

    /// Limit synced song directories. Useful for local MinIO smoke tests.
    #[arg(long, env = "S3_SYNC_SONG_LIMIT")]
    song_limit: Option<usize>,

    /// Skip uploading songs, but still include bundles if enabled.
    #[arg(long, default_value_t = false)]
    skip_songs: bool,

    /// Skip uploading bundle JSON/CB and omit bundle metadata from manifest.
    #[arg(long, default_value_t = false)]
    skip_bundles: bool,

    /// Include bundle metadata but do not upload large .cb files.
    #[arg(long, default_value_t = false)]
    skip_bundle_cb_upload: bool,

    /// Build and print manifest without writing S3 objects.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Do not create or check the bucket before uploading.
    #[arg(long, env = "S3_SYNC_SKIP_CREATE_BUCKET", default_value_t = false)]
    skip_create_bucket: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct StorageManifest {
    #[serde(default)]
    version: String,
    #[serde(default)]
    songs: BTreeMap<String, BTreeMap<String, SongFileMeta>>,
    #[serde(default)]
    bundles: Vec<BundleFileMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct SongFileMeta {
    key: String,
    md5: String,
    size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct BundleFileMeta {
    version: String,
    prev_version: Option<String>,
    app_version: String,
    uuid: String,
    json_size: u64,
    bundle_size: u64,
    json_key: String,
    bundle_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    json_md5: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    bundle_md5: Option<String>,
}

#[derive(Debug)]
struct S3Config {
    endpoint: Option<String>,
    region: String,
    bucket: String,
    access_key_id: String,
    secret_access_key: String,
    force_path_style: bool,
    manifest_key: String,
}

#[derive(Debug, Default)]
struct SyncStats {
    uploaded: usize,
    skipped: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let cli = Cli::parse();
    let config = S3Config::try_from(&cli)?;
    let client = s3_client(&config).await;

    match cli.command {
        Command::Sync(args) => sync(&client, &config, args).await,
    }
}

async fn sync(
    client: &aws_sdk_s3::Client,
    config: &S3Config,
    args: SyncArgs,
) -> anyhow::Result<()> {
    if !args.dry_run && !args.skip_create_bucket {
        ensure_bucket(client, &config.bucket).await?;
    }

    let remote_manifest = load_remote_manifest(client, config).await?;
    let mut stats = SyncStats::default();
    let mut manifest = StorageManifest {
        version: chrono::Utc::now().to_rfc3339(),
        songs: BTreeMap::new(),
        bundles: Vec::new(),
    };

    if !args.skip_songs {
        sync_songs(
            client,
            config,
            &args,
            remote_manifest.as_ref(),
            &mut manifest,
            &mut stats,
        )
        .await?;
    }
    if !args.skip_bundles {
        sync_bundles(
            client,
            config,
            &args,
            remote_manifest.as_ref(),
            &mut manifest,
            &mut stats,
        )
        .await?;
    }

    let manifest_changed = remote_manifest
        .as_ref()
        .map(|remote| !same_manifest_assets(remote, &manifest))
        .unwrap_or(true);
    let manifest_body = serde_json::to_vec_pretty(&manifest)?;
    if args.dry_run {
        println!("{}", String::from_utf8(manifest_body)?);
        eprintln!(
            "dry run: {} objects would upload, {} unchanged objects would skip, manifest_changed={}",
            stats.uploaded, stats.skipped, manifest_changed
        );
    } else if manifest_changed {
        upload_bytes(
            client,
            &config.bucket,
            &config.manifest_key,
            manifest_body,
            Some("application/json"),
        )
        .await?;
        println!(
            "uploaded manifest s3://{}/{} ({} songs, {} bundles)",
            config.bucket,
            config.manifest_key,
            manifest.songs.len(),
            manifest.bundles.len()
        );
    } else {
        println!(
            "manifest unchanged; skipped s3://{}/{}",
            config.bucket, config.manifest_key
        );
    }
    eprintln!(
        "sync summary: {} uploaded, {} skipped unchanged",
        stats.uploaded, stats.skipped
    );

    Ok(())
}

impl TryFrom<&Cli> for S3Config {
    type Error = anyhow::Error;

    fn try_from(cli: &Cli) -> Result<Self, Self::Error> {
        Ok(Self {
            endpoint: cli
                .endpoint
                .clone()
                .filter(|value| !value.trim().is_empty()),
            region: cli.region.clone(),
            bucket: non_empty(cli.bucket.clone())
                .ok_or_else(|| anyhow::anyhow!("--bucket or S3_BUCKET is required"))?,
            access_key_id: cli
                .access_key_id
                .clone()
                .or_else(|| cli.aws_access_key_id.clone())
                .and_then(non_empty)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "--access-key-id, S3_ACCESS_KEY_ID, or AWS_ACCESS_KEY_ID is required"
                    )
                })?,
            secret_access_key: cli
                .secret_access_key
                .clone()
                .or_else(|| cli.aws_secret_access_key.clone())
                .and_then(non_empty)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "--secret-access-key, S3_SECRET_ACCESS_KEY, or AWS_SECRET_ACCESS_KEY is required"
                    )
                })?,
            force_path_style: cli.force_path_style,
            manifest_key: cli.manifest_key.clone(),
        })
    }
}

async fn s3_client(config: &S3Config) -> aws_sdk_s3::Client {
    let credentials = Credentials::new(
        config.access_key_id.clone(),
        config.secret_access_key.clone(),
        None,
        None,
        "arcaea-cli",
    );
    let mut loader = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(config.region.clone()))
        .credentials_provider(credentials);

    if let Some(endpoint) = &config.endpoint {
        loader = loader.endpoint_url(endpoint);
    }

    let shared_config = loader.load().await;
    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .force_path_style(config.force_path_style)
        .request_checksum_calculation(RequestChecksumCalculation::WhenRequired)
        .build();
    aws_sdk_s3::Client::from_conf(s3_config)
}

async fn load_remote_manifest(
    client: &aws_sdk_s3::Client,
    config: &S3Config,
) -> anyhow::Result<Option<StorageManifest>> {
    let output = match client
        .get_object()
        .bucket(&config.bucket)
        .key(&config.manifest_key)
        .send()
        .await
    {
        Ok(output) => output,
        Err(err) if is_not_found_error(&err.to_string()) => {
            eprintln!(
                "remote manifest not found at s3://{}/{}; full upload required",
                config.bucket, config.manifest_key
            );
            return Ok(None);
        }
        Err(err) => return Err(err.into()),
    };

    let bytes = output.body.collect().await?.into_bytes();
    let manifest: StorageManifest = serde_json::from_slice(&bytes)?;
    eprintln!(
        "loaded remote manifest: {} songs, {} bundles",
        manifest.songs.len(),
        manifest.bundles.len()
    );
    Ok(Some(manifest))
}

async fn ensure_bucket(client: &aws_sdk_s3::Client, bucket: &str) -> anyhow::Result<()> {
    if client.head_bucket().bucket(bucket).send().await.is_ok() {
        return Ok(());
    }

    match client.create_bucket().bucket(bucket).send().await {
        Ok(_) => Ok(()),
        Err(err) if err.to_string().contains("BucketAlreadyOwnedByYou") => Ok(()),
        Err(err) if err.to_string().contains("BucketAlreadyExists") => Ok(()),
        Err(err) => Err(err.into()),
    }
}

async fn sync_songs(
    client: &aws_sdk_s3::Client,
    config: &S3Config,
    args: &SyncArgs,
    remote_manifest: Option<&StorageManifest>,
    manifest: &mut StorageManifest,
    stats: &mut SyncStats,
) -> anyhow::Result<()> {
    if !args.songs_dir.exists() {
        return Ok(());
    }

    let mut song_dirs = std::fs::read_dir(&args.songs_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|ty| ty.is_dir()))
        .collect::<Vec<_>>();
    song_dirs.sort_by_key(|entry| entry.file_name());

    for entry in song_dirs
        .into_iter()
        .take(args.song_limit.unwrap_or(usize::MAX))
    {
        let song_id = entry.file_name().to_string_lossy().to_string();
        let mut files = std::fs::read_dir(entry.path())?
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_ok_and(|ty| ty.is_file()))
            .collect::<Vec<_>>();
        files.sort_by_key(|entry| entry.file_name());

        let mut song_files = BTreeMap::new();
        for file in files {
            let file_name = file.file_name().to_string_lossy().to_string();
            if !is_song_download_file(&file_name) {
                continue;
            }

            let path = file.path();
            let key = format!("songs/{song_id}/{file_name}");
            let size = std::fs::metadata(&path)?.len();
            let md5 = file_md5(&path)?;
            let meta = SongFileMeta { key, md5, size };

            if remote_song_matches(remote_manifest, &song_id, &file_name, &meta) {
                stats.skipped += 1;
                if !args.dry_run {
                    println!("skipped unchanged s3://{}/{}", config.bucket, meta.key);
                }
            } else {
                stats.uploaded += 1;
                if !args.dry_run {
                    upload_file(client, &config.bucket, &meta.key, &path, meta.size).await?;
                }
            }

            song_files.insert(file_name, meta);
        }

        if !song_files.is_empty() {
            manifest.songs.insert(song_id, song_files);
        }
    }

    Ok(())
}

async fn sync_bundles(
    client: &aws_sdk_s3::Client,
    config: &S3Config,
    args: &SyncArgs,
    remote_manifest: Option<&StorageManifest>,
    manifest: &mut StorageManifest,
    stats: &mut SyncStats,
) -> anyhow::Result<()> {
    if !args.bundles_dir.exists() {
        return Ok(());
    }

    let mut json_files = std::fs::read_dir(&args.bundles_dir)?
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|ty| ty.is_file())
                && entry.path().extension().is_some_and(|ext| ext == "json")
        })
        .collect::<Vec<_>>();
    json_files.sort_by_key(|entry| entry.file_name());

    for json_file in json_files {
        let json_path = json_file.path();
        let bundle_path = json_path.with_extension("cb");
        if !bundle_path.exists() {
            continue;
        }

        let json_name = file_name(&json_path)?;
        let bundle_name = file_name(&bundle_path)?;
        let json_key = format!("bundles/{json_name}");
        let bundle_key = format!("bundles/{bundle_name}");
        let json_size = std::fs::metadata(&json_path)?.len();
        let bundle_size = std::fs::metadata(&bundle_path)?.len();
        let json_md5 = file_md5(&json_path)?;
        let bundle_md5 = file_md5(&bundle_path)?;
        let json_data: serde_json::Value = serde_json::from_slice(&std::fs::read(&json_path)?)?;

        let meta = BundleFileMeta {
            version: string_field(&json_data, "versionNumber")?,
            prev_version: json_data
                .get("previousVersionNumber")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .or_else(|| Some("0.0.0".to_string())),
            app_version: string_field(&json_data, "applicationVersionNumber")?,
            uuid: string_field(&json_data, "uuid")?,
            json_size,
            bundle_size,
            json_key,
            bundle_key,
            json_md5: Some(json_md5),
            bundle_md5: Some(bundle_md5),
        };

        let remote_bundle = remote_bundle_match(remote_manifest, &meta);
        if bundle_file_matches(
            client,
            &config.bucket,
            remote_bundle,
            &meta.json_key,
            meta.json_size,
            meta.json_md5.as_deref(),
            |bundle| (bundle.json_size, bundle.json_md5.as_deref()),
        )
        .await?
        {
            stats.skipped += 1;
            if !args.dry_run {
                println!("skipped unchanged s3://{}/{}", config.bucket, meta.json_key);
            }
        } else {
            stats.uploaded += 1;
            if !args.dry_run {
                upload_file(
                    client,
                    &config.bucket,
                    &meta.json_key,
                    &json_path,
                    meta.json_size,
                )
                .await?;
            }
        }

        if !args.skip_bundle_cb_upload {
            if bundle_file_matches(
                client,
                &config.bucket,
                remote_bundle,
                &meta.bundle_key,
                meta.bundle_size,
                meta.bundle_md5.as_deref(),
                |bundle| (bundle.bundle_size, bundle.bundle_md5.as_deref()),
            )
            .await?
            {
                stats.skipped += 1;
                if !args.dry_run {
                    println!(
                        "skipped unchanged s3://{}/{}",
                        config.bucket, meta.bundle_key
                    );
                }
            } else {
                stats.uploaded += 1;
                if !args.dry_run {
                    upload_file(
                        client,
                        &config.bucket,
                        &meta.bundle_key,
                        &bundle_path,
                        meta.bundle_size,
                    )
                    .await?;
                }
            }
        }

        manifest.bundles.push(meta);
    }

    Ok(())
}

async fn upload_file(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    path: &Path,
    size: u64,
) -> anyhow::Result<()> {
    for attempt in 1..=3 {
        let body = ByteStream::from_path(path).await?;
        let result = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_length(size as i64)
            .body(body)
            .send()
            .await;
        match result {
            Ok(_) => {
                println!("uploaded s3://{bucket}/{key}");
                return Ok(());
            }
            Err(err) if attempt < 3 => {
                eprintln!("upload failed for s3://{bucket}/{key}, retrying: {err}");
                tokio::time::sleep(Duration::from_secs(attempt)).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!()
}

async fn upload_bytes(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    bytes: Vec<u8>,
    content_type: Option<&str>,
) -> anyhow::Result<()> {
    for attempt in 1..=3 {
        let mut request = client
            .put_object()
            .bucket(bucket)
            .key(key)
            .content_length(bytes.len() as i64)
            .body(ByteStream::from(bytes.clone()));
        if let Some(content_type) = content_type {
            request = request.content_type(content_type);
        }
        match request.send().await {
            Ok(_) => return Ok(()),
            Err(err) if attempt < 3 => {
                eprintln!("upload failed for s3://{bucket}/{key}, retrying: {err}");
                tokio::time::sleep(Duration::from_secs(attempt)).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
    unreachable!()
}

fn file_md5(path: &Path) -> anyhow::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut context = md5::Context::new();
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        context.consume(&buffer[..read]);
    }
    Ok(format!("{:x}", context.compute()))
}

fn remote_song_matches(
    remote_manifest: Option<&StorageManifest>,
    song_id: &str,
    file_name: &str,
    local: &SongFileMeta,
) -> bool {
    remote_manifest
        .and_then(|manifest| manifest.songs.get(song_id))
        .and_then(|files| files.get(file_name))
        .is_some_and(|remote| remote == local)
}

fn remote_bundle_match<'a>(
    remote_manifest: Option<&'a StorageManifest>,
    local: &BundleFileMeta,
) -> Option<&'a BundleFileMeta> {
    remote_manifest.and_then(|manifest| {
        manifest
            .bundles
            .iter()
            .find(|remote| {
                remote.json_key == local.json_key && remote.bundle_key == local.bundle_key
            })
            .or_else(|| {
                manifest.bundles.iter().find(|remote| {
                    remote.version == local.version
                        && remote.uuid == local.uuid
                        && remote.app_version == local.app_version
                })
            })
    })
}

async fn bundle_file_matches(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    remote_bundle: Option<&BundleFileMeta>,
    key: &str,
    local_size: u64,
    local_md5: Option<&str>,
    remote_meta: impl Fn(&BundleFileMeta) -> (u64, Option<&str>),
) -> anyhow::Result<bool> {
    let Some(remote_bundle) = remote_bundle else {
        return Ok(false);
    };

    let (remote_size, remote_md5) = remote_meta(remote_bundle);
    if remote_size != local_size {
        return Ok(false);
    }
    if remote_md5.is_some() && remote_md5 == local_md5 {
        return Ok(true);
    }

    remote_object_matches(client, bucket, key, local_size, local_md5).await
}

async fn remote_object_matches(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    local_size: u64,
    local_md5: Option<&str>,
) -> anyhow::Result<bool> {
    let output = match client.head_object().bucket(bucket).key(key).send().await {
        Ok(output) => output,
        Err(err) if is_not_found_error(&err.to_string()) => return Ok(false),
        Err(err) => return Err(err.into()),
    };

    if output.content_length().unwrap_or_default() as u64 != local_size {
        return Ok(false);
    }

    let Some(local_md5) = local_md5 else {
        return Ok(true);
    };
    Ok(output
        .e_tag()
        .and_then(normalize_etag)
        .is_some_and(|etag| etag.eq_ignore_ascii_case(local_md5)))
}

fn same_manifest_assets(left: &StorageManifest, right: &StorageManifest) -> bool {
    left.songs == right.songs && left.bundles == right.bundles
}

fn normalize_etag(etag: &str) -> Option<String> {
    let value = etag.trim_matches('"');
    if value.is_empty() || value.contains('-') {
        None
    } else {
        Some(value.to_string())
    }
}

fn is_not_found_error(message: &str) -> bool {
    message.contains("NoSuchKey")
        || message.contains("NotFound")
        || message.contains("status code: 404")
        || message.contains("404 Not Found")
}

fn is_song_download_file(file_name: &str) -> bool {
    matches!(
        file_name,
        "0.aff"
            | "1.aff"
            | "2.aff"
            | "3.aff"
            | "4.aff"
            | "base.ogg"
            | "3.ogg"
            | "video.mp4"
            | "video_audio.ogg"
            | "video_720.mp4"
            | "video_1080.mp4"
    )
}

fn string_field(value: &serde_json::Value, key: &str) -> anyhow::Result<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("bundle json missing `{key}`"))
}

fn file_name(path: &Path) -> anyhow::Result<String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("invalid path `{}`", path.display()))
}

fn non_empty(value: String) -> Option<String> {
    if value.trim().is_empty() {
        None
    } else {
        Some(value)
    }
}
