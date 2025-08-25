//! Database initialization binary
//!
//! This binary initializes the database with all required game data including
//! characters, items, courses, roles, and a default admin account.

use std::{io, process};
use Arcaea_server_rs::service::AssetInitService;
use Arcaea_server_rs::Database;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenv::dotenv().ok();

    log::info!("Arcaea Server Database Initialization");
    log::info!("=====================================");

    // Connect to database
    log::info!("Connecting to database...");
    let pool = match Database::new().await {
        Ok(pool) => {
            log::info!("Database connection established");
            pool
        }
        Err(e) => {
            log::error!("Failed to connect to database: {e}");
            process::exit(1);
        }
    };

    // Check if database is already initialized
    log::info!("Checking if database is already initialized...");
    let character_count: i64 = match sqlx::query_scalar("SELECT COUNT(*) FROM `character`")
        .fetch_one(&pool)
        .await
    {
        Ok(count) => count,
        Err(e) => {
            log::error!("Failed to check character table: {e}");
            process::exit(1);
        }
    };

    if character_count > 0 {
        log::warn!(
            "Database appears to already contain data ({character_count} characters found)"
        );
        log::warn!("This will add duplicate data or may cause errors.");
        print!("Continue anyway? (y/N): ");

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_ok() {
            let input = input.trim().to_lowercase();
            if input != "y" && input != "yes" {
                log::info!("Initialization cancelled by user");
                process::exit(0);
            }
        } else {
            log::error!("Failed to read user input");
            process::exit(1);
        }
    }

    // Initialize asset service
    let asset_init_service = AssetInitService::new(pool);

    // Run initialization
    log::info!("Starting database initialization...");
    match asset_init_service.initialize_all().await {
        Ok(()) => {
            log::info!("Database initialization completed successfully!");
            log::info!("The following have been initialized:");
            log::info!("  - Characters (90 characters with stats and skills)");
            log::info!("  - Character cores and items");
            log::info!("  - Game items (cores, world songs, unlocks, banners)");
            log::info!("  - Purchase packs and singles");
            log::info!("  - Courses");
            log::info!("  - Roles and permissions");
            log::info!("  - Admin account (user: admin, code: 123456789)");
            log::info!("");
            log::info!("You can now start the Arcaea server!");
        }
        Err(e) => {
            log::error!("Database initialization failed: {e}");
            log::error!("Please check the error above and try again");
            process::exit(1);
        }
    }
}
