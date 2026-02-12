# Arcaea_server_rs Agent Guide

本文件用于指导后续开发代理在当前仓库内工作。目标是：
- 保持与 Python 版本行为一致（优先保证 API 行为一致）
- 避免已知回归（角色重复、曲包不同步、类型不匹配导致客户端崩溃）
- 让更新流程可重复、可审计

## 1. 当前项目状态（必须理解）

- 数据库：MariaDB + `sqlx`（已迁移管理）
- 服务：Rust（Rocket）
- Python 参考仓：`py_version`（独立 Git 仓，被主仓 ignore）
- 资产目录：**只使用** `./assets`
  - `assets/arc_data.json`
  - `assets/packs.json`
  - `assets/singles.json`
  - `assets/courses.json`
  - `assets/map/*.json`
- 旧的 `src/assets` 已移除，不要恢复。

## 2. 核心数据源约定

### 2.1 Character / Arc Data

- 唯一角色配置源：`assets/arc_data.json`
- 对应逻辑：`src/service/arc_data.rs`
- 环境变量：`ARC_DATA_FILE`（可覆盖默认路径）

启动时（`src/main.rs`）：
1. 若 `CONFIG.update_with_new_character_data == true`，执行 arc_data 同步。
2. 同步后执行 `update_user_char_full()`。

### 2.2 Purchase（Pack / Single）

- 曲包和单曲配置源：
  - `assets/packs.json`
  - `assets/singles.json`
- 启动时必须执行同步（已在 `main.rs` 接入）：
  - `AssetInitService::sync_purchases_from_assets()`
  - 将 `purchase` / `purchase_item` / `item` 做 upsert，保证新资产自动入库。

## 3. 已修复且必须防回归的问题

### 3.1 “每个角色出现两次”

根因：`user_char` / `user_char_full` 曾缺少 `(user_id, character_id)` 唯一键，导致 `ON DUPLICATE KEY` 失效。

当前状态：
- 迁移已添加唯一键并完成历史去重：
  - `migrations/20260212130500_dedupe_user_chars_add_unique_keys.sql`

后续要求：
- 不要删除该唯一键。
- 不要写会绕过唯一键的批量插入逻辑。
- 若调整角色同步逻辑，必须保持幂等。

### 3.2 “最新曲包拿不到”

根因：`assets/packs.json` / `assets/singles.json` 落后且主服务未在启动时同步到数据库。

当前状态：
- 已补齐 assets，并接入启动同步。

后续要求：
- 每次跟进客户端版本时，先更新 assets，再验证 DB 同步结果。

## 4. SQL / sqlx 规范（严格执行）

1. 静态 SQL 一律优先使用宏：
   - `sqlx::query!`
   - `sqlx::query_as!`
   - `sqlx::query_scalar!`
2. 仅在以下情况可保留动态 SQL：
   - 动态 IN 占位符个数
   - 动态表名/列名（且无法轻易枚举分支）
3. 能改成分支宏就不要保留动态 SQL。
4. 任何 SQL 改动后必须：
   - `cargo check`
   - `cargo sqlx prepare`
   - 提交 `.sqlx/` 变更

## 5. 数据库迁移规则

- 只通过 `migrations/*.sql` 管理结构变化。
- 不直接手改线上表结构而不落迁移文件。
- 改动高风险表（如 `user_char*`, `user_item`, `purchase*`）时，迁移里要考虑历史数据兼容（去重、回填、默认值）。

常用命令：
- `cargo sqlx migrate run`
- `cargo sqlx prepare`

## 6. 与 Python 版本对齐策略

优先对齐路径（当前 Python 仓结构）：
- `py_version/latest version/core/*.py`
- `py_version/latest version/server/*.py`
- 资产：
  - `py_version/latest version/database/init/packs.json`
  - `py_version/latest version/database/init/singles.json`

要求：
- 行为优先对齐，不做“自以为合理”的功能简化。
- 如发现 Rust 与 Python 差异，先说明差异点，再提交修复。

## 7. 文件与结构约定

- 业务逻辑：`src/service/*`
- 路由：`src/route/*`
- 数据模型：`src/model/*`
- 配置：`src/config.rs`
- 运行时资产路径解析：`src/service/runtime_assets.rs`

## 8. 提交前检查清单

每次改动至少完成以下检查：

1. `cargo check` 通过
2. 若动了 SQL：`cargo sqlx prepare` 已执行且 `.sqlx` 已提交
3. 若动了 schema：有 migration 文件
4. 若动了 assets：验证 DB 同步后数据可见
5. 关键接口做最小化实测（curl 或既有脚本）

## 9. 额外注意

- 不使用“临时值/占位值”兜底核心业务字段。
- 客户端对 JSON 类型敏感：字段类型必须稳定，避免 int/float 混淆。
- World/Score/Mission 是高风险区域，改动后必须做端到端回归。

