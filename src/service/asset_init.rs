//! Asset initialization service for database seeding
//!
//! This module handles the initialization of game data including characters,
//! items, courses, roles, and default admin account. It replicates the
//! functionality of the Python DatabaseInit class.

use crate::error::{ArcError, ArcResult};
use crate::utils::current_timestamp_ms;
use crate::DbPool;
use serde::{Deserialize, Serialize};
use sqlx::query;
use std::collections::HashMap;

/// Asset initialization service
pub struct AssetInitService {
    pool: DbPool,
}

/// Character core item data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterCoreItem {
    pub core_id: String,
    pub amount: i32,
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

/// Character statistics at different levels
#[derive(Debug, Clone)]
pub struct CharacterStats {
    pub frag1: f64,
    pub prog1: f64,
    pub overdrive1: f64,
    pub frag20: f64,
    pub prog20: f64,
    pub overdrive20: f64,
    pub frag30: f64,
    pub prog30: f64,
    pub overdrive30: f64,
    pub skill_id: String,
    pub skill_unlock_level: i32,
    pub skill_id_uncap: String,
    pub char_type: i32,
}

impl AssetInitService {
    /// Create a new asset initialization service
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Initialize all game assets
    pub async fn initialize_all(&self) -> ArcResult<()> {
        log::info!("Starting asset initialization...");

        // Initialize in order of dependencies
        self.initialize_characters().await?;
        self.initialize_character_cores().await?;
        self.initialize_items().await?;
        self.initialize_packs().await?;
        self.initialize_singles().await?;
        self.initialize_courses().await?;
        self.initialize_roles_and_powers().await?;
        self.initialize_admin_account().await?;

        log::info!("Asset initialization completed successfully");
        Ok(())
    }

