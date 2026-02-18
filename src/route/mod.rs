pub mod admin;
pub mod auth;
pub mod common;
pub mod course;
pub mod download;
pub mod friend;
pub mod legacy;
pub mod mission;
pub mod multiplayer;

pub mod others;
pub mod present;
pub mod purchase;
pub mod score;
pub mod user;
pub mod world;

// Re-export commonly used route types for convenience
pub use common::{
    error_return, error_return_with_code, error_return_with_extra, success_return,
    success_return_no_value, ApiResponse, AuthGuard, RouteResult, CORS,
};
