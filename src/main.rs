//! Arcaea Server Rust Implementation
//!
//! Main application entry point that sets up the Rocket web server
//! with database connections, services, and routes.

use rocket::fairing::AdHoc;
use rocket::{launch, Build, Rocket};

use Arcaea_server_rs::constants::GAME_API_PREFIX;
use Arcaea_server_rs::error::{bad_request, forbidden, internal_error, not_found, unauthorized};
use Arcaea_server_rs::route::CORS;
use Arcaea_server_rs::service::{
    BundleService, CharacterService, DownloadService, NotificationService, ScoreService,
    UserService,
};
use Arcaea_server_rs::{Database, DbPool};

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

/// Configure the Rocket application
async fn configure_rocket() -> Rocket<Build> {
    let prometheus = PrometheusMetrics::new();

    rocket::build()
        .attach(CORS)
        .attach(AdHoc::on_ignite("Database", |rocket| async {
            match Database::new().await {
                Ok(pool) => {
                    log::info!("Database connection established");
                    rocket.manage(pool)
                }
                Err(e) => {
                    log::error!("Failed to connect to database: {}", e);
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

            log::info!("Services initialized");
            rocket
                .manage(user_service)
                .manage(download_service)
                .manage(score_service)
                .manage(notification_service)
                .manage(bundle_service)
                .manage(character_service)
        }))
        // for prometheus telemetry
        .attach(prometheus.clone())
        .mount("/metrics", prometheus)
        .mount("/user", Arcaea_server_rs::route::user::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::others::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::download::routes())
        .mount(GAME_API_PREFIX, Arcaea_server_rs::route::score::routes())
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
