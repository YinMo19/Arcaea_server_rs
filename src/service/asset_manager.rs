//! Asset Manager for dynamic song file and bundle management
//!
//! This module provides functionality for parsing songlist files, managing file caches,
//! and handling user unlock permissions similar to the Python implementation.

use crate::error::{ArcError, ArcResult};
use crate::model::user::UserInfo;
use serde::Deserialize;
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Allowed file names for song downloads
pub const ALLOWED_FILE_NAMES: [&str; 11] = [
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

/// Songlist difficulty information
#[derive(Debug, Clone, Deserialize)]
pub struct Difficulty {
    #[serde(rename = "ratingClass")]
    pub rating_class: i32,
    #[serde(rename = "audioOverride")]
    pub audio_override: Option<bool>,
}

/// Additional file information in songlist
#[derive(Debug, Clone, Deserialize)]
pub struct AdditionalFile {
    #[serde(rename = "fileName")]
    pub file_name: String,
}

/// Song information from songlist
#[derive(Debug, Clone, Deserialize)]
pub struct SongInfo {
    pub id: String,
    pub set: Option<String>,
    pub purchase: Option<String>,
    #[serde(rename = "remoteDl")]
    pub remote_dl: Option<bool>,
    #[serde(rename = "worldUnlock")]
    pub world_unlock: Option<bool>,
    pub difficulties: Option<Vec<Difficulty>>,
    #[serde(rename = "additionalFiles")]
    pub additional_files: Option<Vec<AdditionalFile>>,
}

/// Songlist root structure
#[derive(Debug, Clone, Deserialize)]
pub struct Songlist {
    pub songs: Vec<SongInfo>,
}

/// Cached songlist data
#[derive(Debug, Clone)]
pub struct SonglistCache {
    /// Mapping of song_id to file availability bitmap
    pub songs: HashMap<String, u32>,
    /// Mapping of pack_id to set of song_ids
    pub pack_info: HashMap<String, HashSet<String>>,
    /// Set of free songs
    pub free_songs: HashSet<String>,
    /// Set of world songs (including difficulty variants)
    pub world_songs: HashSet<String>,
    /// Whether songlist was successfully parsed
    pub has_songlist: bool,
}

impl Default for SonglistCache {
    fn default() -> Self {
        Self {
            songs: HashMap::new(),
            pack_info: HashMap::new(),
            free_songs: HashSet::new(),
            world_songs: HashSet::new(),
            has_songlist: false,
        }
    }
}

impl SonglistCache {
    /// Check if a file is available for download for a given song
    pub fn is_available_file(&self, song_id: &str, file_name: &str) -> bool {
        if let Some(&rule) = self.songs.get(song_id) {
            // Check against songlist rules
            for (i, &allowed_file) in ALLOWED_FILE_NAMES.iter().enumerate() {
                if file_name == allowed_file && (rule & (1 << i)) != 0 {
                    return true;
                }
            }
            false
        } else {
            // If song not in songlist, only check against allowed file names
            ALLOWED_FILE_NAMES.contains(&file_name)
        }
    }

    /// Get user's unlocked songs based on packs, singles, and world unlocks
    pub fn get_user_unlocks(&self, user: &UserInfo) -> HashSet<String> {
        let mut unlocks = HashSet::new();

        // Add pack unlocks
        for pack_id in &user.packs {
            if let Some(pack_songs) = self.pack_info.get(pack_id) {
                unlocks.extend(pack_songs.clone());
            }
        }

        // Add single unlocks (from "single" pack)
        if let Some(single_pack) = self.pack_info.get("single") {
            let user_singles: HashSet<String> = user.singles.iter().cloned().collect();
            let single_unlocks: HashSet<String> =
                single_pack.intersection(&user_singles).cloned().collect();
            unlocks.extend(single_unlocks);
        }

        // Add world song unlocks
        let user_world_songs: HashSet<String> = user.world_songs.iter().cloned().collect();
        let world_unlocks: HashSet<String> = self
            .world_songs
            .intersection(&user_world_songs)
            .map(|song| {
                if song.ends_with('3') {
                    song[..song.len() - 1].to_string()
                } else {
                    song.clone()
                }
            })
            .collect();
        unlocks.extend(world_unlocks);

        // Add free songs
        unlocks.extend(self.free_songs.clone());

        unlocks
    }

    /// Parse a single song's file availability into bitmap
    pub fn parse_song_availability(&mut self, song: &SongInfo) -> u32 {
        let mut bitmap = 0u32;

        if song.remote_dl.unwrap_or(false) {
            bitmap |= 32; // base download flag

            // Check for difficulties
            if let Some(ref difficulties) = song.difficulties {
                for diff in difficulties {
                    if diff.rating_class == 3 && diff.audio_override.unwrap_or(false) {
                        bitmap |= 64; // 3.ogg flag
                    }
                    bitmap |= 1 << diff.rating_class; // difficulty flags
                }
            }
        } else {
            // Check for Beyond (difficulty 3) availability
            if let Some(ref difficulties) = song.difficulties {
                if difficulties.iter().any(|d| d.rating_class == 3) {
                    bitmap |= 8;
                }
            }
        }

        // Check for additional files
        if let Some(ref additional_files) = song.additional_files {
            for file in additional_files {
                match file.file_name.as_str() {
                    "video.mp4" => bitmap |= 128,
                    "video_audio.ogg" => bitmap |= 256,
                    "video_720.mp4" => bitmap |= 512,
                    "video_1080.mp4" => bitmap |= 1024,
                    _ => {}
                }
            }
        }

        bitmap
    }

    /// Parse song unlock information
    pub fn parse_song_unlock(&mut self, song: &SongInfo) {
        let song_id = &song.id;

        // Check if it's a free song
        if song.set.as_deref() == Some("base") {
            self.free_songs.insert(song_id.clone());

            // Add Beyond difficulty as world song if available
            if let Some(ref difficulties) = song.difficulties {
                if difficulties.iter().any(|d| d.rating_class == 3) {
                    self.world_songs.insert(format!("{}3", song_id));
                }
            }
            return;
        }

        // Check for world unlock
        if song.world_unlock.unwrap_or(false) {
            self.world_songs.insert(song_id.clone());
        }

        // Skip if no purchase info
        if song.purchase.as_deref().unwrap_or("").is_empty() {
            return;
        }

        // Add to pack info
        if let Some(ref set_name) = song.set {
            self.pack_info
                .entry(set_name.clone())
                .or_insert_with(HashSet::new)
                .insert(song_id.clone());
        }
    }
}

/// File cache for MD5 hashes and file listings
#[derive(Debug, Clone)]
pub struct FileCache {
    /// Cache of file MD5 hashes: (song_id, file_name) -> md5_hash
    pub file_md5_cache: HashMap<(String, String), Option<String>>,
    /// Cache of song file names: song_id -> Vec<file_name>
    pub song_files_cache: HashMap<String, Vec<String>>,
    /// Cache of all song IDs
    pub all_song_ids: Option<Vec<String>>,
}

impl Default for FileCache {
    fn default() -> Self {
        Self {
            file_md5_cache: HashMap::new(),
            song_files_cache: HashMap::new(),
            all_song_ids: None,
        }
    }
}

impl FileCache {
    /// Clear all cached data
    pub fn clear(&mut self) {
        self.file_md5_cache.clear();
        self.song_files_cache.clear();
        self.all_song_ids = None;
    }

    /// Get MD5 hash for a file, with caching
    pub fn get_file_md5(
        &mut self,
        song_file_folder: &str,
        song_id: &str,
        file_name: &str,
    ) -> Option<String> {
        let key = (song_id.to_string(), file_name.to_string());

        if let Some(cached) = self.file_md5_cache.get(&key) {
            return cached.clone();
        }

        let path = Path::new(song_file_folder).join(song_id).join(file_name);
        let md5_hash = if path.is_file() {
            fs::read(&path)
                .ok()
                .map(|contents| format!("{:x}", md5::compute(&contents)))
        } else {
            None
        };

        self.file_md5_cache.insert(key, md5_hash.clone());
        md5_hash
    }

    /// Get file list for a song, with caching
    pub fn get_song_files(
        &mut self,
        song_file_folder: &str,
        song_id: &str,
        songlist: &SonglistCache,
    ) -> Vec<String> {
        if let Some(cached) = self.song_files_cache.get(song_id) {
            return cached.clone();
        }

        let mut files = Vec::new();
        let song_path = Path::new(song_file_folder).join(song_id);

        if let Ok(entries) = fs::read_dir(&song_path) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if songlist.is_available_file(song_id, file_name) {
                                files.push(file_name.to_string());
                            }
                        }
                    }
                }
            }
        }

        self.song_files_cache
            .insert(song_id.to_string(), files.clone());
        files
    }

    /// Get all song IDs, with caching
    pub fn get_all_song_ids(&mut self, song_file_folder: &str) -> Vec<String> {
        if let Some(ref cached) = self.all_song_ids {
            return cached.clone();
        }

        let mut song_ids = Vec::new();
        if let Ok(entries) = fs::read_dir(song_file_folder) {
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

        self.all_song_ids = Some(song_ids.clone());
        song_ids
    }
}

