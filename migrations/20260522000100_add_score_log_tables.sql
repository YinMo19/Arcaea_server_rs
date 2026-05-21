CREATE TABLE IF NOT EXISTS user_score (
  user_id INT NOT NULL,
  song_id VARCHAR(255) NOT NULL,
  difficulty INT NOT NULL,
  time_played BIGINT NOT NULL,
  score INT,
  shiny_perfect_count INT,
  perfect_count INT,
  near_count INT,
  miss_count INT,
  health INT,
  modifier INT,
  clear_type INT,
  rating DOUBLE,
  PRIMARY KEY (user_id, song_id, difficulty, time_played)
);

CREATE TABLE IF NOT EXISTS user_rating (
  user_id INT NOT NULL,
  time BIGINT NOT NULL,
  rating_ptt DOUBLE,
  PRIMARY KEY (user_id, time)
);

CREATE INDEX IF NOT EXISTS idx_user_score_song_difficulty ON user_score (song_id, difficulty);
CREATE INDEX IF NOT EXISTS idx_user_score_time_played ON user_score (time_played);
CREATE INDEX IF NOT EXISTS idx_user_rating_user_time ON user_rating (user_id, time);
