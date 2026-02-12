//! Standalone Link Play server daemon.
//!
//! This binary separates link play from the main HTTP game server.
//! It provides:
//! - TCP control plane (authenticated + AES-GCM encrypted JSON)
//! - UDP data plane (binary protocol parsing)

use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes128Gcm, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::{thread_rng, Rng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::env;
use std::io;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

const PROTOCOL_NAME: [u8; 2] = [0x06, 0x16];
const PROTOCOL_VERSION: u8 = 0x0E;
type EncryptionPayload = ([u8; 12], [u8; 16], Vec<u8>);

#[derive(Debug, Clone)]
struct LinkplayConfig {
    host: String,
    udp_port: u16,
    tcp_port: u16,
    authentication: String,
    tcp_secret_key: String,
    tcp_max_length: usize,

    linkplay_unlock_length: usize,
    room_time_limit_usec: i64,
    cleanup_interval_sec: u64,

    command_interval_usec: i64,
    player_pre_timeout_usec: i64,
    player_timeout_usec: i64,

    countdown_song_ready_usec: i64,
    countdown_song_start_usec: i64,
    countdown_matching_usec: i64,
    countdown_select_song_usec: i64,
    countdown_select_difficulty_usec: i64,
    countdown_result_usec: i64,
}

impl LinkplayConfig {
    fn from_env() -> Self {
        let host = env::var("LINKPLAY_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let udp_port = env_u16("LINKPLAY_UDP_PORT", 10900);
        let tcp_port = env_u16("LINKPLAY_TCP_PORT", 10901);
        let authentication = env::var("LINKPLAY_AUTHENTICATION")
            .unwrap_or_else(|_| "my_link_play_server".to_string());
        let tcp_secret_key =
            env::var("LINKPLAY_TCP_SECRET_KEY").unwrap_or_else(|_| "1145141919810".to_string());
        let tcp_max_length = env_usize("LINKPLAY_TCP_MAX_LENGTH", 0x0FFF_FFFF);

        let linkplay_unlock_length = env_usize("LINKPLAY_UNLOCK_LENGTH", 1024);
        let room_time_limit_usec = env_i64("LINKPLAY_TIME_LIMIT_USEC", 3_600_000_000);
        let cleanup_interval_sec = env_u64("LINKPLAY_CLEANUP_INTERVAL_SEC", 15);

        let command_interval_usec = env_i64("LINKPLAY_COMMAND_INTERVAL_USEC", 1_000_000);
        let player_pre_timeout_usec = env_i64("LINKPLAY_PLAYER_PRE_TIMEOUT_USEC", 3_000_000);
        let player_timeout_usec = env_i64("LINKPLAY_PLAYER_TIMEOUT_USEC", 15_000_000);

        let countdown_song_ready_usec = env_i64("LINKPLAY_COUNTDOWN_SONG_READY_USEC", 4_000_000);
        let countdown_song_start_usec = env_i64("LINKPLAY_COUNTDOWN_SONG_START_USEC", 6_000_000);
        let countdown_matching_usec = env_i64("LINKPLAY_COUNTDOWN_MATCHING_USEC", 15_000_000);
        let countdown_select_song_usec = env_i64("LINKPLAY_COUNTDOWN_SELECT_SONG_USEC", 45_000_000);
        let countdown_select_difficulty_usec =
            env_i64("LINKPLAY_COUNTDOWN_SELECT_DIFFICULTY_USEC", 45_000_000);
        let countdown_result_usec = env_i64("LINKPLAY_COUNTDOWN_RESULT_USEC", 60_000_000);

        Self {
            host,
            udp_port,
            tcp_port,
            authentication,
            tcp_secret_key,
            tcp_max_length,
            linkplay_unlock_length,
            room_time_limit_usec,
            cleanup_interval_sec,
            command_interval_usec,
            player_pre_timeout_usec,
            player_timeout_usec,
            countdown_song_ready_usec,
            countdown_song_start_usec,
            countdown_matching_usec,
            countdown_select_song_usec,
            countdown_select_difficulty_usec,
            countdown_result_usec,
        }
    }

    fn tcp_aes_key(&self) -> [u8; 16] {
        padded_key_16(&self.tcp_secret_key)
    }
}

#[derive(Debug, Clone, Default)]
struct Score {
    difficulty: u8,
    score: u32,
    cleartype: u8,
    timer: u32,
    best_score_flag: u8,
    best_player_flag: u8,
    shiny_perfect_count: u16,
    perfect_count: u16,
    near_count: u16,
    miss_count: u16,
    early_count: u16,
    late_count: u16,
    healthy: i32,
}

impl Score {
    fn new() -> Self {
        Self {
            difficulty: 0xff,
            ..Default::default()
        }
    }

    fn clear(&mut self) {
        *self = Self::new();
    }
}

#[derive(Debug, Clone)]
struct Player {
    player_id: u64,
    player_name: [u8; 16],
    token: u64,

    character_id: u8,
    is_uncapped: u8,

    score: Score,
    last_score: Score,

    finish_flag: u8,
    player_state: u8,
    download_percent: u8,
    online: u8,

    last_timestamp: i64,
    extra_command_queue: Vec<Vec<u8>>,

    song_unlock: Vec<u8>,
    start_command_num: u32,

    voting: u16,
    player_index: u8,
    switch_2: u8,

    rating_ptt: u16,
    is_hide_rating: u8,
    is_staff: u8,
}

impl Player {
    fn empty(player_index: u8, unlock_len: usize) -> Self {
        let mut player_name = [0u8; 16];
        let empty_name = b"EmptyPlayer";
        player_name[..empty_name.len()].copy_from_slice(empty_name);

        Self {
            player_id: 0,
            player_name,
            token: 0,
            character_id: 0xff,
            is_uncapped: 0,
            score: Score::new(),
            last_score: Score::new(),
            finish_flag: 0,
            player_state: 1,
            download_percent: 0,
            online: 0,
            last_timestamp: 0,
            extra_command_queue: Vec::new(),
            song_unlock: vec![0; unlock_len],
            start_command_num: 0,
            voting: 0x8000,
            player_index,
            switch_2: 0,
            rating_ptt: 0,
            is_hide_rating: 0,
            is_staff: 0,
        }
    }

    fn name(&self) -> String {
        let end = self
            .player_name
            .iter()
            .position(|x| *x == 0)
            .unwrap_or(self.player_name.len());
        String::from_utf8_lossy(&self.player_name[..end]).to_string()
    }

    fn set_player_name(&mut self, name: &str) {
        self.player_name = [0u8; 16];
        let bytes = name.as_bytes();
        let n = bytes.len().min(16);
        self.player_name[..n].copy_from_slice(&bytes[..n]);
    }

    fn info_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32);
        push_le_u64(&mut out, self.player_id, 8);
        out.push(self.character_id);
        out.push(self.is_uncapped);
        out.push(self.score.difficulty);
        push_le_u64(&mut out, self.score.score as u64, 4);
        push_le_u64(&mut out, self.score.timer as u64, 4);
        out.push(self.score.cleartype);
        out.push(self.player_state);
        out.push(self.download_percent);
        out.push(self.online);

        push_le_u64(&mut out, self.voting as u64, 2);
        out.push(self.player_index);
        out.push(self.switch_2);
        push_le_u64(&mut out, self.rating_ptt as u64, 2);
        out.push(self.is_hide_rating);
        out.push(self.is_staff);

        out
    }

    fn last_score_info_bytes(&self) -> Vec<u8> {
        if self.player_id == 0 {
            let mut out = vec![0xff, 0xff];
            out.extend_from_slice(&[0u8; 23]);
            return out;
        }

        let mut out = Vec::with_capacity(25);
        out.push(self.character_id);
        out.push(self.last_score.difficulty);
        push_le_u64(&mut out, self.last_score.score as u64, 4);
        out.push(self.last_score.cleartype);
        out.push(self.last_score.best_score_flag);
        out.push(self.last_score.best_player_flag);
        push_le_u64(&mut out, self.last_score.shiny_perfect_count as u64, 2);
        push_le_u64(&mut out, self.last_score.perfect_count as u64, 2);
        push_le_u64(&mut out, self.last_score.near_count as u64, 2);
        push_le_u64(&mut out, self.last_score.miss_count as u64, 2);
        push_le_u64(&mut out, self.last_score.early_count as u64, 2);
        push_le_u64(&mut out, self.last_score.late_count as u64, 2);
        out.extend_from_slice(&self.last_score.healthy.to_le_bytes());
        out
    }
}

#[derive(Debug, Clone)]
struct Room {
    room_id: u64,
    room_code: String,
    share_token: String,

    countdown: u32,
    timestamp: i64,
    state: u8,
    song_idx: u16,
    last_song_idx: u16,

    song_unlock: Vec<u8>,

    host_id: u64,
    players: Vec<Player>,

    interval: u16,
    times: u64,

    round_mode: u8,
    is_public: u8,
    timed_mode: u8,

    selected_voter_player_id: u64,

