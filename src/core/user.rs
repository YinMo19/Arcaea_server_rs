use chrono::Utc;
use rocket::form::Form;
use rocket::serde::json::Json;
use rocket::{get, post, State};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

use crate::core::auth::{AuthenticatedUser, UserAuth};
use crate::core::error::{success_return, ArcError, ArcResult, SuccessResponse};
use crate::core::models::User;

#[derive(Debug, FromForm)]
pub struct UserRegistrationForm {
    pub name: String,
    pub password: String,
    pub email: String,
    #[field(default = "low_version")]
    pub device_id: String,
}

#[derive(Debug, FromForm)]
pub struct CharacterChangeForm {
    pub character: i32,
    pub skill_sealed: String,
}

#[derive(Debug, FromForm)]
pub struct CharacterExpForm {
    pub amount: i32,
}

#[derive(Debug, FromForm)]
pub struct SaveDataForm {
    pub scores_data: String,
    pub scores_checksum: String,
    pub clearlamps_data: String,
    pub clearlamps_checksum: String,
    pub clearedsongs_data: String,
    pub clearedsongs_checksum: String,
    pub unlocklist_data: String,
    pub unlocklist_checksum: String,
    pub installid_data: String,
    pub installid_checksum: String,
    pub devicemodelname_data: String,
    pub devicemodelname_checksum: String,
    pub story_data: String,
    pub story_checksum: String,
    #[field(default = "")]
    pub finalestate_data: String,
    #[field(default = "")]
    pub finalestate_checksum: String,
}

