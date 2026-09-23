# Bingxi 纺织 ERP — 作业约束（每次会话/子任务都适用）

规则原文在 `.monkeycode/MEMORY.md`（IR 优先级最高）。下面是**动手前必须知道的地基事实**，
更完整的清单见 `.monkeycode/docs/CODEBASE_INTERNALS.md`（每条都带 `文件:行号`）。

## 先读这两份，否则一定改错

- `.monkeycode/docs/CODEBASE_INTERNALS.md` — 中间件洋葱、信封、状态词表权威表、JOIN 富化范式、CI/e2e 结构、假绿盲区。
- `.monkeycode/MEMORY.md` — IR 硬规则（验证方式、提交规范、推送授权、单据号生成、注释规范）。

⚠️ `.monkeycode/docs/INTERFACES.md` 是早期设想稿：其中 gRPC、`@bingxi/sdk`、Python SDK、`backend/proto/`、
`backend/docs/postman_collection.json`、`AUTH_001/BIZ_001` 错误码**仓库里都不存在**。不要据它改代码。

## 验证：只有 CI 能证明对错

- 禁止本地 `cargo build/test`、禁止起 dev server、禁止本地跑 Playwright。
- 本地只允许这些静态检查：`prettier --check`、`eslint`、`vue-tsc -b --force`、`cargo fmt`/`rustfmt --check`、
  `playwright test --list`、`bash -n`，以及仓库自带门禁：
  `check-i18n.mjs`、`check-contract.mjs`、`check-api-paths.mjs`、`check-api-envelope.mjs`、
  `check-api-request.mjs`、`check-route-mount.mjs`、`route-snapshot.mjs --check`（均在 `frontend/scripts/`，
  从 `frontend/` 目录跑）。
- 后端 Rust 改动**无法本地验证类型**：必须逐字段对着 `backend/src/models/*.rs` 与 DTO 手写核对，
  并保证 `cargo fmt --check` 能过（fmt 会解析，语法错能本地抓到，类型错抓不到）。
- 必检项是 `ci-cleanup-retry`（`ci-cd.yml:3652`，`needs:` 列全 26 个 job）。**新增 job 必须同时加进它的 `needs`**。
- 推送需要用户明确授权；推送前必须把变更完整写进 PR 描述并重排提交序列。

## 改前后端契约时的三条判据

1. **响应键名以实体/DTO 为准**：多数列表 handler 直接 `serde_json::to_value(Vec<Model>)`，
   出参键 = `backend/src/models/<t>.rs` 里的字段名（snake_case）。前端接口写 camelCase 或自创键 ⇒ 列恒空、
   筛选恒 0 行、`v-if` 按钮恒不可达。NOT NULL 列在前端接口里不许标 `?`（那是掩盖缺键）。
2. **状态词表的唯一事实来源是写入方**（`backend/src/models/status/*`）。大小写混用是故意的
   （如 `sales_return` 是大写 `DRAFT/SUBMITTED/APPROVED/REJECTED/COMPLETED`，`inventory_count` 是小写
   `pending/in_review/completed` 且**没有** `in_progress`）；中文词表（`合格/待检/不合格`、`一等品/二等品/等外品`、
   `正常/报废/已删除`、`待检/合格/不合格`）**禁止英文化**。比较点与门控值必须和写入值逐字符相同。
3. **列表要显示名称只有一种正确写法**：照抄 `PurchaseOrderDto`
   （`services/po/order.rs:19` + `services/po/order_ops/crud.rs:463`：`column_as(...) + LeftJoin +
   into_model::<Dto>()`，单次查询无 N+1）。禁止 `format!("客户 #{}", id)` 造假名，禁止逐行再查。

分页只有 `PaginatedResponse{items,total,page,page_size}`（`utils/response.rs:34`）；
失败只有 `AppError` 一种形状（字符串 code + message + trace_id + timestamp），
`ApiResponse::error` 等旁路构造器已删除，不要再用数字 code 自造错误体。
唯一例外是未连接数据库的 Setup 模式（`bootstrap/routes_bootstrap.rs`）。

## 路由与鉴权（形状影响安全，不是风格问题）

