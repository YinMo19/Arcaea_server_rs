use crate::route::common::{error_return_with_code, ApiResponse};
use rocket::{get, post, routes, Route};
use std::path::PathBuf;

/// Legacy API root (Python baseline old prefix compatibility)
#[get("/")]
pub fn legacy_root_get() -> ApiResponse<()> {
    error_return_with_code(5)
}

/// Legacy API root (Python baseline old prefix compatibility)
#[post("/")]
pub fn legacy_root_post() -> ApiResponse<()> {
    error_return_with_code(5)
}

/// Legacy API catch-all (Python baseline old prefix compatibility)
#[get("/<path..>")]
pub fn legacy_any_get(path: PathBuf) -> ApiResponse<()> {
    let _ = path;
    error_return_with_code(5)
}

/// Legacy API catch-all (Python baseline old prefix compatibility)
#[post("/<path..>")]
pub fn legacy_any_post(path: PathBuf) -> ApiResponse<()> {
    let _ = path;
    error_return_with_code(5)
}

/// Get all legacy routes
pub fn routes() -> Vec<Route> {
    routes![
        legacy_root_get,
        legacy_root_post,
        legacy_any_get,
        legacy_any_post
    ]
}
