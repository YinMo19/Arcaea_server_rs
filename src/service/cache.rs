use redis::{aio::ConnectionManager, AsyncCommands};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::env;

#[derive(Clone)]
pub struct CacheService {
    manager: ConnectionManager,
    key_prefix: String,
}

impl CacheService {
    pub async fn from_env() -> Option<Self> {
        dotenv::dotenv().ok();

        let url = env::var("REDIS_URL")
            .ok()
            .filter(|value| !value.trim().is_empty())?;
        let key_prefix = env::var("REDIS_KEY_PREFIX")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "arcaea".to_string());

        let client = match redis::Client::open(url.as_str()) {
            Ok(client) => client,
            Err(e) => {
                log::warn!("Redis cache disabled: invalid REDIS_URL: {e}");
                return None;
            }
        };

        match client.get_connection_manager().await {
            Ok(manager) => {
                log::info!("Redis cache enabled");
                Some(Self {
                    manager,
                    key_prefix,
                })
            }
            Err(e) => {
                log::warn!("Redis cache disabled: connection failed: {e}");
                None
            }
        }
    }

    pub async fn get_string(&self, key: &str) -> Option<String> {
        let mut conn = self.manager.clone();
        match conn.get(self.key(key)).await {
            Ok(value) => value,
            Err(e) => {
                log::debug!("Redis GET failed for `{key}`: {e}");
                None
            }
        }
    }

    pub async fn set_string(&self, key: &str, value: &str, ttl_seconds: u64) {
        if ttl_seconds == 0 {
            return;
        }

        let mut conn = self.manager.clone();
        let result: redis::RedisResult<()> = conn.set_ex(self.key(key), value, ttl_seconds).await;
        if let Err(e) = result {
            log::debug!("Redis SETEX failed for `{key}`: {e}");
        }
    }

    pub async fn get_i32(&self, key: &str) -> Option<i32> {
        let mut conn = self.manager.clone();
        match conn.get(self.key(key)).await {
            Ok(value) => value,
            Err(e) => {
                log::debug!("Redis GET failed for `{key}`: {e}");
                None
            }
        }
    }

    pub async fn set_i32(&self, key: &str, value: i32, ttl_seconds: u64) {
        if ttl_seconds == 0 {
            return;
        }

        let mut conn = self.manager.clone();
        let result: redis::RedisResult<()> = conn.set_ex(self.key(key), value, ttl_seconds).await;
        if let Err(e) = result {
            log::debug!("Redis SETEX failed for `{key}`: {e}");
        }
    }

    pub async fn get_json<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let value = self.get_string(key).await?;
        match serde_json::from_str(&value) {
            Ok(value) => Some(value),
            Err(e) => {
                log::debug!("Redis JSON decode failed for `{key}`: {e}");
                None
            }
        }
    }

    pub async fn set_json<T>(&self, key: &str, value: &T, ttl_seconds: u64)
    where
        T: Serialize,
    {
        match serde_json::to_string(value) {
            Ok(value) => self.set_string(key, &value, ttl_seconds).await,
            Err(e) => log::debug!("Redis JSON encode failed for `{key}`: {e}"),
        }
    }

    pub async fn del(&self, key: &str) {
        let mut conn = self.manager.clone();
        let result: redis::RedisResult<()> = conn.del(self.key(key)).await;
        if let Err(e) = result {
            log::debug!("Redis DEL failed for `{key}`: {e}");
        }
    }

    fn key(&self, key: &str) -> String {
        format!("{}:{key}", self.key_prefix)
    }
}

pub fn env_ttl_seconds(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(default)
}
