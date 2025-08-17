pub mod auth;
pub mod common;
pub mod download;
pub mod others;
pub mod score;
pub mod user;

// Re-export commonly used route types for convenience
pub use common::{
    error_return, error_return_with_code, error_return_with_extra, success_return, ApiResponse,
    AuthGuard, RouteResult, CORS,
};
