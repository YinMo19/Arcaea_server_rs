use crate::route::common::{success_return, AuthGuard, RouteResult};
use crate::service::PurchaseService;
use rocket::serde::json::Json;
use rocket::{get, post, routes, Route, State};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pack purchase response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackPurchaseResponse {
    pub user_id: i32,
    pub ticket: i32,
    pub packs: Vec<String>,
    pub singles: Vec<String>,
    pub characters: Vec<i32>,
}

/// Special item purchase response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialItemPurchaseResponse {
    pub user_id: i32,
    pub ticket: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stamina: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_stamina_ts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_mode_locked_end_ts: Option<i64>,
}

/// Stamina purchase response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaminaPurchaseResponse {
    pub user_id: i32,
    pub stamina: i32,
    pub max_stamina_ts: i64,
    pub next_fragstam_ts: i64,
    pub world_mode_locked_end_ts: i64,
}

/// Redeem response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemResponse {
    pub coupon: String,
}

/// Pack/Single purchase request structure
#[derive(Debug, Clone, Deserialize)]
pub struct PackSinglePurchaseRequest {
    #[serde(default)]
    pub pack_id: Option<String>,
    #[serde(default)]
    pub single_id: Option<String>,
}

/// Special item purchase request structure
#[derive(Debug, Clone, Deserialize)]
pub struct SpecialItemPurchaseRequest {
    pub item_id: String,
}

/// Redeem code request structure
#[derive(Debug, Clone, Deserialize)]
pub struct RedeemRequest {
    pub code: String,
}

/// Get pack purchase information endpoint
///
/// Returns available pack purchases with pricing and discount information.
#[get("/purchase/bundle/pack")]
pub async fn bundle_pack(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
) -> RouteResult<Vec<Value>> {
    let packs = purchase_service.get_pack_purchases(auth.user_id).await?;
    Ok(success_return(packs))
}

/// Get single song purchase information endpoint
///
/// Returns available single song purchases with pricing and discount information.
#[get("/purchase/bundle/single")]
pub async fn get_single(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
) -> RouteResult<Vec<Value>> {
    let singles = purchase_service.get_single_purchases(auth.user_id).await?;
    Ok(success_return(singles))
}

/// Get bundle purchases endpoint
///
/// Returns bundle purchases (always empty as per Python implementation).
#[get("/purchase/bundle/bundle")]
pub async fn bundle_bundle(purchase_service: &State<PurchaseService>) -> RouteResult<Vec<Value>> {
    let bundles = purchase_service.get_bundle_purchases().await?;
    Ok(success_return(bundles))
}

/// Buy pack or single endpoint
///
/// Handles the purchase of packs or singles, checking user tickets and granting items.
#[post("/purchase/me/pack", data = "<request>")]
pub async fn buy_pack_or_single(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
    request: Json<PackSinglePurchaseRequest>,
) -> RouteResult<Value> {
    let purchase_name = if let Some(ref pack_id) = request.pack_id {
        pack_id
    } else if let Some(ref single_id) = request.single_id {
        single_id
    } else {
        // Return empty success if no pack_id or single_id provided
        return Ok(success_return(serde_json::json!({})));
    };

    let result = purchase_service
        .buy_pack_or_single(auth.user_id, purchase_name)
        .await?;
    Ok(success_return(result))
}

/// Buy special item endpoint
///
/// Special purchases for world mode boost and stamina items.
/// Fixed price of 50 tickets for special items.
#[post("/purchase/me/item", data = "<request>")]
pub async fn buy_special(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
    request: Json<SpecialItemPurchaseRequest>,
) -> RouteResult<Value> {
    let result = purchase_service
        .buy_special_item(auth.user_id, &request.item_id)
        .await?;
    Ok(success_return(result))
}

/// Purchase stamina using fragments endpoint
///
/// Allows users to purchase stamina using fragments once per day.
/// Checks fragment stamina cooldown before allowing purchase.
#[post("/purchase/me/stamina/fragment")]
pub async fn purchase_stamina(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
) -> RouteResult<Value> {
    let result = purchase_service
        .purchase_stamina_with_fragment(auth.user_id)
        .await?;
    Ok(success_return(result))
}

/// Redeem code endpoint
///
/// Allows users to redeem codes for various rewards.
/// Checks if code is valid and hasn't been used by the user.
#[post("/purchase/me/redeem", data = "<request>")]
pub async fn redeem(
    purchase_service: &State<PurchaseService>,
    auth: AuthGuard,
    request: Json<RedeemRequest>,
) -> RouteResult<Value> {
    let result = purchase_service
        .redeem_code(auth.user_id, &request.code)
        .await?;
    Ok(success_return(result))
}

/// Get all purchase routes
pub fn routes() -> Vec<Route> {
    routes![
        bundle_pack,
        get_single,
        bundle_bundle,
        buy_pack_or_single,
        buy_special,
        purchase_stamina,
        redeem
    ]
}