- 路由注册只在 `backend/src/routes/` 下；`.route()` 只写相对路径；绝对前缀在 `routes/mod.rs` 里 `nest` 一次。
  `handlers/` 下不得再有 `pub fn router()`（现存唯一例外：`handlers/sales_return_handler.rs:40`，
  已在 `check-route-mount.mjs` 白名单里登记）。
- 权限键由 **URL 段**推导（`middleware/permission.rs:259` 取 `/api/v1/erp/{seg3}/{seg4}`；action 由
  方法/末段/`?action=` 推导）。所以路径写法不统一 = 鉴权键不统一。
- 挂载重构必须让 `route-snapshot.mjs --check` 输出逐字节不变。
- 前端 `baseURL = '/api/v1/erp'`（`api/request.ts:118`），调用方只写其后部分。
- 同一业务可能存在两个挂载点（例：`/sales/sales-returns` 返回真实体，`/trading/sales-returns` 走
  `handlers/advanced/reorder.rs` 且含造假名）。改字段前先确认改的是哪一个端点。

## 禁止的写法（这些一律算返工）

兜底掩盖（`?? []` / `|| 默认值` 掩盖缺键）、`t('key') || '中文'`（vue-i18n 取不到键会返回键名本身，兜底分支不可达，
只会把缺键降级成另一语言的裸字面量）、源码里的裸中文字符串（i18n 之外的情形，如落库的业务 token 值需明确判断）、
静默失败（每步都要有显式日志，成功失败都打）、变更日志式注释（批次号/"修复xxx"过程；变更说明属于 commit message）、
为"以后可能要用"预留的抽象、把错误改测试用例来蒙过（先判责：源码错改源码，测试错改测试）。

## 前端复用件（别再手搓）

- 状态标签：`utils/sales-status.ts` / `utils/purchase-status.ts` 范式 = 常量数组 + `normalize*Status()`
  （未知 token 抛错）+ `*LabelKey()` + `*TagType()`。反例：`views/sales-returns/composables/srFmts.ts`
  手搓 map + 硬编码中文 + `|| status` 兜底，待清理。
- 提示/日志：`msg`（`utils/message.ts`，走 `message.` 命名空间）与 `logger`（`utils/logger.ts`）。
- 权限：`v-permission`（`directives/permission.ts`）+ `constants/permissions.ts PERMISSIONS`。
- i18n：只有 `locales/zh-CN.ts` / `en-US.ts`。`.vue` 里 `const { t } = useI18n({ useScope: 'global' })`；
  纯 `.ts` 里用 `i18n.global.t`。新键必须中英双侧齐全，否则 `check-i18n` 判负。
- 弹窗取消不是错误：`ElMessageBox` reject `'cancel'`/`'close'`，用 `utils/monitor.ts isDialogDismissal()` 判定。
- 查询分页键：`page` / `page_size`（后端 `Query<T>` 同名；空串筛选由 `request.ts:95 serializeParams`
  与后端 `normalize_empty_query_params` 双侧剔除）。

## 提交规范

- 每项修复 1 个 commit，Conventional Commits 且 **scope 必填**。
- 暂存用显式路径（禁止 `git add -A`/`git add frontend/src`），防止把别人在改的文件一起提交。
- 不新建分支、不新建 PR（当前分支 `260907-feat-piece-domain-phase2`，当前 PR `#941`）。

## Windows 本地环境

- 每个 Bash 命令自带：`export PATH="/c/Users/57231/tools/node-v22.23.2-win-x64:$PATH"`（shell 状态不跨命令保留）。
- `core.autocrlf=true`：工作区 CRLF，**索引是 LF**。判断行尾以 `git ls-files --eol` 为准；
  不要用 `git show HEAD:<path>` 的输出判断行尾（它会应用 checkout 转换）。
- 临时脚本 / commit message 文件放仓库外（`C:/Users/57231/` 或 `C:/tmp/`）。
- GitHub token 只在 `~/.gh_bingxi_token`：禁止打印、写入文件或日志；命令行里内联 `$(cat ...)` 会被脱敏机制打断，
  用 `C:/Users/57231/ghq.sh <api-path>` 代替。
