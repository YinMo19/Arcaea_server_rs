//! Download service for handling song file downloads and token management
//!
//! This service integrates with AssetManager to provide songlist-aware download functionality,
//! user permission checking, and dynamic cache management similar to the Python implementation.

use crate::error::{ArcError, ArcResult};
use crate::model::download::{DownloadAudio, DownloadFile, DownloadSong};
use crate::model::user::UserInfo;
use crate::service::asset_manager::AssetManager;
use base64::Engine as _;
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Download service for handling song file downloads and token management
pub struct DownloadService {
    pool: MySqlPool,
    asset_manager: Arc<AssetManager>,
    download_link_prefix: Option<String>,
    download_time_gap_limit: i64,
    download_times_limit: i32,
}

impl DownloadService {
    /// Create a new download service instance
    pub fn new(
        pool: MySqlPool,
        asset_manager: Arc<AssetManager>,
        download_link_prefix: Option<String>,
        download_time_gap_limit: i64,
        download_times_limit: i32,
    ) -> Self {
        Self {
            pool,
            asset_manager,
            download_link_prefix,
            download_time_gap_limit,
            download_times_limit,
        }
    }

    /// Calculate MD5 hash of a song file using asset manager cache
    pub fn get_song_file_md5(&self, song_id: &str, file_name: &str) -> Option<String> {
        self.asset_manager.get_song_file_md5(song_id, file_name)
    }

    /// Check if a file is allowed for download based on songlist rules
    pub fn is_available_file(&self, song_id: &str, file_name: &str) -> bool {
        self.asset_manager.is_available_file(song_id, file_name)
    }

    /// Generate a download token for a user and file
    pub fn generate_download_token(&self, user_id: i32, song_id: &str, file_name: &str) -> String {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let random_bytes = rand::random::<[u8; 8]>();
        let token_data = format!(
            "{}{}{}{}{}",
            user_id,
            song_id,
            file_name,
            current_time,
            base64::engine::general_purpose::STANDARD.encode(&random_bytes)
        );

        format!("{:x}", md5::compute(token_data.as_bytes()))
    }

    /// Insert or update download token in database
    pub async fn insert_download_token(
        &self,
        user_id: i32,
        song_id: &str,
        file_name: &str,
        token: &str,
    ) -> ArcResult<()> {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        sqlx::query!(
            "INSERT INTO download_token (user_id, song_id, file_name, token, time)
             VALUES (?, ?, ?, ?, ?)
             ON DUPLICATE KEY UPDATE token = VALUES(token), time = VALUES(time)",
            user_id,
            song_id,
            file_name,
            token,
            current_time
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Validate download token and return user_id and creation time
    pub async fn validate_download_token(
        &self,
        song_id: &str,
        file_name: &str,
        token: &str,
    ) -> ArcResult<(i32, i64)> {
        let result = sqlx::query!(
            "SELECT user_id, time FROM download_token
             WHERE song_id = ? AND file_name = ? AND token = ? LIMIT 1",
            song_id,
            file_name,
            token
        )
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                let current_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;

                let token_time = row.time.unwrap_or(0);
                if current_time - token_time > self.download_time_gap_limit {
                    return Err(ArcError::no_access(
                        format!("The token `{}` has expired.", token),
                        403,
                    ));
                }

                Ok((row.user_id, token_time))
            }
            None => Err(ArcError::no_access(
                format!("The token `{}` is not valid.", token),
                403,
            )),
        }
    }

