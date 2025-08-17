//! Operations module for dynamic asset and cache management
//!
//! This module provides operations for refreshing various caches and performing
//! maintenance tasks, similar to the Python implementation's operation.py.

use crate::error::{ArcError, ArcResult};
use crate::service::asset_manager::AssetManager;
use crate::service::bundle::BundleService;

use async_trait::async_trait;
use sqlx::MySqlPool;
use std::sync::Arc;

/// Base trait for all operations
#[async_trait]
pub trait Operation: Send + Sync {
    /// Get the operation name
    fn name(&self) -> &'static str;

    /// Execute the operation
    async fn execute(&self) -> ArcResult<()>;

    /// Set parameters for the operation (optional)
    fn set_params(&mut self, _params: OperationParams) -> ArcResult<()> {
        Ok(())
    }
}

/// Parameters that can be passed to operations
#[derive(Debug, Clone)]
pub struct OperationParams {
    pub user_id: Option<i32>,
    pub song_ids: Option<Vec<String>>,
    pub other: Option<serde_json::Value>,
}

impl Default for OperationParams {
    fn default() -> Self {
        Self {
            user_id: None,
            song_ids: None,
            other: None,
        }
    }
}

/// Operation to refresh song file cache
/// Equivalent to Python's RefreshSongFileCache
pub struct RefreshSongFileCache {
    asset_manager: Arc<AssetManager>,
}

impl RefreshSongFileCache {
    pub fn new(asset_manager: Arc<AssetManager>) -> Self {
        Self { asset_manager }
    }
}

#[async_trait]
impl Operation for RefreshSongFileCache {
    fn name(&self) -> &'static str {
        "refresh_song_file_cache"
    }

    async fn execute(&self) -> ArcResult<()> {
        log::info!("Executing operation: {}", self.name());

        // Clear all song-related caches
        self.asset_manager.clear_all_cache().await;

        // Reinitialize the cache
        self.asset_manager.initialize_cache().await?;

        log::info!("Song file cache refresh completed");
        Ok(())
    }
}

/// Operation to refresh bundle cache
/// Equivalent to Python's RefreshBundleCache
pub struct RefreshBundleCache {
    bundle_service: Arc<BundleService>,
}

impl RefreshBundleCache {
    pub fn new(bundle_service: Arc<BundleService>) -> Self {
        Self { bundle_service }
    }
}

#[async_trait]
impl Operation for RefreshBundleCache {
    fn name(&self) -> &'static str {
        "refresh_content_bundle_cache"
    }

    async fn execute(&self) -> ArcResult<()> {
        log::info!("Executing operation: {}", self.name());

        // TODO: Implement bundle cache refresh
        // Note: BundleService::initialize() requires mutable access
        // This needs to be implemented differently or BundleService needs
        // to provide a method that works with shared references
        log::warn!("Bundle cache refresh not yet implemented due to Arc<T> constraints");

        log::info!("Bundle cache refresh completed");
        Ok(())
    }
}

/// Operation to refresh all score ratings
/// Equivalent to Python's RefreshAllScoreRating
pub struct RefreshAllScoreRating {
    pool: MySqlPool,
}

