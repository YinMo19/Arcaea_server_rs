use crate::error::{ArcError, ArcResult};
use crate::model::download::{DownloadAudio, DownloadFile, DownloadSong};
use crate::model::user::UserInfo;
use base64::{engine::general_purpose, Engine as _};

use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Song file names that are allowed for download
const ALLOWED_FILE_NAMES: [&str; 11] = [
    "0.aff",
    "1.aff",
    "2.aff",
    "3.aff",
    "4.aff",
    "base.ogg",
    "3.ogg",
    "video.mp4",
    "video_audio.ogg",
    "video_720.mp4",
    "video_1080.mp4",
];

/// Download service for handling song file downloads and token management
pub struct DownloadService {
    pool: MySqlPool,
    song_file_folder_path: String,
    songlist_file_path: String,
    download_link_prefix: Option<String>,
    download_time_gap_limit: i64,
    download_times_limit: i32,
}

impl DownloadService {
    /// Create a new download service instance
    pub fn new(
        pool: MySqlPool,
        song_file_folder_path: String,
        songlist_file_path: String,
        download_link_prefix: Option<String>,
        download_time_gap_limit: i64,
        download_times_limit: i32,
    ) -> Self {
        Self {
            pool,
            song_file_folder_path,
            songlist_file_path,
            download_link_prefix,
            download_time_gap_limit,
            download_times_limit,
        }
    }

    /// Calculate MD5 hash of a song file
    pub fn get_song_file_md5(&self, song_id: &str, file_name: &str) -> Option<String> {
        let path = Path::new(&self.song_file_folder_path)
            .join(song_id)
            .join(file_name);

        if !path.is_file() {
            return None;
        }

        match fs::read(&path) {
            Ok(contents) => Some(format!("{:x}", md5::compute(&contents))),
            Err(_) => None,
        }
    }

    /// Check if a file is allowed for download based on songlist rules
    pub fn is_available_file(&self, _song_id: &str, file_name: &str) -> bool {
        // TODO: Implement songlist parsing logic
        // For now, just check if file name is in allowed list
        ALLOWED_FILE_NAMES.contains(&file_name)
    }

    /// Generate a download token for a user and file
    pub fn generate_download_token(&self, user_id: i32, song_id: &str, file_name: &str) -> String {
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let token_data = format!(
            "{}{}{}{}{}",
            user_id,
            song_id,
            file_name,
            current_time,
            base64::engine::general_purpose::STANDARD.encode(&rand::random::<[u8; 8]>())
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

    /// Validate download token
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

                if current_time - row.time.unwrap_or(0) > self.download_time_gap_limit {
                    return Err(ArcError::no_access(
                        "Download token has expired".to_string(),
                        403,
                    ));
                }

                Ok((row.user_id, row.time.unwrap_or(0)))
            }
            None => Err(ArcError::no_access(
                "Invalid download token".to_string(),
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
    pub async fn check_download_limit(&self, _user_id: i32) -> ArcResult<bool> {
        // TODO: Implement proper rate limiting with timestamp checking
        // For now, return false (not limited)
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
            format!(
                "{}{}?t={}",
                prefix,
                format!("{}/{}", song_id, file_name),
                token
            )
        } else {
            // Return relative URL if no prefix is configured
            format!("/download/{}/{}?t={}", song_id, file_name, token)
        }
    }

    /// Get list of available files for a song
    pub fn get_song_file_names(&self, song_id: &str) -> Vec<String> {
        let song_path = Path::new(&self.song_file_folder_path).join(song_id);
        let mut files = Vec::new();

        if let Ok(entries) = fs::read_dir(&song_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if self.is_available_file(song_id, file_name) {
                                files.push(file_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        files
    }

    /// Get list of all song IDs
    pub fn get_all_song_ids(&self) -> Vec<String> {
        let mut song_ids = Vec::new();

        if let Ok(entries) = fs::read_dir(&self.song_file_folder_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        if let Some(dir_name) = entry.file_name().to_str() {
                            song_ids.push(dir_name.to_string());
                        }
                    }
                }
            }
        }

        song_ids
    }

    /// Generate download list for user
    pub async fn generate_download_list(
        &self,
        user: &UserInfo,
        song_ids: Option<Vec<String>>,
        include_urls: bool,
    ) -> ArcResult<HashMap<String, DownloadSong>> {
        // Check download limit if URLs are requested
        if include_urls && self.check_download_limit(user.user_id).await? {
            return Err(ArcError::rate_limit(
                "You have reached the download limit".to_string(),
                903,
                -999,
            ));
        }

        let target_song_ids = song_ids.unwrap_or_else(|| self.get_all_song_ids());
        let mut download_songs = HashMap::new();
        let mut download_tokens = Vec::new();

        // Clear expired tokens
        self.clear_expired_download_tokens().await?;

        for song_id in target_song_ids {
            let song_path = Path::new(&self.song_file_folder_path).join(&song_id);
            if !song_path.exists() {
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
                let (url, _token) = if include_urls {
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

                match file_name.as_str() {
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
                            file_name: Some(file_name.clone()),
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

    /// Get user's unlocked songs (placeholder for songlist integration)
    pub async fn get_user_unlocks(&self, _user: &UserInfo) -> ArcResult<HashSet<String>> {
        // TODO: Implement proper songlist parsing and user unlock checking
        // For now, return all available songs
        Ok(self.get_all_song_ids().into_iter().collect())
    }
}

/// Download service utilities
impl DownloadService {
    /// Initialize download service with default configuration
    pub fn with_defaults(pool: MySqlPool) -> Self {
        Self::new(
            pool,
            "./songs".to_string(),
            "./songlist".to_string(),
            None,
            3600, // 1 hour token expiry
            100,  // 100 downloads per day
        )
    }

    /// Set song file folder path
    pub fn with_song_folder_path(mut self, path: String) -> Self {
        self.song_file_folder_path = path;
        self
    }

    /// Set songlist file path
    pub fn with_songlist_path(mut self, path: String) -> Self {
        self.songlist_file_path = path;
        self
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
}