    /// Initialize character data
    async fn initialize_characters(&self) -> ArcResult<()> {
        log::info!("Initializing characters...");

        let characters = Self::get_character_data();

        for (i, (name, stats)) in characters.iter().enumerate() {
            let character_id = i as i32;
            let uncapped_characters = Self::get_uncapped_character_ids();
            let skill_requires_uncap = if character_id == 2 { 1 } else { 0 };
            let (max_level, is_uncapped) = if uncapped_characters.contains(&character_id) {
                (30, 1)
            } else {
                (20, 0)
            };

            query!(
                r#"
                INSERT INTO `character` (
                    character_id, name, max_level, frag1, prog1, overdrive1,
                    frag20, prog20, overdrive20, frag30, prog30, overdrive30,
                    skill_id, skill_unlock_level, skill_requires_uncap,
                    skill_id_uncap, char_type, is_uncapped
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                character_id,
                name,
                max_level,
                stats.frag1,
                stats.prog1,
                stats.overdrive1,
                stats.frag20,
                stats.prog20,
                stats.overdrive20,
                stats.frag30,
                stats.prog30,
                stats.overdrive30,
                stats.skill_id,
                stats.skill_unlock_level,
                skill_requires_uncap,
                stats.skill_id_uncap,
                stats.char_type,
                is_uncapped
            )
            .execute(&self.pool)
            .await
            .map_err(|e| {
                ArcError::input(format!("Failed to insert character {character_id}: {e}"))
            })?;
        }

        // Insert special shirahime character
        query!(
            r#"
            INSERT INTO `character` (
                character_id, name, max_level, frag1, prog1, overdrive1,
                frag20, prog20, overdrive20, frag30, prog30, overdrive30,
                skill_id, skill_unlock_level, skill_requires_uncap,
                skill_id_uncap, char_type, is_uncapped
            ) VALUES (99, 'shirahime', 20, 38, 33, 28, 66, 58, 50, 66, 58, 50, 'frags_preferred_song', 0, 0, '', 0, 0)
            "#
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to insert shirahime character: {e}")))?;

        log::info!("Characters initialized successfully");
        Ok(())
    }

    /// Initialize character cores
    async fn initialize_character_cores(&self) -> ArcResult<()> {
        log::info!("Initializing character cores...");

        let char_cores = Self::get_character_cores();

        for (character_id, cores) in char_cores {
            for core in cores {
                query!(
                    "INSERT INTO char_item (character_id, item_id, type, amount) VALUES (?, ?, 'core', ?)",
                    character_id,
                    core.core_id,
                    core.amount
                )
                .execute(&self.pool)
                .await
                .map_err(|e| ArcError::input(format!("Failed to insert character core: {e}")))?;
            }
        }

        log::info!("Character cores initialized successfully");
        Ok(())
    }

    /// Initialize game items
    async fn initialize_items(&self) -> ArcResult<()> {
        log::info!("Initializing items...");

        // Initialize cores
        let cores = Self::get_cores();
        for core in cores {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'core', 1)",
                core
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert core item: {e}")))?;
        }

        // Initialize world songs
        let world_songs = Self::get_world_songs();
        for song in world_songs {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'world_song', 1)",
                song
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert world song: {e}")))?;
        }

        // Initialize world unlocks
        let world_unlocks = Self::get_world_unlocks();
        for unlock in world_unlocks {
            query!(
                "INSERT INTO item (item_id, type, is_available) VALUES (?, 'world_unlock', 1)",
                unlock
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert world unlock: {e}")))?;
        }

        // Initialize course banners
        let course_banners = Self::get_course_banners();
        for banner in course_banners {
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

        let packs_data = include_str!("../assets/packs.json");
        let packs: Vec<PurchaseItem> = serde_json::from_str(packs_data)
            .map_err(|e| ArcError::input(format!("Failed to parse packs.json: {e}")))?;

        for pack in packs {
            self.insert_purchase_item(pack).await?;
        }

        log::info!("Packs initialized successfully");
        Ok(())
    }

    /// Initialize single purchases
    async fn initialize_singles(&self) -> ArcResult<()> {
        log::info!("Initializing singles...");

        let singles_data = include_str!("../assets/singles.json");
        let singles: Vec<PurchaseItem> = serde_json::from_str(singles_data)
            .map_err(|e| ArcError::input(format!("Failed to parse singles.json: {e}")))?;

        for single in singles {
            self.insert_purchase_item(single).await?;
        }

        log::info!("Singles initialized successfully");
        Ok(())
    }

    /// Initialize courses
    async fn initialize_courses(&self) -> ArcResult<()> {
        log::info!("Initializing courses...");

        let courses_data = include_str!("../assets/courses.json");
        let courses: Vec<CourseData> = serde_json::from_str(courses_data)
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
    async fn initialize_admin_account(&self) -> ArcResult<()> {
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
        for character_id in 0..90 {
            query!(
                r#"
                INSERT INTO user_char (user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag)
                VALUES (?, ?, 1, 0, 0, 0, 0)
                "#,
                user_id,
                character_id
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
    async fn insert_purchase_item(&self, purchase: PurchaseItem) -> ArcResult<()> {
        // Insert purchase
        query!(
            r#"
            INSERT INTO purchase (purchase_name, price, orig_price, discount_from, discount_to, discount_reason)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
            purchase.name,
            purchase.price,
            purchase.orig_price,
            purchase.discount_from,
            purchase.discount_to,
            purchase.discount_reason
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ArcError::input(format!("Failed to insert purchase: {e}")))?;

        // Insert purchase items
        for item in purchase.items {
            query!(
                "INSERT INTO purchase_item (purchase_name, item_id, type, amount) VALUES (?, ?, ?, ?)",
                purchase.name,
                item.id,
                item.item_type,
                item.amount.unwrap_or(1)
            )
            .execute(&self.pool)
            .await
            .map_err(|e| ArcError::input(format!("Failed to insert purchase item: {e}")))?;
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

    /// Get character static data
    fn get_character_data() -> Vec<(&'static str, CharacterStats)> {
        Self::get_all_character_data()
    }

    /// Get all character data based on Python arc_data.py
    fn get_all_character_data() -> Vec<(&'static str, CharacterStats)> {
        let char_names = [
            "hikari",
            "tairitsu",
            "kou",
            "sapphire",
            "lethe",
            "hikari&tairitsu(reunion)",
            "Tairitsu(Axium)",
            "Tairitsu(Grievous Lady)",
            "stella",
            "Hikari & Fisica",
            "ilith",
            "eto",
            "luna",
            "shirabe",
            "Hikari(Zero)",
            "Hikari(Fracture)",
            "Hikari(Summer)",
            "Tairitsu(Summer)",
            "Tairitsu & Trin",
            "ayu",
            "Eto & Luna",
            "yume",
            "Seine & Hikari",
            "saya",
            "Tairitsu & Chuni Penguin",
            "Chuni Penguin",
            "haruna",
            "nono",
            "MTA-XXX",
            "MDA-21",
            "kanae",
            "Hikari(Fantasia)",
            "Tairitsu(Sonata)",
            "sia",
            "DORO*C",
            "Tairitsu(Tempest)",
            "brillante",
            "Ilith(Summer)",
            "etude",
            "Alice & Tenniel",
            "Luna & Mia",
            "areus",
            "seele",
            "isabelle",
            "mir",
            "lagrange",
            "linka",
            "nami",
            "Saya & Elizabeth",
            "lily",
            "kanae(midsummer)",
            "alice&tenniel(minuet)",
            "tairitsu(elegy)",
            "marija",
            "vita",
            "hikari(fatalis)",
            "saki",
            "setsuna",
            "amane",
            "kou(winter)",
            "lagrange(aria)",
            "lethe(apophenia)",
            "shama(UNiVERSE)",
            "milk(UNiVERSE)",
            "shikoku",
            "mika yurisaki",
            "Mithra Tercera",
            "Toa Kozukata",
            "Nami(Twilight)",
            "Ilith & Ivy",
            "Hikari & Vanessa",
            "Maya",
            "Insight(Ascendant - 8th Seeker)",
            "Luin",
            "Vita(Cadenza)",
            "Ai-chan",
            "Luna & Ilot",
            "Eto & Hoppe",
            "Forlorn(Ascendant - 6th Seeker)",
            "Chinatsu",
            "Tsumugi",
            "Nai",
            "Selene Sheryl (MIR-203)",
            "Salt",
            "Acid",
            "Hikari & Selene Sheryl (Fracture & MIR-203)",
            "Hikari & El Clear",
            "Tairitsu & El Fail",
            "Nami & Sui (Twilight)",
            "Nonoka",
        ];

        let skill_ids = [
            "gauge_easy",
            "",
            "",
            "",
            "note_mirror",
            "skill_reunion",
            "",
            "gauge_hard",
            "frag_plus_10_pack_stellights",
            "gauge_easy|frag_plus_15_pst&prs",
            "gauge_hard|fail_frag_minus_100",
            "frag_plus_5_side_light",
            "visual_hide_hp",
            "frag_plus_5_side_conflict",
            "challenge_fullcombo_0gauge",
            "gauge_overflow",
            "gauge_easy|note_mirror",
            "note_mirror",
            "visual_tomato_pack_tonesphere",
            "frag_rng_ayu",
            "gaugestart_30|gaugegain_70",
            "combo_100-frag_1",
            "audio_gcemptyhit_pack_groovecoaster",
            "gauge_saya",
            "gauge_chuni",
            "kantandeshou",
            "gauge_haruna",
            "frags_nono",
            "gauge_pandora",
            "gauge_regulus",
            "omatsuri_daynight",
            "",
            "",
            "sometimes(note_mirror|frag_plus_5)",
            "scoreclear_aa|visual_scoregauge",
            "gauge_tempest",
            "gauge_hard",
            "gauge_ilith_summer",
            "",
            "note_mirror|visual_hide_far",
            "frags_ongeki",
            "gauge_areus",
            "gauge_seele",
            "gauge_isabelle",
            "gauge_exhaustion",
            "skill_lagrange",
            "gauge_safe_10",
            "frags_nami",
            "skill_elizabeth",
            "skill_lily",
            "skill_kanae_midsummer",
            "",
            "",
            "visual_ghost_skynotes",
            "skill_vita",
            "skill_fatalis",
            "frags_ongeki_slash",
            "frags_ongeki_hard",
            "skill_amane",
            "skill_kou_winter",
            "",
            "gauge_hard|note_mirror",
            "skill_shama",
            "skill_milk",
            "skill_shikoku",
            "skill_mika",
            "skill_mithra",
            "skill_toa",
            "skill_nami_twilight",
            "skill_ilith_ivy",
            "skill_hikari_vanessa",
            "skill_maya",
            "skill_intruder",
            "skill_luin",
            "",
            "skill_aichan",
            "skill_luna_ilot",
            "skill_eto_hoppe",
            "skill_nell",
            "skill_chinatsu",
            "skill_tsumugi",
            "skill_nai",
            "skill_selene",
            "skill_salt",
            "skill_acid",
            "skill_hikari_selene",
            "skill_hikari_clear",
            "skill_tairitsu_fail",
            "skill_nami_sui",
            "skill_nonoka",
        ];

        let skill_ids_uncap = [
            "",
            "",
            "frags_kou",
            "",
            "visual_ink",
            "",
            "",
            "",
            "",
            "",
            "ilith_awakened_skill",
            "eto_uncap",
            "luna_uncap",
            "shirabe_entry_fee",
            "",
            "",
            "",
            "",
            "",
            "ayu_uncap",
            "",
            "frags_yume",
            "",
            "skill_saya_uncap",
            "",
            "",
            "",
            "",
            "",
            "",
            "skill_kanae_uncap",
            "",
            "",
            "",
            "skill_doroc_uncap",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "skill_luin_uncap",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ];

        let skill_unlock_levels = [
            0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 8, 8, 8, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 14, 0, 0, 8,
            8, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 8, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0,
        ];

        let frag1 = [
            55.0, 55.0, 60.0, 50.0, 47.0, 79.0, 47.0, 57.0, 41.0, 22.0, 50.0, 54.0, 60.0, 56.0,
            78.0, 42.0, 41.0, 61.0, 52.0, 50.0, 52.0, 32.0, 42.0, 55.0, 45.0, 58.0, 43.0, 0.5,
            68.0, 50.0, 62.0, 45.0, 45.0, 52.0, 44.0, 27.0, 59.0, 0.0, 45.0, 50.0, 50.0, 47.0,
            47.0, 61.0, 43.0, 42.0, 38.0, 25.0, 58.0, 50.0, 61.0, 45.0, 45.0, 38.0, 34.0, 27.0,
            18.0, 56.0, 47.0, 30.0, 45.0, 57.0, 56.0, 47.0, 33.0, 26.0, 29.0, 66.0, 40.0, 33.0,
            51.0, 27.0, 50.0, 60.0, 45.0, 50.0, 38.0, 22.0, 63.0, 37.0, 23.0, 59.0, 45.0, 20.0,
            43.0, 50.0, 22.0, 37.0, 26.0, 47.0,
        ];
        let prog1 = [
            35.0, 55.0, 47.0, 50.0, 60.0, 70.0, 60.0, 70.0, 58.0, 45.0, 70.0, 45.0, 42.0, 46.0,
            61.0, 67.0, 49.0, 44.0, 28.0, 45.0, 24.0, 46.0, 52.0, 59.0, 62.0, 33.0, 58.0, 25.0,
            63.0, 69.0, 50.0, 45.0, 45.0, 51.0, 34.0, 70.0, 62.0, 70.0, 45.0, 32.0, 32.0, 61.0,
            47.0, 47.0, 37.0, 42.0, 50.0, 50.0, 45.0, 41.0, 61.0, 45.0, 45.0, 58.0, 50.0, 130.0,
            18.0, 57.0, 55.0, 50.0, 45.0, 70.0, 37.5, 29.0, 44.0, 26.0, 26.0, 35.0, 40.0, 33.0,
            58.0, 31.0, 40.0, 50.0, 45.0, 41.0, 12.0, 31.0, 72.0, 40.0, 16.0, 33.0, 35.0, 23.0,
            24.0, 46.0, 26.0, 49.0, 32.0, 35.0,
        ];
        let overdrive1 = [
            35.0, 55.0, 25.0, 50.0, 47.0, 70.0, 72.0, 57.0, 41.0, 7.0, 10.0, 32.0, 65.0, 31.0,
            61.0, 53.0, 31.0, 47.0, 38.0, 12.0, 39.0, 18.0, 48.0, 65.0, 45.0, 55.0, 44.0, 25.0,
            46.0, 44.0, 33.0, 45.0, 45.0, 37.0, 25.0, 27.0, 50.0, 20.0, 45.0, 63.0, 21.0, 47.0,
            61.0, 47.0, 65.0, 80.0, 38.0, 30.0, 49.0, 15.0, 34.0, 45.0, 45.0, 38.0, 67.0, 120.0,
            44.0, 33.0, 55.0, 50.0, 45.0, 57.0, 31.0, 29.0, 65.0, 26.0, 29.0, 42.5, 40.0, 33.0,
            58.0, 31.0, 35.0, 34.0, 45.0, 41.0, 12.0, 19.0, 38.0, 40.0, 26.0, 39.0, 56.0, 20.0,
            25.0, 46.0, 18.0, 71.0, 29.0, 25.0,
        ];
        let frag20 = [
            78.0, 80.0, 90.0, 75.0, 70.0, 79.0, 70.0, 79.0, 65.0, 40.0, 50.0, 80.0, 90.0, 82.0,
            0.0, 61.0, 67.0, 92.0, 85.0, 50.0, 86.0, 52.0, 65.0, 85.0, 67.0, 88.0, 64.0, 0.5, 95.0,
            70.0, 95.0, 50.0, 80.0, 87.0, 71.0, 50.0, 85.0, 0.0, 80.0, 75.0, 50.0, 70.0, 70.0,
            90.0, 65.0, 80.0, 61.0, 50.0, 68.0, 60.0, 90.0, 67.0, 50.0, 60.0, 51.0, 50.0, 35.0,
            85.0, 47.0, 50.0, 75.0, 80.0, 90.0, 80.0, 50.0, 51.0, 54.0, 100.0, 50.0, 58.0, 51.0,
            40.0, 115.0, 70.0, 50.0, 61.6, 48.0, 37.0, 90.0, 60.0, 50.0, 92.0, 66.0, 44.0, 79.0,
            50.0, 47.0, 55.0, 49.0, 79.0,
        ];
        let prog20 = [
            61.0, 80.0, 70.0, 75.0, 90.0, 70.0, 90.0, 102.0, 84.0, 78.0, 105.0, 67.0, 63.0, 68.0,
            0.0, 99.0, 80.0, 66.0, 46.0, 83.0, 40.0, 73.0, 80.0, 90.0, 93.0, 50.0, 86.0, 78.0,
            89.0, 98.0, 75.0, 80.0, 50.0, 64.0, 55.0, 100.0, 90.0, 110.0, 80.0, 50.0, 74.0, 90.0,
            70.0, 70.0, 56.0, 80.0, 79.0, 55.0, 65.0, 59.0, 90.0, 50.0, 90.0, 90.0, 75.0, 210.0,
            35.0, 86.0, 92.0, 80.0, 75.0, 100.0, 60.0, 50.0, 68.0, 51.0, 50.0, 53.0, 85.0, 58.0,
            96.0, 47.0, 80.0, 80.0, 67.0, 41.0, 55.0, 50.0, 103.0, 66.0, 35.0, 52.0, 65.0, 50.0,
            43.0, 84.0, 55.0, 73.0, 59.0, 60.0,
        ];
        let overdrive20 = [
            61.0, 80.0, 47.0, 75.0, 70.0, 70.0, 95.0, 79.0, 65.0, 31.0, 50.0, 59.0, 90.0, 58.0,
            0.0, 78.0, 50.0, 70.0, 62.0, 49.0, 64.0, 46.0, 73.0, 95.0, 67.0, 84.0, 70.0, 78.0,
            69.0, 70.0, 50.0, 80.0, 80.0, 63.0, 25.0, 50.0, 72.0, 55.0, 50.0, 95.0, 55.0, 70.0,
            90.0, 70.0, 99.0, 80.0, 61.0, 40.0, 69.0, 62.0, 51.0, 90.0, 67.0, 60.0, 100.0, 200.0,
            85.0, 50.0, 92.0, 50.0, 75.0, 80.0, 49.5, 50.0, 100.0, 51.0, 54.0, 65.5, 59.5, 58.0,
            96.0, 47.0, 75.0, 54.0, 90.0, 41.0, 34.0, 30.0, 55.0, 66.0, 55.0, 62.0, 81.0, 44.0,
            46.0, 84.0, 39.0, 105.0, 55.0, 43.0,
        ];
        let frag30 = [
            88.0, 90.0, 100.0, 75.0, 80.0, 89.0, 70.0, 79.0, 65.0, 40.0, 50.0, 90.0, 100.0, 92.0,
            0.0, 61.0, 67.0, 92.0, 85.0, 50.0, 86.0, 62.0, 65.0, 95.0, 67.0, 88.0, 74.0, 0.5,
            105.0, 80.0, 105.0, 50.0, 80.0, 87.0, 81.0, 50.0, 95.0, 0.0, 80.0, 75.0, 50.0, 70.0,
            80.0, 100.0, 65.0, 80.0, 61.0, 50.0, 68.0, 60.0, 90.0, 67.0, 50.0, 60.0, 51.0, 50.0,
            35.0, 85.0, 47.0, 50.0, 75.0, 80.0, 90.0, 80.0, 50.0, 51.0, 64.0, 100.0, 50.0, 58.0,
            51.0, 40.0, 115.0, 80.0, 50.0, 61.6, 48.0, 37.0, 90.0, 60.0, 50.0, 102.0, 76.0, 44.0,
            89.0, 50.0, 47.0, 55.0, 49.0, 79.0,
        ];
        let prog30 = [
            71.0, 90.0, 80.0, 75.0, 100.0, 80.0, 90.0, 102.0, 84.0, 78.0, 110.0, 77.0, 73.0, 78.0,
            0.0, 99.0, 80.0, 66.0, 46.0, 93.0, 40.0, 83.0, 80.0, 100.0, 93.0, 50.0, 96.0, 88.0,
            99.0, 108.0, 85.0, 80.0, 50.0, 64.0, 65.0, 100.0, 100.0, 110.0, 80.0, 50.0, 74.0, 90.0,
            80.0, 80.0, 56.0, 80.0, 79.0, 55.0, 65.0, 59.0, 90.0, 50.0, 90.0, 90.0, 75.0, 210.0,
            35.0, 86.0, 92.0, 80.0, 75.0, 100.0, 60.0, 50.0, 68.0, 51.0, 60.0, 53.0, 85.0, 58.0,
            96.0, 47.0, 80.0, 90.0, 67.0, 41.0, 55.0, 50.0, 103.0, 66.0, 35.0, 62.0, 75.0, 50.0,
            53.0, 84.0, 55.0, 73.0, 59.0, 60.0,
        ];
        let overdrive30 = [
            71.0, 90.0, 57.0, 75.0, 80.0, 80.0, 95.0, 79.0, 65.0, 31.0, 50.0, 69.0, 100.0, 68.0,
            0.0, 78.0, 50.0, 70.0, 62.0, 59.0, 64.0, 56.0, 73.0, 105.0, 67.0, 84.0, 80.0, 88.0,
            79.0, 80.0, 60.0, 80.0, 80.0, 63.0, 35.0, 50.0, 82.0, 55.0, 50.0, 95.0, 55.0, 70.0,
            100.0, 80.0, 99.0, 80.0, 61.0, 40.0, 69.0, 62.0, 51.0, 90.0, 67.0, 60.0, 100.0, 200.0,
            85.0, 50.0, 92.0, 50.0, 75.0, 80.0, 49.5, 50.0, 100.0, 51.0, 64.0, 65.5, 59.5, 58.0,
            96.0, 47.0, 75.0, 64.0, 90.0, 41.0, 34.0, 30.0, 55.0, 66.0, 55.0, 72.0, 91.0, 44.0,
            56.0, 84.0, 39.0, 105.0, 55.0, 43.0,
        ];
        let char_types = [
            1, 0, 0, 0, 0, 0, 0, 2, 0, 1, 2, 0, 0, 0, 2, 3, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2,
            2, 0, 0, 0, 0, 0, 2, 2, 2, 0, 0, 0, 2, 2, 2, 1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 3, 0, 2,
            2, 0, 0, 2, 0, 0, 2, 0, 2, 2, 1, 0, 2, 0, 4, 2, 0, 0, 0, 0, 4, 0, 0, 0, 2, 0, 2, 2, 0,
            2, 0, 0,
        ];

        let mut characters = Vec::new();

        for i in 0..90 {
            let stats = CharacterStats {
                frag1: frag1[i],
                prog1: prog1[i],
                overdrive1: overdrive1[i],
                frag20: frag20[i],
                prog20: prog20[i],
                overdrive20: overdrive20[i],
                frag30: frag30[i],
                prog30: prog30[i],
                overdrive30: overdrive30[i],
                skill_id: skill_ids[i].to_string(),
                skill_unlock_level: skill_unlock_levels[i],
                skill_id_uncap: skill_ids_uncap[i].to_string(),
                char_type: char_types[i],
            };

            characters.push((char_names[i], stats));
        }

        characters
    }

    /// Get uncapped character IDs
    fn get_uncapped_character_ids() -> std::collections::HashSet<i32> {
        [
            0, 1, 2, 4, 5, 10, 11, 12, 13, 19, 21, 23, 26, 27, 28, 29, 30, 34, 36, 42, 43, 66, 73,
            81, 82, 84,
        ]
        .iter()
        .cloned()
        .collect()
    }

    /// Get character cores mapping
    fn get_character_cores() -> HashMap<i32, Vec<CharacterCoreItem>> {
        let mut cores = HashMap::new();

        cores.insert(
            0,
            vec![
                CharacterCoreItem {
                    core_id: "core_hollow".to_string(),
                    amount: 25,
                },
                CharacterCoreItem {
                    core_id: "core_desolate".to_string(),
                    amount: 5,
                },
            ],
        );

        cores.insert(
            1,
            vec![
                CharacterCoreItem {
                    core_id: "core_hollow".to_string(),
                    amount: 5,
                },
                CharacterCoreItem {
                    core_id: "core_desolate".to_string(),
                    amount: 25,
                },
            ],
        );

        cores.insert(
            2,
            vec![
                CharacterCoreItem {
                    core_id: "core_hollow".to_string(),
                    amount: 5,
                },
                CharacterCoreItem {
                    core_id: "core_crimson".to_string(),
                    amount: 25,
                },
            ],
        );

        cores.insert(
            4,
            vec![
                CharacterCoreItem {
                    core_id: "core_ambivalent".to_string(),
                    amount: 25,
                },
                CharacterCoreItem {
                    core_id: "core_desolate".to_string(),
                    amount: 5,
                },
            ],
        );

        cores.insert(
            5,
            vec![CharacterCoreItem {
                core_id: "core_hollow".to_string(),
                amount: 0,
            }],
        );

        cores.insert(
            10,
            vec![CharacterCoreItem {
                core_id: "core_umbral".to_string(),
                amount: 30,
            }],
        );

        // Add more character cores based on Python data as needed
        cores.insert(
            11,
            vec![
                CharacterCoreItem {
                    core_id: "core_binary".to_string(),
                    amount: 25,
                },
                CharacterCoreItem {
                    core_id: "core_hollow".to_string(),
                    amount: 5,
                },
            ],
        );

        cores.insert(
            12,
            vec![
                CharacterCoreItem {
                    core_id: "core_binary".to_string(),
                    amount: 25,
                },
                CharacterCoreItem {
                    core_id: "core_desolate".to_string(),
                    amount: 5,
                },
            ],
        );

        cores.insert(
            13,
            vec![CharacterCoreItem {
                core_id: "core_scarlet".to_string(),
                amount: 30,
            }],
        );

        cores.insert(
            19,
            vec![CharacterCoreItem {
                core_id: "core_colorful".to_string(),
                amount: 30,
            }],
        );

        cores.insert(
            21,
            vec![CharacterCoreItem {
                core_id: "core_scarlet".to_string(),
                amount: 30,
            }],
        );

        cores.insert(
            23,
            vec![
                CharacterCoreItem {
                    core_id: "core_desolate".to_string(),
                    amount: 5,
                },
                CharacterCoreItem {
                    core_id: "core_serene".to_string(),
                    amount: 25,
                },
            ],
        );

        cores.insert(
            26,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            27,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            28,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            29,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            30,
            vec![
                CharacterCoreItem {
                    core_id: "core_hollow".to_string(),
                    amount: 5,
                },
                CharacterCoreItem {
                    core_id: "core_sunset".to_string(),
                    amount: 25,
                },
            ],
        );

        cores.insert(
            34,
            vec![CharacterCoreItem {
                core_id: "core_tanoc".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            36,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            42,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            43,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            66,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            73,
            vec![CharacterCoreItem {
                core_id: "core_wacca".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            81,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            82,
            vec![CharacterCoreItem {
                core_id: "core_chunithm".to_string(),
                amount: 15,
            }],
        );

        cores.insert(
            84,
            vec![CharacterCoreItem {
                core_id: "core_maimai".to_string(),
                amount: 15,
            }],
        );

        cores
    }

    /// Get all core item IDs
    fn get_cores() -> Vec<&'static str> {
        vec![
            "core_hollow",
            "core_desolate",
            "core_chunithm",
            "core_crimson",
            "core_ambivalent",
            "core_scarlet",
            "core_groove",
            "core_generic",
            "core_binary",
            "core_colorful",
            "core_course_skip_purchase",
            "core_umbral",
            "core_wacca",
            "core_sunset",
            "core_tanoc",
            "core_serene",
            "core_maimai",
        ]
    }

    /// Get world song IDs
    fn get_world_songs() -> Vec<&'static str> {
        vec![
            "babaroque",
            "shadesoflight",
            "kanagawa",
            "lucifer",
            "anokumene",
            "ignotus",
            "rabbitintheblackroom",
            "qualia",
            "redandblue",
            "bookmaker",
            "darakunosono",
            "espebranch",
            "blacklotus",
            "givemeanightmare",
            "vividtheory",
            "onefr",
            "gekka",
            "vexaria3",
            "infinityheaven3",
            "fairytale3",
            "goodtek3",
            "suomi",
            "rugie",
            "faintlight",
            "harutopia",
            "goodtek",
            "dreaminattraction",
            "syro",
            "diode",
            "freefall",
            "grimheart",
            "blaster",
            "cyberneciacatharsis",
            "monochromeprincess",
            "revixy",
            "vector",
            "supernova",
            "nhelv",
            "purgatorium3",
            "dement3",
            "crossover",
            "guardina",
            "axiumcrisis",
            "worldvanquisher",
            "sheriruth",
            "pragmatism",
            "gloryroad",
            "etherstrike",
            "corpssansorganes",
            "lostdesire",
            "blrink",
            "essenceoftwilight",
            "lapis",
            "solitarydream",
            "lumia3",
            "purpleverse",
            "moonheart3",
            "glow",
            "enchantedlove",
            "take",
            "lifeispiano",
            "vandalism",
            "nexttoyou3",
            "lostcivilization3",
            "turbocharger",
            "bookmaker3",
            "laqryma3",
            "kyogenkigo",
            "hivemind",
            "seclusion",
            "quonwacca3",
            "bluecomet",
            "energysynergymatrix",
            "gengaozo",
            "lastendconductor3",
            "antithese3",
            "qualia3",
            "kanagawa3",
            "heavensdoor3",
            "pragmatism3",
            "nulctrl",
            "avril",
            "ddd",
            "merlin3",
            "omakeno3",
            "nekonote",
            "sanskia",
            "altair",
            "mukishitsu",
            "trapcrow",
            "redandblue3",
            "ignotus3",
            "singularity3",
            "dropdead3",
            "arcahv",
            "freefall3",
            "partyvinyl3",
            "tsukinimurakumo",
            "mantis",
            "worldfragments",
            "astrawalkthrough",
            "chronicle",
            "trappola3",
            "letsrock",
            "shadesoflight3",
            "teriqma3",
            "impact3",
            "lostemotion",
            "gimmick",
            "lawlesspoint",
            "hybris",
            "ultimatetaste",
            "rgb",
            "matenrou",
            "dynitikos",
            "amekagura",
            "fantasy",
            "aloneandlorn",
            "felys",
            "onandon",
            "hotarubinoyuki",
            "oblivia3",
            "libertas3",
            "einherjar3",
            "purpleverse3",
            "viciousheroism3",
            "inkarusi3",
            "cyberneciacatharsis3",
            "alephzero",
            "hellohell",
            "ichirin",
            "awakeninruins",
            "morningloom",
            "lethalvoltage",
            "leaveallbehind",
            "desive",
            "oldschoolsalvage",
            "distortionhuman",
            "epitaxy",
            "hailstone",
            "furetemitai",
            "prayer",
            "astralexe",
            "trpno",
            "blackmirror",
            "tau",
            "snowwhite3",
            "altale3",
            "energysynergymatrix3",
            "anokumene3",
            "nhelv3",
            "wontbackdown",
            "someday",
        ]
    }

    /// Get world unlock IDs
    fn get_world_unlocks() -> Vec<&'static str> {
        vec![
            "scenery_chap1",
            "scenery_chap2",
            "scenery_chap3",
            "scenery_chap4",
            "scenery_chap5",
            "scenery_chap6",
            "scenery_chap7",
            "scenery_chap8",
            "scenery_beyond",
        ]
    }

    /// Get course banner IDs
    fn get_course_banners() -> Vec<String> {
        (1..=11).map(|i| format!("course_banner_{i}")).collect()
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
