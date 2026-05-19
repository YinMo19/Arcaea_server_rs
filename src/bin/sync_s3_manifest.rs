use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use clap::{Parser, Subcommand};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

#[derive(Debug, Serialize)]
struct StorageManifest {
    version: String,
    songs: BTreeMap<String, BTreeMap<String, SongFileMeta>>,
    bundles: Vec<BundleFileMeta>,
}

#[derive(Debug, Serialize)]
struct SongFileMeta {
    key: String,
    md5: String,
    size: u64,
}

#[derive(Debug, Serialize)]
struct BundleFileMeta {
    version: String,
    prev_version: Option<String>,
    app_version: String,
    uuid: String,
    json_size: u64,
    bundle_size: u64,
    json_key: String,
    bundle_key: String,
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

    let mut manifest = StorageManifest {
        version: chrono::Utc::now().to_rfc3339(),
        songs: BTreeMap::new(),
        bundles: Vec::new(),
    };

    if !args.skip_songs {
        sync_songs(client, config, &args, &mut manifest).await?;
    }
    if !args.skip_bundles {
        sync_bundles(client, config, &args, &mut manifest).await?;
    }

    let manifest_body = serde_json::to_vec_pretty(&manifest)?;
    if args.dry_run {
        println!("{}", String::from_utf8(manifest_body)?);
    } else {
        client
            .put_object()
            .bucket(&config.bucket)
            .key(&config.manifest_key)
            .body(ByteStream::from(manifest_body))
            .content_type("application/json")
            .send()
            .await?;

        println!(
            "uploaded manifest s3://{}/{} ({} songs, {} bundles)",
            config.bucket,
            config.manifest_key,
            manifest.songs.len(),
            manifest.bundles.len()
        );
    }

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
        .build();
    aws_sdk_s3::Client::from_conf(s3_config)
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
    manifest: &mut StorageManifest,
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
            if !args.dry_run {
                upload_file(client, &config.bucket, &key, &path).await?;
            }

            let bytes = std::fs::read(&path)?;
            let size = bytes.len() as u64;
            let md5 = format!("{:x}", md5::compute(&bytes));
            song_files.insert(file_name, SongFileMeta { key, md5, size });
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
    manifest: &mut StorageManifest,
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

        if !args.dry_run {
            upload_file(client, &config.bucket, &json_key, &json_path).await?;
            if !args.skip_bundle_cb_upload {
                upload_file(client, &config.bucket, &bundle_key, &bundle_path).await?;
            }
        }

        let json_data: serde_json::Value = serde_json::from_slice(&std::fs::read(&json_path)?)?;
        manifest.bundles.push(BundleFileMeta {
            version: string_field(&json_data, "versionNumber")?,
            prev_version: json_data
                .get("previousVersionNumber")
                .and_then(|value| value.as_str())
                .map(ToOwned::to_owned)
                .or_else(|| Some("0.0.0".to_string())),
            app_version: string_field(&json_data, "applicationVersionNumber")?,
            uuid: string_field(&json_data, "uuid")?,
            json_size: std::fs::metadata(&json_path)?.len(),
            bundle_size: std::fs::metadata(&bundle_path)?.len(),
            json_key,
            bundle_key,
        });
    }

    Ok(())
}

async fn upload_file(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    path: &Path,
) -> anyhow::Result<()> {
    let body = ByteStream::from_path(path).await?;
    client
        .put_object()
        .bucket(bucket)
        .key(key)
        .body(body)
        .send()
        .await?;
    println!("uploaded s3://{bucket}/{key}");
    Ok(())
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
