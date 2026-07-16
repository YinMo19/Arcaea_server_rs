INSERT IGNORE INTO role (role_id, caption) VALUES
  ('chart_editor', '曲目定数编辑员');

INSERT IGNORE INTO power (power_id, caption) VALUES
  ('web_chart_constant_edit', 'Web 曲目定数编辑权限');

INSERT IGNORE INTO role_power (role_id, power_id) VALUES
  ('system', 'web_chart_constant_edit'),
  ('admin', 'web_chart_constant_edit'),
  ('chart_editor', 'web_chart_constant_edit');