#[derive(Debug, FromForm)]
pub struct SettingForm {
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user_id: i32,
    pub name: String,
    pub join_date: i64,
    pub user_code: String,
    pub rating_ptt: i32,
    pub character_id: i32,
    pub is_skill_sealed: i32,
    pub is_char_uncapped: i32,
    pub is_char_uncapped_override: i32,
    pub is_hide_rating: i32,
    pub ticket: i32,
    pub world_rank_score: i32,
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub next_fragstam_ts: i64,
    pub world_mode_locked_end_ts: i64,
    pub beyond_boost_gauge: f64,
    pub kanae_stored_prog: f64,
    pub mp_notification_enabled: i32,
    pub favorite_character: i32,
    pub max_stamina_notification_enabled: i32,
    pub insight_state: i32,
    pub cores: Vec<CoreItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreItem {
    pub item_id: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub character_id: i32,
    pub name: String,
    pub max_level: i32,
    pub level: i32,
    pub exp: f64,
    pub level_exp: f64,
    pub skill_id: String,
    pub skill_id_uncap: String,
    pub skill_requires_uncap: i32,
    pub char_type: i32,
    pub is_uncapped: i32,
    pub is_uncapped_override: i32,
    pub uncap_cores: Vec<CoreItem>,
}

#[derive(Debug, Serialize)]
pub struct ToggleInvasionResponse {
    pub user_id: i32,
    pub insight_state: i32,
}

#[derive(Debug, Serialize)]
pub struct CharacterChangeResponse {
    pub user_id: i32,
    pub character: i32,
}

#[derive(Debug, Serialize)]
pub struct CharacterUncapResponse {
    pub user_id: i32,
    pub character: Vec<CharacterInfo>,
    pub cores: Vec<CoreItem>,
}

#[derive(Debug, Serialize)]
pub struct SaveDataResponse {
    pub user_id: i32,
    pub story: String,
    pub devicemodelname: String,
    pub installid: String,
    pub unlocklist: String,
    pub clearedsongs: String,
    pub clearlamps: String,
    pub scores: String,
    pub version: i32,
    #[serde(rename = "createdAt")]
    pub created_at: i64,
    pub finalestate: String,
}

#[derive(Debug, Serialize)]
pub struct RegistrationResponse {
    pub user_id: i32,
    pub access_token: String,
}

#[post("/", data = "<registration_form>")]
pub async fn register(
    pool: &State<SqlitePool>,
    registration_form: Form<UserRegistrationForm>,
) -> Result<Json<SuccessResponse<RegistrationResponse>>, ArcError> {
    let user_auth = UserAuth::new((*pool).clone());

    let login_response = user_auth
        .register_user(
            &registration_form.name,
            &registration_form.password,
            &registration_form.email,
            &registration_form.device_id,
            "127.0.0.1", // TODO: Get real IP
        )
        .await?;

    let response = RegistrationResponse {
        user_id: login_response.user_id,
        access_token: login_response.access_token,
    };

    Ok(success_return(response))
}

#[get("/me")]
pub async fn user_me(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<UserResponse>>, ArcError> {
    let cores = get_user_cores(pool, user.user.user_id).await?;

    let user_response = UserResponse {
        user_id: user.user.user_id,
        name: user.user.name,
        join_date: user.user.join_date,
        user_code: user.user.user_code,
        rating_ptt: user.user.rating_ptt,
        character_id: user.user.character_id,
        is_skill_sealed: user.user.is_skill_sealed,
        is_char_uncapped: user.user.is_char_uncapped,
        is_char_uncapped_override: user.user.is_char_uncapped_override,
        is_hide_rating: user.user.is_hide_rating,
        ticket: user.user.ticket,
        world_rank_score: user.user.world_rank_score,
        stamina: user.user.stamina,
        max_stamina_ts: user.user.max_stamina_ts,
        next_fragstam_ts: user.user.next_fragstam_ts,
        world_mode_locked_end_ts: user.user.world_mode_locked_end_ts,
        beyond_boost_gauge: user.user.beyond_boost_gauge,
        kanae_stored_prog: user.user.kanae_stored_prog,
        mp_notification_enabled: user.user.mp_notification_enabled,
        favorite_character: user.user.favorite_character,
        max_stamina_notification_enabled: user.user.max_stamina_notification_enabled,
        insight_state: user.user.insight_state.unwrap_or(4),
        cores,
    };

    Ok(success_return(user_response))
}

#[post("/me/toggle_invasion")]
pub async fn toggle_invasion(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<ToggleInvasionResponse>>, ArcError> {
    let current_state = user.user.insight_state.unwrap_or(4);
    let new_state = if current_state == 2 { 0 } else { 2 };

    sqlx::query("UPDATE user SET insight_state = ? WHERE user_id = ?")
        .bind(new_state)
        .bind(user.user.user_id)
        .execute(pool.inner())
        .await?;

    let response = ToggleInvasionResponse {
        user_id: user.user.user_id,
        insight_state: new_state,
    };

    Ok(success_return(response))
}

#[post("/me/character", data = "<character_form>")]
pub async fn character_change(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    character_form: Form<CharacterChangeForm>,
) -> Result<Json<SuccessResponse<CharacterChangeResponse>>, ArcError> {
    let character_id = character_form.character;
    let skill_sealed = character_form.skill_sealed == "true";

    // Check if user owns this character
    let owned_char =
        sqlx::query("SELECT level FROM user_char WHERE user_id = ? AND character_id = ?")
            .bind(user.user.user_id)
            .bind(character_id)
            .fetch_optional(pool.inner())
            .await?;

    if owned_char.is_none() {
        return Err(ArcError::no_data("Character not owned"));
    }

    // Update user's current character
    sqlx::query("UPDATE user SET character_id = ?, is_skill_sealed = ? WHERE user_id = ?")
        .bind(character_id)
        .bind(if skill_sealed { 1 } else { 0 })
        .bind(user.user.user_id)
        .execute(pool.inner())
        .await?;

    let response = CharacterChangeResponse {
        user_id: user.user.user_id,
        character: character_id,
    };

    Ok(success_return(response))
}

#[post("/me/character/<character_id>/toggle_uncap")]
pub async fn toggle_uncap(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    character_id: i32,
) -> Result<Json<SuccessResponse<CharacterUncapResponse>>, ArcError> {
    // Get character info
    let char_row = sqlx::query("SELECT * FROM user_char WHERE user_id = ? AND character_id = ?")
        .bind(user.user.user_id)
        .bind(character_id)
        .fetch_optional(pool.inner())
        .await?;

    let char_row = match char_row {
        Some(row) => row,
        None => return Err(ArcError::no_data("Character not found")),
    };

    let current_override: i32 = char_row.get("is_uncapped_override");
    let new_override = if current_override == 0 { 1 } else { 0 };

    // Update override status
    sqlx::query(
        "UPDATE user_char SET is_uncapped_override = ? WHERE user_id = ? AND character_id = ?",
    )
    .bind(new_override)
    .bind(user.user.user_id)
    .bind(character_id)
    .execute(pool.inner())
    .await?;

    let character_info = get_character_info(pool, user.user.user_id, character_id).await?;
    let cores = get_user_cores(pool, user.user.user_id).await?;

    let response = CharacterUncapResponse {
        user_id: user.user.user_id,
        character: vec![character_info],
        cores,
    };

    Ok(success_return(response))
}

#[post("/me/character/<character_id>/uncap")]
pub async fn character_first_uncap(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    character_id: i32,
) -> Result<Json<SuccessResponse<CharacterUncapResponse>>, ArcError> {
    // Check if character can be uncapped and user has enough cores
    let char_info = sqlx::query(
        "SELECT c.is_uncapped, uc.level, uc.is_uncapped
         FROM character c
         JOIN user_char uc ON c.character_id = uc.character_id
         WHERE uc.user_id = ? AND c.character_id = ?",
    )
    .bind(user.user.user_id)
    .bind(character_id)
    .fetch_optional(pool.inner())
    .await?;

    let char_info = match char_info {
        Some(row) => row,
        None => return Err(ArcError::no_data("Character not found")),
    };

    let can_uncap: i32 = char_info.get("is_uncapped");
    let level: i32 = char_info.get("level");
    let is_uncapped: i32 = char_info.get("is_uncapped");

    if can_uncap == 0 {
        return Err(ArcError::item_unavailable("Character cannot be uncapped"));
    }

    if is_uncapped == 1 {
        return Err(ArcError::data_exist("Character already uncapped"));
    }

    if level < 20 {
        return Err(ArcError::input_error("Character must be level 20 to uncap"));
    }

    // Check cores required (get from char_item table)
    let core_requirements = sqlx::query(
        "SELECT item_id, amount FROM char_item WHERE character_id = ? AND type = 'core'",
    )
    .bind(character_id)
    .fetch_all(pool.inner())
    .await?;

    // Check if user has enough cores
    for core_req in &core_requirements {
        let core_id: String = core_req.get("item_id");
        let required_amount: i32 = core_req.get("amount");

        let user_cores = sqlx::query(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = 'core'",
        )
        .bind(user.user.user_id)
        .bind(&core_id)
        .fetch_optional(pool.inner())
        .await?;

        let user_amount = match user_cores {
            Some(row) => row.get::<i32, _>("amount"),
            None => 0,
        };

        if user_amount < required_amount {
            return Err(ArcError::item_not_enough(&format!(
                "Not enough {}",
                core_id
            )));
        }
    }

    // Deduct cores
    for core_req in &core_requirements {
        let core_id: String = core_req.get("item_id");
        let required_amount: i32 = core_req.get("amount");

        sqlx::query(
            "UPDATE user_item SET amount = amount - ? WHERE user_id = ? AND item_id = ? AND type = 'core'"
        )
        .bind(required_amount)
        .bind(user.user.user_id)
        .bind(&core_id)
        .execute(pool.inner())
        .await?;
    }

    // Uncap character
    sqlx::query("UPDATE user_char SET is_uncapped = 1 WHERE user_id = ? AND character_id = ?")
        .bind(user.user.user_id)
        .bind(character_id)
        .execute(pool.inner())
        .await?;

    let character_info = get_character_info(pool, user.user.user_id, character_id).await?;
    let cores = get_user_cores(pool, user.user.user_id).await?;

    let response = CharacterUncapResponse {
        user_id: user.user.user_id,
        character: vec![character_info],
        cores,
    };

    Ok(success_return(response))
}

#[post("/me/character/<character_id>/exp", data = "<exp_form>")]
pub async fn character_exp(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    character_id: i32,
    exp_form: Form<CharacterExpForm>,
) -> Result<Json<SuccessResponse<CharacterUncapResponse>>, ArcError> {
    let amount = exp_form.amount;

    if amount <= 0 {
        return Err(ArcError::input_error("Amount must be positive"));
    }

    // Check if user has enough cores
    let user_cores = sqlx::query(
        "SELECT amount FROM user_item WHERE user_id = ? AND item_id = 'core_generic' AND type = 'core'"
    )
    .bind(user.user.user_id)
    .fetch_optional(pool.inner())
    .await?;

    let current_cores = match user_cores {
        Some(row) => row.get::<i32, _>("amount"),
        None => 0,
    };

    if current_cores < amount {
        return Err(ArcError::item_not_enough("Not enough core_generic"));
    }

    // Deduct cores
    sqlx::query(
        "UPDATE user_item SET amount = amount - ? WHERE user_id = ? AND item_id = 'core_generic' AND type = 'core'"
    )
    .bind(amount)
    .bind(user.user.user_id)
    .execute(pool.inner())
    .await?;

    // Add EXP to character (each core gives 10000 EXP)
    let exp_gain = amount as f64 * 10000.0;

    sqlx::query("UPDATE user_char SET exp = exp + ? WHERE user_id = ? AND character_id = ?")
        .bind(exp_gain)
        .bind(user.user.user_id)
        .bind(character_id)
        .execute(pool.inner())
        .await?;

    // Update character level based on new EXP
    update_character_level(pool, user.user.user_id, character_id).await?;

    let character_info = get_character_info(pool, user.user.user_id, character_id).await?;
    let cores = get_user_cores(pool, user.user.user_id).await?;

    let response = CharacterUncapResponse {
        user_id: user.user.user_id,
        character: vec![character_info],
        cores,
    };

    Ok(success_return(response))
}

#[get("/me/save")]
pub async fn cloud_get(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<SaveDataResponse>>, ArcError> {
    let save_row = sqlx::query("SELECT * FROM user_save WHERE user_id = ?")
        .bind(user.user.user_id)
        .fetch_optional(pool.inner())
        .await?;

    let response = match save_row {
        Some(row) => SaveDataResponse {
            user_id: user.user.user_id,
            story: row.get::<String, _>("story_data"),
            devicemodelname: row.get::<String, _>("devicemodelname_data"),
            installid: row.get::<String, _>("installid_data"),
            unlocklist: row.get::<String, _>("unlocklist_data"),
            clearedsongs: row.get::<String, _>("clearedsongs_data"),
            clearlamps: row.get::<String, _>("clearlamps_data"),
            scores: row.get::<String, _>("scores_data"),
            version: 1,
            created_at: row.get::<i64, _>("createdAt"),
            finalestate: row
                .get::<Option<String>, _>("finalestate_data")
                .unwrap_or_default(),
        },
        None => {
            // Create empty save data
            let created_at = Utc::now().timestamp_millis();
            sqlx::query(
                r#"INSERT INTO user_save (
                    user_id, scores_data, clearlamps_data, clearedsongs_data,
                    unlocklist_data, installid_data, devicemodelname_data,
                    story_data, createdAt, finalestate_data
                ) VALUES (?, '', '', '', '', '', '', '', ?, '')"#,
            )
            .bind(user.user.user_id)
            .bind(created_at)
            .execute(pool.inner())
            .await?;

            SaveDataResponse {
                user_id: user.user.user_id,
                story: String::new(),
                devicemodelname: String::new(),
                installid: String::new(),
                unlocklist: String::new(),
                clearedsongs: String::new(),
                clearlamps: String::new(),
                scores: String::new(),
                version: 1,
                created_at,
                finalestate: String::new(),
            }
        }
    };

    Ok(success_return(response))
}

#[post("/me/save", data = "<save_form>")]
pub async fn cloud_post(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    save_form: Form<SaveDataForm>,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ArcError> {
    // Update or insert save data
    sqlx::query(
        r#"INSERT OR REPLACE INTO user_save (
            user_id, scores_data, clearlamps_data, clearedsongs_data,
            unlocklist_data, installid_data, devicemodelname_data,
            story_data, createdAt, finalestate_data
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"#,
    )
    .bind(user.user.user_id)
    .bind(&save_form.scores_data)
    .bind(&save_form.clearlamps_data)
    .bind(&save_form.clearedsongs_data)
    .bind(&save_form.unlocklist_data)
    .bind(&save_form.installid_data)
    .bind(&save_form.devicemodelname_data)
    .bind(&save_form.story_data)
    .bind(Utc::now().timestamp_millis())
    .bind(&save_form.finalestate_data)
    .execute(pool.inner())
    .await?;

    let response = serde_json::json!({
        "user_id": user.user.user_id
    });

    Ok(success_return(response))
}

#[post("/me/setting/<set_arg>", data = "<setting_form>")]
pub async fn sys_set(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
    set_arg: String,
    setting_form: Form<SettingForm>,
) -> Result<Json<SuccessResponse<UserResponse>>, ArcError> {
    let value = &setting_form.value;

    match set_arg.as_str() {
        "favorite_character" => {
            let char_id: i32 = value
                .parse()
                .map_err(|_| ArcError::input_error("Invalid character ID"))?;

            sqlx::query("UPDATE user SET favorite_character = ? WHERE user_id = ?")
                .bind(char_id)
                .bind(user.user.user_id)
                .execute(pool.inner())
                .await?;
        }
        "is_hide_rating" | "max_stamina_notification_enabled" | "mp_notification_enabled" => {
            let bool_value = value == "true";
            let int_value = if bool_value { 1 } else { 0 };

            let query = format!("UPDATE user SET {} = ? WHERE user_id = ?", set_arg);
            sqlx::query(&query)
                .bind(int_value)
                .bind(user.user.user_id)
                .execute(pool.inner())
                .await?;
        }
        _ => return Err(ArcError::input_error("Invalid setting")),
    }

    // Return updated user info
    let updated_user = sqlx::query("SELECT * FROM user WHERE user_id = ?")
        .bind(user.user.user_id)
        .fetch_one(pool.inner())
        .await?;

    let updated_user_model = User::from_row(&updated_user)?;
    let cores = get_user_cores(pool, user.user.user_id).await?;

    let user_response = UserResponse {
        user_id: updated_user_model.user_id,
        name: updated_user_model.name,
        join_date: updated_user_model.join_date,
        user_code: updated_user_model.user_code,
        rating_ptt: updated_user_model.rating_ptt,
        character_id: updated_user_model.character_id,
        is_skill_sealed: updated_user_model.is_skill_sealed,
        is_char_uncapped: updated_user_model.is_char_uncapped,
        is_char_uncapped_override: updated_user_model.is_char_uncapped_override,
        is_hide_rating: updated_user_model.is_hide_rating,
        ticket: updated_user_model.ticket,
        world_rank_score: updated_user_model.world_rank_score,
        stamina: updated_user_model.stamina,
        max_stamina_ts: updated_user_model.max_stamina_ts,
        next_fragstam_ts: updated_user_model.next_fragstam_ts,
        world_mode_locked_end_ts: updated_user_model.world_mode_locked_end_ts,
        beyond_boost_gauge: updated_user_model.beyond_boost_gauge,
        kanae_stored_prog: updated_user_model.kanae_stored_prog,
        mp_notification_enabled: updated_user_model.mp_notification_enabled,
        favorite_character: updated_user_model.favorite_character,
        max_stamina_notification_enabled: updated_user_model.max_stamina_notification_enabled,
        insight_state: updated_user_model.insight_state.unwrap_or(4),
        cores,
    };

    Ok(success_return(user_response))
}

#[post("/me/request_delete")]
pub async fn user_delete(
    pool: &State<SqlitePool>,
    user: AuthenticatedUser,
) -> Result<Json<SuccessResponse<serde_json::Value>>, ArcError> {
    // TODO: Check if ALLOW_SELF_ACCOUNT_DELETE config is enabled
    // For now, allow deletion

    // Delete user and related data
    sqlx::query("DELETE FROM user WHERE user_id = ?")
        .bind(user.user.user_id)
        .execute(pool.inner())
        .await?;

    let response = serde_json::json!({
        "user_id": user.user.user_id
    });

    Ok(success_return(response))
}

#[post("/email/resend_verify")]
pub fn email_resend_verify() -> Result<Json<SuccessResponse<()>>, ArcError> {
    Err(ArcError::with_error_code(
        "Email verification unavailable.",
        151,
    ))
}

async fn get_user_cores(pool: &State<SqlitePool>, user_id: i32) -> ArcResult<Vec<CoreItem>> {
    let rows =
        sqlx::query("SELECT item_id, amount FROM user_item WHERE user_id = ? AND type = 'core'")
            .bind(user_id)
            .fetch_all(pool.inner())
            .await?;

    let mut cores = Vec::new();
    for row in rows {
        cores.push(CoreItem {
            item_id: row.get("item_id"),
            amount: row.get("amount"),
        });
    }

    Ok(cores)
}

async fn get_character_info(
    pool: &State<SqlitePool>,
    user_id: i32,
    character_id: i32,
) -> ArcResult<CharacterInfo> {
    let row = sqlx::query(
        r#"SELECT
            c.character_id, c.name, c.max_level, c.skill_id, c.skill_id_uncap,
            c.skill_requires_uncap, c.char_type, c.is_uncapped as can_uncap,
            uc.level, uc.exp, uc.is_uncapped, uc.is_uncapped_override
        FROM character c
        JOIN user_char uc ON c.character_id = uc.character_id
        WHERE uc.user_id = ? AND c.character_id = ?"#,
    )
    .bind(user_id)
    .bind(character_id)
    .fetch_one(pool.inner())
    .await?;

    let level: i32 = row.get("level");
    let exp: f64 = row.get("exp");
    let max_level: i32 = row.get("max_level");

    // Calculate level exp (exp needed for current level)
    let level_exp = calculate_level_exp(level, max_level);

    // Get uncap cores
    let core_rows = sqlx::query(
        "SELECT item_id, amount FROM char_item WHERE character_id = ? AND type = 'core'",
    )
    .bind(character_id)
    .fetch_all(pool.inner())
    .await?;

    let mut uncap_cores = Vec::new();
    for core_row in core_rows {
        uncap_cores.push(CoreItem {
            item_id: core_row.get("item_id"),
            amount: core_row.get("amount"),
        });
    }

    Ok(CharacterInfo {
        character_id: row.get("character_id"),
        name: row.get("name"),
        max_level: row.get("max_level"),
        level,
        exp,
        level_exp,
        skill_id: row.get("skill_id"),
        skill_id_uncap: row.get("skill_id_uncap"),
        skill_requires_uncap: row.get("skill_requires_uncap"),
        char_type: row.get("char_type"),
        is_uncapped: row.get("is_uncapped"),
        is_uncapped_override: row.get("is_uncapped_override"),
        uncap_cores,
    })
}

async fn update_character_level(
    pool: &State<SqlitePool>,
    user_id: i32,
    character_id: i32,
) -> ArcResult<()> {
    let char_row = sqlx::query(
        "SELECT uc.exp, c.max_level, uc.is_uncapped FROM user_char uc JOIN character c ON uc.character_id = c.character_id WHERE uc.user_id = ? AND uc.character_id = ?"
    )
    .bind(user_id)
    .bind(character_id)
    .fetch_one(pool.inner())
    .await?;

    let exp: f64 = char_row.get("exp");
    let max_level: i32 = char_row.get("max_level");
    let is_uncapped: i32 = char_row.get("is_uncapped");

    let actual_max_level = if is_uncapped == 1 { max_level } else { 20 };
    let new_level = calculate_level_from_exp(exp, actual_max_level);

    sqlx::query("UPDATE user_char SET level = ? WHERE user_id = ? AND character_id = ?")
        .bind(new_level)
        .bind(user_id)
        .bind(character_id)
        .execute(pool.inner())
        .await?;

    Ok(())
}

fn calculate_level_from_exp(exp: f64, max_level: i32) -> i32 {
    // Simple level calculation - each level requires increasingly more EXP
    let mut level = 1;
    let mut required_exp = 0.0;

    while level < max_level {
        let exp_for_next_level = calculate_exp_for_level(level + 1);
        if exp < required_exp + exp_for_next_level {
            break;
        }
        required_exp += exp_for_next_level;
        level += 1;
    }

    level
}

fn calculate_exp_for_level(level: i32) -> f64 {
    // Each level requires more EXP (simple formula)
    (level as f64 * 1000.0) + ((level - 1) as f64 * 500.0)
}

fn calculate_level_exp(level: i32, max_level: i32) -> f64 {
    if level >= max_level {
        0.0
    } else {
        calculate_exp_for_level(level + 1)
    }
}
