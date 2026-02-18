# Admin Web Migration (Python -> Rust)

本文档用于把 Python 管理后台（`py_version/latest version/web/index.py`）逐步迁移到 Rust Askama。

## 1. Python 管理端功能清单

### 1.1 认证

- `GET /web/login`
- `POST /web/login`
- `GET /web/logout`

### 1.2 首页与查询

- `GET /web/` / `GET /web/index`
- `GET|POST /web/singleplayer`
- `GET|POST /web/singleplayerptt`
- `GET /web/allplayer`
- `GET /web/allsong`
- `GET|POST /web/singlecharttop`
- `GET /web/allchar`
- `GET /web/allitem`
- `GET /web/allpurchase`
- `GET /web/allpresent`
- `GET /web/allredeem`
- `GET /web/redeem/<code>`

### 1.3 系统/运维动作

- `GET|POST /web/updatedatabase` (上传 sqlite 并迁移)
- `POST /web/updatedatabase/refreshsonghash`
- `POST /web/updatedatabase/refreshsbundle`
- `POST /web/updatedatabase/refreshsongrating`
- `GET|POST /web/updateusersave` (save -> best_score)

### 1.4 歌曲/角色/物品/购买

- `GET /web/changesong`
- `POST /web/changesong/addsong`
- `POST /web/changesong/deletesong`
- `GET /web/changechar`
- `POST /web/changesong/editchar`
- `POST /web/changesong/updatechar`
- `GET|POST /web/changeitem`
- `POST /web/changeitem/delete`
- `GET|POST /web/changepurchase`
- `POST /web/changepurchase/delete`
- `GET|POST /web/changepurchaseitem`
- `POST /web/changepurchaseitem/delete`

### 1.5 用户与成绩管理

- `GET /web/changeuser`
- `POST /web/changeuser/edituser`
- `GET /web/changeuserpurchase`
- `POST /web/changeuserpurchase/edituser`
- `GET|POST /web/changeuserpwd`
- `GET|POST /web/banuser`
- `POST /web/banuser/deleteuserscore`
- `GET /web/changescore`
- `POST /web/changescore/delete`

### 1.6 奖励/兑换码

- `GET /web/changepresent`
- `POST /web/changepresent/addpresent`
- `POST /web/changepresent/deletepresent`
- `GET|POST /web/deliverpresent`
- `GET /web/changeredeem`
- `POST /web/changeredeem/addredeem`
- `POST /web/changeredeem/deleteredeem`

## 2. Rust 当前进度（第 1 阶段）

已实现（Askama + Rocket）：

- `GET /web/login`
- `POST /web/login`
- `GET /web/logout`
- `GET /web/` / `GET /web/index`（总览）
- `GET /web/users`（列表）
- `GET /web/allplayer`（兼容别名 -> 同玩家列表）
- `GET /web/allsong`
- `GET /web/singleplayer`
- `POST /web/singleplayer`
- `GET /web/singleplayerptt`
- `POST /web/singleplayerptt`
- `GET /web/singlecharttop`
- `POST /web/singlecharttop`
- `GET /web/allchar`
- `GET /web/allitem`
- `GET /web/allpurchase`
- `GET /web/allpresent`
- `GET /web/allredeem`
- `GET /web/users/<user_id>`（详情）
- `POST /web/users/<user_id>/ticket`（改 ticket）
- `POST /web/users/<user_id>/ban`（封禁）
- `POST /web/users/<user_id>/scores/delete`（删该用户成绩）
- `GET /web/static/admin.css`

## 3. 分阶段迁移策略

### Phase 1 (已启动)

- 现代化后台 UI 框架（Askama 模板 + CSS）
- 认证与玩家管理最小闭环

### Phase 2 (已完成)

- 查询页面补齐：`singleplayer` / `singleplayerptt` / `allsong` / `singlecharttop`
- 角色、物品、购买、奖励、兑换码“只读页面”

### Phase 3

- 写操作页面补齐：`changesong` / `changeitem` / `changepurchase` / `changepurchaseitem`
- 奖励和兑换码增删改发放

### Phase 4

- 运维动作与高风险操作：
  - `refreshsonghash`
  - `refreshsbundle`
  - `refreshsongrating`
  - `updateusersave`
  - `changeuserpwd`
  - 批量用户操作

### Phase 5

- `updatedatabase` 重设计（MariaDB 模式下不再走 sqlite 文件上传）
- 增加统一审计日志（admin_audit_log）

## 4. 注意事项

- Python 的 `updatedatabase` 是 SQLite 迁移流，不适合直接照搬到 MariaDB 架构。
- Python 封禁语义是“清空密码 + 清登录 session”，Rust 已按该语义实现。
- 高风险动作必须逐步加入二次确认与审计日志。
