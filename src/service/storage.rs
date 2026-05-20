use crate::error::{ArcError, ArcResult};
use crate::service::cache::{env_ttl_seconds, CacheService};
use aws_config::{BehaviorVersion, Region};
use aws_credential_types::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageBackend {
    Local,
    S3,
}

#[derive(Debug, Clone)]
pub struct StorageConfig {
    backend: StorageBackend,
    s3: Option<S3StorageConfig>,
}

#[derive(Debug, Clone)]
pub struct S3StorageConfig {
    pub endpoint: Option<String>,
    pub region: String,
    pub bucket: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub force_path_style: bool,
    pub manifest_key: String,
    pub presign_expires_seconds: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageManifest {
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub songs: HashMap<String, HashMap<String, SongFileMeta>>,
    #[serde(default)]
    pub bundles: Vec<BundleFileMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongFileMeta {
    pub key: String,
    #[serde(default)]
    pub md5: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleFileMeta {
    pub version: String,
    #[serde(default)]
    pub prev_version: Option<String>,
    pub app_version: String,
    pub uuid: String,
    pub json_size: u64,
    pub bundle_size: u64,
    pub json_key: String,
    pub bundle_key: String,
}

#[derive(Clone)]
pub struct StorageService {
    config: StorageConfig,
    s3: Option<S3Storage>,
    cache: Option<CacheService>,
    presign_cache_ttl_seconds: u64,
}

impl std::fmt::Debug for StorageService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StorageService")
            .field("backend", &self.config.backend)
            .field("has_s3", &self.s3.is_some())
            .field("has_cache", &self.cache.is_some())
            .finish()
    }
}

#[derive(Clone)]
struct S3Storage {
    config: S3StorageConfig,
    client: Client,
    manifest: Arc<RwLock<StorageManifest>>,
}

impl StorageConfig {
    pub fn from_env() -> ArcResult<Self> {
        let backend = match env::var("STORAGE_BACKEND")
            .unwrap_or_else(|_| "local".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "" | "local" => StorageBackend::Local,
            "s3" => StorageBackend::S3,
            other => {
                return Err(ArcError::input(format!(
                    "Invalid STORAGE_BACKEND `{other}`, expected `local` or `s3`"
                )))
            }
        };

        let s3 = if backend == StorageBackend::S3 {
            Some(S3StorageConfig::from_env()?)
        } else {
            None
        };

        Ok(Self { backend, s3 })
    }
}

impl S3StorageConfig {
    fn from_env() -> ArcResult<Self> {
        let bucket = required_env("S3_BUCKET")?;
        let access_key_id = env::var("S3_ACCESS_KEY_ID")
            .or_else(|_| env::var("AWS_ACCESS_KEY_ID"))
            .map_err(|_| ArcError::input("S3_ACCESS_KEY_ID or AWS_ACCESS_KEY_ID is required"))?;
        let secret_access_key = env::var("S3_SECRET_ACCESS_KEY")
            .or_else(|_| env::var("AWS_SECRET_ACCESS_KEY"))
            .map_err(|_| {
                ArcError::input("S3_SECRET_ACCESS_KEY or AWS_SECRET_ACCESS_KEY is required")
            })?;

        Ok(Self {
            endpoint: env::var("S3_ENDPOINT")
                .ok()
                .filter(|value| !value.trim().is_empty()),
            region: env::var("S3_REGION")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "us-east-1".to_string()),
            bucket,
            access_key_id,
            secret_access_key,
            force_path_style: env_bool("S3_FORCE_PATH_STYLE", false),
            manifest_key: env::var("S3_MANIFEST_KEY")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| "manifest.json".to_string()),
            presign_expires_seconds: env::var("S3_PRESIGN_EXPIRES_SECONDS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(3600),
        })
    }
}

impl StorageService {
    pub async fn from_env() -> ArcResult<Self> {
        let config = StorageConfig::from_env()?;
        Self::from_config(config).await
    }

    async fn from_config(config: StorageConfig) -> ArcResult<Self> {
        let s3 = match config.s3.clone() {
            Some(s3_config) => Some(S3Storage::new(s3_config).await?),
            None => None,
        };

        Ok(Self {
            config,
            s3,
            cache: None,
            presign_cache_ttl_seconds: env_ttl_seconds("REDIS_PRESIGN_TTL_SECONDS", 300),
        })
    }

    pub fn with_cache(mut self, cache: Option<CacheService>) -> Self {
        self.cache = cache;
        self
    }

    fn presign_song_cache_key(song_id: &str, file_name: &str) -> String {
        format!("download:presign:{song_id}:{file_name}")
    }

    pub fn is_s3(&self) -> bool {
        self.config.backend == StorageBackend::S3
    }

    pub async fn refresh_manifest(&self) -> ArcResult<()> {
        if let Some(s3) = &self.s3 {
            s3.refresh_manifest().await?;
        }
        Ok(())
    }