/// Main asset manager for songs and bundles
#[allow(unused)]
#[derive(Debug, Clone)]
pub struct AssetManager {
    pool: MySqlPool,
    song_file_folder: PathBuf,
    songlist_file_path: PathBuf,
    bundle_folder: PathBuf,

    /// Songlist cache protected by RwLock
    songlist_cache: Arc<RwLock<SonglistCache>>,
    /// File cache protected by RwLock
    file_cache: Arc<RwLock<FileCache>>,

    /// Whether to pre-calculate file hashes
    pre_calculate_hashes: bool,
}

impl AssetManager {
    /// Create new asset manager
    pub fn new(
        pool: MySqlPool,
        song_folder: PathBuf,
        songlist_path: PathBuf,
        bundle_folder: PathBuf,
    ) -> Self {
        Self {
            pool,
            song_file_folder: song_folder,
            songlist_file_path: songlist_path,
            bundle_folder,
            songlist_cache: Arc::new(RwLock::new(SonglistCache::default())),
            file_cache: Arc::new(RwLock::new(FileCache::default())),
            pre_calculate_hashes: true,
        }
    }

    /// Set whether to pre-calculate file hashes
    pub fn set_pre_calculate_hashes(mut self, enabled: bool) -> Self {
        self.pre_calculate_hashes = enabled;
        self
    }

