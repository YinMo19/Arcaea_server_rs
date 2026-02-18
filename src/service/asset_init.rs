//! Asset initialization service for database seeding
//!
//! This module handles the initialization of game data including characters,
//! items, courses, roles, and default admin account. It replicates the
//! functionality of the Python DatabaseInit class.

use crate::error::{ArcError, ArcResult};
use crate::service::arc_data::{arc_data_file_path_from_env, load_arc_data_from_file, ArcData};
use crate::service::runtime_assets::asset_path;
use crate::utils::current_timestamp_ms;
use crate::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::query;

/// Asset initialization service
pub struct AssetInitService {
    pool: DbPool,
}

/// Purchase item data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseItem {
    pub name: String,
    pub price: i32,
    pub orig_price: i32,
    pub discount_from: Option<i64>,
    pub discount_to: Option<i64>,
    pub discount_reason: Option<String>,
    pub items: Vec<PurchaseItemDetail>,
}

/// Purchase item detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseItemDetail {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: String,
    #[serde(default = "default_amount")]
    pub amount: Option<i32>,
    pub is_available: Option<bool>,
}

fn default_amount() -> Option<i32> {
    Some(1)
}

/// Course data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseData {
    pub course_id: String,
    pub course_name: String,
    pub dan_name: Option<String>,
    pub style: i32,
    pub gauge_requirement: String,
    pub flag_as_hidden_when_requirements_not_met: bool,
    pub can_start: bool,
    pub songs: Vec<CourseSong>,
    pub requirements: Vec<Requirements>,
    // pub items: Vec<PurchaseItemDetail>,
    pub is_completed: bool,
    pub high_score: i32,
    pub best_clear_type: i32,
    pub rewards: Vec<String>,
}

/// requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    pub value: String,
    pub r#type: String,
}

/// Course song data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseSong {
    pub id: String,
    pub difficulty: i32,
    pub flag_as_hidden: bool,
}

impl AssetInitService {
    /// Create a new asset initialization service
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Initialize all game assets
    pub async fn initialize_all(&self) -> ArcResult<()> {
        log::info!("Starting asset initialization...");
        let arc_data_path = arc_data_file_path_from_env();
        log::info!("Loading arc_data from `{}`...", arc_data_path);
        let arc_data = load_arc_data_from_file(&arc_data_path)?;

        // Initialize in order of dependencies
        self.initialize_characters(&arc_data).await?;
        self.initialize_character_cores(&arc_data).await?;
        self.initialize_items(&arc_data).await?;
        self.initialize_packs().await?;
        self.initialize_singles().await?;
        self.initialize_courses().await?;
        self.initialize_roles_and_powers().await?;
        self.initialize_admin_account(&arc_data).await?;

        log::info!("Asset initialization completed successfully");
        Ok(())
    }

    /// Sync packs and singles from runtime assets into purchase-related tables.
    ///
    /// This is safe to run on every startup: rows are upserted.
    pub async fn sync_purchases_from_assets(&self) -> ArcResult<(usize, usize)> {
        let packs = Self::load_purchase_assets("packs.json")?;
        let singles = Self::load_purchase_assets("singles.json")?;

        for pack in &packs {
            self.insert_purchase_item(pack).await?;
        }
        for single in &singles {
            self.insert_purchase_item(single).await?;
        }

        Ok((packs.len(), singles.len()))
    }

    fn load_purchase_assets(file_name: &str) -> ArcResult<Vec<PurchaseItem>> {
        let path = asset_path(file_name);
        let data = std::fs::read_to_string(&path)
            .map_err(|e| ArcError::input(format!("Failed to read {}: {e}", path.display())))?;
        serde_json::from_str(&data)
            .map_err(|e| ArcError::input(format!("Failed to parse {file_name}: {e}")))
    }

