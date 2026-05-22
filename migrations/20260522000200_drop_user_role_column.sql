INSERT IGNORE INTO role (role_id, caption) VALUES
  ('system', '系统'),
  ('admin', '管理员'),
  ('user', '用户');

INSERT IGNORE INTO user_role (user_id, role_id)
SELECT user_id, 'admin'
FROM user
WHERE role = 1;

CREATE INDEX IF NOT EXISTS idx_user_role_role_id ON user_role (role_id, user_id);

ALTER TABLE user DROP COLUMN IF EXISTS role;