    command_queue: Vec<Vec<u8>>,

    next_state_timestamp: i64,
}

impl Room {
    fn new(room_id: u64, room_code: String, share_token: String, unlock_len: usize) -> Self {
        let players = (0u8..4)
            .map(|idx| Player::empty(idx, unlock_len))
            .collect::<Vec<_>>();

        Self {
            room_id,
            room_code,
            share_token,
            countdown: 0xffff_ffff,
            timestamp: now_usec(),
            state: 0,
            song_idx: 0xffff,
            last_song_idx: 0xffff,
            song_unlock: vec![0xff; unlock_len],
            host_id: 0,
            players,
            interval: 1000,
            times: 100,
            round_mode: 1,
            is_public: 0,
            timed_mode: 0,
            selected_voter_player_id: 0,
            command_queue: Vec::new(),
            next_state_timestamp: 0,
        }
    }

    fn set_state(&mut self, state: u8) {
        self.state = state;
        self.countdown = 0xffff_ffff;
    }

    fn command_queue_length(&self) -> u32 {
        self.command_queue.len() as u32
    }

    fn player_num(&self) -> usize {
        self.players.iter().filter(|p| p.player_id != 0).count()
    }

    fn is_enterable(&self) -> bool {
        let n = self.player_num();
        n > 0 && n < 4 && self.state == 2
    }

    fn is_matchable(&self) -> bool {
        let n = self.player_num();
        self.is_public == 1 && n > 0 && n < 4 && self.state == 1
    }

    fn is_playing(&self) -> bool {
        matches!(self.state, 4..=7)
    }

    fn get_players_info(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 * (32 + 1 + 16));
        for p in &self.players {
            out.extend_from_slice(&p.info_bytes());
            out.push(0);
            out.extend_from_slice(&p.player_name);
        }
        out
    }

    fn get_player_last_score(&self) -> Vec<u8> {
        if self.last_song_idx == 0xffff {
            let mut one = vec![0xff, 0xff];
            one.extend_from_slice(&[0u8; 23]);
            let mut out = Vec::with_capacity(25 * 4);
            for _ in 0..4 {
                out.extend_from_slice(&one);
            }
            return out;
        }

        let mut out = Vec::with_capacity(25 * 4);
        for p in &self.players {
            out.extend_from_slice(&p.last_score_info_bytes());
        }
        out
    }

    fn room_info_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        push_le_u64(&mut out, self.host_id, 8);
        out.push(self.state);
        push_le_u64(&mut out, self.countdown as u64, 4);
        push_le_u64(&mut out, self.timestamp as u64, 8);
        push_le_u64(&mut out, self.song_idx as u64, 2);
        push_le_u64(&mut out, self.interval as u64, 2);
        push_le_u64(&mut out, self.times, 7);
        out.extend_from_slice(&self.get_player_last_score());
        push_le_u64(&mut out, self.last_song_idx as u64, 2);
        out.push(self.round_mode);
        out.push(self.is_public);
        out.push(self.timed_mode);
        push_le_u64(&mut out, self.selected_voter_player_id, 8);
        out
    }

    fn make_round(&mut self) {
        for i in 0..4 {
            if self.players[i].player_id == self.host_id {
                for j in 1..4 {
                    let idx = (i + j) % 4;
                    let player = &self.players[idx];
                    if player.player_id != 0 {
                        self.host_id = player.player_id;
                        info!(
                            "Player `{}` becomes the host of room `{}`",
                            player.name(),
                            self.room_code
                        );
                        break;
                    }
                }
                break;
            }
        }
    }

    fn delete_player(&mut self, player_index: usize, cfg: &LinkplayConfig) {
        if player_index >= 4 {
            return;
        }
        let player = self.players[player_index].clone();
        if player.player_id == 0 {
            return;
        }

        if player.player_id == self.host_id {
            self.make_round();
        }

        info!(
            "Player `{}` leaves room `{}`",
            player.name(),
            self.room_code
        );

        self.players[player_index] = Player::empty(player_index as u8, cfg.linkplay_unlock_length);
        self.update_song_unlock(cfg.linkplay_unlock_length);

        if matches!(self.state, 2 | 3) {
            self.set_state(1);
            self.song_idx = 0xffff;
            self.voting_clear();
        }

        if matches!(self.state, 1 | 2) && self.timed_mode == 1 && self.player_num() <= 1 {
            self.next_state_timestamp = 0;
            self.countdown = 0xffff_ffff;
        }
    }

    fn update_song_unlock(&mut self, unlock_len: usize) {
        let mut room_unlock = vec![0xff; unlock_len];
        for p in &self.players {
            if p.player_id == 0 {
                continue;
            }
            for (idx, target) in room_unlock.iter_mut().enumerate() {
                let src = p.song_unlock.get(idx).copied().unwrap_or(0);
                *target &= src;
            }
        }
        self.song_unlock = room_unlock;
    }

    fn check_player_online(&mut self, now: i64, cfg: &LinkplayConfig) -> (bool, Vec<usize>) {
        let mut kicked = false;
        let mut changed_indexes = Vec::new();

        for i in 0..4 {
            let p = &self.players[i];
            if p.player_id == 0 || p.last_timestamp == 0 {
                continue;
            }

            if now - p.last_timestamp >= cfg.player_timeout_usec {
                self.delete_player(i, cfg);
                kicked = true;
                changed_indexes.push(i);
            } else if p.online == 1 && now - p.last_timestamp >= cfg.player_pre_timeout_usec {
                self.players[i].online = 0;
                changed_indexes.push(i);
            }
        }

        (kicked, changed_indexes)
    }

    fn is_ready(&self, old_state: u8, player_state: u8) -> bool {
        if self.state != old_state {
            return false;
        }

        for p in &self.players {
            if p.player_id != 0 && (p.player_state != player_state || p.online == 0) {
                return false;
            }
        }
        true
    }

    fn is_finish(&self) -> bool {
        if self.state != 7 {
            return false;
        }

        for p in &self.players {
            if p.player_id != 0 && (p.finish_flag == 0 || p.online == 0) {
                return false;
            }
        }

        true
    }

    fn make_finish(&mut self) {
        self.set_state(8);
        self.last_song_idx = self.song_idx;

        let mut max_score: u32 = 0;
        let mut max_indexes = Vec::new();

        for i in 0..4 {
            if self.players[i].player_id == 0 {
                continue;
            }

            self.players[i].finish_flag = 0;
            self.players[i].last_score = self.players[i].score.clone();
            self.players[i].last_score.best_player_flag = 0;

            let score = self.players[i].last_score.score;
            if score > max_score {
                max_score = score;
                max_indexes.clear();
                max_indexes.push(i);
            } else if score == max_score {
                max_indexes.push(i);
            }
        }

        for i in max_indexes {
            self.players[i].last_score.best_player_flag = 1;
        }

        self.voting_clear();
        for p in &mut self.players {
            p.score.clear();
        }

        info!(
            "Room `{}` finishes song `{}`",
            self.room_code, self.song_idx
        );
        for p in &self.players {
            if p.player_id != 0 {
                info!(
                    "- Player `{}` - score={}, clear={}, diff={}",
                    p.name(),
                    p.last_score.score,
                    p.last_score.cleartype,
                    p.last_score.difficulty
                );
            }
        }
    }

    fn is_all_player_voted(&self) -> bool {
        if self.state != 2 {
            return false;
        }

        for p in &self.players {
            if p.player_id != 0 && p.voting == 0x8000 {
                return false;
            }
        }

        true
    }

    fn random_song(&mut self, unlock_len: usize) {
        let mut random_list = Vec::new();
        for i in 0..unlock_len {
            for j in 0..8 {
                if self.song_unlock.get(i).copied().unwrap_or(0) & (1 << j) != 0 {
                    random_list.push((i * 8 + j) as u16);
                }
            }
        }

        if random_list.is_empty() {
            self.song_idx = 0;
        } else {
            let idx = thread_rng().gen_range(0..random_list.len());
            self.song_idx = random_list[idx];
        }
    }

    fn make_voting(&mut self, unlock_len: usize) {
        self.set_state(3);
        self.selected_voter_player_id = 0;

        let mut vote_list = Vec::new();
        let mut vote_player_id = Vec::new();

        for p in &self.players {
            if p.player_id == 0 || p.voting == 0xffff || p.voting == 0x8000 {
                continue;
            }
            vote_list.push(p.voting);
            vote_player_id.push(p.player_id);
        }

        if vote_list.is_empty() {
            self.random_song(unlock_len);
        } else {
            let idx = thread_rng().gen_range(0..vote_list.len());
            self.song_idx = vote_list[idx].wrapping_mul(5);
            self.selected_voter_player_id = vote_player_id[idx];
        }

        info!("Room `{}` votes song `{}`", self.room_code, self.song_idx);
    }

    fn voting_clear(&mut self) {
        self.selected_voter_player_id = 0;
        for p in &mut self.players {
            p.voting = 0x8000;
        }
    }

    fn should_next_state(&mut self, now: i64, cfg: &LinkplayConfig) -> bool {
        if self.timed_mode == 0 && !matches!(self.state, 4..=6) {
            self.countdown = 0xffff_ffff;
            return false;
        }

        if self.countdown == 0xffff_ffff {
            if self.is_public == 1 && self.state == 1 {
                self.next_state_timestamp = now + cfg.countdown_matching_usec;
            } else if self.state == 2 {
                self.next_state_timestamp = now + cfg.countdown_select_song_usec;
            } else if self.state == 3 {
                self.next_state_timestamp = now + cfg.countdown_select_difficulty_usec;
            } else if self.state == 4 {
                self.next_state_timestamp = now + cfg.countdown_song_ready_usec;
            } else if matches!(self.state, 5 | 6) {
                self.next_state_timestamp = now + cfg.countdown_song_start_usec;
            } else if self.state == 8 {
                self.next_state_timestamp = now + cfg.countdown_result_usec;
            } else {
                return false;
            }
        }

        let diff = (self.next_state_timestamp - now) / 1000;
        if diff <= 0 {
            self.countdown = 0;
            true
        } else {
            self.countdown = u32::try_from(diff).unwrap_or(u32::MAX);
            false
        }
    }

    fn to_room_dict(&self) -> RoomDict {
        let mut players = Vec::new();
        for p in &self.players {
            if p.player_id == 0 {
                continue;
            }

            players.push(RoomPlayerDict {
                multiplay_player_id: p.player_id,
                name: p.name(),
                is_online: p.online == 1,
                character_id: p.character_id,
                is_uncapped: p.is_uncapped == 1,
                rating_ptt: p.rating_ptt as i32,
                is_hide_rating: p.is_hide_rating == 1,
                last_song: SongScoreDict {
                    difficulty: p.last_score.difficulty,
                    score: p.last_score.score as i32,
                    cleartype: p.last_score.cleartype,
                    shine_perfect: p.last_score.shiny_perfect_count,
                    perfect: p.last_score.perfect_count,
                    near: p.last_score.near_count,
                    miss: p.last_score.miss_count,
                    early: p.last_score.early_count,
                    late: p.last_score.late_count,
                },
                song: SongScoreSimpleDict {
                    difficulty: p.score.difficulty,
                    score: p.score.score as i32,
                    cleartype: p.score.cleartype,
                },
                player_state: p.player_state,
                last_timestamp: p.last_timestamp,
                is_host: p.player_id == self.host_id,
            });
        }

        RoomDict {
            room_id: self.room_id,
            room_code: self.room_code.clone(),
            share_token: self.share_token.clone(),
            state: self.state,
            song_idx: self.song_idx,
            last_song_idx: if self.is_playing() {
                0xffff
            } else {
                self.last_song_idx
            },
            host_id: self.host_id,
            players,
            round_mode: self.round_mode,
            last_timestamp: self.timestamp,
            is_enterable: self.is_enterable(),
            is_matchable: self.is_matchable(),
            is_playing: self.is_playing(),
            is_public: self.is_public == 1,
            timed_mode: self.timed_mode == 1,
        }
    }

    fn to_match_room_dict(&self) -> MatchRoomDict {
        MatchRoomDict {
            room_id: self.room_id,
            room_code: self.room_code.clone(),
            share_token: self.share_token.clone(),
            is_matchable: self.is_matchable(),
            next_state_timestamp: self.next_state_timestamp,
            song_unlock: BASE64.encode(&self.song_unlock),
            players: self
                .players
                .iter()
                .map(|p| MatchPlayerDict {
                    player_id: p.player_id,
                    name: p.name(),
                    rating_ptt: p.rating_ptt as i32,
                })
                .collect(),
        }
    }
}

