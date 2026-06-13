CREATE TABLE IF NOT EXISTS user_checkin (
  user_id INT NOT NULL,
  checkin_date DATE NOT NULL,
  reward_ticket INT NOT NULL,
  created_at BIGINT NOT NULL,
  PRIMARY KEY (user_id, checkin_date),
  INDEX idx_user_checkin_date (checkin_date)
);
