use crate::error::{ArcError, ArcResult};
use crate::DbPool;
use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

const LINKPLAY_TIMEOUT_SEC: u64 = 5;
const LINKPLAY_UNLOCK_LENGTH: usize = 1024;
const LINKPLAY_MATCH_GET_ROOMS_INTERVAL_SEC: i64 = 4;
const LINKPLAY_MATCH_TIMEOUT_SEC: i64 = 15;
const LINKPLAY_MATCH_MEMORY_CLEAN_INTERVAL_SEC: i64 = 60;
const LINKPLAY_MATCH_PTT_ABS: [i32; 8] = [5, 20, 50, 100, 200, 500, 1000, 2000];
const LINKPLAY_MATCH_UNLOCK_MIN: [i32; 8] = [1000, 800, 500, 300, 200, 100, 50, 1];

#[derive(Debug, Clone)]
struct LinkplayClientConfig {
    host: String,
    tcp_port: u16,
    udp_port: u16,
    display_host: String,
    authentication: String,
    tcp_aes_key: [u8; 16],
}

impl LinkplayClientConfig {
    fn from_env() -> Self {
        let host = env::var("LINKPLAY_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let tcp_port = env::var("LINKPLAY_TCP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(10901);
        let udp_port = env::var("LINKPLAY_UDP_PORT")
            .ok()
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(10900);
        let display_host = env::var("LINKPLAY_DISPLAY_HOST").unwrap_or_default();
        let authentication = env::var("LINKPLAY_AUTHENTICATION")
            .unwrap_or_else(|_| "my_link_play_server".to_string());
        let secret =
            env::var("LINKPLAY_TCP_SECRET_KEY").unwrap_or_else(|_| "1145141919810".to_string());

        Self {
            host,
            tcp_port,
            udp_port,
            display_host,
            authentication,
            tcp_aes_key: padded_key_16(&secret),
        }
    }

    fn endpoint_host(&self) -> String {
        if self.display_host.is_empty() {
            self.host.clone()
        } else {
            self.display_host.clone()
        }
    }
}

#[derive(Debug, Clone)]
struct MatchPlayer {
    user_id: i32,
    name: String,
    rating_ptt: i32,
    is_hide_rating: bool,
    song_unlock: Vec<u8>,
    last_match_timestamp_sec: i64,
    match_times: i32,
    match_room: Option<MatchRoomRef>,
}

#[derive(Debug, Clone)]
struct MatchRoomRef {
    room_code: String,
}

#[derive(Debug, Clone)]
struct MatchRoomCache {
    room_code: String,
    next_state_timestamp: i64,
    song_unlock: Vec<u8>,
    players: Vec<MatchRoomPlayerCache>,
}

#[derive(Debug, Clone)]
struct MatchRoomPlayerCache {
    player_id: u64,
    rating_ptt: i32,
}

#[derive(Debug, Default)]
struct MatchStoreState {
    last_get_rooms_timestamp_sec: i64,
    room_cache: Vec<MatchRoomCache>,
    player_queue: HashMap<i32, MatchPlayer>,
    last_memory_clean_timestamp_sec: i64,
}

#[derive(Debug, Clone)]
pub struct MultiplayerService {
    pool: DbPool,
    cfg: LinkplayClientConfig,
    state: Arc<Mutex<MatchStoreState>>,
}

impl MultiplayerService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            cfg: LinkplayClientConfig::from_env(),
            state: Arc::new(Mutex::new(MatchStoreState::default())),
        }
    }

    pub async fn room_create(
        &self,
        user_id: i32,
        client_song_map: &HashMap<String, Vec<bool>>,
    ) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let song_unlock = get_song_unlock(client_song_map);
        let (name, rating_ptt, is_hide_rating) = self.select_user_about_link_play(user_id).await?;

        let mut result = self
            .remote_create_room(
                &name,
                &song_unlock,
                rating_ptt,
                is_hide_rating,
                None,
                user_id,
            )
            .await?;
        self.add_endpoint_and_port(&mut result);
        Ok(result)
    }

    pub async fn room_join(
        &self,
        user_id: i32,
        room_code: &str,
        client_song_map: &HashMap<String, Vec<bool>>,
    ) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let song_unlock = get_song_unlock(client_song_map);
        let (name, rating_ptt, is_hide_rating) = self.select_user_about_link_play(user_id).await?;

        let mut result = self
            .remote_join_room(
                room_code,
                &name,
                &song_unlock,
                rating_ptt,
                is_hide_rating,
                None,
                user_id,
            )
            .await?;
        self.add_endpoint_and_port(&mut result);
        Ok(result)
    }

    pub async fn room_update(&self, user_id: i32, token: u64) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let (_, rating_ptt, is_hide_rating) = self.select_user_about_link_play(user_id).await?;

        let mut result = self
            .remote_update_room(token, rating_ptt, is_hide_rating, user_id)
            .await?;
        self.add_endpoint_and_port(&mut result);
        Ok(result)
    }

    pub async fn room_status(&self, share_token: &str) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let room_data = self.remote_select_room(None, Some(share_token)).await?;
        let room_code = room_data
            .get("room_code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        Ok(json!({ "roomId": room_code }))
    }

    pub async fn room_invite_share_token(&self, room_code: &str) -> ArcResult<String> {
        self.ensure_linkplay_available()?;
        let room_data = self.remote_select_room(Some(room_code), None).await?;
        let share_token = room_data
            .get("share_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        if share_token.is_empty() {
            return Err(ArcError::input("Missing share_token from link play server"));
        }
        Ok(share_token)
    }

    pub async fn user_linkplay_name(&self, user_id: i32) -> ArcResult<String> {
        let (name, _, _) = self.select_user_about_link_play(user_id).await?;
        Ok(name)
    }

    pub async fn matchmaking_join(
        &self,
        user_id: i32,
        client_song_map: &HashMap<String, Vec<bool>>,
    ) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let song_unlock = get_song_unlock(client_song_map);
        let (name, rating_ptt, is_hide_rating) = self.select_user_about_link_play(user_id).await?;

        {
            let mut state = self.state.lock().await;
            state.player_queue.insert(
                user_id,
                MatchPlayer {
                    user_id,
                    name,
                    rating_ptt,
                    is_hide_rating,
                    song_unlock,
                    last_match_timestamp_sec: now_sec(),
                    match_times: 0,
                    match_room: None,
                },
            );
        }

        let matched = self.match_internal(user_id).await?;
        if let Some(mut r) = matched {
            self.add_endpoint_and_port(&mut r);
            Ok(r)
        } else {
            Ok(json!({
                "userId": user_id,
                "status": 2
            }))
        }
    }

    pub async fn matchmaking_status(&self, user_id: i32) -> ArcResult<Value> {
        self.ensure_linkplay_available()?;
        let matched = self.match_internal(user_id).await?;
        if let Some(mut r) = matched {
            self.add_endpoint_and_port(&mut r);
            Ok(r)
        } else {
            Ok(json!({
                "userId": user_id,
                "status": 0
            }))
        }
    }

    pub async fn matchmaking_leave(&self, user_id: i32) -> ArcResult<()> {
        self.ensure_linkplay_available()?;
        let mut state = self.state.lock().await;
        state.player_queue.remove(&user_id);
        Ok(())
    }

    async fn match_internal(&self, user_id: i32) -> ArcResult<Option<Value>> {
        self.memory_clean().await;

        let user = {
            let state = self.state.lock().await;
            state.player_queue.get(&user_id).cloned()
        };
        let mut user = match user {
            Some(u) => u,
            None => {
                return Err(ArcError::Base {
                    message: format!("User `{user_id}` not found in match queue."),
                    error_code: 999,
                    api_error_code: -999,
                    extra_data: None,
                    status: 200,
                })
            }
        };

        if let Some(room_ref) = user.match_room.clone() {
            let joined = self
                .remote_join_room(
                    &room_ref.room_code,
                    &user.name,
                    &user.song_unlock,
                    user.rating_ptt,
                    user.is_hide_rating,
                    Some(user.match_times),
                    user.user_id,
                )
                .await?;

            let mut state = self.state.lock().await;
            state.player_queue.remove(&user_id);
            return Ok(Some(joined));
        }

        self.refresh_rooms().await?;

        let (rule, ptt_abs, unlock_min, room_cache) = {
            let state = self.state.lock().await;
            let rule = usize::min(
                user.match_times.max(0) as usize,
                usize::min(
                    LINKPLAY_MATCH_PTT_ABS.len() - 1,
                    LINKPLAY_MATCH_UNLOCK_MIN.len() - 1,
                ),
            );
            let ptt_abs = LINKPLAY_MATCH_PTT_ABS[rule];
            let unlock_min = LINKPLAY_MATCH_UNLOCK_MIN[rule];
            (rule, ptt_abs, unlock_min, state.room_cache.clone())
        };

        for room in room_cache {
            let mut ok = true;
            let mut num = 0;
            for p in &room.players {
                if p.player_id != 0 {
                    num += 1;
                    if (user.rating_ptt - p.rating_ptt).abs() >= ptt_abs {
                        ok = false;
                        break;
                    }
                }
            }

            let now_usec = now_usec();
            let enter_timing_ok = now_usec + 2_000_000 < room.next_state_timestamp
                || room.next_state_timestamp <= 0
                || num == 1;
            let unlock_ok =
                calc_available_chart_num(&user.song_unlock, &room.song_unlock) >= unlock_min;

            if ok && unlock_ok && enter_timing_ok {
                let joined = self
                    .remote_join_room(
                        &room.room_code,
                        &user.name,
                        &user.song_unlock,
                        user.rating_ptt,
                        user.is_hide_rating,
                        Some(user.match_times),
                        user.user_id,
                    )
                    .await?;

                let mut state = self.state.lock().await;
                state.room_cache.clear();
                state.last_get_rooms_timestamp_sec = 0;
                state.player_queue.remove(&user_id);
                return Ok(Some(joined));
            }
        }

        let now = now_sec();
        let peer_candidate = {
            let state = self.state.lock().await;
            let mut found: Option<i32> = None;
            for p in state.player_queue.values() {
                if p.user_id == user_id
                    || now - p.last_match_timestamp_sec > LINKPLAY_MATCH_TIMEOUT_SEC
                {
                    continue;
                }

                let new_rule = usize::min(rule, p.match_times.max(0) as usize);
                if (user.rating_ptt - p.rating_ptt).abs() < LINKPLAY_MATCH_PTT_ABS[new_rule]
                    && calc_available_chart_num(&user.song_unlock, &p.song_unlock)
                        >= LINKPLAY_MATCH_UNLOCK_MIN[new_rule]
                {
                    found = Some(p.user_id);
                    break;
                }
            }
            found
        };

        if let Some(peer_id) = peer_candidate {
            let created = self
                .remote_create_room(
                    &user.name,
                    &user.song_unlock,
                    user.rating_ptt,
                    user.is_hide_rating,
                    Some(user.match_times),
                    user.user_id,
                )
                .await?;

            let room_code = created
                .get("roomCode")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let mut state = self.state.lock().await;
            state.player_queue.remove(&user_id);
            if let Some(peer) = state.player_queue.get_mut(&peer_id) {
                peer.match_room = Some(MatchRoomRef { room_code });
            }
            return Ok(Some(created));
        }

        {
            let mut state = self.state.lock().await;
            if let Some(u) = state.player_queue.get_mut(&user_id) {
                u.match_times += 1;
                u.last_match_timestamp_sec = now;
                user.match_times = u.match_times;
            }
        }

        Ok(None)
    }

    async fn memory_clean(&self) {
        let now = now_sec();
        let mut state = self.state.lock().await;
        if now - state.last_memory_clean_timestamp_sec < LINKPLAY_MATCH_MEMORY_CLEAN_INTERVAL_SEC {
            return;
        }
        state.last_memory_clean_timestamp_sec = now;
        state
            .player_queue
            .retain(|_, p| now - p.last_match_timestamp_sec <= LINKPLAY_MATCH_TIMEOUT_SEC);
    }

    async fn refresh_rooms(&self) -> ArcResult<()> {
        let now = now_sec();
        {
            let state = self.state.lock().await;
            if now - state.last_get_rooms_timestamp_sec < LINKPLAY_MATCH_GET_ROOMS_INTERVAL_SEC {
                return Ok(());
            }
        }

        let remote_rooms = self.remote_get_match_rooms(100).await?;
        let mut state = self.state.lock().await;
        state.room_cache = remote_rooms;
        state.last_get_rooms_timestamp_sec = now;
        Ok(())
    }

    async fn select_user_about_link_play(&self, user_id: i32) -> ArcResult<(String, i32, bool)> {
        let row = sqlx::query!(
            "SELECT name, rating_ptt, is_hide_rating FROM user WHERE user_id = ?",
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Err(ArcError::no_data("No user.", 108));
        };

        let name = row.name.unwrap_or_default();
        let rating_ptt = row.rating_ptt.unwrap_or(0);
        let is_hide_rating = row.is_hide_rating.unwrap_or(0);

        Ok((name, rating_ptt, is_hide_rating != 0))
    }

    async fn remote_create_room(
        &self,
        name: &str,
        song_unlock: &[u8],
        rating_ptt: i32,
        is_hide_rating: bool,
        match_times: Option<i32>,
        user_id: i32,
    ) -> ArcResult<Value> {
        let mut data = json!({
            "name": name,
            "song_unlock": BASE64.encode(song_unlock),
            "rating_ptt": rating_ptt,
            "is_hide_rating": is_hide_rating,
        });
        if let Some(v) = match_times {
            data["match_times"] = Value::Number(serde_json::Number::from(v));
        }

        let r = self.remote_request("create_room", data).await?;
        let rd = r
            .get("data")
            .ok_or_else(|| ArcError::input("Missing data from link play server"))?;

        let room_code = rd
            .get("room_code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let room_id = rd
            .get("room_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let token = rd.get("token").and_then(Value::as_u64).unwrap_or_default();
        let key = rd.get("key").and_then(Value::as_str).unwrap_or_default();
        let player_id = rd
            .get("player_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();

        Ok(json!({
            "roomId": room_id.to_string(),
            "roomCode": room_code,
            "orderedAllowedSongs": BASE64.encode(song_unlock),
            "shareToken": "abcde12345",
            "userId": user_id,
            "playerId": player_id.to_string(),
            "token": token.to_string(),
            "key": key,
        }))
    }

    async fn remote_join_room(
        &self,
        room_code: &str,
        name: &str,
        song_unlock: &[u8],
        rating_ptt: i32,
        is_hide_rating: bool,
        match_times: Option<i32>,
        user_id: i32,
    ) -> ArcResult<Value> {
        let mut data = json!({
            "name": name,
            "song_unlock": BASE64.encode(song_unlock),
            "room_code": room_code,
            "rating_ptt": rating_ptt,
            "is_hide_rating": is_hide_rating,
        });
        if let Some(v) = match_times {
            data["match_times"] = Value::Number(serde_json::Number::from(v));
        }

        let r = self.remote_request("join_room", data).await?;
        let rd = r
            .get("data")
            .ok_or_else(|| ArcError::input("Missing data from link play server"))?;

        let room_code = rd
            .get("room_code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let room_id = rd
            .get("room_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let token = rd.get("token").and_then(Value::as_u64).unwrap_or_default();
        let key = rd.get("key").and_then(Value::as_str).unwrap_or_default();
        let player_id = rd
            .get("player_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let ordered_allowed_songs = rd
            .get("song_unlock")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        Ok(json!({
            "roomId": room_id.to_string(),
            "roomCode": room_code,
            "orderedAllowedSongs": ordered_allowed_songs,
            "shareToken": "abcde12345",
            "userId": user_id,
            "playerId": player_id.to_string(),
            "token": token.to_string(),
            "key": key,
        }))
    }

    async fn remote_update_room(
        &self,
        token: u64,
        rating_ptt: i32,
        is_hide_rating: bool,
        user_id: i32,
    ) -> ArcResult<Value> {
        let r = self
            .remote_request(
                "update_room",
                json!({
                    "token": token,
                    "rating_ptt": rating_ptt,
                    "is_hide_rating": is_hide_rating,
                }),
            )
            .await?;
        let rd = r
            .get("data")
            .ok_or_else(|| ArcError::input("Missing data from link play server"))?;

        let room_code = rd
            .get("room_code")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let room_id = rd
            .get("room_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let key = rd.get("key").and_then(Value::as_str).unwrap_or_default();
        let player_id = rd
            .get("player_id")
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let ordered_allowed_songs = rd
            .get("song_unlock")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        Ok(json!({
            "roomId": room_id.to_string(),
            "roomCode": room_code,
            "orderedAllowedSongs": ordered_allowed_songs,
            "shareToken": "abcde12345",
            "userId": user_id,
            "playerId": player_id.to_string(),
            "token": token.to_string(),
            "key": key,
        }))
    }

    async fn remote_select_room(
        &self,
        room_code: Option<&str>,
        share_token: Option<&str>,
    ) -> ArcResult<Value> {
        let r = self
            .remote_request(
                "select_room",
                json!({
                    "room_code": room_code,
                    "share_token": share_token
                }),
            )
            .await?;
        let rd = r
            .get("data")
            .ok_or_else(|| ArcError::input("Missing data from link play server"))?;
        Ok(rd.clone())
    }

    async fn remote_get_match_rooms(&self, limit: i32) -> ArcResult<Vec<MatchRoomCache>> {
        let r = self
            .remote_request("get_match_rooms", json!({ "limit": limit }))
            .await?;

        let rd = r
            .get("data")
            .ok_or_else(|| ArcError::input("Missing data from link play server"))?;
        let rooms = rd
            .get("rooms")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut out = Vec::with_capacity(rooms.len());
        for room in rooms {
            let room_code = room
                .get("room_code")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let next_state_timestamp = room
                .get("next_state_timestamp")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let song_unlock = room
                .get("song_unlock")
                .and_then(Value::as_str)
                .and_then(|s| BASE64.decode(s).ok())
                .unwrap_or_else(|| vec![0; LINKPLAY_UNLOCK_LENGTH]);

            let players = room
                .get("players")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|p| MatchRoomPlayerCache {
                    player_id: p.get("player_id").and_then(Value::as_u64).unwrap_or(0),
                    rating_ptt: p.get("rating_ptt").and_then(Value::as_i64).unwrap_or(0) as i32,
                })
                .collect::<Vec<_>>();

            out.push(MatchRoomCache {
                room_code,
                next_state_timestamp,
                song_unlock: normalize_unlock(song_unlock, LINKPLAY_UNLOCK_LENGTH),
                players,
            });
        }

        Ok(out)
    }

    async fn remote_request(&self, endpoint: &str, data: Value) -> ArcResult<Value> {
        let body = json!({
            "endpoint": endpoint,
            "data": data
        });
        let plaintext = serde_json::to_vec(&body)?;

        let mut iv = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut iv);

        let cipher = Aes128Gcm::new_from_slice(&self.cfg.tcp_aes_key)
            .map_err(|e| ArcError::input(format!("Invalid linkplay key: {e}")))?;
        let nonce = Nonce::from_slice(&iv);
        let mut ciphertext = plaintext;
        let tag = cipher
            .encrypt_in_place_detached(nonce, b"", &mut ciphertext)
            .map_err(|e| ArcError::input(format!("Failed to encrypt linkplay request: {e}")))?;

        let mut packet =
            Vec::with_capacity(self.cfg.authentication.len() + 8 + 12 + 16 + ciphertext.len());
        packet.extend_from_slice(self.cfg.authentication.as_bytes());
        packet.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
        packet.extend_from_slice(&iv);
        packet.extend_from_slice(tag.as_slice());
        packet.extend_from_slice(&ciphertext);

        let addr = format!("{}:{}", self.cfg.host, self.cfg.tcp_port);
        let mut stream = tokio::time::timeout(
            Duration::from_secs(LINKPLAY_TIMEOUT_SEC),
            TcpStream::connect(addr),
        )
        .await
        .map_err(|_| ArcError::Base {
            message: "Timeout when connecting to link play server.".to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?
        .map_err(|e| ArcError::Base {
            message: format!("Link play connection failed: {e}"),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?;

        tokio::time::timeout(
            Duration::from_secs(LINKPLAY_TIMEOUT_SEC),
            stream.write_all(&packet),
        )
        .await
        .map_err(|_| ArcError::Base {
            message: "Timeout when sending to link play server.".to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?
        .map_err(|e| ArcError::Base {
            message: format!("Link play send failed: {e}"),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?;

        let mut len_buf = [0u8; 8];
        tokio::time::timeout(
            Duration::from_secs(LINKPLAY_TIMEOUT_SEC),
            stream.read_exact(&mut len_buf),
        )
        .await
        .map_err(|_| ArcError::Base {
            message: "Timeout when waiting for data from link play server.".to_string(),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?
        .map_err(|e| ArcError::Base {
            message: format!("Link play read length failed: {e}"),
            error_code: 108,
            api_error_code: -999,
            extra_data: None,
            status: 400,
        })?;

        let cipher_len = u64::from_le_bytes(len_buf) as usize;
        let mut riv = [0u8; 12];
        let mut rtag = [0u8; 16];
        let mut rcipher = vec![0u8; cipher_len];

        stream
            .read_exact(&mut riv)
            .await
            .map_err(|e| ArcError::Base {
                message: format!("Link play read iv failed: {e}"),
                error_code: 108,
                api_error_code: -999,
                extra_data: None,
                status: 400,
            })?;
        stream
            .read_exact(&mut rtag)
            .await
            .map_err(|e| ArcError::Base {
                message: format!("Link play read tag failed: {e}"),
                error_code: 108,
                api_error_code: -999,
                extra_data: None,
                status: 400,
            })?;
        stream
            .read_exact(&mut rcipher)
            .await
            .map_err(|e| ArcError::Base {
                message: format!("Link play read cipher failed: {e}"),
                error_code: 108,
                api_error_code: -999,
                extra_data: None,
                status: 400,
            })?;

        let nonce = Nonce::from_slice(&riv);
        let tag = aes_gcm::aead::generic_array::GenericArray::from_slice(&rtag);
        cipher
            .decrypt_in_place_detached(nonce, b"", &mut rcipher, tag)
            .map_err(|e| ArcError::Base {
                message: format!("Failed to decrypt link play response: {e}"),
                error_code: 108,
                api_error_code: -999,
                extra_data: None,
                status: 400,
            })?;

        let recv: Value = serde_json::from_slice(&rcipher)?;
        let code = recv.get("code").and_then(Value::as_i64).unwrap_or(999) as i32;
        if code != 0 {
            return Err(ArcError::Base {
                message: format!("Link Play error code: {code}"),
                error_code: code,
                api_error_code: -999,
                extra_data: None,
                status: 400,
            });
        }
        Ok(recv)
    }

    fn ensure_linkplay_available(&self) -> ArcResult<()> {
        if self.cfg.host.trim().is_empty() {
            return Err(ArcError::Base {
                message: "The link play server is unavailable.".to_string(),
                error_code: 151,
                api_error_code: -999,
                extra_data: None,
                status: 404,
            });
        }
        Ok(())
    }

    fn add_endpoint_and_port(&self, value: &mut Value) {
        value["endPoint"] = Value::String(self.cfg.endpoint_host());
        value["port"] = Value::Number(serde_json::Number::from(self.cfg.udp_port));
    }
}

fn get_song_unlock(client_song_map: &HashMap<String, Vec<bool>>) -> Vec<u8> {
    let mut unlock = vec![0u8; LINKPLAY_UNLOCK_LENGTH];
    for (k, v) in client_song_map {
        let song_idx = match k.parse::<usize>() {
            Ok(n) => n,
            Err(_) => continue,
        };
        for i in 0..5usize {
            let enabled = v.get(i).copied().unwrap_or(false);
            if !enabled {
                continue;
            }
            let index = song_idx * 5 + i;
            if index >= LINKPLAY_UNLOCK_LENGTH * 8 {
                continue;
            }
            unlock[index / 8] |= 1 << (index % 8);
        }
    }
    unlock
}

fn calc_available_chart_num(a: &[u8], b: &[u8]) -> i32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| (x & y).count_ones() as i32)
        .sum()
}

fn normalize_unlock(mut unlock: Vec<u8>, unlock_len: usize) -> Vec<u8> {
    if unlock.len() > unlock_len {
        unlock.truncate(unlock_len);
    } else if unlock.len() < unlock_len {
        unlock.resize(unlock_len, 0);
    }
    unlock
}

fn padded_key_16(input: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = input.as_bytes();
    let n = bytes.len().min(16);
    out[..n].copy_from_slice(&bytes[..n]);
    out
}

fn now_sec() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs() as i64
}

fn now_usec() -> i64 {
    let d = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    d.as_secs() as i64 * 1_000_000 + d.subsec_micros() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchmakingJoinRequest {
    #[serde(rename = "clientSongMap")]
    pub client_song_map: HashMap<String, Vec<bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplayerUpdateRequest {
    pub token: Value,
}

impl MultiplayerUpdateRequest {
    pub fn token_u64(&self) -> ArcResult<u64> {
        value_as_u64(&self.token).ok_or_else(|| ArcError::input("Invalid token."))
    }
}

fn value_as_u64(v: &Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(n) = v.as_i64() {
        return u64::try_from(n).ok();
    }
    if let Some(s) = v.as_str() {
        return s.parse::<u64>().ok();
    }
    None
}
