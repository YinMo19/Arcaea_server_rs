use crate::core::constants;
use crate::core::models::{self, GameInfo, LevelStep, SingleCall};
use crate::core::user::user_me;
use models::{ArcError, DEFAULT_ERR};
use rocket::serde::json::{json, serde_json, Value};
use std::time::SystemTime;
use url::Url;

fn parse_calls<'r>(calls: &str) -> Vec<SingleCall<'r>> {
    serde_json::from_str::<Vec<SingleCall>>(calls).expect("Failed to parse calls.")
}

fn bundle_pack() -> Result<Value, ArcError<'static>> {
    // TODO
    Ok(json!({"result": "bundle_pack"}))
}

fn download_song() -> Result<Value, ArcError<'static>> {
    // TODO
    Ok(json!({"result": "download_song"}))
}

/// wraps the given value in a success response.
fn success_return(value: Option<Value>) -> Value {
    let mut response = json!({"success": true});
    if let Some(val) = value {
        response["value"] = val;
    }
    response
}

/// Get Game info.
fn game_info() -> Result<Value, ArcError<'static>> {
    let level_steps: Vec<LevelStep> = constants::LEVEL_STEPS
        .iter()
        .map(|&(level, level_exp)| LevelStep { level, level_exp })
        .collect();
    let curr_ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as i64;

    let game_info = GameInfo {
        max_stamina: constants::MAX_STAMINA,
        stamina_recover_tick: constants::STAMINA_RECOVER_TICK,
        core_exp: constants::CORE_EXP,
        curr_ts,
        level_steps,
        world_ranking_enabled: true,
        is_byd_chapter_unlocked: true,
    };

    Ok(success_return(Some(
        serde_json::to_value(game_info).expect("Failed to serialize game_info"),
    )))
}

/// handle endpoint from aggregate request.
fn handle_endpoint(path: &str) -> Result<Value, ArcError> {
    match path {
        "/user/me" => user_me(),
        "/purchase/bundle/pack" => bundle_pack(),
        "/serve/download/me/song" => bundle_pack(),
        "/game/info" => game_info(),
        "/present/me" => bundle_pack(),
        "/world/map/me" => bundle_pack(),
        "/score/song/friend" => bundle_pack(),
        "/purchase/bundle/bundle" => bundle_pack(),
        "/finale/progress" => bundle_pack(),
        "/purchase/bundle/single" => download_song(),
        _ => Err(ArcError::new("Unknown endpoint", 404, 404, None, 404)),
    }
}

/// Integrated requests
#[get("/compose/aggregate?<calls>")]
pub async fn aggregate(calls: &str) -> Value {
    let get_list = parse_calls(calls);
    if get_list.len() > 10 {
        return json!(DEFAULT_ERR);
    }

    let mut finally_response = json!({
        "success": true,
        "value": []
    });

    for call in get_list {
        let endpoint = call.endpoint.as_ref();
        let to_parse_url = format!("https://Arcaea.YinMo19.top{}", endpoint);
        let parsed_url = Url::parse(&to_parse_url).expect("Err parse url.");

        // let query_pairs = parsed_url.query_pairs();
        // let query_map: HashMap<_, _> = query_pairs.into_iter().collect();

        let mut resp_value = match handle_endpoint(parsed_url.path()) {
            Ok(resp) => resp,
            Err(_) => {
                return json!(DEFAULT_ERR);
            }
        };
        println!("{}", resp_value);

        let resp_obj = resp_value.as_object_mut().unwrap();
        if resp_obj.get("success") == Some(&Value::Bool(false)) {
            let mut error_response = json!({
                "success": false,
                "error_code": resp_obj.get("error_code").cloned().unwrap_or(Value::Null),
                "id": call.id
            });

            if let Some(extra) = resp_obj.get("extra") {
                error_response["extra"] = extra.clone();
            }

            return error_response;
        } else {
            let value = resp_obj.get("value").cloned().unwrap_or(Value::Null);
            finally_response["value"]
                .as_array_mut()
                .unwrap()
                .push(json!({
                    "id": call.id,
                    "value": value
                }));
        }
    }

    finally_response
}