    pub fn all_song_ids(&self) -> Option<Vec<String>> {
        let s3 = self.s3.as_ref()?;
        let manifest = s3.manifest.read().unwrap();
        let mut song_ids: Vec<String> = manifest.songs.keys().cloned().collect();
        song_ids.sort();
        Some(song_ids)
    }

    pub fn song_file_names(&self, song_id: &str) -> Option<Vec<String>> {
        let s3 = self.s3.as_ref()?;
        let manifest = s3.manifest.read().unwrap();
        let files = manifest.songs.get(song_id)?;
        let mut file_names: Vec<String> = files.keys().cloned().collect();
        file_names.sort();
        Some(file_names)
    }

    pub fn song_file_md5(&self, song_id: &str, file_name: &str) -> Option<String> {
        let s3 = self.s3.as_ref()?;
        let manifest = s3.manifest.read().unwrap();
        manifest
            .songs
            .get(song_id)
            .and_then(|files| files.get(file_name))
            .and_then(|file| file.md5.clone())
    }

    pub async fn presign_song(&self, song_id: &str, file_name: &str) -> ArcResult<Option<String>> {
        let Some(s3) = &self.s3 else {
            return Ok(None);
        };

        let cache_key = Self::presign_song_cache_key(song_id, file_name);
        if let Some(cache) = &self.cache {
            if let Some(url) = cache.get_string(&cache_key).await {
                return Ok(Some(url));
            }
        }

        let key = {
            let manifest = s3.manifest.read().unwrap();
            manifest
                .songs
                .get(song_id)
                .and_then(|files| files.get(file_name))
                .map(|file| file.key.clone())
        };

        match key {
            Some(key) => {
                let url = s3.presign_get(&key).await?;
                if let Some(cache) = &self.cache {
                    let ttl = self
                        .presign_cache_ttl_seconds
                        .min(s3.config.presign_expires_seconds.saturating_sub(60));
                    cache.set_string(&cache_key, &url, ttl).await;
                }
                Ok(Some(url))
            }
            None => Ok(None),
        }
    }

    pub fn bundle_entries(&self) -> Option<Vec<BundleFileMeta>> {
        let s3 = self.s3.as_ref()?;
        let manifest = s3.manifest.read().unwrap();
        Some(manifest.bundles.clone())
    }

    pub async fn presign_bundle_json(&self, bundle: &BundleFileMeta) -> ArcResult<Option<String>> {
        let Some(s3) = &self.s3 else {
            return Ok(None);
        };
        Ok(Some(s3.presign_get(&bundle.json_key).await?))
    }

    pub async fn presign_bundle_file(&self, bundle: &BundleFileMeta) -> ArcResult<Option<String>> {
        let Some(s3) = &self.s3 else {
            return Ok(None);
        };
        Ok(Some(s3.presign_get(&bundle.bundle_key).await?))
    }
}

impl S3Storage {
    async fn new(config: S3StorageConfig) -> ArcResult<Self> {
        let credentials = Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "arcaea-env",
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
        let client = Client::from_conf(s3_config);

        let storage = Self {
            config,
            client,
            manifest: Arc::new(RwLock::new(StorageManifest::default())),
        };
        storage.refresh_manifest().await?;
        Ok(storage)
    }

    async fn refresh_manifest(&self) -> ArcResult<()> {
        log::info!(
            "Loading S3 storage manifest from s3://{}/{}",
            self.config.bucket,
            self.config.manifest_key
        );
        let output = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(&self.config.manifest_key)
            .send()
            .await
            .map_err(|e| ArcError::input(format!("Failed to fetch S3 manifest: {e}")))?;

        let bytes = output
            .body
            .collect()
            .await
            .map_err(|e| ArcError::input(format!("Failed to read S3 manifest: {e}")))?
            .into_bytes();
        let manifest: StorageManifest = serde_json::from_slice(&bytes)?;

        log::info!(
            "Loaded S3 manifest: {} songs, {} bundles",
            manifest.songs.len(),
            manifest.bundles.len()
        );
        *self.manifest.write().unwrap() = manifest;
        Ok(())
    }

    async fn presign_get(&self, key: &str) -> ArcResult<String> {
        let config =
            PresigningConfig::expires_in(Duration::from_secs(self.config.presign_expires_seconds))
                .map_err(|e| ArcError::input(format!("Invalid S3 presign expiry: {e}")))?;

        let request = self
            .client
            .get_object()
            .bucket(&self.config.bucket)
            .key(key)
            .presigned(config)
            .await
            .map_err(|e| ArcError::input(format!("Failed to presign S3 object `{key}`: {e}")))?;

        Ok(request.uri().to_string())
    }
}

fn required_env(key: &str) -> ArcResult<String> {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| ArcError::input(format!("{key} is required")))
}

fn env_bool(key: &str, default: bool) -> bool {
    match env::var(key) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}