struct CommandSender {
    timestamp: i64,
    random_code: Option<[u8; 8]>,
}

impl CommandSender {
    fn new(room: &mut Room) -> Self {
        let timestamp = now_usec();
        room.timestamp = timestamp + 1;
        Self {
            timestamp,
            random_code: None,
        }
    }

    fn random_code(&mut self) -> [u8; 8] {
        if let Some(code) = self.random_code {
            return code;
        }

        let mut code = [0u8; 8];
        thread_rng().fill_bytes(&mut code[..4]);
        code[4..].fill(0);
        self.random_code = Some(code);
        code
    }

    fn set_random_code(&mut self, code: [u8; 8]) {
        self.random_code = Some(code);
    }

    fn command_encode(parts: &[&[u8]]) -> Vec<u8> {
        let mut out = Vec::new();
        for p in parts {
            out.extend_from_slice(p);
        }

        let rem = out.len() % 16;
        if rem != 0 {
            let pad = 16 - rem;
            out.extend(std::iter::repeat_n(pad as u8, pad));
        }

        out
    }

    fn command_prefix(&self, room: &Room, command: u8) -> Vec<u8> {
        let mut len = room.command_queue_length();
        if (0x10..=0x1f).contains(&command) {
            len = len.saturating_add(1);
        }

        let mut out = Vec::new();
        out.extend_from_slice(&PROTOCOL_NAME);
        out.push(command);
        out.push(PROTOCOL_VERSION);
        push_le_u64(&mut out, room.room_id, 8);
        push_le_u64(&mut out, len as u64, 4);
        out
    }

    fn command_0c(&mut self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x0c);
        let random_code = self.random_code();

        let state = vec![room.state];
        let countdown = room.countdown.to_le_bytes();
        let timestamp = (self.timestamp as u64).to_le_bytes();

        Self::command_encode(&[&prefix, &random_code, &state, &countdown, &timestamp])
    }

    fn command_0d(&mut self, room: &Room, code: u8) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x0d);
        let random_code = self.random_code();
        Self::command_encode(&[&prefix, &random_code, &[code]])
    }

    fn command_0e(&self, room: &Room, player_index: usize) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x0e);
        let p = &room.players[player_index.min(3)];
        let info = p.info_bytes();
        let last_score = p.last_score.score.to_le_bytes();
        let last_timer = p.last_score.timer.to_le_bytes();
        let zeros = [0u8; 4];

        Self::command_encode(&[&prefix, &info, &last_score, &zeros, &last_timer, &zeros])
    }

    fn command_0f(&self, room: &Room, player_index: usize, song_idx: u16) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x0f);
        let player_id = room.players[player_index.min(3)].player_id.to_le_bytes();
        let song = song_idx.to_le_bytes();
        Self::command_encode(&[&prefix, &player_id, &song])
    }

    fn command_10(&mut self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x10);
        let random_code = self.random_code();
        let host = room.host_id.to_le_bytes();
        Self::command_encode(&[&prefix, &random_code, &host])
    }

    fn command_11(&mut self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x11);
        let random_code = self.random_code();
        let info = room.get_players_info();
        Self::command_encode(&[&prefix, &random_code, &info])
    }

    fn command_12(&mut self, room: &Room, player_index: usize) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x12);
        let random_code = self.random_code();
        let idx = [u8::try_from(player_index.min(255)).unwrap_or(0)];
        let info = room.players[player_index.min(3)].info_bytes();
        Self::command_encode(&[&prefix, &random_code, &idx, &info])
    }

    fn command_13(&mut self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x13);
        let random_code = self.random_code();
        let info = room.room_info_bytes();
        Self::command_encode(&[&prefix, &random_code, &info])
    }

    fn command_14(&mut self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x14);
        let random_code = self.random_code();
        Self::command_encode(&[&prefix, &random_code, &room.song_unlock])
    }

    fn command_15(&self, room: &Room) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x15);
        let players = room.get_players_info();
        let room_info = room.room_info_bytes();
        Self::command_encode(&[&prefix, &players, &room.song_unlock, &room_info])
    }

    fn command_21(&self, room: &Room, player_index: usize, sticker_id: u16) -> Vec<u8> {
        let prefix = self.command_prefix(room, 0x21);
        let player_id = room.players[player_index.min(3)].player_id.to_le_bytes();
        let sticker = sticker_id.to_le_bytes();
        Self::command_encode(&[&prefix, &player_id, &sticker])
    }
}

struct CommandParser<'a> {
    room: &'a mut Room,
    player_index: usize,
    cfg: &'a LinkplayConfig,
    sender: CommandSender,
    command: &'a [u8],
}

impl<'a> CommandParser<'a> {
    fn new(
        room: &'a mut Room,
        player_index: usize,
        cfg: &'a LinkplayConfig,
        command: &'a [u8],
    ) -> Self {
        let sender = CommandSender::new(room);
        Self {
            room,
            player_index: player_index.min(3),
            cfg,
            sender,
            command,
        }
    }

