use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::CourseService;
use crate::DbPool;
use rocket::serde::json::Value;
use rocket::{get, routes, Route, State};

/// Course me endpoint
///
/// Python baseline: `GET /course/me`
#[get("/course/me")]
pub async fn course_me(pool: &State<DbPool>, auth: AuthGuard) -> RouteResult<Value> {
    let service = CourseService::new(pool.inner().clone());
    let result = service.get_course_me(auth.user_id).await?;
    Ok(success_return(result))
}

/// Get all course routes
pub fn routes() -> Vec<Route> {
    routes![course_me]
}