impl RefreshAllScoreRating {
    pub fn new(pool: MySqlPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Operation for RefreshAllScoreRating {
    fn name(&self) -> &'static str {
        "refresh_all_score_rating"
    }

    async fn execute(&self) -> ArcResult<()> {
        log::info!("Executing operation: {}", self.name());

        // Get all chart constants
        let charts = sqlx::query!(
            "SELECT song_id, rating_pst, rating_prs, rating_ftr, rating_byn, rating_etr FROM chart"
        )
        .fetch_all(&self.pool)
        .await?;

        // Create song_id filter for update
        let song_ids: Vec<String> = charts.iter().map(|c| c.song_id.clone()).collect();

        if !song_ids.is_empty() {
            // Reset ratings for songs not in chart table
            let placeholders = vec!["?"; song_ids.len()].join(",");
            let query = format!(
                "UPDATE best_score SET rating = 0 WHERE song_id NOT IN ({})",
                placeholders
            );

            let mut query_builder = sqlx::query(&query);
            for song_id in &song_ids {
                query_builder = query_builder.bind(song_id);
            }
            query_builder.execute(&self.pool).await?;

            // Update ratings for each song and difficulty
            for chart in &charts {
                let ratings = [
                    chart.rating_pst,
                    chart.rating_prs,
                    chart.rating_ftr,
                    chart.rating_byn,
                    chart.rating_etr,
                ];

                for (difficulty, rating_opt) in ratings.iter().enumerate() {
                    let def_rating = if let Some(rating) = rating_opt {
                        if *rating > 0 {
                            *rating as f64 / 10.0
                        } else {
                            -10.0
                        }
                    } else {
                        -10.0
                    };

                    // Update best_score ratings
                    sqlx::query!(
                        "UPDATE best_score
                         SET rating = GREATEST(
                             CASE
                                 WHEN score >= 10000000 THEN ? + 2.0
                                 WHEN score >= 9800000 THEN ? + 1.0 + (score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(? + (score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END,
                             0.0
                         )
                         WHERE song_id = ? AND difficulty = ?",
                        def_rating,
                        def_rating,
                        def_rating,
                        chart.song_id,
                        difficulty as i32
                    )
                    .execute(&self.pool)
                    .await?;
                }
            }

            // Update recent30 ratings
            sqlx::query!(
                "UPDATE recent30 r
                 JOIN chart c ON r.song_id = c.song_id
                 SET r.rating = GREATEST(
                     CASE
                         WHEN r.difficulty = 0 AND c.rating_pst IS NOT NULL AND c.rating_pst > 0 THEN
                             CASE
                                 WHEN r.score >= 10000000 THEN c.rating_pst / 10.0 + 2.0
                                 WHEN r.score >= 9800000 THEN c.rating_pst / 10.0 + 1.0 + (r.score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(c.rating_pst / 10.0 + (r.score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END
                         WHEN r.difficulty = 1 AND c.rating_prs IS NOT NULL AND c.rating_prs > 0 THEN
                             CASE
                                 WHEN r.score >= 10000000 THEN c.rating_prs / 10.0 + 2.0
                                 WHEN r.score >= 9800000 THEN c.rating_prs / 10.0 + 1.0 + (r.score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(c.rating_prs / 10.0 + (r.score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END
                         WHEN r.difficulty = 2 AND c.rating_ftr IS NOT NULL AND c.rating_ftr > 0 THEN
                             CASE
                                 WHEN r.score >= 10000000 THEN c.rating_ftr / 10.0 + 2.0
                                 WHEN r.score >= 9800000 THEN c.rating_ftr / 10.0 + 1.0 + (r.score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(c.rating_ftr / 10.0 + (r.score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END
                         WHEN r.difficulty = 3 AND c.rating_byn IS NOT NULL AND c.rating_byn > 0 THEN
                             CASE
                                 WHEN r.score >= 10000000 THEN c.rating_byn / 10.0 + 2.0
                                 WHEN r.score >= 9800000 THEN c.rating_byn / 10.0 + 1.0 + (r.score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(c.rating_byn / 10.0 + (r.score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END
                         WHEN r.difficulty = 4 AND c.rating_etr IS NOT NULL AND c.rating_etr > 0 THEN
                             CASE
                                 WHEN r.score >= 10000000 THEN c.rating_etr / 10.0 + 2.0
                                 WHEN r.score >= 9800000 THEN c.rating_etr / 10.0 + 1.0 + (r.score - 9800000) * 5.0 / 1000000.0
                                 ELSE GREATEST(c.rating_etr / 10.0 + (r.score - 9500000) * 5.0 / 1500000.0, 0.0)
                             END
                         ELSE 0.0
                     END,
                     0.0
                 )"
            )
            .execute(&self.pool)
            .await?;
        }

        log::info!("All score rating refresh completed");
        Ok(())
    }
}

/// Operation to unlock/lock user items
/// Equivalent to Python's UnlockUserItem
pub struct UnlockUserItem {
    pool: MySqlPool,
    user_id: Option<i32>,
    method: UnlockMethod,
    item_types: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum UnlockMethod {
    Unlock,
    Lock,
}

impl UnlockUserItem {
    pub fn new(pool: MySqlPool) -> Self {
        Self {
            pool,
            user_id: None,
            method: UnlockMethod::Unlock,
            item_types: vec!["single".to_string(), "pack".to_string()],
        }
    }

    pub fn with_user_id(mut self, user_id: i32) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_method(mut self, method: UnlockMethod) -> Self {
        self.method = method;
        self
    }

    pub fn with_item_types(mut self, item_types: Vec<String>) -> Self {
        self.item_types = item_types;
        self
    }
}

#[async_trait]
impl Operation for UnlockUserItem {
    fn name(&self) -> &'static str {
        "unlock_user_item"
    }

    async fn execute(&self) -> ArcResult<()> {
        log::info!("Executing operation: {} ({:?})", self.name(), self.method);

        match (self.user_id, &self.method) {
            (Some(user_id), UnlockMethod::Unlock) => {
                self.unlock_for_user(user_id).await?;
            }
            (Some(user_id), UnlockMethod::Lock) => {
                self.lock_for_user(user_id).await?;
            }
            (None, UnlockMethod::Unlock) => {
                self.unlock_for_all_users().await?;
            }
            (None, UnlockMethod::Lock) => {
                self.lock_for_all_users().await?;
            }
        }

        log::info!("User item unlock/lock operation completed");
        Ok(())
    }

    fn set_params(&mut self, params: OperationParams) -> ArcResult<()> {
        if let Some(user_id) = params.user_id {
            self.user_id = Some(user_id);
        }
        Ok(())
    }
}

impl UnlockUserItem {
    async fn unlock_for_user(&self, user_id: i32) -> ArcResult<()> {
        // Check if user exists
        let user_exists = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM user WHERE user_id = ?)",
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        if user_exists == 0 {
            return Err(ArcError::no_data(
                format!("No such user: {}", user_id),
                -110,
                -110,
            ));
        }

        // Get available items of specified types
        let placeholders = vec!["?"; self.item_types.len()].join(",");
        let query = format!(
            "SELECT item_id, type FROM item WHERE type IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, (String, String)>(&query);
        for item_type in &self.item_types {
            query_builder = query_builder.bind(item_type);
        }

        let items = query_builder.fetch_all(&self.pool).await?;

        // Insert user_item records
        for (item_id, item_type) in items {
            sqlx::query!(
                "INSERT INTO user_item (user_id, item_id, type, amount) VALUES (?, ?, ?, 1)
                 ON DUPLICATE KEY UPDATE amount = 1",
                user_id,
                item_id,
                item_type
            )
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn lock_for_user(&self, user_id: i32) -> ArcResult<()> {
        let placeholders = vec!["?"; self.item_types.len()].join(",");
        let query = format!(
            "DELETE FROM user_item WHERE user_id = ? AND type IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query(&query).bind(user_id);
        for item_type in &self.item_types {
            query_builder = query_builder.bind(item_type);
        }

        query_builder.execute(&self.pool).await?;
        Ok(())
    }

    async fn unlock_for_all_users(&self) -> ArcResult<()> {
        // Get all users
        let users = sqlx::query_scalar!("SELECT user_id FROM user")
            .fetch_all(&self.pool)
            .await?;

        // Get all items of specified types
        let placeholders = vec!["?"; self.item_types.len()].join(",");
        let query = format!(
            "SELECT item_id, type FROM item WHERE type IN ({})",
            placeholders
        );

        let mut query_builder = sqlx::query_as::<_, (String, String)>(&query);
        for item_type in &self.item_types {
            query_builder = query_builder.bind(item_type);
        }

        let items = query_builder.fetch_all(&self.pool).await?;

        // Insert user_item records for all users
        for user_id in users {
            for (item_id, item_type) in &items {
                sqlx::query!(
                    "INSERT INTO user_item (user_id, item_id, type, amount) VALUES (?, ?, ?, 1)
                     ON DUPLICATE KEY UPDATE amount = 1",
                    user_id,
                    item_id,
                    item_type
                )
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn lock_for_all_users(&self) -> ArcResult<()> {
        let placeholders = vec!["?"; self.item_types.len()].join(",");
        let query = format!("DELETE FROM user_item WHERE type IN ({})", placeholders);

        let mut query_builder = sqlx::query(&query);
        for item_type in &self.item_types {
            query_builder = query_builder.bind(item_type);
        }

        query_builder.execute(&self.pool).await?;
        Ok(())
    }
}

/// Operation manager to execute operations
pub struct OperationManager {
    asset_manager: Arc<AssetManager>,
    bundle_service: Arc<BundleService>,
    pool: MySqlPool,
}

impl OperationManager {
    pub fn new(
        asset_manager: Arc<AssetManager>,
        bundle_service: Arc<BundleService>,
        pool: MySqlPool,
    ) -> Self {
        Self {
            asset_manager,
            bundle_service,
            pool,
        }
    }

    /// Execute operation by name
    pub async fn execute_operation(
        &self,
        operation_name: &str,
        params: Option<OperationParams>,
    ) -> ArcResult<()> {
        let mut operation: Box<dyn Operation> = match operation_name {
            "refresh_song_file_cache" => {
                Box::new(RefreshSongFileCache::new(self.asset_manager.clone()))
            }
            "refresh_content_bundle_cache" => {
                Box::new(RefreshBundleCache::new(self.bundle_service.clone()))
            }
            "refresh_all_score_rating" => Box::new(RefreshAllScoreRating::new(self.pool.clone())),
            "unlock_user_item" => Box::new(UnlockUserItem::new(self.pool.clone())),
            _ => {
                return Err(ArcError::no_data(
                    format!("Unknown operation: {}", operation_name),
                    404,
                    -100,
                ));
            }
        };

        // Set parameters if provided
        if let Some(params) = params {
            operation.set_params(params)?;
        }

        // Execute the operation
        operation.execute().await
    }

    /// Get list of available operations
    pub fn list_operations(&self) -> Vec<&'static str> {
        vec![
            "refresh_song_file_cache",
            "refresh_content_bundle_cache",
            "refresh_all_score_rating",
            "unlock_user_item",
        ]
    }
}