    /// Initialize character data
    async fn initialize_characters(&self, arc_data: &ArcData) -> ArcResult<()> {
        log::info!("Initializing characters...");

        for character in &arc_data.characters {
            query!(
                r#"
                INSERT INTO `character` (
                    character_id, name, max_level, frag1, prog1, overdrive1,
                    frag20, prog20, overdrive20, frag30, prog30, overdrive30,
                    skill_id, skill_unlock_level, skill_requires_uncap,
                    skill_id_uncap, char_type, is_uncapped
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                character.character_id,
                character.name,
                character.max_level,
                character.frag1,
                character.prog1,
                character.overdrive1,
                character.frag20,
                character.prog20,
                character.overdrive20,
                character.frag30,
                character.prog30,
                character.overdrive30,
                character.skill_id,
                character.skill_unlock_level,
                character.skill_requires_uncap,
                character.skill_id_uncap,
                character.char_type,
                character.is_uncapped
            )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                ArcError::input(format!(
                    "Failed to insert character {}: {e}",
                    character.character_id
                ))
            })?;
        }

        log::info!("Characters initialized successfully");
        Ok(())
    }

    /// Initialize character cores
    async fn initialize_character_cores(&self, arc_data: &ArcData) -> ArcResult<()> {
        log::info!("Initializing character cores...");

        for core in &arc_data.character_cores {
            query!(
                "INSERT INTO char_item (character_id, item_id, type, amount) VALUES (?, ?, ?, ?)",
                core.character_id,
                core.item_id,
                core.item_type,
                core.amount
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert character core: {e}")))?;
        }

        log::info!("Character cores initialized successfully");
        Ok(())
    }

    /// Initialize game items
    async fn initialize_items(&self, arc_data: &ArcData) -> ArcResult<()> {
        log::info!("Initializing items...");

        // Initialize cores
        for core in &arc_data.cores {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'core', 1)",
                core
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert core item: {e}")))?;
        }

        // Initialize world songs
        for song in &arc_data.world_songs {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'world_song', 1)",
                song
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert world song: {e}")))?;
        }

        // Initialize world unlocks
        for unlock in &arc_data.world_unlocks {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'world_unlock', 1)",
                unlock
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert world unlock: {e}")))?;
        }

        // Initialize course banners
        for banner in &arc_data.course_banners {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'course_banner', 1)",
                banner
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert course banner: {e}")))?;
        }

        // Initialize basic items
        let basic_items = [
            ("fragment", "fragment"),
            ("memory", "memory"),
            ("anni5tix", "anni5tix"),
            ("pick_ticket", "pick_ticket"),
            ("innocence", "world_song"), // 新手任务奖励曲
        ];

        for (item_id, item_type) in basic_items {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, ?, 1)",
                item_id,
                item_type
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert basic item: {e}")))?;
        }

        log::info!("Items initialized successfully");
        Ok(())
    }

    /// Initialize pack purchases
    async fn initialize_packs(&self) -> ArcResult<()> {
        log::info!("Initializing packs...");

        let packs = Self::load_purchase_assets("packs.json")?;

        for pack in &packs {
            self.insert_purchase_item(pack).await?;
        }

        log::info!("Packs initialized successfully");
        Ok(())
    }

    /// Initialize single purchases
    async fn initialize_singles(&self) -> ArcResult<()> {
        log::info!("Initializing singles...");

        let singles = Self::load_purchase_assets("singles.json")?;

        for single in &singles {
            self.insert_purchase_item(single).await?;
        }

        log::info!("Singles initialized successfully");
        Ok(())
    }

    /// Initialize courses
    async fn initialize_courses(&self) -> ArcResult<()> {
        log::info!("Initializing courses...");

        let courses_path = asset_path("courses.json");
        let courses_data = std::fs::read_to_string(&courses_path).map_err(|e| {
            ArcError::input(format!("Failed to read {}: {e}", courses_path.display()))
        })?;
        let courses: Vec<CourseData> = serde_json::from_str(&courses_data)
            .map_err(|e| ArcError::input(format!("Failed to parse courses.json: {e}")))?;

        for course in courses {
            self.insert_course(course).await?;
        }

        log::info!("Courses initialized successfully");
        Ok(())
    }

    /// Initialize roles and powers
    async fn initialize_roles_and_powers(&self) -> ArcResult<()> {
        log::info!("Initializing roles and powers...");

        let roles = Self::get_roles();
        let powers = Self::get_powers();
        let role_powers = Self::get_role_powers();

        // Insert roles
        for (role_id, caption) in roles {
            query!(
                "INSERT INTO role (role_id, caption) VALUES (?, ?)",
                role_id,
                caption
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert role: {e}")))?;
        }

        // Insert powers
        for (power_id, caption) in powers {
            query!(
                "INSERT INTO power (power_id, caption) VALUES (?, ?)",
                power_id,
                caption
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert power: {e}")))?;
        }

        // Insert role-power relationships
        for (role_id, power_list) in role_powers {
            for power_id in power_list {
                query!(
                    "INSERT INTO role_power (role_id, power_id) VALUES (?, ?)",
                    role_id,
                    power_id
                )
                .execute(&self.pool)
                .await
                .map_err(|e| ArcError::input(format!("Failed to insert role power: {e}")))?;
            }
        }

        log::info!("Roles and powers initialized successfully");
        Ok(())
    }

    /// Initialize admin account
    async fn initialize_admin_account(&self, arc_data: &ArcData) -> ArcResult<()> {
        log::info!("Initializing admin account...");

        let user_id = 2000000i32;
        let user_code = "123456789";
        let name = "admin";
        let email = "admin@admin.com";
        let password_hash = "8c6976e5b5410415bde908bd4dee15dfb167a9c873fc4bb8a81f6f2ab448a918"; // admin
        let now = current_timestamp_ms();
        let memories = 114514i32;

        // Insert admin user
        query!(
            r#"
            INSERT INTO user (
                user_id, name, password, join_date, user_code, rating_ptt,
                character_id, is_skill_sealed, is_char_uncapped, is_char_uncapped_override,
                is_hide_rating, favorite_character, max_stamina_notification_enabled,
                current_map, ticket, prog_boost, email
            ) VALUES (?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, -1, 0, '', ?, 0, ?)
            "#,
            user_id,
            name,
            password_hash,
            now,
            user_code,
            memories,
            email
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to insert admin user: {e}")))?;

        // Insert user characters for admin
        for character in &arc_data.characters {
            query!(
                r#"
                INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
                VALUES (?, ?, 1, 0, 0, 0, 0)
                "#,
                user_id,
                character.character_id
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert user character: {e}")))?;
        }

        // Assign admin role
        query!(
            "INSERT INTO user_role (user_id, role_id) VALUES (?, 'admin')",
            user_id
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to assign admin role: {e}")))?;

        log::info!("Admin account initialized successfully");
        Ok(())
    }

    /// Insert a purchase item with its details
    async fn insert_purchase_item(&self, purchase: &PurchaseItem) -> ArcResult<()> {
        // Insert purchase
        query!(
            r#"
            INSERT INTO purchase (purchase_name, price, orig_price, discount_from, discount_to, discount_reason)
            VALUES (?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                price = VALUES(price),
                orig_price = VALUES(orig_price),
                discount_from = VALUES(discount_from),
                discount_to = VALUES(discount_to),
                discount_reason = VALUES(discount_reason)
            "#,
            purchase.name,
            purchase.price,
            purchase.orig_price,
            purchase.discount_from,
            purchase.discount_to,
            purchase.discount_reason.as_deref()
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to insert purchase: {e}")))?;

        // Insert purchase items
        for item in &purchase.items {
            let amount = item.amount.unwrap_or(1);
            let is_available = if item.is_available.unwrap_or(true) {
                1
            } else {
                0
            };

            query!(
                "INSERT INTO purchase_item (purchase_name, item_id, type, amount) VALUES (?, ?, ?, ?)
                 ON DUPLICATE KEY UPDATE amount = VALUES(amount)",
                purchase.name,
                item.id,
                item.item_type,
                amount
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert purchase item: {e}")))?;

            // Python parity: purchasable entries should also exist in `item`.
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, ?, ?)
                 ON DUPLICATE KEY UPDATE is_available = VALUES(is_available)",
                item.id,
                item.item_type,
                is_available
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert purchasable item: {e}")))?;
        }

        Ok(())
    }

    /// Insert a course with its details
    async fn insert_course(&self, course: CourseData) -> ArcResult<()> {
        // Insert course
        query!(
            r#"
            INSERT INTO course (course_id, course_name, dan_name, style, gauge_requirement,
                              flag_as_hidden_when_requirements_not_met, can_start)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            course.course_id,
            course.course_name,
            course.dan_name,
            course.style,
            course.gauge_requirement,
            course.flag_as_hidden_when_requirements_not_met,
            course.can_start
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to insert course: {e}")))?;

        // Insert course charts
        for (index, chart) in course.songs.iter().enumerate() {
            query!(
                r#"
                INSERT INTO course_chart (course_id, song_id, difficulty, flag_as_hidden, song_index)
                VALUES (?, ?, ?, ?, ?)
                "#,
                course.course_id,
                chart.id,
                chart.difficulty,
                chart.flag_as_hidden,
                index as i32,
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert course chart: {e}")))?;
        }

        // Insert course requirements
        for requirement in course.requirements {
            query!(
                "INSERT INTO course_requirement (course_id, required_id) VALUES (?, ?)",
                course.course_id,
                requirement.value
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert course requirement: {e}")))?;
        }

        // Insert course items
        for reward in course.rewards {
            let (amount, item_id, item_type) =
                if let Some(fragment_str) = reward.strip_prefix("fragment") {
                    (
                        fragment_str.parse().unwrap_or(1),
                        String::from("fragment"),
                        String::from("fragment"),
                    )
                } else if reward.starts_with("course_banner") {
                    (1, reward, String::from("course_banner"))
                } else if let Some(reward) = reward.strip_prefix("core_generic_") {
                    (
                        reward.parse().unwrap_or(1),
                        String::from("core_generic"),
                        String::from("core"),
                    )
                } else {
                    // unreachable!
                    panic!("Unknown reward type: {reward}");
                };
            query!(
                "INSERT INTO course_item (course_id, item_id, type, amount) VALUES (?, ?, ?, ?)",
                course.course_id,
                item_id,
                item_type,
                amount
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert course item: {e}")))?;
        }

        Ok(())
    }

    /// Get roles data
    fn get_roles() -> Vec<(&'static str, &'static str)> {
        vec![
            ("system", "系统"),
            ("admin", "管理员"),
            ("user", "用户"),
            ("selecter", "查询接口"),
        ]
    }

    /// Get powers data
    fn get_powers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("system", "系统权限"),
            ("select", "总体查询权限"),
            ("select_me", "自我查询权限"),
            ("change", "总体修改权限"),
            ("change_me", "自我修改权限"),
            ("grant", "授权权限"),
            ("grant_inf", "下级授权权限"),
            ("select_song_rank", "歌曲排行榜查询权限"),
            ("select_song_info", "歌曲信息查询权限"),
            ("select_song_rank_top", "歌曲排行榜有限查询权限"),
        ]
    }

    /// Get role-power mappings
    fn get_role_powers() -> Vec<(&'static str, Vec<&'static str>)> {
        vec![
            (
                "system",
                vec![
                    "system",
                    "select",
                    "select_me",
                    "change",
                    "change_me",
                    "grant",
                    "grant_inf",
                    "select_song_rank",
                    "select_song_info",
                    "select_song_rank_top",
                ],
            ),
            (
                "admin",
                vec![
                    "select",
                    "select_me",
                    "change_me",
                    "grant_inf",
                    "select_song_rank_top",
                    "select_song_info",
                ],
            ),
            (
                "user",
                vec![
                    "select_me",
                    "change_me",
                    "select_song_rank",
                    "select_song_info",
                ],
            ),
            ("selecter", vec!["select"]),
        ]
    }
}
