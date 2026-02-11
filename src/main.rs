//! Arcaea Server Rust Implementation
//!
//! Main application entry point that sets up the Rocket web server
//! with database connections, services, and routes.

use rocket::fairing::AdHoc;
use rocket::{launch, Build, Rocket};

use std::collections::HashSet;
use Arcaea_server_rs::constants::GAME_API_PREFIX;
use Arcaea_server_rs::error::{bad_request, forbidden, internal_error, not_found, unauthorized};
use Arcaea_server_rs::route::others::bundle_download;
use Arcaea_server_rs::route::CORS;
use Arcaea_server_rs::service::{
    AssetManager, BundleService, CharacterService, DownloadService, ItemService,
    MultiplayerService, NotificationService, OperationManager, PresentService, PurchaseService,
    ScoreService, UserService, WorldService,
};
use Arcaea_server_rs::{config, Database, DbPool};

use rocket_prometheus::PrometheusMetrics;

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
    PresentService,
    WorldService,
    PurchaseService,
    ItemService,
    std::sync::Arc<AssetManager>,
    OperationManager,
    MultiplayerService,
) {
    // Initialize AssetManager with proper paths
    let asset_manager = std::sync::Arc::new(
        AssetManager::with_defaults(pool.clone())
            .with_song_folder(std::path::PathBuf::from("./songs"))
            .with_songlist_path(std::path::PathBuf::from("./songs/songlist"))
            .with_bundle_folder(std::path::PathBuf::from("./bundles"))
            .set_pre_calculate_hashes(true),
    );

    // Initialize asset cache on startup
    log::info!("Initializing asset cache...");
    if let Err(e) = asset_manager.initialize_cache().await {
        log::error!("Failed to initialize asset cache: {e}");
        std::process::exit(1);
    }
    log::info!("Asset cache initialized successfully");

    let user_service = UserService::new(pool.clone());
    let download_service = DownloadService::new(
        pool.clone(),
        asset_manager.clone(),
        None, // download_link_prefix
        3600, // download_time_gap_limit (1 hour)
        100,  // download_times_limit
    );
    let score_service = ScoreService::new(pool.clone());
    let notification_service = NotificationService::new(pool.clone());
    let item_service = ItemService::new(pool.clone());
    let mut bundle_service = BundleService::new(
        pool.clone(),
        std::path::PathBuf::from("bundles"),
        config::CONFIG.bundle_download_link_prefix.clone(),
    );

    // Initialize bundle service
    log::info!("Initializing bundle service...");
    if let Err(e) = bundle_service.initialize().await {
        log::error!("Failed to initialize bundle service: {e}");
        std::process::exit(1);
    }
    log::info!("Bundle service initialized successfully");

    let character_service = CharacterService::new(pool.clone());

    // initialise all the character.
    if let Err(e) = character_service.update_user_char_full().await {
        log::error!("Failed to initialize full character: {e}");
        std::process::exit(1);
    }

    let present_service = PresentService::new(pool.clone());
    let world_service = WorldService::new(pool.clone());
    let purchase_service = PurchaseService::new(pool.clone());
    let multiplayer_service = MultiplayerService::new(pool.clone());
    let operation_manager = OperationManager::new(
        asset_manager.clone(),
        std::sync::Arc::new(bundle_service.clone()),
        pool.clone(),
    );

    (
        user_service,
        download_service,
        score_service,
        notification_service,
        bundle_service,
        character_service,
        present_service,
        world_service,
        purchase_service,
        item_service,
        asset_manager,
        operation_manager,
        multiplayer_service,
    )
}

/// Configure the Rocket application
async fn configure_rocket() -> Rocket<Build> {
    let prometheus = PrometheusMetrics::new();

    let mut rocket = rocket::build()
        .attach(CORS)
        .attach(AdHoc::on_ignite("Database", |rocket| async {
            match Database::connect().await {
                Ok(pool) => {
                    log::info!("Database connection established");
                    rocket.manage(pool)
                }
                Err(e) => {
                    log::error!("Failed to connect to database: {e}");
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
                present_service,
                world_service,
                purchase_service,
                item_service,
                asset_manager,
                operation_manager,
                multiplayer_service,
            ) = init_services(pool).await;

            log::info!("Services initialized");
            rocket
                .manage(user_service)
                .manage(download_service)
                .manage(score_service)
                .manage(notification_service)
                .manage(bundle_service)
                .manage(character_service)
                .manage(present_service)
                .manage(world_service)
                .manage(purchase_service)
                .manage(item_service)
                .manage(asset_manager)
                .manage(operation_manager)
                .manage(multiplayer_service)
        }))
        // for prometheus telemetry
        .attach(prometheus.clone())
        .mount("/metrics", prometheus)
        .mount("/user", Arcaea_server_rs::route::user::routes())
        .mount(
            "/account",
            rocket::routes![
                Arcaea_server_rs::route::user::register,
                Arcaea_server_rs::route::user::user_delete,
                Arcaea_server_rs::route::user::email_resend_verify,
                Arcaea_server_rs::route::user::email_verify
            ],
        )
        .mount("/auth", Arcaea_server_rs::route::auth::routes())
        .mount("/", rocket::routes![bundle_download])
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::others::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::course::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::mission::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::friend::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::download::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::score::routes())
        .mount(
            GAME_API_PREFIX,
            Arcaea_server_rs::route::multiplayer::routes(),
        )
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::present::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::world::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::purchase::routes())
        .register(
            "/",
            rocket::catchers![
                not_found,
                internal_error,
                bad_request,
                unauthorized,
                forbidden,
            ],
        );

    let mut seen_old_prefixes = HashSet::new();
    for prefix in Arcaea_server_rs::constants::OLD_GAME_API_PREFIX {
        let p = normalize_prefix(prefix);
        if !p.is_empty() && seen_old_prefixes.insert(p.clone()) {
            rocket = rocket.mount(&p, Arcaea_server_rs::route::legacy::routes());
        }
    }
    for prefix in &config::CONFIG.old_game_api_prefix {
        let p = normalize_prefix(prefix);
        if !p.is_empty() && seen_old_prefixes.insert(p.clone()) {
            rocket = rocket.mount(&p, Arcaea_server_rs::route::legacy::routes());
        }
    }

    rocket
}

fn normalize_prefix(prefix: &str) -> String {
    let trimmed = prefix.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

/// Application entry point
#[launch]
async fn rocket() -> _ {
    // init log
    tracing_subscriber::fmt::init();

    // Print startup banner
    log::info!("Arcaea Server Rust Edition");
    log::info!("Version: {}", Arcaea_server_rs::ARCAEA_SERVER_VERSION);
    log::info!("Starting server...");

    // Load environment variables
    dotenv::dotenv().ok();

    // Configure and launch the application
    configure_rocket().await
}