    fn get_commands(mut self) -> Vec<Vec<u8>> {
        let returned = self.dispatch_command();

        let mut out = Vec::new();

        let client_no = self.c_u32(12);
        let start = client_no.max(self.room.players[self.player_index].start_command_num);

        let mut flag_13 = false;
        for i in start as usize..self.room.command_queue.len() {
            if self.room.command_queue[i].get(2).copied() == Some(0x13) {
                if flag_13 {
                    break;
                }
                flag_13 = true;
            }
            out.push(self.room.command_queue[i].clone());
        }

        if !self.room.players[self.player_index]
            .extra_command_queue
            .is_empty()
        {
            let all = std::mem::take(&mut self.room.players[self.player_index].extra_command_queue);
            if all.len() > 12 {
                out.extend_from_slice(&all[all.len() - 12..]);
            } else {
                out.extend_from_slice(&all);
            }
        }

        if let Some(mut cmds) = returned {
            out.append(&mut cmds);
        }

        out
    }

    fn dispatch_command(&mut self) -> Option<Vec<Vec<u8>>> {
        let cmd = self.c_u8(2);

        match cmd {
            0x01 => self.command_01(),
            0x02 => self.command_02(),
            0x03 => self.command_03(),
            0x04 => self.command_04(),
            0x06 => self.command_06(),
            0x07 => self.command_07(),
            0x08 => self.command_08(),
            0x09 => self.command_09(),
            0x0a => self.command_0a(),
            0x0b => self.command_0b(),
            0x20 => self.command_20(),
            0x22 => self.command_22(),
            0x23 => self.command_23(),
            // Compatibility: some clients may send room setting update on 0x24.
            0x24 => self.command_22(),
            _ => {
                warn!(
                    "Unknown UDP command=0x{cmd:02x} room={} player_index={} len={}",
                    self.room.room_code,
                    self.player_index,
                    self.command.len()
                );
                None
            }
        }
    }

    fn c_u8(&self, idx: usize) -> u8 {
        self.command.get(idx).copied().unwrap_or(0)
    }

    fn c_u16(&self, idx: usize) -> u16 {
        read_u16_le(self.command, idx)
    }

    fn c_u32(&self, idx: usize) -> u32 {
        read_u32_le(self.command, idx)
    }

    fn c_u64(&self, idx: usize) -> u64 {
        read_u64_le(self.command, idx)
    }

    fn c_random_code(&self) -> [u8; 8] {
        let mut code = [0u8; 8];
        if self.command.len() >= 24 {
            code.copy_from_slice(&self.command[16..24]);
        }
        code
    }

    fn command_01(&mut self) -> Option<Vec<Vec<u8>>> {
        let player_id = self.c_u64(24);
        for p in &self.room.players {
            if p.player_id == player_id && p.online == 1 {
                self.room.host_id = player_id;
                info!(
                    "Player `{}` becomes the host of room `{}`",
                    p.name(),
                    self.room.room_code
                );
            }
        }

        self.sender.set_random_code(self.c_random_code());
        let cmd = self.sender.command_10(self.room);
        self.room.command_queue.push(cmd);
        None
    }

    fn command_02(&mut self) -> Option<Vec<Vec<u8>>> {
        if self.room.round_mode == 3 {
            warn!("Error: round_mode == 3 in command 02");
            return None;
        }

        self.sender.set_random_code(self.c_random_code());
        let song_idx = self.c_u16(24);

        let mut flag = 5u8;
        if self.room.state == 2 {
            flag = 0;
            self.room.set_state(3);
            self.room.song_idx = song_idx;

            let cmd11 = self.sender.command_11(self.room);
            self.room.command_queue.push(cmd11);
            let cmd13 = self.sender.command_13(self.room);
            self.room.command_queue.push(cmd13);
        }

        Some(vec![self.sender.command_0d(self.room, flag)])
    }

    fn command_03(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());

        let score = self.c_u32(24);
        let cleartype = self.c_u8(28);
        let difficulty = self.c_u8(29);
        let best_score_flag = self.c_u8(30);
        let shiny_perfect_count = self.c_u16(31);
        let perfect_count = self.c_u16(33);
        let near_count = self.c_u16(35);
        let miss_count = self.c_u16(37);
        let early_count = self.c_u16(39);
        let late_count = self.c_u16(41);
        let healthy = self.c_u32(43) as i32;

        let p = &mut self.room.players[self.player_index];
        p.score.score = score;
        p.score.cleartype = cleartype;
        p.score.difficulty = difficulty;
        p.score.best_score_flag = best_score_flag;
        p.score.shiny_perfect_count = shiny_perfect_count;
        p.score.perfect_count = perfect_count;
        p.score.near_count = near_count;
        p.score.miss_count = miss_count;
        p.score.early_count = early_count;
        p.score.late_count = late_count;
        p.score.healthy = healthy;
        p.finish_flag = 1;
        p.last_timestamp = p
            .last_timestamp
            .saturating_sub(self.cfg.command_interval_usec);

        self.room.last_song_idx = self.room.song_idx;

        let cmd12 = self.sender.command_12(self.room, self.player_index);
        self.room.command_queue.push(cmd12);

        if self.room.is_finish() {
            self.room.make_finish();
            let cmd13 = self.sender.command_13(self.room);
            self.room.command_queue.push(cmd13);
        }