    /// Initialize all caches
    pub async fn initialize_cache(&self) -> ArcResult<()> {
        log::info!("Initializing asset cache...");

        // Parse songlist
        self.parse_songlist().await?;

        // Pre-calculate file hashes if enabled
        if self.pre_calculate_hashes {
            self.pre_calculate_file_hashes().await?;
        }

        log::info!("Asset cache initialization completed");
        Ok(())
    }

    /// Clear all caches
    pub async fn clear_all_cache(&self) {
        log::info!("Clearing all asset caches...");

        {
            let mut songlist = self.songlist_cache.write().unwrap();
            *songlist = SonglistCache::default();
        }

        {
            let mut file_cache = self.file_cache.write().unwrap();
            file_cache.clear();
        }

        log::info!("All asset caches cleared");
    }

    /// Reload all caches (clear + initialize)
    pub async fn reload_cache(&self) -> ArcResult<()> {
        log::info!("Reloading asset cache...");
        self.clear_all_cache().await;
        self.initialize_cache().await?;
        log::info!("Asset cache reload completed");
        Ok(())
    }

    /// Parse songlist file
    async fn parse_songlist(&self) -> ArcResult<()> {
        if !self.songlist_file_path.exists() {
            log::warn!("Songlist file not found: {:?}", self.songlist_file_path);
            return Ok(());
        }

        let content = fs::read_to_string(&self.songlist_file_path)
            .map_err(|e| ArcError::no_data(format!("Failed to read songlist: {}", e), 108))?;

        let songlist: Songlist = serde_json::from_str(&content)
            .map_err(|e| ArcError::no_data(format!("Failed to parse songlist: {}", e), 108))?;

        let mut cache = self.songlist_cache.write().unwrap();
        cache.has_songlist = true;

        // Parse each song
        for song in &songlist.songs {
            let bitmap = cache.parse_song_availability(song);
            cache.songs.insert(song.id.clone(), bitmap);
            cache.parse_song_unlock(song);
        }

        log::info!("Parsed {} songs from songlist", songlist.songs.len());
        Ok(())
    }

