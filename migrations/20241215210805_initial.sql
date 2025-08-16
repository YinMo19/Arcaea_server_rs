-- MariaDB migration from SQLite
CREATE TABLE IF NOT EXISTS config (id VARCHAR(255) PRIMARY KEY, value TEXT);

CREATE TABLE user (
  user_id INT AUTO_INCREMENT PRIMARY KEY,
  name VARCHAR(255) UNIQUE,
  password TEXT,
  join_date BIGINT,
  user_code VARCHAR(255),
  rating_ptt INT,
  character_id INT,
  is_skill_sealed TINYINT,
  is_char_uncapped TINYINT,
  is_char_uncapped_override TINYINT,
  is_hide_rating TINYINT,
  song_id TEXT,
  difficulty INT,
  score INT,
  shiny_perfect_count INT,
  perfect_count INT,
  near_count INT,
  miss_count INT,
  health INT,
  modifier INT,
  time_played BIGINT,
  clear_type INT,
  rating DOUBLE,
  favorite_character INT,
  max_stamina_notification_enabled TINYINT,
  current_map TEXT,
  ticket INT,
  prog_boost INT,
  email VARCHAR(255),
  world_rank_score INT,
  ban_flag TEXT,
  next_fragstam_ts BIGINT,
  max_stamina_ts BIGINT,
  stamina INT,
  world_mode_locked_end_ts BIGINT,
  beyond_boost_gauge DOUBLE DEFAULT 0,
  kanae_stored_prog DOUBLE DEFAULT 0,
  mp_notification_enabled TINYINT DEFAULT 1,
  highest_rating_ptt INT DEFAULT 0,
  insight_state INT DEFAULT 4
);

CREATE TABLE IF NOT EXISTS login (
  access_token TEXT,
  user_id INT,
  login_time BIGINT,
  login_ip TEXT,
  login_device TEXT,
  PRIMARY KEY (access_token (255), user_id)
);

CREATE TABLE IF NOT EXISTS friend (
  user_id_me INT,
  user_id_other INT,
  PRIMARY KEY (user_id_me, user_id_other)
);

CREATE TABLE IF NOT EXISTS best_score (
  user_id INT,
  song_id VARCHAR(255),
  difficulty INT,
  score INT,
  shiny_perfect_count INT,
  perfect_count INT,
  near_count INT,
  miss_count INT,
  health INT,
  modifier INT,
  time_played BIGINT,
  best_clear_type INT,
  clear_type INT,
  rating DOUBLE DEFAULT 0,
  score_v2 DOUBLE DEFAULT 0,
  PRIMARY KEY (user_id, song_id, difficulty)
);

CREATE TABLE IF NOT EXISTS user_char (
  user_id INT,
  character_id INT,
  level INT,
  exp DOUBLE,
  is_uncapped TINYINT,
  is_uncapped_override TINYINT,
  skill_flag INT,
  PRIMARY KEY (user_id, character_id)
);

CREATE TABLE IF NOT EXISTS user_char_full (
  user_id INT,
  character_id INT,
  level INT,
  exp DOUBLE,
  is_uncapped TINYINT,
  is_uncapped_override TINYINT,
  skill_flag INT,
  PRIMARY KEY (user_id, character_id)
);

CREATE TABLE IF NOT EXISTS `character` (
  character_id INT PRIMARY KEY,
  name TEXT,
  max_level INT,
  frag1 DOUBLE,
  prog1 DOUBLE,
  overdrive1 DOUBLE,
  frag20 DOUBLE,
  prog20 DOUBLE,
  overdrive20 DOUBLE,
  frag30 DOUBLE,
  prog30 DOUBLE,
  overdrive30 DOUBLE,
  skill_id TEXT,
  skill_unlock_level INT,
  skill_requires_uncap TINYINT,
  skill_id_uncap TEXT,
  char_type INT,
  is_uncapped TINYINT
);

CREATE TABLE IF NOT EXISTS char_item (
  character_id INT,
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (character_id, item_id, type)
);

CREATE TABLE IF NOT EXISTS recent30 (
  user_id INT,
  r_index INT,
  time_played BIGINT,
  song_id VARCHAR(255),
  difficulty INT,
  score INT DEFAULT 0,
  shiny_perfect_count INT DEFAULT 0,
  perfect_count INT DEFAULT 0,
  near_count INT DEFAULT 0,
  miss_count INT DEFAULT 0,
  health INT DEFAULT 0,
  modifier INT DEFAULT 0,
  clear_type INT DEFAULT 0,
  rating DOUBLE DEFAULT 0,
  PRIMARY KEY (user_id, r_index)
);

CREATE TABLE IF NOT EXISTS user_world (
  user_id INT,
  map_id VARCHAR(255),
  curr_position INT,
  curr_capture DOUBLE,
  is_locked TINYINT,
  PRIMARY KEY (user_id, map_id)
);

CREATE TABLE IF NOT EXISTS songplay_token (
  token VARCHAR(255) PRIMARY KEY,
  user_id INT,
  song_id VARCHAR(255),
  difficulty INT,
  course_id VARCHAR(255),
  course_state INT,
  course_score INT,
  course_clear_type INT,
  stamina_multiply INT,
  fragment_multiply INT,
  prog_boost_multiply INT,
  beyond_boost_gauge_usage INT,
  skill_cytusii_flag TEXT,
  skill_chinatsu_flag TEXT,
  invasion_flag INT
);

