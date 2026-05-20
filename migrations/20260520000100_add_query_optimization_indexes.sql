CREATE INDEX IF NOT EXISTS idx_user_user_code ON user (user_code);
CREATE INDEX IF NOT EXISTS idx_user_email ON user (email);
CREATE INDEX IF NOT EXISTS idx_user_world_rank_score ON user (world_rank_score);
CREATE INDEX IF NOT EXISTS idx_user_rating_list ON user (rating_ptt DESC, user_id ASC);

CREATE INDEX IF NOT EXISTS idx_login_user_time ON login (user_id, login_time);
CREATE INDEX IF NOT EXISTS idx_login_time_user ON login (login_time, user_id);
CREATE INDEX IF NOT EXISTS idx_login_user_device ON login (user_id, login_device(255));

CREATE INDEX IF NOT EXISTS idx_best_score_rank ON best_score (song_id, difficulty, score DESC, time_played DESC);
CREATE INDEX IF NOT EXISTS idx_best_score_user_rating ON best_score (user_id, rating DESC);

CREATE INDEX IF NOT EXISTS idx_recent30_user_time ON recent30 (user_id, time_played DESC);
CREATE INDEX IF NOT EXISTS idx_recent30_song ON recent30 (song_id);

CREATE INDEX IF NOT EXISTS idx_download_token_lookup ON download_token (song_id, file_name, token);
CREATE INDEX IF NOT EXISTS idx_download_token_time ON download_token (time);

CREATE INDEX IF NOT EXISTS idx_bundle_download_token_time ON bundle_download_token (time);

CREATE INDEX IF NOT EXISTS idx_notification_timestamp ON notification (timestamp);
CREATE INDEX IF NOT EXISTS idx_notification_user_timestamp ON notification (user_id, timestamp);

CREATE INDEX IF NOT EXISTS idx_songplay_token_user ON songplay_token (user_id);

CREATE INDEX IF NOT EXISTS idx_friend_other_me ON friend (user_id_other, user_id_me);

CREATE INDEX IF NOT EXISTS idx_user_item_type ON user_item (type, user_id);

CREATE INDEX IF NOT EXISTS idx_item_type ON item (type);
CREATE INDEX IF NOT EXISTS idx_purchase_item_type ON purchase_item (type, purchase_name);
