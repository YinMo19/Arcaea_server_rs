use crate::config::Constants;
use crate::error::ArcResult;
use crate::DbPool;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct CourseService {
    pool: DbPool,
}

impl CourseService {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    pub async fn get_course_me(&self, user_id: i32) -> ArcResult<Value> {
        let core_ticket = sqlx::query!(
            "SELECT amount FROM user_item WHERE user_id = ? AND item_id = ? AND type = ?",
            user_id,
            "core_course_skip_purchase",
            "core"
        )
        .fetch_optional(&self.pool)
        .await?
        .and_then(|r| r.amount)
        .unwrap_or(0);

        let course_rows = sqlx::query!(
            "SELECT course_id, course_name, dan_name, style, gauge_requirement, flag_as_hidden_when_requirements_not_met, can_start FROM course ORDER BY course_id"
        )
        .fetch_all(&self.pool)
        .await?;

        let chart_rows = sqlx::query!(
            "SELECT course_id, song_id, difficulty, flag_as_hidden, song_index FROM course_chart ORDER BY course_id, song_index"
        )
        .fetch_all(&self.pool)
        .await?;

        let requirement_rows = sqlx::query!(
            "SELECT course_id, required_id FROM course_requirement ORDER BY course_id, required_id"
        )
        .fetch_all(&self.pool)
        .await?;

        let item_rows = sqlx::query!(
            "SELECT course_id, item_id, type, amount FROM course_item ORDER BY course_id, item_id, type"
        )
        .fetch_all(&self.pool)
        .await?;

        let user_course_rows = sqlx::query!(
            "SELECT course_id, high_score, best_clear_type FROM user_course WHERE user_id = ?",
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut chart_map: HashMap<String, Vec<Value>> = HashMap::new();
        for row in chart_rows {
            chart_map.entry(row.course_id).or_default().push(json!({
                "id": row.song_id,
                "difficulty": row.difficulty.unwrap_or(0),
                "flag_as_hidden": row.flag_as_hidden.unwrap_or(0) != 0,
            }));
        }

        let mut requirement_map: HashMap<String, Vec<String>> = HashMap::new();
        for row in requirement_rows {
            requirement_map
                .entry(row.course_id)
                .or_default()
                .push(row.required_id);
        }

        let mut item_map: HashMap<String, Vec<(String, String, i32)>> = HashMap::new();
        for row in item_rows {
            item_map.entry(row.course_id).or_default().push((
                row.item_id,
                row.r#type,
                row.amount.unwrap_or(1),
            ));
        }

        let mut user_course_map: HashMap<String, (i32, i32)> = HashMap::new();
        let mut completed_course_ids = HashSet::new();
        for row in user_course_rows {
            let high_score = row.high_score.unwrap_or(0);
            let best_clear_type = row.best_clear_type.unwrap_or(0);
            if best_clear_type != 0 {
                completed_course_ids.insert(row.course_id.clone());
            }
            user_course_map.insert(row.course_id, (high_score, best_clear_type));
        }

        let mut courses = Vec::with_capacity(course_rows.len());
        for row in course_rows {
            let course_id = row.course_id;
            let (high_score, best_clear_type) =
                user_course_map.get(&course_id).copied().unwrap_or((0, 0));

            let requirements = requirement_map
                .remove(&course_id)
                .unwrap_or_default()
                .into_iter()
                .map(|required_id| {
                    let mut req = json!({
                        "value": required_id,
                        "type": "course",
                    });
                    if completed_course_ids
                        .contains(req.get("value").and_then(Value::as_str).unwrap_or_default())
                    {
                        req["is_fullfilled"] = Value::Bool(true);
                    }
                    req
                })
                .collect::<Vec<_>>();

            let rewards = item_map
                .remove(&course_id)
                .unwrap_or_default()
                .into_iter()
                .map(|(item_id, item_type, amount)| {
                    course_reward_string(&item_id, &item_type, amount)
                })
                .collect::<Vec<_>>();

            courses.push(json!({
                "course_id": course_id,
                "course_name": row.course_name.unwrap_or_default(),
                "dan_name": row.dan_name.unwrap_or_default(),
                "style": row.style.unwrap_or(1),
                "gauge_requirement": row.gauge_requirement.unwrap_or_else(|| "default".to_string()),
                "flag_as_hidden_when_requirements_not_met": row.flag_as_hidden_when_requirements_not_met.unwrap_or(0) != 0,
                "can_start": row.can_start.unwrap_or(1) != 0,
                "requirements": requirements,
                "songs": chart_map.remove(&course_id).unwrap_or_default(),
                "rewards": rewards,
                "is_completed": best_clear_type != 0,
                "high_score": high_score,
                "best_clear_type": best_clear_type,
            }));
        }

        Ok(json!({
            "courses": courses,
            "stamina_cost": Constants::COURSE_STAMINA_COST,
            "course_skip_purchase_ticket": core_ticket,
        }))
    }
}

fn course_reward_string(item_id: &str, item_type: &str, amount: i32) -> String {
    match item_type {
        "fragment" => format!("fragment{amount}"),
        "core" => format!("{item_id}_{amount}"),
        _ => item_id.to_string(),
    }
}