CREATE TABLE IF NOT EXISTS item (
  item_id VARCHAR(255),
  type VARCHAR(255),
  is_available TINYINT,
  PRIMARY KEY (item_id, type)
);

CREATE TABLE IF NOT EXISTS user_item (
  user_id INT,
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (user_id, item_id, type)
);

CREATE TABLE IF NOT EXISTS purchase (
  purchase_name VARCHAR(255) PRIMARY KEY,
  price INT,
  orig_price INT,
  discount_from BIGINT,
  discount_to BIGINT,
  discount_reason TEXT
);

CREATE TABLE IF NOT EXISTS purchase_item (
  purchase_name VARCHAR(255),
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (purchase_name, item_id, type)
);

CREATE TABLE IF NOT EXISTS user_save (
  user_id INT PRIMARY KEY,
  scores_data TEXT,
  clearlamps_data TEXT,
  clearedsongs_data TEXT,
  unlocklist_data TEXT,
  installid_data TEXT,
  devicemodelname_data TEXT,
  story_data TEXT,
  createdAt BIGINT,
  finalestate_data TEXT
);

CREATE TABLE IF NOT EXISTS present (
  present_id VARCHAR(255) PRIMARY KEY,
  expire_ts BIGINT,
  description TEXT
);

CREATE TABLE IF NOT EXISTS user_present (
  user_id INT,
  present_id VARCHAR(255),
  PRIMARY KEY (user_id, present_id)
);

CREATE TABLE IF NOT EXISTS present_item (
  present_id VARCHAR(255),
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (present_id, item_id, type)
);

CREATE TABLE IF NOT EXISTS chart (
  song_id VARCHAR(255) PRIMARY KEY,
  name TEXT,
  rating_pst INT DEFAULT -1,
  rating_prs INT DEFAULT -1,
  rating_ftr INT DEFAULT -1,
  rating_byn INT DEFAULT -1,
  rating_etr INT DEFAULT -1
);

CREATE TABLE IF NOT EXISTS redeem (code VARCHAR(255) PRIMARY KEY, type INT);

CREATE TABLE IF NOT EXISTS user_redeem (
  user_id INT,
  code VARCHAR(255),
  PRIMARY KEY (user_id, code)
);

CREATE TABLE IF NOT EXISTS redeem_item (
  code VARCHAR(255),
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (code, item_id, type)
);

CREATE TABLE IF NOT EXISTS role (role_id VARCHAR(255) PRIMARY KEY, caption TEXT);

CREATE TABLE IF NOT EXISTS user_role (
  user_id INT,
  role_id VARCHAR(255),
  PRIMARY KEY (user_id, role_id)
);

CREATE TABLE IF NOT EXISTS power(power_id VARCHAR(255) PRIMARY KEY, caption TEXT);

CREATE TABLE IF NOT EXISTS role_power (
  role_id VARCHAR(255),
  power_id VARCHAR(255),
  PRIMARY KEY (role_id, power_id)
);

CREATE TABLE IF NOT EXISTS api_login (
  user_id INT,
  token VARCHAR(255),
  login_time BIGINT,
  login_ip TEXT,
  PRIMARY KEY (user_id, token)
);

CREATE TABLE IF NOT EXISTS course (
  course_id VARCHAR(255) PRIMARY KEY,
  course_name TEXT,
  dan_name TEXT,
  style INT,
  gauge_requirement TEXT,
  flag_as_hidden_when_requirements_not_met TINYINT,
  can_start TINYINT
);

CREATE TABLE IF NOT EXISTS user_course (
  user_id INT,
  course_id VARCHAR(255),
  high_score INT,
  best_clear_type INT,
  PRIMARY KEY (user_id, course_id)
);

CREATE TABLE IF NOT EXISTS course_chart (
  course_id VARCHAR(255),
  song_id VARCHAR(255),
  difficulty INT,
  flag_as_hidden TINYINT,
  song_index INT,
  PRIMARY KEY (course_id, song_index)
);

CREATE TABLE IF NOT EXISTS course_requirement (
  course_id VARCHAR(255),
  required_id VARCHAR(255),
  PRIMARY KEY (course_id, required_id)
);

CREATE TABLE IF NOT EXISTS course_item (
  course_id VARCHAR(255),
  item_id VARCHAR(255),
  type VARCHAR(255),
  amount INT,
  PRIMARY KEY (course_id, item_id, type)
);

CREATE TABLE IF NOT EXISTS user_mission (
  user_id INT,
  mission_id VARCHAR(255),
  status INT,
  PRIMARY KEY (user_id, mission_id)
);

CREATE TABLE IF NOT EXISTS user_kvdata (
  user_id INT,
  class VARCHAR(255),
  `key` VARCHAR(255),
  idx INT,
  value TEXT,
  PRIMARY KEY (user_id, class, `key`, idx)
);

CREATE INDEX IF NOT EXISTS best_score_1 ON best_score (song_id, difficulty);

CREATE TABLE IF NOT EXISTS user_custom_course (
  user_id INT,
  custom_course TEXT,
  PRIMARY KEY (user_id)
);

CREATE TABLE IF NOT EXISTS download_token (
  user_id INT,
  song_id VARCHAR(255),
  file_name VARCHAR(255),
  token VARCHAR(255),
  time BIGINT,
  PRIMARY KEY (user_id, song_id, file_name)
);
