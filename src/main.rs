#[macro_use]
extern crate rocket;

pub mod core;
use core::config_manager::GAME_API_PREFIX;
use core::others::aggregate;

#[launch]
fn rocket() -> _ {
    rocket::build().mount(GAME_API_PREFIX, routes![aggregate])
}
