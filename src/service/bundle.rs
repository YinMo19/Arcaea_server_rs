use crate::error::{ArcError, ArcResult};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Content bundle information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentBundle {
    pub version: String,
    pub prev_version: Option<String>,
    pub app_version: String,
    pub uuid: String,
    pub json_size: u64,
    pub bundle_size: u64,
    pub json_path: String,
    pub bundle_path: String,
    pub json_url: Option<String>,
    pub bundle_url: Option<String>,
}

impl ContentBundle {
    /// Parse version string into tuple for comparison
    pub fn parse_version(version: &str) -> (u32, u32, u32) {
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    }

    /// Get version as comparable tuple
    pub fn version_tuple(&self) -> (u32, u32, u32) {
        Self::parse_version(&self.version)
    }

    /// Create from JSON bundle metadata
    pub fn from_json(
        json_data: &serde_json::Value,
        json_path: String,
        bundle_path: String,
    ) -> ArcResult<Self> {
        let version = json_data["versionNumber"]
            .as_str()
            .ok_or_else(|| ArcError::input("Missing versionNumber in bundle JSON"))?
            .to_string();

        let prev_version = json_data["previousVersionNumber"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| Some("0.0.0".to_string()));

        let app_version = json_data["applicationVersionNumber"]
            .as_str()
            .ok_or_else(|| ArcError::input("Missing applicationVersionNumber in bundle JSON"))?
            .to_string();

        let uuid = json_data["uuid"]
            .as_str()
            .ok_or_else(|| ArcError::input("Missing uuid in bundle JSON"))?
            .to_string();

        // Calculate file sizes
        let json_size = fs::metadata(&json_path)
            .map_err(|e| ArcError::Io {
                message: format!("Failed to get JSON file size: {e}"),
            })?
            .len();

        let bundle_size = fs::metadata(&bundle_path)
            .map_err(|e| ArcError::Io {
                message: format!("Failed to get bundle file size: {e}"),
            })?
            .len();

        Ok(ContentBundle {
            version,
            prev_version,
            app_version,
            uuid,
            json_size,
            bundle_size,
            json_path,
            bundle_path,
            json_url: None,
            bundle_url: None,
        })
    }

    /// Convert to response format
    pub fn to_response(&self) -> BundleResponse {
        BundleResponse {
            content_bundle_version: self.version.clone(),
            app_version: self.app_version.clone(),
            json_size: self.json_size,
            bundle_size: self.bundle_size,
            json_url: self.json_url.clone().unwrap_or("Error".to_string()),
            bundle_url: self.bundle_url.clone().unwrap_or("Error".to_string()),
        }
    }
}

/// Bundle response for API
#[derive(Debug, Serialize, Deserialize)]
pub struct BundleResponse {
    #[serde(rename = "contentBundleVersion")]
    pub content_bundle_version: String,
    #[serde(rename = "appVersion")]
    pub app_version: String,
    #[serde(rename = "jsonSize")]
    pub json_size: u64,
    #[serde(rename = "bundleSize")]
    pub bundle_size: u64,
    #[serde(rename = "jsonUrl")]
    pub json_url: String,
    #[serde(rename = "bundleUrl")]
    pub bundle_url: String,
}

/// Bundle download response
#[derive(Debug, Serialize, Deserialize)]
pub struct BundleDownloadResponse {
    #[serde(rename = "orderedResults")]
    pub ordered_results: Vec<BundleResponse>,
}

/// Bundle service for managing content bundles
#[derive(Clone)]
pub struct BundleService {
    pool: MySqlPool,
    bundle_folder: PathBuf,
    cache: std::sync::Arc<RwLock<BundleCache>>,
    strict_mode: bool,
    download_prefix: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct BundleCache {
    bundles: HashMap<String, Vec<ContentBundle>>, // app_version -> bundles
    max_bundle_version: HashMap<String, String>,  // app_version -> max_version
    next_versions: HashMap<String, Vec<String>>,  // version -> next_versions
    version_tuple_bundles: HashMap<(String, String), ContentBundle>, // (version, prev_version) -> bundle
}

impl BundleService {
    /// Create a new bundle service
    pub fn new(pool: MySqlPool, bundle_folder: PathBuf, download_prefix: Option<String>) -> Self {
        Self {
            pool,
            bundle_folder,
            cache: std::sync::Arc::new(RwLock::new(BundleCache::default())),
            strict_mode: false,
            download_prefix,
        }
    }

    /// Set strict mode for bundle version checking
    pub fn set_strict_mode(&mut self, strict: bool) {
        self.strict_mode = strict;
    }

    /// Set download URL prefix
    pub fn set_download_prefix(&mut self, prefix: Option<String>) {
        self.download_prefix = prefix;
    }

    /// Initialize bundle parser by scanning bundle directory
    pub async fn initialize(&self) -> ArcResult<()> {
        let new_cache = self.parse_bundles()?;
        let mut cache = self.cache.write().await;
        *cache = new_cache;
        Ok(())
    }

