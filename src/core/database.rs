use std::error::Error;

use rocket_db_pools::sqlx::{self, migrate::MigrateDatabase};
use sqlx::{migrate::Migrator, sqlite::SqlitePoolOptions, Sqlite};
use std::path::Path;

const CORE_DB_URL: &str = "sqlite://database/core.db";
const USER_DB_URL: &str = "sqlite://database/user.db";

/// Initialize the database.
pub async fn init_check_database_all() {
    let _ = create_database(CORE_DB_URL).await;
    let _ = create_database(USER_DB_URL).await;

    let _ = check_database("core", CORE_DB_URL).await;
    let _ = check_database("user", USER_DB_URL).await;
}

/// check if database exists, create if not.
async fn create_database(database_name: &str) -> Result<(), Box<dyn Error>> {
    if !Sqlite::database_exists(database_name)
        .await
        .unwrap_or(false)
    {
        println!("Creating database {}", database_name);
        match Sqlite::create_database(database_name).await {
            Ok(_) => println!("Create db:{} success", database_name),
            Err(error) => panic!("error: {}", error),
        }
    }
    Ok(())
}

/// Read the directory /database/migrations/<path>/<<timestamp>-<name>.sql>
/// and execute the sql file to migrate.
async fn check_database(path: &str, database_name: &str) -> Result<(), Box<dyn Error>> {
    let m = Migrator::new(Path::new("./database/migrations").join(path)).await?;
    let pool = SqlitePoolOptions::new().connect(database_name).await?;
    let _ = m.run(&pool).await;
    println!("Migrate {} successfully.", database_name);

    Ok(())
}
