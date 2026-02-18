# Admin Web Migration (Python -> Rust)

本文档用于将 Python 管理后台（`py_version/latest version/web/index.py`）迁移到 Rust + Askama，并明确“可编辑、可新增、可删除”的现代化后台计划。

## 1. Python 功能基线（逻辑对齐目标）

### 1.1 已确认的 Python 路由能力

来自 `py_version/latest version/web/index.py` 的主要模块：

- 认证：`/web/login`、`/web/logout`
- 查询：`/web/index`、`/web/singleplayer`、`/web/singleplayerptt`、`/web/allplayer`、`/web/allsong`、`/web/singlecharttop`
- 资源只读：`/web/allchar`、`/web/allitem`、`/web/allpurchase`、`/web/allpresent`、`/web/allredeem`、`/web/redeem/<code>`
- 资源写操作：
  - 歌曲：`/web/changesong`、`/web/changesong/addsong`、`/web/changesong/deletesong`
  - 角色：`/web/changechar`、`/web/changesong/editchar`、`/web/changesong/updatechar`
  - 物品：`/web/changeitem`、`/web/changeitem/delete`
  - 购买：`/web/changepurchase`、`/web/changepurchase/delete`
  - 购买物品：`/web/changepurchaseitem`、`/web/changepurchaseitem/delete`
  - 奖励：`/web/changepresent`、`/web/changepresent/addpresent`、`/web/changepresent/deletepresent`、`/web/deliverpresent`
  - 兑换码：`/web/changeredeem`、`/web/changeredeem/addredeem`、`/web/changeredeem/deleteredeem`
- 用户/成绩运维：
  - `changeuser`（改 ticket）
  - `changeuserpurchase`（用户/全量 unlock/lock）
  - `changeuserpwd`（改密码）
  - `banuser`（封禁）
  - `banuser/deleteuserscore`（删指定用户全部成绩）
  - `changescore/delete`（按条件删成绩）
  - `updateusersave`（save -> best_score）
- 系统运维：
  - `updatedatabase`（Python 为 sqlite 上传流）
  - `updatedatabase/refreshsonghash`
  - `updatedatabase/refreshsbundle`
  - `updatedatabase/refreshsongrating`

### 1.2 Python 表单字段基线（用于 Rust UI 对齐）

只参考逻辑，不照搬旧 UI：

- `changesong`: `sid`, `name_en`, `rating_pst/prs/ftr/byd/etr`
- `changechar`: `id`, `level`, `skill_id`, `skill_id_uncap` + `updatechar`
- `changeitem`: `item_id`, `type`, `is_available`
- `changepurchase`: `purchase_name`, `price`, `orig_price`, `discount_from/to`, `discount_reason`
- `changepurchaseitem`: `purchase_name`, `item_id`, `type`, `amount`
- `changepresent`: `present_id`, `description`, `expire_ts`, `item_id`, `type`, `amount`
- `deliverpresent`: `present_id` + (`name`/`user_code`)（单人或全量）
- `changeredeem`: `code`/`redeem_amount`, `redeem_type`, `item_id`, `type`, `amount`
- `changeuser`: (`name`/`user_code`) + `ticket`（支持全量）
- `changeuserpurchase`: (`name`/`user_code`) + `method`（unlock/lock，支持全量）
- `changeuserpwd`: (`name`/`user_code`) + `pwd`, `pwd2`
- `banuser`: (`name`/`user_code`)
- `changescore`: `sid`, `difficulty`, (`name`/`user_code`)

## 2. Rust 当前状态（截至 2026-02-18）

### 2.1 已完成

- Askama 后台框架：`/web` 路由、侧边栏、卡片式页面、基础样式
- 认证：`/web/login`, `/web/logout`
- 查询与只读：
  - `/web`, `/web/index`
  - `/web/users`, `/web/allplayer`
  - `/web/allsong`
  - `/web/singleplayer`（GET/POST）
  - `/web/singleplayerptt`（GET/POST）
  - `/web/singlecharttop`（GET/POST）
  - `/web/allchar`, `/web/allitem`, `/web/allpurchase`, `/web/allpresent`, `/web/allredeem`
- 用户动作（局部）：
  - `/web/users/<user_id>/ticket`
  - `/web/users/<user_id>/ban`
  - `/web/users/<user_id>/scores/delete`

### 2.2 主要差距（你提到的“多数功能未实现”）

- 大部分“写操作页面”尚未实现（新增/编辑/删除）
- Python 的批量运维入口尚未落地（`updateusersave`、`refresh*` 等）
- 兑换码使用详情页（`/web/redeem/<code>`）未实现
- 目前页面更偏“查询页”，编辑表单与交互还不完整

## 3. 新迁移计划（面向可编辑现代 UI）

### Phase A：后台交互基建（先做）

目标：先把“可编辑 UI 框架”打好，避免后续每页重复造轮子。

- 统一组件：表格、分页、搜索栏、筛选器、空状态、Flash 提示
- 统一表单：输入校验、错误提示、确认弹窗（删除/高风险操作）
- 页面交互：列表 + 侧边抽屉/模态编辑（Create / Update / Delete）
- 统一权限保护：未登录重定向、危险操作二次确认

验收标准：

- 至少 1 个模块完成“列表 + 新增 + 编辑 + 删除”全链路并复用组件

### Phase B：资源 CRUD（歌曲/角色/物品/购买）

目标：优先实现高频运营配置页面，全部支持表单编辑。

- `changesong`: 新增/删除歌曲
- `changechar`: 编辑角色字段 + 执行用户角色同步
- `changeitem`: 新增/删除物品
- `changepurchase`: 新增/删除购买项
- `changepurchaseitem`: 新增/删除购买项下物品

验收标准：

- 上述 5 个模块全部具备可视化表单与成功/失败反馈
- 关键写 SQL 使用 `sqlx::query! / query_as! / query_scalar!`

### Phase C：奖励与兑换码（可编辑 + 发放）

- `changepresent`: 新增/删除奖励
- `deliverpresent`: 按用户/全量发放
- `changeredeem`: 单码新增、批量随机码新增、删除
- `redeem/<code>`: 展示兑换码使用用户列表

验收标准：

- 奖励与兑换码形成完整运营闭环（建、查、删、发放、追踪）

### Phase D：用户与成绩运维

- `changeuser`: 改 ticket（单用户/全量）
- `changeuserpurchase`: 解锁/封锁购买（单用户/全量）
- `changeuserpwd`: 修改用户密码
- `banuser`: 封禁用户（语义对齐 Python：清空密码 + 清登录）
- `banuser/deleteuserscore`: 删除指定用户成绩
- `changescore/delete`: 条件删除成绩
- `updateusersave`: save 同步到 best_score（单用户/全量）

验收标准：

- 所有动作均有操作结果反馈和错误提示
- 高风险动作有明确确认步骤

### Phase E：系统运维页与 MariaDB 重设计

- `refreshsonghash`, `refreshsbundle`, `refreshsongrating` Web 化
- `updatedatabase` 改造为 MariaDB 场景下可用的“数据同步/校验工具页”
- 不保留 Python 的 sqlite 上传迁移路径
- 引入 `admin_audit_log`（操作人、操作对象、前后值、时间）

## 4. 实施原则

- 只参考 Python 的业务逻辑与边界条件，不照搬其页面结构
- UI 以现代管理后台为目标：可编辑表单、批量操作、清晰反馈
- 静态 SQL 全部使用 sqlx 宏；仅动态 SQL 保留 function 版本
- 高风险操作（删分、封禁、批量覆盖）必须二次确认 + 审计
