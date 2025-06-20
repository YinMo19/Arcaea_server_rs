use sqlx::Row;
use std::error::Error;

use colored::*;
use rocket_db_pools::{
    sqlx::{self, migrate::MigrateDatabase},
    Database,
};
use sqlx::{
    migrate::Migrator,
    sqlite::{SqlitePoolOptions, SqliteRow},
    Sqlite,
};
use std::path::Path;

pub const CORE_DB_URL: &str = "sqlite://database/core.db";

#[derive(Database, Clone)]
#[database("core_db")]
pub struct Core(sqlx::SqlitePool);

impl Core {
    pub async fn token_get_id(self, token: &str) -> Result<i32, Box<dyn Error>> {
        let user_id = sqlx::query("select user_id from login where access_token = ?")
            .bind(token)
            .map(|row: SqliteRow| row.try_get::<i32, _>(0))
            .fetch_one(&self.0)
            .await??;

        Ok(user_id)
    }
}

/// Initialize the database.
pub async fn init_check_database_all() {
    if !Sqlite::database_exists(CORE_DB_URL).await.unwrap_or(false) {
        println!(
            "{} {}",
            "Creating database".green().bold(),
            CORE_DB_URL.blue().bold()
        );
        match Sqlite::create_database(CORE_DB_URL).await {
            Ok(_) => println!(
                "{}{} {}",
                "Create db:".green().bold(),
                CORE_DB_URL.blue().bold(),
                "success!".green().bold()
            ),
            Err(error) => panic!("{}{}", "error: ".red().bold(), error),
        }
    }

    let m = Migrator::new(Path::new("./migrations"))
        .await
        .expect("migrator new error.");
    let pool = SqlitePoolOptions::new()
        .connect(CORE_DB_URL)
        .await
        .expect("pool create error.");

    let _result = m.run(&pool).await.expect("migrate error.");
    println!(
        "{}{}{}",
        "Migrate ".green().bold(),
        CORE_DB_URL.blue().bold(),
        " successfully.".green().bold()
    );
}
