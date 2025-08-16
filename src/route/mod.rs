pub mod common;
pub mod others;
pub mod user;

// Re-export commonly used route types for convenience
pub use common::{
    error_return, error_return_with_code, error_return_with_extra, success_return, ApiResponse,
    AuthGuard, RouteResult, CORS,
};

use rocket::Route;

/// Get all application routes
pub fn get_all_routes() -> Vec<Route> {
    let mut routes = Vec::new();

    // Add user routes with /user prefix
    routes.extend(user::routes());

    // Add others routes (no prefix needed as they have their own paths)
    routes.extend(others::routes());

    routes
}