    /// Pre-calculate file hashes for all songs
    async fn pre_calculate_file_hashes(&self) -> ArcResult<()> {
        let song_ids = {
            let mut file_cache = self.file_cache.write().unwrap();
            file_cache.get_all_song_ids(self.song_file_folder.to_str().unwrap())
        };

        let songlist_cache = self.songlist_cache.read().unwrap().clone();
        let mut file_cache = self.file_cache.write().unwrap();

        for song_id in &song_ids {
            let files = file_cache.get_song_files(
                self.song_file_folder.to_str().unwrap(),
                song_id,
                &songlist_cache,
            );

            for file_name in &files {
                file_cache.get_file_md5(
                    self.song_file_folder.to_str().unwrap(),
                    song_id,
                    file_name,
                );
            }
        }

        log::info!("Pre-calculated hashes for {} songs", song_ids.len());
        Ok(())
    }

    /// Get file MD5 hash
    pub fn get_song_file_md5(&self, song_id: &str, file_name: &str) -> Option<String> {
        let mut file_cache = self.file_cache.write().unwrap();
        file_cache.get_file_md5(self.song_file_folder.to_str().unwrap(), song_id, file_name)
    }

    /// Check if file is available for download
    pub fn is_available_file(&self, song_id: &str, file_name: &str) -> bool {
        let songlist_cache = self.songlist_cache.read().unwrap();
        songlist_cache.is_available_file(song_id, file_name)
    }

    /// Get song file names
    pub fn get_song_file_names(&self, song_id: &str) -> Vec<String> {
        let songlist_cache = self.songlist_cache.read().unwrap().clone();
        let mut file_cache = self.file_cache.write().unwrap();
        file_cache.get_song_files(
            self.song_file_folder.to_str().unwrap(),
            song_id,
            &songlist_cache,
        )
    }

    /// Get all song IDs
    pub fn get_all_song_ids(&self) -> Vec<String> {
        let mut file_cache = self.file_cache.write().unwrap();
        file_cache.get_all_song_ids(self.song_file_folder.to_str().unwrap())
    }

    /// Get user's unlocked songs
    pub fn get_user_unlocks(&self, user: &UserInfo) -> HashSet<String> {
        let songlist_cache = self.songlist_cache.read().unwrap();
        songlist_cache.get_user_unlocks(user)
    }

    /// Check if songlist is loaded
    pub fn has_songlist(&self) -> bool {
        self.songlist_cache.read().unwrap().has_songlist
    }

    /// Check if download should be forbidden when user has no items
    pub fn should_forbid_download_when_no_item(&self, user: &UserInfo) -> bool {
        // TODO: Add config for this setting
        let forbid = false; // Config.DOWNLOAD_FORBID_WHEN_NO_ITEM
        forbid && self.has_songlist() && self.get_user_unlocks(user).is_empty()
    }
}

/// Utility functions for asset management operations
impl AssetManager {
    /// Create asset manager with default paths
    pub fn with_defaults(pool: MySqlPool) -> Self {
        Self::new(
            pool,
            PathBuf::from("./songs"),
            PathBuf::from("./songlist"),
            PathBuf::from("./bundles"),
        )
    }

    /// Set song file folder path
    pub fn with_song_folder(mut self, path: PathBuf) -> Self {
        self.song_file_folder = path;
        self
    }

    /// Set songlist file path
    pub fn with_songlist_path(mut self, path: PathBuf) -> Self {
        self.songlist_file_path = path;
        self
    }

    /// Set bundle folder path
    pub fn with_bundle_folder(mut self, path: PathBuf) -> Self {
        self.bundle_folder = path;
        self
    }
}