        None
    }

    fn command_04(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());
        let target_player_id = self.c_u64(24);

        let mut flag = 2u8;
        if self.room.players[self.player_index].player_id == self.room.host_id
            && target_player_id != self.room.host_id
        {
            for i in 0..4 {
                if self.room.players[i].player_id == target_player_id {
                    flag = 1;
                    self.room.delete_player(i, self.cfg);
                    let cmd12 = self.sender.command_12(self.room, i);
                    self.room.command_queue.push(cmd12);
                    let cmd14 = self.sender.command_14(self.room);
                    self.room.command_queue.push(cmd14);
                    break;
                }
            }
        }

        Some(vec![self.sender.command_0d(self.room, flag)])
    }

    fn command_06(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());
        self.room.set_state(1);
        self.room.song_idx = 0xffff;
        self.room.voting_clear();

        let cmd13 = self.sender.command_13(self.room);
        self.room.command_queue.push(cmd13);
        None
    }

    fn command_07(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());

        let start = 24;
        let end = start + self.cfg.linkplay_unlock_length;
        let mut unlock = vec![0u8; self.cfg.linkplay_unlock_length];
        if self.command.len() > start {
            let actual_end = self.command.len().min(end);
            let len = actual_end.saturating_sub(start);
            unlock[..len].copy_from_slice(&self.command[start..actual_end]);
        }

        self.room.players[self.player_index].song_unlock = unlock;
        self.room
            .update_song_unlock(self.cfg.linkplay_unlock_length);

        let cmd14 = self.sender.command_14(self.room);
        self.room.command_queue.push(cmd14);
        None
    }

    fn command_08(&mut self) -> Option<Vec<Vec<u8>>> {
        warn!("Command 08 is outdated");
        None
    }

    fn command_09(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());
        let now = self.sender.timestamp;

        if self.c_u32(12) == 0 {
            self.room.players[self.player_index].online = 1;
            self.room.set_state(1);
            self.room
                .update_song_unlock(self.cfg.linkplay_unlock_length);
            let start_no = self.room.command_queue_length();
            self.room.players[self.player_index].start_command_num = start_no;
            let cmd15 = self.sender.command_15(self.room);
            self.room.command_queue.push(cmd15);
            return None;
        }

        let mut flag_0c = false;

        if now - self.room.players[self.player_index].last_timestamp
            >= self.cfg.command_interval_usec
        {
            flag_0c = true;
            self.room.players[self.player_index].last_timestamp = now;
        }

        let (mut flag_13, changed_indexes) = self.room.check_player_online(now, self.cfg);
        for idx in changed_indexes {
            let cmd12 = self.sender.command_12(self.room, idx);
            self.room.command_queue.push(cmd12);
        }

        let mut flag_11 = false;
        let mut flag_12 = false;
        let mut deleted_current_player = false;

        if self.room.players[self.player_index].online == 0 {
            flag_12 = true;
            self.room.players[self.player_index].online = 1;
        }

        if self.room.timed_mode == 1
            && matches!(self.room.state, 1 | 2)
            && self.room.players[self.player_index].player_state == 8
        {
            self.room.delete_player(self.player_index, self.cfg);
            let cmd12 = self.sender.command_12(self.room, self.player_index);
            self.room.command_queue.push(cmd12);
            let cmd14 = self.sender.command_14(self.room);
            self.room.command_queue.push(cmd14);
            deleted_current_player = true;
        }

        if self.room.is_ready(1, 1)
            && ((self.room.player_num() > 1 && self.room.is_public == 0)
                || (self.room.is_public == 1 && self.room.player_num() == 4))
        {
            flag_13 = true;
            self.room.set_state(2);
        }

        if self.room.state == 1
            && self.room.is_public == 1
            && self.room.player_num() > 1
            && self.room.should_next_state(now, self.cfg)
        {
            flag_0c = true;
            flag_13 = true;
            self.room.set_state(2);
        }

        if matches!(self.room.state, 2 | 3) && self.room.player_num() < 2 {
            flag_13 = true;
            self.room.set_state(1);
        }

        if self.room.state == 2 && self.room.should_next_state(now, self.cfg) {
            flag_0c = true;
            self.room.set_state(3);
            flag_13 = true;
            if self.room.round_mode == 3 {
                self.room.make_voting(self.cfg.linkplay_unlock_length);
            } else {
                self.room.random_song(self.cfg.linkplay_unlock_length);
            }
        }

        let score_24 = self.c_u32(24);
        if !deleted_current_player {
            let new_player_state = self.c_u8(32);
            if self.room.players[self.player_index].player_state != new_player_state {
                flag_12 = true;
                self.room.players[self.player_index].player_state = new_player_state;
            }

            let new_diff = self.c_u8(33);
            if self.room.players[self.player_index].score.difficulty != new_diff
                && !matches!(self.room.players[self.player_index].player_state, 5..=8)
            {
                flag_12 = true;
                self.room.players[self.player_index].score.difficulty = new_diff;
            }

            let new_clear = self.c_u8(34);
            if self.room.players[self.player_index].score.cleartype != new_clear
                && !matches!(self.room.players[self.player_index].player_state, 7 | 8)
            {
                flag_12 = true;
                self.room.players[self.player_index].score.cleartype = new_clear;
            }

            let download = self.c_u8(35);
            if self.room.players[self.player_index].download_percent != download {
                flag_12 = true;
                self.room.players[self.player_index].download_percent = download;
            }

            let character_id = self.c_u8(36);
            if self.room.players[self.player_index].character_id != character_id {
                flag_12 = true;
                self.room.players[self.player_index].character_id = character_id;
            }

            let uncapped = self.c_u8(37);
            if self.room.players[self.player_index].is_uncapped != uncapped {
                flag_12 = true;
                self.room.players[self.player_index].is_uncapped = uncapped;
            }

            if self.room.state == 3 && self.room.players[self.player_index].score.score != score_24
            {
                flag_12 = true;
                self.room.players[self.player_index].score.score = score_24;
            }
        }

        if self.room.is_ready(3, 4)
            || (self.room.state == 3 && self.room.should_next_state(now, self.cfg))
        {
            flag_13 = true;
            flag_0c = true;
            self.room.set_state(4);

            if self.room.round_mode == 2 {
                self.room.make_round();
            }
            info!("Room `{}` starts playing", self.room.room_code);

            for p in &mut self.room.players {
                p.finish_flag = 0;
            }
        }

        if self.room.state == 4 && self.room.should_next_state(now, self.cfg) {
            self.room.set_state(5);
            flag_11 = true;
            flag_13 = true;
        }

        if self.room.state == 5 {
            flag_13 = true;
            if self.room.is_ready(5, 6) {
                self.room.set_state(6);
            }
            if self.room.is_ready(5, 7) {
                self.room.set_state(7);
            }
        }

        if matches!(self.room.state, 5 | 6) && self.room.should_next_state(now, self.cfg) {
            self.room.set_state(7);
            flag_13 = true;
        }

        if matches!(self.room.state, 7 | 8) {
            let player_now_timer = self.c_u32(28);
            let p = &mut self.room.players[self.player_index];
            if p.score.timer < player_now_timer || (player_now_timer == 0 && p.score.timer != 0) {
                p.last_score.timer = p.score.timer;
                p.last_score.score = p.score.score;
                p.score.timer = player_now_timer;
                p.score.score = score_24;
            }

            if p.score.timer != 0 || self.room.state != 8 {
                let cmd0e = self.sender.command_0e(self.room, self.player_index);
                for i in 0..4 {
                    self.room.players[i].extra_command_queue.push(cmd0e.clone());
                }
            }

            if self.room.is_ready(8, 1) {
                flag_13 = true;
                self.room.set_state(1);
                self.room.song_idx = 0xffff;
            }

            if self.room.state == 8 && self.room.should_next_state(now, self.cfg) {
                flag_0c = true;
                flag_13 = true;
                self.room.set_state(1);
                self.room.song_idx = 0xffff;
            }

            if self.room.timed_mode == 1
                && matches!(self.room.state, 1 | 2)
                && self.room.players[self.player_index].player_state == 8
            {
                self.room.delete_player(self.player_index, self.cfg);
                let cmd12 = self.sender.command_12(self.room, self.player_index);
                self.room.command_queue.push(cmd12);
                let cmd14 = self.sender.command_14(self.room);
                self.room.command_queue.push(cmd14);
            }

            if self.room.is_finish() {
                self.room.make_finish();
                flag_13 = true;
            }
        }

        if flag_11 {
            let cmd11 = self.sender.command_11(self.room);
            self.room.command_queue.push(cmd11);
        }
        if flag_12 {
            let cmd12 = self.sender.command_12(self.room, self.player_index);
            self.room.command_queue.push(cmd12);
        }
        if flag_13 {
            let cmd13 = self.sender.command_13(self.room);
            self.room.command_queue.push(cmd13);
        }

        if flag_0c {
            Some(vec![self.sender.command_0c(self.room)])
        } else {
            None
        }
    }

    fn command_0a(&mut self) -> Option<Vec<Vec<u8>>> {
        self.room.delete_player(self.player_index, self.cfg);

        let cmd12 = self.sender.command_12(self.room, self.player_index);
        self.room.command_queue.push(cmd12);
        let cmd13 = self.sender.command_13(self.room);
        self.room.command_queue.push(cmd13);
        let cmd14 = self.sender.command_14(self.room);
        self.room.command_queue.push(cmd14);

        None
    }

    fn command_0b(&mut self) -> Option<Vec<Vec<u8>>> {
        let song_idx = self.c_u16(16);
        let cmd = self
            .sender
            .command_0f(self.room, self.player_index, song_idx);

        for i in 0..4 {
            if i != self.player_index && self.room.players[i].online == 1 {
                self.room.players[i].extra_command_queue.push(cmd.clone());
            }
        }

        None
    }

    fn command_20(&mut self) -> Option<Vec<Vec<u8>>> {
        let sticker_id = self.c_u16(16);
        let cmd = self
            .sender
            .command_21(self.room, self.player_index, sticker_id);

        for i in 0..4 {
            if i != self.player_index && self.room.players[i].online == 1 {
                self.room.players[i].extra_command_queue.push(cmd.clone());
            }
        }

        None
    }

    fn command_22(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());

        self.room.is_public = self.c_u8(25);
        if self.room.is_public == 0 {
            self.room.round_mode = self.c_u8(24);
            self.room.timed_mode = self.c_u8(26);
        } else {
            self.room.round_mode = 3;
            self.room.timed_mode = 1;
            self.room.set_state(1);
        }

        let cmd11 = self.sender.command_11(self.room);
        self.room.command_queue.push(cmd11);
        let cmd13 = self.sender.command_13(self.room);
        self.room.command_queue.push(cmd13);

        Some(vec![self.sender.command_0d(self.room, 1)])
    }

    fn command_23(&mut self) -> Option<Vec<Vec<u8>>> {
        self.sender.set_random_code(self.c_random_code());

        // Keep Python behavior in voting paths: stale players should not block all-voted flow.
        let _ = self
            .room
            .check_player_online(self.sender.timestamp, self.cfg);

        if self.room.player_num() < 2 {
            return Some(vec![self.sender.command_0d(self.room, 6)]);
        }
        if self.room.state != 2 {
            return Some(vec![self.sender.command_0d(self.room, 5)]);
        }

        self.room.players[self.player_index].voting = self.c_u16(24);
        info!(
            "Player `{}` votes for song `{}`",
            self.room.players[self.player_index].name(),
            self.room.players[self.player_index].voting
        );

        let cmd12 = self.sender.command_12(self.room, self.player_index);
        self.room.command_queue.push(cmd12);

        if self.room.is_all_player_voted() {
            self.room.make_voting(self.cfg.linkplay_unlock_length);
            let cmd13 = self.sender.command_13(self.room);
            self.room.command_queue.push(cmd13);
        }

        Some(vec![self.sender.command_0d(self.room, 1)])
    }
}

#[derive(Debug, Clone)]
struct Session {
    token: u64,
    key: [u8; 16],
    room_id: u64,
    player_id: u64,
    player_index: usize,
}

#[derive(Debug, Default)]
struct Store {
    sessions: HashMap<u64, Session>,
    rooms: HashMap<u64, Room>,
    room_code_index: HashMap<String, u64>,
    share_token_index: HashMap<String, u64>,
    used_player_ids: HashSet<u64>,
}

