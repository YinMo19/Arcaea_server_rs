use crate::error::{ArcError, ArcResult};
use crate::service::ItemService;
use crate::DbPool;
use serde_json::{json, Value};
use std::collections::HashMap;

pub struct MissionService {
    pool: DbPool,
    item_service: ItemService,
}

impl MissionService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            item_service: ItemService::new(pool.clone()),
            pool,
        }
    }

    pub fn parse_mission_form(form: &HashMap<String, String>) -> Vec<String> {
        let mut missions = Vec::new();
        let mut idx = 1usize;
        loop {
            let key = format!("mission_{idx}");
            let Some(v) = form.get(&key) else {
                break;
            };
            missions.push(v.clone());
            idx += 1;
        }
        missions
    }

    pub async fn clear_mission(&self, user_id: i32, mission_id: &str) -> ArcResult<Value> {
        let _ = mission_rewards(mission_id)
            .ok_or_else(|| ArcError::no_data(format!("Mission `{mission_id}` not found"), 108))?;

        sqlx::query!(
            "INSERT INTO user_mission (user_id, mission_id, status) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE status = VALUES(status)",
            user_id,
            mission_id,
            2
        )
        .execute(&self.pool)
        .await?;

        Ok(json!({
            "mission_id": mission_id,
            "status": "cleared",
        }))
    }

    pub async fn claim_mission(&self, user_id: i32, mission_id: &str) -> ArcResult<Value> {
        let rewards = mission_rewards(mission_id)
            .ok_or_else(|| ArcError::no_data(format!("Mission `{mission_id}` not found"), 108))?;

        sqlx::query!(
            "INSERT INTO user_mission (user_id, mission_id, status) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE status = VALUES(status)",
            user_id,
            mission_id,
            4
        )
        .execute(&self.pool)
        .await?;

        for reward in &rewards {
            self.item_service
                .claim_item(user_id, reward.item_id, reward.item_type, reward.amount)
                .await?;
        }

        let items = rewards
            .iter()
            .map(|reward| {
                json!({
                    "id": reward.item_id,
                    "type": reward.item_type,
                    "amount": reward.amount,
                })
            })
            .collect::<Vec<_>>();

        Ok(json!({
            "mission_id": mission_id,
            "status": "claimed",
            "items": items,
        }))
    }
}

#[derive(Debug, Clone)]
struct MissionReward {
    item_id: &'static str,
    item_type: &'static str,
    amount: i32,
}

fn mission_rewards(mission_id: &str) -> Option<Vec<MissionReward>> {
    let one = |item_id: &'static str, item_type: &'static str, amount: i32| -> Vec<MissionReward> {
        vec![MissionReward {
            item_id,
            item_type,
            amount,
        }]
    };

    match mission_id {
        "mission_1_1_tutorial" => Some(one("fragment", "fragment", 10)),
        "mission_1_2_clearsong" => Some(one("fragment", "fragment", 10)),
        "mission_1_3_settings" => Some(one("fragment", "fragment", 10)),
        "mission_1_4_allsongsview" => Some(one("fragment", "fragment", 10)),
        "mission_1_5_fragunlock" => Some(one("core_generic", "core", 1)),
        "mission_1_end" => Some(one("fragment", "fragment", 100)),
        "mission_2_1_account" => Some(one("fragment", "fragment", 20)),
        "mission_2_2_profile" => Some(one("fragment", "fragment", 20)),
        "mission_2_3_partner" => Some(one("fragment", "fragment", 20)),
        "mission_2_4_usestamina" => Some(one("core_generic", "core", 1)),
        "mission_2_5_prologuestart" => Some(one("core_generic", "core", 1)),
        "mission_2_end" => Some(one("core_generic", "core", 3)),
        "mission_3_1_prsclear" => Some(one("fragment", "fragment", 50)),
        "mission_3_2_etherdrop" => Some(one("stamina", "stamina", 2)),
        "mission_3_3_step50" => Some(one("fragment", "fragment", 50)),
        "mission_3_4_frag60" => Some(one("stamina", "stamina", 2)),
        "mission_3_end" => Some(one("stamina", "stamina", 6)),
        "mission_4_1_exgrade" => Some(one("fragment", "fragment", 100)),
        "mission_4_2_potential350" => Some(one("stamina", "stamina", 2)),
        "mission_4_3_twomaps" => Some(one("fragment", "fragment", 100)),
        "mission_4_4_worldsongunlock" => Some(one("core_generic", "core", 3)),
        "mission_4_5_prologuefinish" => Some(one("stamina", "stamina", 2)),
        "mission_4_end" => Some(one("innocence", "world_song", 1)),
        "mission_5_1_songgrouping" => Some(one("fragment", "fragment", 50)),
        "mission_5_2_partnerlv12" => Some(one("fragment", "fragment", 250)),
        "mission_5_3_cores" => Some(one("core_generic", "core", 3)),
        "mission_5_4_courseclear" => Some(one("core_generic", "core", 3)),
        "mission_5_end" => Some(one("pick_ticket", "pick_ticket", 1)),
        _ => None,
    }
}