    /// Parse all bundles from the bundle directory
    fn parse_bundles(&self) -> ArcResult<BundleCache> {
        let mut cache = BundleCache::default();
        if !self.bundle_folder.exists() {
            return Ok(cache);
        }

        // Walk through bundle directory
        let bundle_folder = self.bundle_folder.clone();
        self.scan_directory(&bundle_folder, &mut cache)?;

        // Sort bundles by version and set max versions
        for (app_version, bundles) in cache.bundles.iter_mut() {
            bundles.sort_by_key(|a| a.version_tuple());
            if let Some(last_bundle) = bundles.last() {
                cache
                    .max_bundle_version
                    .insert(app_version.clone(), last_bundle.version.clone());
            }
        }

        Ok(cache)
    }

    /// Recursively scan directory for bundle files
    fn scan_directory(&self, dir: &Path, cache: &mut BundleCache) -> ArcResult<()> {
        let entries = fs::read_dir(dir).map_err(|e| ArcError::Io {
            message: format!("Failed to read directory: {e}"),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| ArcError::Io {
                message: format!("Failed to read directory entry: {e}"),
            })?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(&path, cache)?;
            } else if path.extension().is_some_and(|ext| ext == "json") {
                self.process_bundle_json(&path, cache)?;
            }
        }

        Ok(())
    }

    /// Process a bundle JSON file
    fn process_bundle_json(&self, json_path: &Path, cache: &mut BundleCache) -> ArcResult<()> {
        let json_content = fs::read_to_string(json_path).map_err(|e| ArcError::Io {
            message: format!("Failed to read JSON file: {e}"),
        })?;

        let json_data: serde_json::Value = serde_json::from_str(&json_content)?;

        // Find corresponding .cb file
        let bundle_path = json_path.with_extension("cb");
        if !bundle_path.exists() {
            return Err(ArcError::input(format!(
                "Bundle file not found: {bundle_path:?}"
            )));
        }

        let json_rel_path = json_path
            .strip_prefix(&self.bundle_folder)
            .map_err(|_| ArcError::input("Invalid JSON path"))?
            .to_string_lossy()
            .replace('\\', "/");

        let bundle_rel_path = {
            let path = bundle_path
                .strip_prefix(&self.bundle_folder)
                .map_err(|_| ArcError::input("Invalid bundle path"))?
                .to_string_lossy();
            path.replace('\\', "/")
        };

        let bundle = ContentBundle::from_json(
            &json_data,
            json_path.to_string_lossy().to_string(),
            bundle_path.to_string_lossy().to_string(),
        )?;

        // Store bundle with relative paths
        let mut bundle_with_rel_paths = bundle.clone();
        bundle_with_rel_paths.json_path = json_rel_path;
        bundle_with_rel_paths.bundle_path = bundle_rel_path;

        // Add to collections
        cache
            .bundles
            .entry(bundle.app_version.clone())
            .or_default()
            .push(bundle_with_rel_paths.clone());

        let prev_version = bundle
            .prev_version
            .clone()
            .unwrap_or_else(|| "0.0.0".to_string());
        cache.version_tuple_bundles.insert(
            (bundle.version.clone(), prev_version.clone()),
            bundle_with_rel_paths,
        );

        cache
            .next_versions
            .entry(prev_version.clone())
            .or_default()
            .push(bundle.version);

        Ok(())
    }

    /// Get bundle list for client update
    pub async fn get_bundle_list(
        &self,
        app_version: &str,
        bundle_version: Option<&str>,
        device_id: Option<&str>,
    ) -> ArcResult<Vec<BundleResponse>> {
        let cache = self.cache.read().await;

        if self.strict_mode {
            let empty_vec = Vec::new();
            let bundles = cache.bundles.get(app_version).unwrap_or(&empty_vec);
            return Ok(bundles.iter().map(|b| b.to_response()).collect());
        }

        let current_version = bundle_version.unwrap_or("0.0.0");

        let target_version = cache
            .max_bundle_version
            .get(app_version)
            .ok_or_else(|| {
                ArcError::no_data(
                    format!("No bundles found for app version: {app_version}"),
                    404,
                )
            })?
            .clone();

        if current_version == target_version.as_str() {
            return Ok(Vec::new());
        }

        // Find update path using BFS
        let update_path =
            Self::find_update_path(&cache.next_versions, current_version, &target_version)?;
        if update_path.is_empty() {
            return Ok(Vec::new());
        }

        let mut matched_bundles = Vec::new();
        for i in 1..update_path.len() {
            let version = &update_path[i];
            let prev_version = &update_path[i - 1];

            if let Some(bundle) = cache
                .version_tuple_bundles
                .get(&(version.clone(), prev_version.clone()))
            {
                if ContentBundle::parse_version(version)
                    <= ContentBundle::parse_version(current_version)
                {
                    continue;
                }
                matched_bundles.push(bundle.clone());
            }
        }
        drop(cache);

        // Generate download tokens and URLs
        let mut results = Vec::new();
        let current_time = chrono::Utc::now().timestamp();

        for bundle in matched_bundles {
            let mut bundle_with_urls = bundle.clone();

            // Generate download tokens
            let json_token = self.generate_token();
            let bundle_token = self.generate_token();

            // Store tokens in database
            self.store_download_token(&json_token, &bundle.json_path, current_time, device_id)
                .await?;
            self.store_download_token(&bundle_token, &bundle.bundle_path, current_time, device_id)
                .await?;

            // Generate URLs
            bundle_with_urls.json_url = Some(self.generate_download_url(&json_token));
            bundle_with_urls.bundle_url = Some(self.generate_download_url(&bundle_token));

            let response = bundle_with_urls.to_response();
            results.push(response);
        }

        Ok(results)
    }