impl Store {
    fn create_room(
        &mut self,
        cfg: &LinkplayConfig,
        name: String,
        song_unlock: Vec<u8>,
        rating_ptt: i32,
        is_hide_rating: bool,
        match_times: Option<i64>,
    ) -> Value {
        let room_id = self.generate_room_id();
        let room_code = self.generate_room_code();
        let share_token = self.generate_share_token();

        let mut room = Room::new(
            room_id,
            room_code.clone(),
            share_token.clone(),
            cfg.linkplay_unlock_length,
        );

        let player_id = self.generate_player_id();

        let mut host = Player::empty(0, cfg.linkplay_unlock_length);
        host.player_id = player_id;
        host.set_player_name(&name);
        host.song_unlock = normalize_unlock(song_unlock, cfg.linkplay_unlock_length);
        host.rating_ptt = i32_to_u16(rating_ptt);
        host.is_hide_rating = bool_to_u8(is_hide_rating);

        room.host_id = player_id;
        room.song_unlock = host.song_unlock.clone();
        room.players[0] = host;

        if match_times.is_some() {
            room.is_public = 1;
            room.round_mode = 3;
            room.timed_mode = 1;
        }

        let token = self.generate_host_token(room_id);
        let key = random_fixed_16();

        self.sessions.insert(
            token,
            Session {
                token,
                key,
                room_id,
                player_id,
                player_index: 0,
            },
        );

        self.room_code_index.insert(room_code.clone(), room_id);
        self.share_token_index.insert(share_token.clone(), room_id);
        self.rooms.insert(room_id, room);

        info!("TCP-Create room `{}` by player `{}`", room_code, name);

        json!({
            "code": 0,
            "data": {
                "room_code": room_code,
                "room_id": room_id,
                "token": token,
                "key": BASE64.encode(key),
                "player_id": player_id,
            }
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn join_room(
        &mut self,
        cfg: &LinkplayConfig,
        room_code: String,
        name: String,
        song_unlock: Vec<u8>,
        rating_ptt: i32,
        is_hide_rating: bool,
        match_times: Option<i64>,
    ) -> Value {
        let room_code_upper = room_code.to_ascii_uppercase();

        let Some(room_id) = self.room_code_index.get(&room_code_upper).copied() else {
            return err_code(1202);
        };

        let Some(room) = self.rooms.get(&room_id) else {
            return err_code(1202);
        };

        let player_num = room.player_num();
        if player_num == 4 {
            return err_code(1201);
        }
        if player_num == 0 {
            return err_code(1202);
        }
        if !matches!(room.state, 0..=2) || (room.is_public == 1 && match_times.is_none()) {
            return err_code(1205);
        }

        let Some(slot) = room.players.iter().position(|p| p.player_id == 0) else {
            return err_code(1201);
        };

        let token = self.generate_token();
        let key = random_fixed_16();
        let player_id = self.generate_player_id();

        let Some(room) = self.rooms.get_mut(&room_id) else {
            return err_code(1202);
        };

        let mut p = Player::empty(slot as u8, cfg.linkplay_unlock_length);
        p.player_id = player_id;
        p.token = token;
        p.set_player_name(&name);
        p.song_unlock = normalize_unlock(song_unlock, cfg.linkplay_unlock_length);
        p.rating_ptt = i32_to_u16(rating_ptt);
        p.is_hide_rating = bool_to_u8(is_hide_rating);

        room.players[slot] = p;
        room.update_song_unlock(cfg.linkplay_unlock_length);

        self.sessions.insert(
            token,
            Session {
                token,
                key,
                room_id,
                player_id,
                player_index: slot,
            },
        );

        info!("TCP-Player `{}` joins room `{}`", name, room_code_upper);

        json!({
            "code": 0,
            "data": {
                "room_code": room_code_upper,
                "room_id": room_id,
                "token": token,
                "key": BASE64.encode(key),
                "player_id": player_id,
                "song_unlock": BASE64.encode(&room.song_unlock),
            }
        })
    }

    fn update_room(&mut self, token: u64, rating_ptt: i32, is_hide_rating: bool) -> Value {
        let Some(session) = self.sessions.get(&token).cloned() else {
            return err_code(108);
        };

        let Some(room) = self.rooms.get_mut(&session.room_id) else {
            return err_code(108);
        };

        if room.players[session.player_index].player_id != session.player_id {
            return err_code(108);
        }

        room.players[session.player_index].rating_ptt = i32_to_u16(rating_ptt);
        room.players[session.player_index].is_hide_rating = bool_to_u8(is_hide_rating);

        let mut sender = CommandSender::new(room);
        let cmd12 = sender.command_12(room, session.player_index);
        room.command_queue.push(cmd12);

        info!("TCP-Room `{}` info update", room.room_code);

        json!({
            "code": 0,
            "data": {
                "room_code": room.room_code,
                "room_id": room.room_id,
                "key": BASE64.encode(session.key),
                "player_id": session.player_id,
                "song_unlock": BASE64.encode(&room.song_unlock),
            }
        })
    }

    fn get_rooms(&self, offset: usize, limit: usize) -> Value {
        let mut listed = 0usize;
        let mut skipped = 0usize;
        let mut rooms = Vec::new();
        let mut reach_limit = false;
        let mut has_more = false;

        let mut all_rooms: Vec<&Room> = self.rooms.values().collect();
        all_rooms.sort_by_key(|x| x.room_id);

        for room in all_rooms {
            if room.player_num() == 0 {
                continue;
            }

            if skipped < offset {
                skipped += 1;
                continue;
            }

            if reach_limit {
                has_more = true;
                break;
            }

            listed += 1;
            rooms.push(room.to_room_dict());

            if listed >= limit.min(100) {
                reach_limit = true;
            }
        }

        json!({
            "code": 0,
            "data": {
                "amount": listed,
                "offset": offset,
                "limit": limit.min(100),
                "has_more": has_more,
                "rooms": rooms,
            }
        })
    }

    fn select_room(&self, room_code: Option<String>, share_token: Option<String>) -> Value {
        let room = if let Some(code) = room_code {
            let code = code.to_ascii_uppercase();
            let room_id = self.room_code_index.get(&code).copied();
            room_id.and_then(|id| self.rooms.get(&id))
        } else if let Some(token) = share_token {
            let room_id = self.share_token_index.get(&token).copied();
            room_id.and_then(|id| self.rooms.get(&id))
        } else {
            None
        };

        let Some(room) = room else {
            return err_code(108);
        };

        json!({
            "code": 0,
            "data": {
                "room_id": room.room_id,
                "room_code": room.room_code,
                "share_token": room.share_token,
                "is_enterable": room.is_enterable(),
                "is_matchable": room.is_matchable(),
                "is_playing": room.is_playing(),
                "is_public": room.is_public == 1,
                "timed_mode": room.timed_mode == 1,
            }
        })
    }

    fn get_match_rooms(&self, limit: usize) -> Value {
        let mut rooms = Vec::new();

        let mut all_rooms: Vec<&Room> = self.rooms.values().collect();
        all_rooms.sort_by_key(|x| x.room_id);

        for room in all_rooms {
            if !room.is_matchable() {
                continue;
            }

            rooms.push(room.to_match_room_dict());
            if rooms.len() >= limit.min(100) {
                break;
            }
        }

        json!({
            "code": 0,
            "data": {
                "amount": rooms.len(),
                "rooms": rooms,
            }
        })
    }

    fn clear_player_session(&mut self, token: u64, cfg: &LinkplayConfig) {
        let Some(session) = self.sessions.remove(&token) else {
            return;
        };

        if let Some(room) = self.rooms.get_mut(&session.room_id) {
            if room.players[session.player_index].player_id == session.player_id {
                room.delete_player(session.player_index, cfg);
            }
        }

        self.used_player_ids.remove(&session.player_id);

        if let Some(room) = self.rooms.get(&session.room_id) {
            if room.player_num() == 0 {
                self.remove_room(session.room_id);
            }
        }
    }

    fn remove_room(&mut self, room_id: u64) {
        let Some(room) = self.rooms.remove(&room_id) else {
            return;
        };

        info!("Clean room `{}`", room.room_code);

        self.room_code_index.remove(&room.room_code);
        self.share_token_index.remove(&room.share_token);

        for p in &room.players {
            if p.player_id != 0 {
                self.used_player_ids.remove(&p.player_id);
            }
        }

        let stale_tokens = self
            .sessions
            .iter()
            .filter_map(|(token, session)| {
                if session.room_id == room_id {
                    Some(*token)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for token in stale_tokens {
            self.sessions.remove(&token);
        }
    }

    fn cleanup(&mut self, now: i64, cfg: &LinkplayConfig) {
        let stale_rooms = self
            .rooms
            .iter()
            .filter_map(|(room_id, room)| {
                if now - room.timestamp >= cfg.room_time_limit_usec {
                    Some(*room_id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for room_id in stale_rooms {
            self.remove_room(room_id);
        }

        let stale_sessions = self
            .sessions
            .iter()
            .filter_map(|(token, session)| {
                let room = self.rooms.get(&session.room_id)?;
                let p = room.players.get(session.player_index)?;
                if p.player_id == 0 {
                    return Some(*token);
                }
                if p.last_timestamp != 0 && now - p.last_timestamp >= cfg.room_time_limit_usec {
                    return Some(*token);
                }
                None
            })
            .collect::<Vec<_>>();

        for token in stale_sessions {
            self.clear_player_session(token, cfg);
        }
    }

    fn generate_room_id(&self) -> u64 {
        unique_random_u64(|x| !self.rooms.contains_key(&x) && x != 0)
    }

    fn generate_host_token(&self, room_id: u64) -> u64 {
        if !self.sessions.contains_key(&room_id) {
            room_id
        } else {
            self.generate_token()
        }
    }

    fn generate_token(&self) -> u64 {
        unique_random_u64(|x| !self.sessions.contains_key(&x) && x != 0)
    }

    fn generate_player_id(&mut self) -> u64 {
        let mut rng = thread_rng();
        loop {
            let id = rng.gen_range(1..=0xFF_FFFFu64);
            if !self.used_player_ids.contains(&id) {
                self.used_player_ids.insert(id);
                return id;
            }
        }
    }

    fn generate_room_code(&self) -> String {
        const LETTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let mut rng = thread_rng();

        loop {
            let mut code = String::with_capacity(6);
            for _ in 0..4 {
                code.push(LETTERS[rng.gen_range(0..LETTERS.len())] as char);
            }
            for _ in 0..2 {
                code.push(char::from(b'0' + rng.gen_range(0..10) as u8));
            }

            if !self.room_code_index.contains_key(&code) {
                return code;
            }
        }
    }

    fn generate_share_token(&self) -> String {
        const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        let mut rng = thread_rng();

        loop {
            let mut token = String::with_capacity(10);
            for _ in 0..10 {
                token.push(CHARS[rng.gen_range(0..CHARS.len())] as char);
            }

            if !self.share_token_index.contains_key(&token) {
                return token;
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct TcpRequest {
    endpoint: String,
    #[serde(default)]
    data: Value,
}

#[derive(Debug, Serialize)]
struct SongScoreDict {
    difficulty: u8,
    score: i32,
    cleartype: u8,
    shine_perfect: u16,
    perfect: u16,
    near: u16,
    miss: u16,
    early: u16,
    late: u16,
}

#[derive(Debug, Serialize)]
struct SongScoreSimpleDict {
    difficulty: u8,
    score: i32,
    cleartype: u8,
}

#[derive(Debug, Serialize)]
struct RoomPlayerDict {
    multiplay_player_id: u64,
    name: String,
    is_online: bool,
    character_id: u8,
    is_uncapped: bool,
    rating_ptt: i32,
    is_hide_rating: bool,
    last_song: SongScoreDict,
    song: SongScoreSimpleDict,
    player_state: u8,
    last_timestamp: i64,
    is_host: bool,
}

#[derive(Debug, Serialize)]
struct RoomDict {
    room_id: u64,
    room_code: String,
    share_token: String,
    state: u8,
    song_idx: u16,
    last_song_idx: u16,
    host_id: u64,
    players: Vec<RoomPlayerDict>,
    round_mode: u8,
    last_timestamp: i64,
    is_enterable: bool,
    is_matchable: bool,
    is_playing: bool,
    is_public: bool,
    timed_mode: bool,
}

#[derive(Debug, Serialize)]
struct MatchPlayerDict {
    player_id: u64,
    name: String,
    rating_ptt: i32,
}

#[derive(Debug, Serialize)]
struct MatchRoomDict {
    room_id: u64,
    room_code: String,
    share_token: String,
    is_matchable: bool,
    next_state_timestamp: i64,
    song_unlock: String,
    players: Vec<MatchPlayerDict>,
}

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    let cfg = Arc::new(LinkplayConfig::from_env());
    info!(
        "Starting linkplayd on {} (UDP {}, TCP {})",
        cfg.host, cfg.udp_port, cfg.tcp_port
    );

    let state = Arc::new(RwLock::new(Store::default()));

    let tcp_state = state.clone();
    let tcp_cfg = cfg.clone();
    let tcp_task = tokio::spawn(async move {
        if let Err(err) = run_tcp_server(tcp_state, tcp_cfg).await {
            error!("TCP server stopped with error: {err}");
        }
    });

    let udp_state = state.clone();
    let udp_cfg = cfg.clone();
    let udp_task = tokio::spawn(async move {
        if let Err(err) = run_udp_server(udp_state, udp_cfg).await {
            error!("UDP server stopped with error: {err}");
        }
    });

    let cleaner_state = state.clone();
    let cleaner_cfg = cfg.clone();
    let cleanup_task = tokio::spawn(async move {
        run_cleanup_loop(cleaner_state, cleaner_cfg).await;
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl-C, shutting down linkplayd");
        }
        _ = tcp_task => {
            warn!("TCP task exited");
        }
        _ = udp_task => {
            warn!("UDP task exited");
        }
        _ = cleanup_task => {
            warn!("cleanup task exited");
        }
    }

    Ok(())
}

async fn run_cleanup_loop(state: Arc<RwLock<Store>>, cfg: Arc<LinkplayConfig>) {
    let mut ticker = tokio::time::interval(Duration::from_secs(cfg.cleanup_interval_sec.max(1)));
    loop {
        ticker.tick().await;
        let now = now_usec();
        let mut guard = state.write().await;
        guard.cleanup(now, &cfg);
    }
}

async fn run_tcp_server(state: Arc<RwLock<Store>>, cfg: Arc<LinkplayConfig>) -> io::Result<()> {
    let addr = format!("{}:{}", cfg.host, cfg.tcp_port);
    let listener = TcpListener::bind(&addr).await?;
    info!("Link Play TCP server listening on {addr}");

    loop {
        let (stream, _peer) = listener.accept().await?;

        let state = state.clone();
        let cfg = cfg.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_tcp_connection(stream, state, cfg).await {
                warn!("TCP connection closed: {err}");
            }
        });
    }
}

async fn handle_tcp_connection(
    mut stream: TcpStream,
    state: Arc<RwLock<Store>>,
    cfg: Arc<LinkplayConfig>,
) -> io::Result<()> {
    let auth_len = cfg.authentication.len();
    let mut auth_buf = vec![0u8; auth_len];
    stream.read_exact(&mut auth_buf).await?;

    if auth_buf != cfg.authentication.as_bytes() {
        stream.write_all(b"No authentication").await?;
        warn!("TCP-No authentication");
        return Ok(());
    }

    let mut len_buf = [0u8; 8];
    stream.read_exact(&mut len_buf).await?;
    let cipher_len = u64::from_le_bytes(len_buf) as usize;
    if cipher_len > cfg.tcp_max_length {
        stream.write_all(b"Body too long").await?;
        warn!("TCP-Body too long");
        return Ok(());
    }

    let mut iv = [0u8; 12];
    let mut tag = [0u8; 16];
    let mut ciphertext = vec![0u8; cipher_len];
    stream.read_exact(&mut iv).await?;
    stream.read_exact(&mut tag).await?;
    stream.read_exact(&mut ciphertext).await?;

    let plaintext = match decrypt_bytes(&cfg.tcp_aes_key(), &iv, &tag, ciphertext) {
        Ok(v) => v,
        Err(err) => {
            warn!("Failed to decrypt TCP payload: {err}");
            return Ok(());
        }
    };

    let req: TcpRequest = match serde_json::from_slice(&plaintext) {
        Ok(v) => v,
        Err(err) => {
            warn!("Invalid TCP JSON body: {err}");
            let response = err_code(999);
            let packet = encrypt_tcp_response(&cfg.tcp_aes_key(), &response)?;
            stream.write_all(&packet).await?;
            return Ok(());
        }
    };

    let response = handle_control_plane_request(&state, &cfg, req).await;
    let packet = encrypt_tcp_response(&cfg.tcp_aes_key(), &response)?;
    stream.write_all(&packet).await?;

    Ok(())
}

async fn handle_control_plane_request(
    state: &Arc<RwLock<Store>>,
    cfg: &Arc<LinkplayConfig>,
    req: TcpRequest,
) -> Value {
    let mut guard = state.write().await;

    match req.endpoint.as_str() {
        "debug" => json!({"code": 0, "data": {"hello_world": "ok"}}),
        "create_room" => {
            let Some(name) = data_get_string(&req.data, "name") else {
                return err_code(999);
            };
            let unlock = decode_unlock(&req.data, "song_unlock", cfg.linkplay_unlock_length);
            let rating_ptt = data_get_i32(&req.data, "rating_ptt").unwrap_or(0);
            let is_hide_rating = data_get_bool(&req.data, "is_hide_rating").unwrap_or(false);
            let match_times = data_get_i64(&req.data, "match_times");
            guard.create_room(cfg, name, unlock, rating_ptt, is_hide_rating, match_times)
        }
        "join_room" => {
            let Some(room_code) = data_get_string(&req.data, "room_code") else {
                return err_code(999);
            };
            let Some(name) = data_get_string(&req.data, "name") else {
                return err_code(999);
            };
            let unlock = decode_unlock(&req.data, "song_unlock", cfg.linkplay_unlock_length);
            let rating_ptt = data_get_i32(&req.data, "rating_ptt").unwrap_or(0);
            let is_hide_rating = data_get_bool(&req.data, "is_hide_rating").unwrap_or(false);
            let match_times = data_get_i64(&req.data, "match_times");
            guard.join_room(
                cfg,
                room_code,
                name,
                unlock,
                rating_ptt,
                is_hide_rating,
                match_times,
            )
        }
        "update_room" => {
            let Some(token) = data_get_u64(&req.data, "token") else {
                return err_code(999);
            };
            let rating_ptt = data_get_i32(&req.data, "rating_ptt").unwrap_or(0);
            let is_hide_rating = data_get_bool(&req.data, "is_hide_rating").unwrap_or(false);
            guard.update_room(token, rating_ptt, is_hide_rating)
        }
        "get_rooms" => {
            let offset = data_get_usize(&req.data, "offset").unwrap_or(0);
            let limit = data_get_usize(&req.data, "limit").unwrap_or(100);
            guard.get_rooms(offset, limit)
        }
        "select_room" => {
            let room_code = data_get_string(&req.data, "room_code");
            let share_token = data_get_string(&req.data, "share_token");
            guard.select_room(room_code, share_token)
        }
        "get_match_rooms" => {
            let limit = data_get_usize(&req.data, "limit").unwrap_or(100);
            guard.get_match_rooms(limit)
        }
        _ => err_code(999),
    }
}

async fn run_udp_server(state: Arc<RwLock<Store>>, cfg: Arc<LinkplayConfig>) -> io::Result<()> {
    let addr = format!("{}:{}", cfg.host, cfg.udp_port);
    let socket = UdpSocket::bind(&addr).await?;
    info!("Link Play UDP server listening on {addr}");

    let mut buf = vec![0u8; 8192];

    loop {
        let (n, peer) = socket.recv_from(&mut buf).await?;
        if n < 36 {
            continue;
        }

        let packet = &buf[..n];
        let token = read_u64_le(packet, 0);

        let session = {
            let guard = state.read().await;
            guard.sessions.get(&token).cloned()
        };

        let Some(session) = session else {
            continue;
        };

        let mut iv = [0u8; 12];
        let mut tag = [0u8; 16];
        iv.copy_from_slice(&packet[8..20]);
        tag.copy_from_slice(&packet[20..36]);

        let payload = match decrypt_bytes(&session.key, &iv, &tag, packet[36..].to_vec()) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if payload.len() < 3 || payload[0..2] != PROTOCOL_NAME {
            continue;
        }

        let commands = {
            let mut guard = state.write().await;

            let Some(room) = guard.rooms.get_mut(&session.room_id) else {
                continue;
            };

            let parser = CommandParser::new(room, session.player_index, &cfg, &payload);
            let mut commands = parser.get_commands();

            if room.players[session.player_index].player_id == 0 {
                guard.clear_player_session(session.token, &cfg);
                commands.retain(|cmd| cmd.get(2).copied() == Some(0x12));
            }

            commands
        };

        for cmd in commands {
            let (resp_iv, resp_tag, resp_cipher) = match encrypt_bytes(&session.key, &cmd) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let mut out = Vec::with_capacity(8 + 12 + 16 + resp_cipher.len());
            out.extend_from_slice(&session.token.to_le_bytes());
            out.extend_from_slice(&resp_iv);
            out.extend_from_slice(&resp_tag);
            out.extend_from_slice(&resp_cipher);
            let _ = socket.send_to(&out, peer).await;
        }
    }
}

fn err_code(code: i32) -> Value {
    json!({ "code": code })
}

fn encrypt_tcp_response(key: &[u8; 16], data: &Value) -> io::Result<Vec<u8>> {
    let plaintext = serde_json::to_vec(data)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;

    let (iv, tag, ciphertext) = encrypt_bytes(key, &plaintext)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let mut out = Vec::with_capacity(8 + 12 + 16 + ciphertext.len());
    out.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    out.extend_from_slice(&iv);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

fn encrypt_bytes(key: &[u8; 16], plaintext: &[u8]) -> Result<EncryptionPayload, String> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|e| e.to_string())?;

    let mut iv = [0u8; 12];
    thread_rng().fill_bytes(&mut iv);
    let nonce = Nonce::from_slice(&iv);

    let mut buffer = plaintext.to_vec();
    let tag = cipher
        .encrypt_in_place_detached(nonce, b"", &mut buffer)
        .map_err(|e| e.to_string())?;

    let mut tag_bytes = [0u8; 16];
    tag_bytes.copy_from_slice(&tag);

    Ok((iv, tag_bytes, buffer))
}

fn decrypt_bytes(
    key: &[u8; 16],
    iv: &[u8; 12],
    tag: &[u8; 16],
    mut ciphertext: Vec<u8>,
) -> Result<Vec<u8>, String> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(iv);
    let tag = aes_gcm::aead::generic_array::GenericArray::from_slice(tag);

    cipher
        .decrypt_in_place_detached(nonce, b"", &mut ciphertext, tag)
        .map_err(|e| e.to_string())?;

    Ok(ciphertext)
}

fn decode_unlock(data: &Value, key: &str, unlock_len: usize) -> Vec<u8> {
    let raw = data
        .get(key)
        .and_then(Value::as_str)
        .and_then(|s| BASE64.decode(s).ok())
        .unwrap_or_default();
    normalize_unlock(raw, unlock_len)
}

fn normalize_unlock(mut unlock: Vec<u8>, unlock_len: usize) -> Vec<u8> {
    if unlock.len() > unlock_len {
        unlock.truncate(unlock_len);
    } else if unlock.len() < unlock_len {
        unlock.resize(unlock_len, 0);
    }
    unlock
}

fn data_get_string(data: &Value, key: &str) -> Option<String> {
    data.get(key).and_then(Value::as_str).map(|s| s.to_string())
}

fn data_get_bool(data: &Value, key: &str) -> Option<bool> {
    let v = data.get(key)?;
    if let Some(b) = v.as_bool() {
        return Some(b);
    }
    if let Some(n) = v.as_i64() {
        return Some(n != 0);
    }
    if let Some(s) = v.as_str() {
        return Some(matches!(s, "1" | "true" | "True"));
    }
    None
}

fn data_get_i64(data: &Value, key: &str) -> Option<i64> {
    let v = data.get(key)?;
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        return s.parse::<i64>().ok();
    }
    None
}

fn data_get_u64(data: &Value, key: &str) -> Option<u64> {
    let v = data.get(key)?;
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

fn data_get_i32(data: &Value, key: &str) -> Option<i32> {
    data_get_i64(data, key).and_then(|x| i32::try_from(x).ok())
}

fn data_get_usize(data: &Value, key: &str) -> Option<usize> {
    data_get_u64(data, key).and_then(|x| usize::try_from(x).ok())
}

fn unique_random_u64<F>(mut predicate: F) -> u64
where
    F: FnMut(u64) -> bool,
{
    let mut rng = thread_rng();
    loop {
        let candidate = rng.next_u64();
        if predicate(candidate) {
            return candidate;
        }
    }
}

fn random_fixed_16() -> [u8; 16] {
    let mut out = [0u8; 16];
    thread_rng().fill_bytes(&mut out);
    out
}

fn push_le_u64(out: &mut Vec<u8>, value: u64, len: usize) {
    let raw = value.to_le_bytes();
    out.extend_from_slice(&raw[..len.min(raw.len())]);
}

fn read_u16_le(data: &[u8], start: usize) -> u16 {
    let mut bytes = [0u8; 2];
    if start < data.len() {
        let end = (start + 2).min(data.len());
        let len = end - start;
        bytes[..len].copy_from_slice(&data[start..end]);
    }
    u16::from_le_bytes(bytes)
}

fn read_u32_le(data: &[u8], start: usize) -> u32 {
    let mut bytes = [0u8; 4];
    if start < data.len() {
        let end = (start + 4).min(data.len());
        let len = end - start;
        bytes[..len].copy_from_slice(&data[start..end]);
    }
    u32::from_le_bytes(bytes)
}

fn read_u64_le(data: &[u8], start: usize) -> u64 {
    let mut bytes = [0u8; 8];
    if start < data.len() {
        let end = (start + 8).min(data.len());
        let len = end - start;
        bytes[..len].copy_from_slice(&data[start..end]);
    }
    u64::from_le_bytes(bytes)
}

fn bool_to_u8(v: bool) -> u8 {
    if v {
        1
    } else {
        0
    }
}

fn i32_to_u16(v: i32) -> u16 {
    if v <= 0 {
        0
    } else {
        u16::try_from(v).unwrap_or(u16::MAX)
    }
}

fn padded_key_16(input: &str) -> [u8; 16] {
    let mut out = [0u8; 16];
    let bytes = input.as_bytes();
    let n = bytes.len().min(16);
    out[..n].copy_from_slice(&bytes[..n]);
    out
}

fn now_usec() -> i64 {
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    (dur.as_secs() as i64) * 1_000_000 + (dur.subsec_micros() as i64)
}

fn env_u16(key: &str, default: u16) -> u16 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(default)
}

fn env_i64(key: &str, default: i64) -> i64 {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(default)
}

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(default)
}
