#[macro_use]
extern crate rocket;

pub mod core;

use core::auth::{auth_login, email_verify};
use core::config_manager::GAME_API_PREFIX;
use core::database::{init_check_database_all, Core, CORE_DB_URL};
use core::error::ArcError;
use core::notification::init_notification_db;
use core::others::*;
use core::user::{
    character_change, character_exp, character_first_uncap, cloud_get, cloud_post,
    email_resend_verify, register, sys_set, toggle_invasion, toggle_uncap, user_delete, user_me,
};
use core::users_api::*;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Request, Response};
use rocket_db_pools::Database;
use sqlx::SqlitePool;

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(rocket::http::Header::new(
            "Access-Control-Allow-Origin",
            "*",
        ));
        response.set_header(rocket::http::Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(rocket::http::Header::new(
            "Access-Control-Allow-Headers",
            "*",
        ));
        response.set_header(rocket::http::Header::new(
            "Access-Control-Allow-Credentials",
            "true",
        ));
    }
}

#[catch(404)]
fn not_found() -> Result<rocket::serde::json::Json<core::error::ErrorResponse>, ArcError> {
    Err(ArcError::with_error_code("Endpoint not found", 151))
}

#[catch(500)]
fn internal_error() -> Result<rocket::serde::json::Json<core::error::ErrorResponse>, ArcError> {
    Err(ArcError::new("Internal server error"))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    // Initialize database
    init_check_database_all().await;

    // Initialize notification system
    if let Err(e) = init_notification_db().await {
        eprintln!("Warning: Failed to initialize notification database: {}", e);
    }

    // Create the main database pool for application use
    let pool = SqlitePool::connect(CORE_DB_URL)
        .await
        .expect("Failed to create database pool");

    let _rocket = rocket::build()
        .attach(Core::init())
        .attach(CORS)
        .manage(pool)
        .register("/", catchers![not_found, internal_error])
        .mount(
            &format!("/{}", GAME_API_PREFIX),
            routes![
                // Others endpoints
                game_info,
                notification_me,
                game_content_bundle,
                download_song,
                finale_progress,
                finale_start,
                finale_end,
                insight_complete,
                applog_me,
                aggregate,
            ],
        )
        .mount("/auth", routes![auth_login, email_verify])
        .mount(
            "/user",
            routes![
                register,
                user_me,
                toggle_invasion,
                character_change,
                toggle_uncap,
                character_first_uncap,
                character_exp,
                cloud_get,
                cloud_post,
                sys_set,
                user_delete,
                email_resend_verify
            ],
        )
        .mount(
            &format!("/{}/user", GAME_API_PREFIX),
            routes![
                create_user,
                get_users,
                get_user,
                update_user,
                get_user_b30,
                get_user_best,
                get_user_r30,
                get_user_role,
                get_user_rating_history
            ],
        )
        .launch()
        .await?;

    Ok(())
}
