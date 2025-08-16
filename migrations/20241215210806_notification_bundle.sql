-- Add notification and bundle download tables
CREATE TABLE IF NOT EXISTS notification (
  user_id INT,
  id INT,
  type VARCHAR(255),
  content TEXT,
  sender_user_id INT,
  sender_name VARCHAR(255),
  timestamp BIGINT,
  PRIMARY KEY (user_id, id)
);

CREATE TABLE IF NOT EXISTS bundle_download_token (
  token VARCHAR(255) PRIMARY KEY,
  file_path TEXT,
  time BIGINT,
  device_id VARCHAR(255)
);