    /// Find update path from current version to target version using BFS
    fn find_update_path(
        next_versions: &HashMap<String, Vec<String>>,
        current_version: &str,
        target_version: &str,
    ) -> ArcResult<Vec<String>> {
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut paths: HashMap<String, Vec<String>> = HashMap::new();

        queue.push_back(current_version.to_string());
        paths.insert(
            current_version.to_string(),
            vec![current_version.to_string()],
        );
        visited.insert(current_version.to_string());

        while let Some(version) = queue.pop_front() {
            if version == target_version {
                return Ok(paths.get(&version).unwrap().clone());
            }

            if let Some(next_list) = next_versions.get(&version) {
                for next_version in next_list {
                    if !visited.contains(next_version) {
                        visited.insert(next_version.clone());
                        let mut new_path = paths.get(&version).unwrap().clone();
                        new_path.push(next_version.clone());
                        paths.insert(next_version.clone(), new_path);
                        queue.push_back(next_version.clone());
                    }
                }
            }
        }

        Err(ArcError::no_data(
            format!("No update path found from {current_version} to {target_version}"),
            404,
        ))
    }

    /// Generate a random download token
    fn generate_token(&self) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..64)
            .map(|_| format!("{:02x}", rng.gen::<u8>()))
            .collect()
    }

    /// Store download token in database
    async fn store_download_token(
        &self,
        token: &str,
        file_path: &str,
        timestamp: i64,
        device_id: Option<&str>,
    ) -> ArcResult<()> {
        sqlx::query!(
            "INSERT INTO bundle_download_token (token, file_path, time, device_id) VALUES (?, ?, ?, ?)",
            token,
            file_path,
            timestamp,
            device_id.unwrap_or("")
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Generate download URL for token
    fn generate_download_url(&self, token: &str) -> String {
        if let Some(prefix) = &self.download_prefix {
            let mut url = prefix.clone();
            if !url.ends_with('/') {
                url.push('/');
            }
            format!("{url}{token}")
        } else {
            // Default to relative URL
            format!("/bundle_download/{token}")
        }
    }

    /// Get file path by download token
    pub async fn get_file_path_by_token(&self, token: &str, _ip: &str) -> ArcResult<String> {
        let result = sqlx::query!(
            "SELECT file_path, time, device_id FROM bundle_download_token WHERE token = ?",
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        let (file_path, create_time, _device_id) = result
            .map(|r| (r.file_path, r.time, r.device_id))
            .ok_or_else(|| ArcError::no_access("Invalid token".to_string(), 403))?;

        let current_time = chrono::Utc::now().timestamp();
        const DOWNLOAD_TIME_LIMIT: i64 = 3600; // 1 hour
        let create_time = create_time.unwrap_or(0);

        if current_time - create_time > DOWNLOAD_TIME_LIMIT {
            return Err(ArcError::no_access("Expired token".to_string(), 403));
        }

        // TODO: Implement rate limiting for bundle downloads
        // Check if this is a .cb file and apply rate limiting

        Ok(file_path.unwrap_or_default())
    }

    /// Clean up expired download tokens
    pub async fn cleanup_expired_tokens(&self) -> ArcResult<u64> {
        let current_time = chrono::Utc::now().timestamp();
        const DOWNLOAD_TIME_LIMIT: i64 = 3600; // 1 hour
        let expire_threshold = current_time - DOWNLOAD_TIME_LIMIT;

        let result = sqlx::query!(
            "DELETE FROM bundle_download_token WHERE time < ?",
            expire_threshold
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Get bundle file as bytes for download
    pub async fn get_bundle_file(&self, file_path: &str) -> ArcResult<Vec<u8>> {
        let full_path = self.bundle_folder.join(file_path);

        if !full_path.exists() {
            return Err(ArcError::no_data("File not found".to_string(), 404));
        }

        let content = fs::read(&full_path).map_err(|e| ArcError::Io {
            message: format!("Failed to read file: {e}"),
        })?;

        Ok(content)
    }

    /// Get bundle file path for serving
    pub async fn get_bundle_file_path(&self, file_path: &str) -> ArcResult<PathBuf> {
        let full_path = self.bundle_folder.join(file_path);

        if !full_path.exists() {
            return Err(ArcError::no_data("File not found".to_string(), 404));
        }

        Ok(full_path)
    }
}
