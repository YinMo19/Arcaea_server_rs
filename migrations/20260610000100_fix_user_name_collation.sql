-- Fix user.name collation from utf8mb4_bin to utf8mb4_unicode_ci
-- utf8mb4_bin causes MariaDB to report the column as VARBINARY,
-- which breaks sqlx's compile-time type checking (expects VARCHAR).
--
-- "Admin" is a case-insensitive duplicate of "admin" — rename it first.
UPDATE user SET name = 'Admin2' WHERE user_id = 2000008 AND name = 'Admin';

ALTER TABLE user MODIFY name VARCHAR(255) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
