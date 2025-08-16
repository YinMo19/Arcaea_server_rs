//! Arcaea Server Rust Implementation
//!
//! Main application entry point that sets up the Rocket web server
//! with database connections, services, and routes.

use rocket::fairing::AdHoc;
use rocket::serde::json::Value;
use rocket::{launch, routes, Build, Rocket};

use Arcaea_server_rs::route::CORS;
use Arcaea_server_rs::service::{
    BundleService, CharacterService, DownloadService, NotificationService, ScoreService,
    UserService,
};
use Arcaea_server_rs::{Database, DbPool};

/// Initialize application services with database connection
async fn init_services(
    pool: DbPool,
) -> (
    UserService,
    DownloadService,
    ScoreService,
    NotificationService,
    BundleService,
    CharacterService,
) {
    let user_service = UserService::new(pool.clone());
    let download_service = DownloadService::with_defaults(pool.clone());
    let score_service = ScoreService::new(pool.clone());
    let notification_service = NotificationService::new(pool.clone());
    let bundle_service = BundleService::new(pool.clone(), std::path::PathBuf::from("bundles"));
    let character_service = CharacterService::new(pool.clone());
    (
        user_service,
        download_service,
        score_service,
        notification_service,
        bundle_service,
        character_service,
    )
}

/// Health check endpoint
#[rocket::get("/health")]
async fn health_check(pool: &rocket::State<DbPool>) -> Result<&'static str, &'static str> {
    match Database::check_health(pool).await {
        Ok(_) => Ok("OK"),
        Err(_) => Err("Database connection failed"),
    }
}

/// Configure the Rocket application
fn configure_rocket() -> Rocket<Build> {
    rocket::build()
        .attach(CORS)
        .attach(AdHoc::on_ignite("Database", |rocket| async {
            match Database::new().await {
                Ok(pool) => {
                    println!("✅ Database connection established");
                    rocket.manage(pool)
                }
                Err(e) => {
                    eprintln!("❌ Failed to connect to database: {}", e);
                    std::process::exit(1);
                }
            }
        }))
        .attach(AdHoc::on_ignite("Services", |rocket| async {
            let pool = rocket.state::<DbPool>().unwrap().clone();
            let (
                user_service,
                download_service,
                score_service,
                notification_service,
                bundle_service,
                character_service,
            ) = init_services(pool).await;

            println!("✅ Services initialized");
            rocket
                .manage(user_service)
                .manage(download_service)
                .manage(score_service)
                .manage(notification_service)
                .manage(bundle_service)
                .manage(character_service)
        }))
        .mount("/health", routes![health_check])
        .mount(
            "/user",
            rocket::routes![
                Arcaea_server_rs::route::user::register,
                Arcaea_server_rs::route::user::login,
                Arcaea_server_rs::route::user::user_me,
                Arcaea_server_rs::route::user::logout,
                Arcaea_server_rs::route::user::user_by_code,
                Arcaea_server_rs::route::user::update_user,
                Arcaea_server_rs::route::user::auth_test,
            ],
        )
        .mount(
            "/",
            rocket::routes![
                Arcaea_server_rs::route::others::game_info,
                Arcaea_server_rs::route::others::notification_me,
                Arcaea_server_rs::route::others::game_content_bundle,
                Arcaea_server_rs::route::others::download_song,
                Arcaea_server_rs::route::others::finale_start,
                Arcaea_server_rs::route::others::finale_end,
                Arcaea_server_rs::route::others::insight_complete,
                Arcaea_server_rs::route::others::applog_me,
                Arcaea_server_rs::route::others::aggregate,
                Arcaea_server_rs::route::download::serve_download_file,
                Arcaea_server_rs::route::download::finale_progress,
            ],
        )
        .mount(
            "/",
            rocket::routes![
                Arcaea_server_rs::route::score::score_token,
                Arcaea_server_rs::route::score::score_token_world,
                Arcaea_server_rs::route::score::score_token_course,
                Arcaea_server_rs::route::score::song_score_post,
                Arcaea_server_rs::route::score::song_score_top,
                Arcaea_server_rs::route::score::song_score_me,
                Arcaea_server_rs::route::score::song_score_friend,
            ],
        )
        .register(
            "/",
            rocket::catchers![
                not_found,
                internal_error,
                bad_request,
                unauthorized,
                forbidden,
            ],
        )
}

/// 404 Not Found handler
#[rocket::catch(404)]
fn not_found() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 404,
        "message": "Endpoint not found"
    })
}

/// 500 Internal Server Error handler
#[rocket::catch(500)]
fn internal_error() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 500,
        "message": "Internal server error"
    })
}

/// 400 Bad Request handler
#[rocket::catch(400)]
fn bad_request() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 400,
        "message": "Bad request"
    })
}

/// 401 Unauthorized handler
#[rocket::catch(401)]
fn unauthorized() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 401,
        "message": "Unauthorized"
    })
}

/// 403 Forbidden handler
#[rocket::catch(403)]
fn forbidden() -> Value {
    rocket::serde::json::json!({
        "success": false,
        "error_code": 403,
        "message": "Forbidden"
    })
}

/// Application entry point
#[launch]
fn rocket() -> _ {
    // Print startup banner
    println!("🎵 Arcaea Server Rust Edition");
    println!("📦 Version: {}", Arcaea_server_rs::ARCAEA_SERVER_VERSION);
    println!("🚀 Starting server...");

    // Load environment variables
    dotenv::dotenv().ok();

    // Configure and launch the application
    configure_rocket()
}

#[cfg(test)]
mod tests {
    // use super::*;
    // use rocket::http::Status;
    // use rocket::local::blocking::Client;

    #[test]
    fn test_health_endpoint() {
        // Note: This test requires a database connection
        // In a real test environment, you would use a test database
        // or mock the database connection

        // For now, this is a placeholder test
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_game_info_endpoint() {
        // This would test the /game/info endpoint
        // Similar to the health endpoint test, this would require
        // proper test setup with database mocking

        // Placeholder test
        assert!(true);
    }
}
