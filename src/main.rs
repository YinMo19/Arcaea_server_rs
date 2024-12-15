#[macro_use]
extern crate rocket;

pub mod core;
use core::config_manager::GAME_API_PREFIX;
use core::database::init_check_database_all;
use core::others::aggregate;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    init_check_database_all().await;
    
    let _rocket = rocket::build()
        // .attach(database::MessageLog::init())
        .mount(GAME_API_PREFIX, routes![aggregate])
        .launch()
        .await?;

    Ok(())
}
