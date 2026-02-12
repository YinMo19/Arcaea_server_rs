use std::env;
use std::path::PathBuf;

pub const DEFAULT_ASSET_DIR: &str = "./assets";

pub fn asset_dir() -> PathBuf {
    if let Ok(value) = env::var("ASSET_DIR") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    PathBuf::from(DEFAULT_ASSET_DIR)
}

pub fn asset_path(relative: &str) -> PathBuf {
    asset_dir().join(relative)
}