    /// Clear expired download tokens
    pub async fn clear_expired_download_tokens(&self) -> ArcResult<()> {
        let cutoff_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64
            - self.download_time_gap_limit;

        sqlx::query!("DELETE FROM download_token WHERE time < ?", cutoff_time)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    /// Check if user has reached download limit
    /// TODO: Implement proper rate limiting similar to Python's ArcLimiter
    pub async fn check_download_limit(&self, _user_id: i32) -> ArcResult<bool> {
        // For now, return false (not limited)
        // In the future, implement rate limiting based on download_times_limit
        Ok(false)
    }

    /// Generate download URL for a file
    pub fn generate_download_url(&self, song_id: &str, file_name: &str, token: &str) -> String {
        if let Some(ref prefix) = self.download_link_prefix {
            let prefix = if prefix.ends_with('/') {
                prefix.clone()
            } else {
                format!("{}/", prefix)
            };
            format!("{}{}/{}?t={}", prefix, song_id, file_name, token)
        } else {
            // Use relative URL pattern similar to Python's url_for
            format!("/download/{}/{}?t={}", song_id, file_name, token)
        }
    }

    /// Get list of available files for a song using asset manager
    pub fn get_song_file_names(&self, song_id: &str) -> Vec<String> {
        self.asset_manager.get_song_file_names(song_id)
    }

    /// Get list of all song IDs using asset manager
    pub fn get_all_song_ids(&self) -> Vec<String> {
        self.asset_manager.get_all_song_ids()
    }

    /// Generate download list for user with proper permission checking
    pub async fn generate_download_list(
        &self,
        user: &UserInfo,
        song_ids: Option<Vec<String>>,
        include_urls: bool,
    ) -> ArcResult<HashMap<String, DownloadSong>> {
        // Check if download should be forbidden when user has no unlocked items
        if self.asset_manager.should_forbid_download_when_no_item(user) {
            return Ok(HashMap::new());
        }

        // Check download limit if URLs are requested
        if include_urls && self.check_download_limit(user.user_id).await? {
            return Err(ArcError::rate_limit(
                "You have reached the download limit.".to_string(),
                903,
                -999,
            ));
        }

        // Get target song IDs
        let target_song_ids = if let Some(song_ids) = song_ids {
            // Filter requested songs by user's unlocked songs
            if self.asset_manager.has_songlist() {
                let user_unlocks = self.asset_manager.get_user_unlocks(user);
                song_ids
                    .into_iter()
                    .filter(|id| user_unlocks.contains(id))
                    .collect()
            } else {
                song_ids
            }
        } else {
            // Get all songs, filtered by user unlocks if songlist is available
            let all_songs = self.get_all_song_ids();
            if self.asset_manager.has_songlist() {
                let user_unlocks = self.asset_manager.get_user_unlocks(user);
                all_songs
                    .into_iter()
                    .filter(|id| user_unlocks.contains(id))
                    .collect()
            } else {
                all_songs
            }
        };

        let mut download_songs = HashMap::new();
        let mut download_tokens = Vec::new();

        // Clear expired tokens before generating new ones
        if include_urls {
            self.clear_expired_download_tokens().await?;
        }

        for song_id in target_song_ids {
            // Check if song directory exists
            if !self.song_exists(&song_id) {
                continue;
            }

            let mut download_song = DownloadSong {
                audio: None,
                chart: None,
                additional_files: None,
            };

            let file_names = self.get_song_file_names(&song_id);

            for file_name in file_names {
                let checksum = self.get_song_file_md5(&song_id, &file_name);
                let (url, token) = if include_urls {
                    let token = self.generate_download_token(user.user_id, &song_id, &file_name);
                    let url = self.generate_download_url(&song_id, &file_name, &token);
                    download_tokens.push((
                        user.user_id,
                        song_id.clone(),
                        file_name.clone(),
                        token.clone(),
                    ));
                    (Some(url), Some(token))
                } else {
                    (None, None)
                };

                self.process_file_into_song(&mut download_song, &file_name, checksum, url, token);
            }

            download_songs.insert(song_id, download_song);
        }

        // Insert all download tokens at once if URLs are included
        if include_urls && !download_tokens.is_empty() {
            for (user_id, song_id, file_name, token) in download_tokens {
                self.insert_download_token(user_id, &song_id, &file_name, &token)
                    .await?;
            }
        }

        Ok(download_songs)
    }

    /// Process a file into the appropriate section of DownloadSong
    fn process_file_into_song(
        &self,
        download_song: &mut DownloadSong,
        file_name: &str,
        checksum: Option<String>,
        url: Option<String>,
        _token: Option<String>,
    ) {
        match file_name {
            "base.ogg" => {
                let audio = DownloadAudio {
                    checksum: checksum.clone(),
                    url: url.clone(),
                    difficulty_3: None,
                };
                download_song.audio = Some(audio);
            }
            "3.ogg" => {
                if let Some(ref mut audio) = download_song.audio {
                    audio.difficulty_3 = Some(DownloadFile {
                        checksum,
                        url,
                        file_name: None,
                    });
                } else {
                    let audio = DownloadAudio {
                        checksum: None,
                        url: None,
                        difficulty_3: Some(DownloadFile {
                            checksum,
                            url,
                            file_name: None,
                        }),
                    };
                    download_song.audio = Some(audio);
                }
            }
            "video.mp4" | "video_audio.ogg" | "video_720.mp4" | "video_1080.mp4" => {
                let additional_file = DownloadFile {
                    checksum,
                    url,
                    file_name: Some(file_name.to_string()),
                };

                if let Some(ref mut additional_files) = download_song.additional_files {
                    additional_files.push(additional_file);
                } else {
                    download_song.additional_files = Some(vec![additional_file]);
                }
            }
            chart_file if chart_file.ends_with(".aff") => {
                let difficulty_key = chart_file.chars().next().unwrap().to_string();
                let chart_entry = DownloadFile {
                    checksum,
                    url,
                    file_name: None,
                };

                if let Some(ref mut chart) = download_song.chart {
                    chart.insert(difficulty_key, chart_entry);
                } else {
                    let mut chart = HashMap::new();
                    chart.insert(difficulty_key, chart_entry);
                    download_song.chart = Some(chart);
                }
            }
            _ => {}
        }
    }

    /// Get user's unlocked songs using asset manager
    pub fn get_user_unlocks(&self, user: &UserInfo) -> std::collections::HashSet<String> {
        self.asset_manager.get_user_unlocks(user)
    }

    /// Check if a song directory exists
    fn song_exists(&self, song_id: &str) -> bool {
        let all_songs = self.get_all_song_ids();
        all_songs.contains(&song_id.to_string())
    }

    /// Initialize song data cache (equivalent to Python's initialize_cache)
    pub async fn initialize_cache(&self) -> ArcResult<()> {
        self.asset_manager.initialize_cache().await
    }

    /// Clear all song data cache (equivalent to Python's clear_all_cache)
    pub async fn clear_all_cache(&self) {
        self.asset_manager.clear_all_cache().await
    }

    /// Reload all caches (clear + initialize)
    pub async fn reload_cache(&self) -> ArcResult<()> {
        self.asset_manager.reload_cache().await
    }
}

/// Download service builder and utility methods
impl DownloadService {
    /// Initialize download service with default configuration
    pub fn with_defaults(pool: MySqlPool) -> Self {
        let asset_manager = Arc::new(AssetManager::with_defaults(pool.clone()));
        Self::new(
            pool,
            asset_manager,
            None,
            3600, // 1 hour token expiry
            100,  // 100 downloads per day
        )
    }

    /// Set download link prefix
    pub fn with_download_prefix(mut self, prefix: Option<String>) -> Self {
        self.download_link_prefix = prefix;
        self
    }

    /// Set download time gap limit
    pub fn with_time_gap_limit(mut self, limit: i64) -> Self {
        self.download_time_gap_limit = limit;
        self
    }

    /// Set download times limit
    pub fn with_times_limit(mut self, limit: i32) -> Self {
        self.download_times_limit = limit;
        self
    }

    /// Get reference to asset manager
    pub fn asset_manager(&self) -> &AssetManager {
        &self.asset_manager
    }
}
