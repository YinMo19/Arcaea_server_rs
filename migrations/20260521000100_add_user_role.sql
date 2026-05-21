ALTER TABLE user ADD COLUMN IF NOT EXISTS role TINYINT NOT NULL DEFAULT 0;

UPDATE user u
JOIN user_role ur ON ur.user_id = u.user_id AND ur.role_id = 'admin'
SET u.role = 1;

UPDATE user SET role = 1 WHERE name = 'admin';
