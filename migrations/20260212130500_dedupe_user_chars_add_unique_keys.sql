-- Ensure user characters are unique per (user_id, character_id).
-- This fixes duplicate rows caused by ON DUPLICATE KEY statements
-- when unique constraints were absent.

CREATE TEMPORARY TABLE tmp_user_char AS
SELECT
    user_id,
    character_id,
    MAX(level) AS level,
    MAX(exp) AS exp,
    MAX(is_uncapped) AS is_uncapped,
    MAX(is_uncapped_override) AS is_uncapped_override,
    MAX(skill_flag) AS skill_flag
FROM user_char
GROUP BY user_id, character_id;

DELETE FROM user_char;

INSERT INTO user_char (
    user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
)
SELECT
    user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
FROM tmp_user_char;

DROP TEMPORARY TABLE tmp_user_char;

CREATE TEMPORARY TABLE tmp_user_char_full AS
SELECT
    user_id,
    character_id,
    MAX(level) AS level,
    MAX(exp) AS exp,
    MAX(is_uncapped) AS is_uncapped,
    MAX(is_uncapped_override) AS is_uncapped_override,
    MAX(skill_flag) AS skill_flag
FROM user_char_full
GROUP BY user_id, character_id;

DELETE FROM user_char_full;

INSERT INTO user_char_full (
    user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
)
SELECT
    user_id, character_id, level, exp, is_uncapped, is_uncapped_override, skill_flag
FROM tmp_user_char_full;

DROP TEMPORARY TABLE tmp_user_char_full;

ALTER TABLE user_char
    ADD UNIQUE KEY uk_user_char_user_character (user_id, character_id);

ALTER TABLE user_char_full
    ADD UNIQUE KEY uk_user_char_full_user_character (user_id, character_id);
