use crate::core::models;
use models::ArcError;
use rocket::serde::json::{json, Value};

pub fn user_me() -> Result<Value, ArcError<'static>> {
    // TODO
    Ok(json!({"result": "user_me"}))
}
