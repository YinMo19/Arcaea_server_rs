ALTER TABLE user
  ADD COLUMN custom_banner TEXT,
  ADD COLUMN is_allow_marketing_email TINYINT DEFAULT 0,
  ADD COLUMN is_profile_public TINYINT DEFAULT 0;
