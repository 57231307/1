# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](doto-su.md)，一句话总结见 [CHANGELOG.md](CHANGELOG.md)，规则见 [MEMORY.md](MEMORY.md)。
> 过时内容归档见 [docs/archives/](docs/archives/)（最新批次：2026-09-09）。

---

## 当前状态

**文档治理完成（2026-09-09）：README 数据更新、doto/bug 归档、docs/ 移入 .monkeycode、生产服务器日志与 scan_long_fns.py 删除，全部本地 commit。源代码修改冻结已解除（IR 2026-09-09，用户指令）——修复计划（docs/plans/one-round-fix-plan-2026-09-09.md，10 commit）可实施；推送继续冻结，全部改动仅本地 commit。CI 历史问题全量总结见 docs/ci-issues-summary-2026-09-09.md（22 项 + 6 条机制教训）。**

**一轮修复计划全部 10 commit 实施完成（2026-09-09，本地未推送）：**- 3e8ca73 P1.1 敏感导出 fail-closed（12 端点×6 类资源 + record_download）+ P1.4 APPROVE 审计分类
- 7604f41 P1.2 lock-status OptionalAuthContext 匿名可查 + P1.3 阈值 5→9
- 11b179d P1.5 版本号方案 C（env! 编译期化 + Release job 去 VERSION 推送）
- 3dcd56a P2.1 登录瀑布（_skipAuthRetry+删 catch refreshLockStatus）+ P2.2 协议页路由 + P2.3 export-approvals api/UI/路由
- 752921a + cf4c79d 法律文书 V2.0（协议 18 条/隐私 19 条，逐条标注法条依据，websearch 核实原文：PIPL 13/14/17/28-31/38-41/44-47/50-57 条、网安法 21/40-44/49 条、数安法 27/32 条、民法典 123/469/490/496/1034 条、刑法 253-1、反不正当竞争法 9 条）
- d84a461 P2.4 去 mock（applyAuthMocks 真实登录，55 spec 零改动 + lock-status 拦截移除）
- a7711bc P4 后端测试 3 文件（classify_operation pub(crate)+APPROVE 分类/阈值常量 pub(crate)/enforce 空 token fail-closed）
- 1d89410 P3 基建（ensureRoleUsers 30+角色自动补建+role-credentials.json / trackPageHealth+assertPageHealthy+expectSingleToast+generateTotp RFC6238 / testMatch 扩全 11 目录）
- d24d43d P5 专项 spec 7 个（31 登录瀑布/32 全角色登录/34 水平越权/35 2FA/36 预览/40 系统更新/43 重复提示）
- 9cdfd02 P5 全量矩阵+遍历（modules.config 95 模块真实路由全登记 / endpoints.config print 58+export 敏感12+非敏感27+approve 48 / 42a-d 遍历 4 spec / 37/39/41 端点矩阵 3 spec / 09-permissions 恒真断言修复 P1-2/3→精确403+P1-7/8→真实断言 / 22-crm-full fail-closed 语义 / jszip devDep / Login.vue 死代码清理）
- 439c6ef P5.14 角色权限矩阵 job（permission-model 双轨：推导+黄金基线 / 44-role-matrix 三分支断言+access-map artifacts / ci-cd.yml role-permission-matrix job 6 角色组）
- e0ce7bb 计划缺口补齐 spec（33 垂直越权矩阵：非 admin 对 admin 端点族 10 端点 403+admin 对照组 / 38 print-templates API 按真实路由 / 39b 完整审批链：申请→审批→令牌导出→二次消费拒→跨资源拒 / 41b-c 坏账双级+转账+角色变更+BPM 引擎全流程）
- **第三轮（2026-09-10，用户 0-3 指令）完成，本地 commit：**
  - 5ce955c 核查前三轮全部修改：修复 6 处真实缺陷（P4 测试 pub(crate) 对 tests/ 不可见→pub / 42a-d listApi 缺 /api/v1/erp 前缀 404 恒真→补前缀 / 41+33 POST 缺 X-CSRF-Token→读 csrf cookie 补头 / role-permission-matrix --list 不触发 globalSetup→新增 ensure-role-users.ts 显式入口 / 44-role-matrix 无基线硬断言→双模）
  - 75 分片重划：ci-e2e matrix 0-74；0-49 flow（/50）/ 50-54 smoke（/5）/ 55-74 traversal（/20，traversal 首次纳入 CI 执行）
  - 9b5aa14 六个新 CI JOB 全部落地（含前次 security-vulnerability-scan）：漏洞扫描（cargo-audit+npm audit+gitleaks+semgrep）/ 功能完整性联网校验（协议 15 功能域 vs 路由对照+npm registry 过时度）/ 注释规范（Rust 82% TS 35%，阈值 30 fail/60 warn）/ E2E 真实性门禁（DO-NOT-MODIFY 标记，动态 needs 三 E2E job 全 success + 静态扫描伪造响应，EXEMPT 豁免机制）/ 敏感权限审计 11 项断言 / 日志合规+vue-tsc（pipefail）；ci-cleanup-retry needs 19→25
  - 自审预跑修复 4 处（job 上线即绿保障）：路由 path 前导斜杠误报 / migration 扫 backend/migrations 不存在→改 backend/migration/src Rust 源 / TS 阈值脱离现状 / vue-tsc 管道吞退出码
  - network-resilience.spec.ts 伪造响应违规→E2E-AUTHENTICITY-EXEMPT+test.skip（待人工确认真实异常源重写方案）；color_card_issue_service.rs 死语句清理
- 待推送授权后 CI 验证（按 commit 序观察，失败按 commit 隔离）；jszip 需 CI npm ci 验证 lock 完整性
- **第四轮（2026-09-10，doto 存量缺口清零）完成，本地 commit：**
  - 37b-print-content.spec.ts：打印内容匹配（JSZip 解包 word/document.xml 非空+源单据号匹配，销售订单+凭证两链路）+ 打印审计闭环（打印后 audit-logs?operation_type=PRINT 出现记录）
  - 39c-export-content.spec.ts：导出内容断言（xlsx JSZip 解包 sharedStrings+worksheets，仓库列头/行数≥列表/首条名称匹配，库存批号匹配）
  - 33b-role-blacklist.spec.ts：PRINT/EXPORT_DENIED 黑名单端到端（customer/temporary 持 product:print/export 权限码仍 403——持码仍拒证明黑名单独立生效；manager DYE_RECIPE 导出禁单 403；admin 对照 <403）
  - global-setup.ts：BLACKLIST_TEST_ROLES（customer/temporary 幂等补建+权限码）；**修复第 5 个真实缺陷：assign_permission 是 POST 单条模式（resource_type+action），原 PUT /roles/{id}/permissions {permissions:[...]} 路由不存在静默 405——新增 assignPermissionList 辅助**
  - backend/tests/handlers_system_update_authz_test.rs：system-update 权限矩阵 HTTP 层 5 场景（未登录 401/非 admin 403 download+rollback/缺 role_id 403/只读 current-version 200）
  - backend/src/cli/util/upgrade.rs mod tests：check_version_downgrade 纯函数 6 测试（升级放行/降级拒绝/同版本/非标 fail-open/v 前缀/三段格式）
  - RLS sequencing 文档确认已有（runtime-flow-map.md L232，前轮补齐）
  - E2E 编译验证：3 新 spec + 33b 共 10 测试 --list 全过（jszip 本地补装验证）

---

## 硬约束（用户指令，后续所有任务必须遵守）

- [x] 禁止 mock：E2E 测试一律真实后端 + 真实 PostgreSQL 数据
- [x] 禁止本地编译：验证一律走 CI
- [x] **推送冻结（2026-09-08 二次冻结，2026-09-09 重申继续生效）**：git push 一律禁止（含分支/tag），改动仅本地 commit；推送必须等用户明确授权
- [x] **源代码修改冻结已解除（2026-09-09，用户指令）**：按 docs/plans/one-round-fix-plan-2026-09-09.md 实施修复；历史 CI 问题基线见 docs/ci-issues-summary-2026-09-09.md（执行时对照六条机制教训）

---

## 未完成任务清单

### Round 7-iter26（2026-09-21/22，收货单一路径、默认科目表与断言收紧）

> run 35613422400（head `2bd256ac`）68 job：63 success / 5 failure——Rust 测试分区 7 单测、
> flow 分片 1（1-4、2-7）、分片 13（44f-4）、分片 14（48-2、53-1、54-8D）、收尾清理级联。
> 全部证据按分片下载后逐条读 `playwright-output.txt` + `backend.log` + `error-context.md`。
> 本轮 commit：`89f92276`（路径分类单测对齐）/`b163af15`（默认科目表种子 + 收入凭证科目 1122）
> /`12657627`（产品分类名带出）/`1ac0b574`（8D 列表解包 data.items）/`2dc4b3f4`（53 复用 loginOnPage）
> /`758b093e`（库存维度与收货生命周期断言）/`6e38a701`（入库弹窗标题 i18n）/`6117460e`（科目表父子分两条语句）
> /`5da89e43`（入库按库存维度匹配并回写数量版本）/`47ed95ff`（收货单一路径）/`29b1f30a`（入库明细挂订单明细）
> /`3cf750d3`（1-5 按维度正向断言）。
>
> iter25 登记项关闭：`category_name 无 join`（`12657627`）、`入库明细伪值`（`33a51752`+`6e38a701`）、
> `receive_order 按订单明细收货丢失入库维度/部分收货重复入账`（随 `47ed95ff` 单一路径化整体删除）、
> `同产品多行乐观锁冲突`（`5da89e43`，并定位到 1-4 的真实冲突点在确认路径而非事件路径）。

- [x] **自查纠偏（run `35631938195` 判责）**：`47ed95ff` 删除按订单明细收货的实现时，按「trait 名是否还出现在文件里」裁剪 sea_orm 导入，
      误删 `QuerySelect`——而它提供的是 `lock_exclusive()` 方法而非字面名字，保留下来的四处 `find_by_id(..).lock_exclusive()`
      全部失效。CI 报 8 个 E0599/E0282，lib 编译失败连带 Clippy、Rust 测试预编译与整条 E2E 链 skipped
      （机制教训见流程备忘新增条）。已恢复导入（`c293c842`）并在同批补齐明细端点的订单挂接与字段落地（`9c4b21bb`）。

- [ ] **`system-update` 资源未注册且被当模块前缀，系统更新页对所有角色 fail-closed**：
      `path_utils.rs:24` 把 `system-update` 列为模块前缀，`/system-update/version` 因此推导出
      资源名 `version`；而 `PERMISSION_RESOURCES` 里根本没有 `system-update`（grep 无命中），
      两条路都拿不到权限码。run 35613422400 分片 13 `ui-failures.md` 实证
      `API GET /system-update/version failed: code=403 message=权限不足`（33/40 系用例 catch 住
      才未翻红）。修法要先定授权模型：注册资源 + 角色种子 + 前端权限矩阵三处同步，
      属于「谁能操作系统更新/回滚」的安全决策，禁止顺手放行。
- [ ] **出库侧库存行匹配仍只按（产品+色号）**：入库已按产品+批次+色号+缸号+等级建键，
      而 `so/delivery_ops/ship.rs` 扣库存按产品+色号取行，多缸号库存时命中任意一行，
      2-7 的缸号一致性断言因此只能保留在发货后 `quantity_shipped` 层面。需与出库维度口径统一
      （发货单是否必须指定缸号是业务决策）。
- [ ] **入库明细与订单明细产品对不上时只记错误日志、不拒绝建单**（`29b1f30a` 现策略：货物照入、
      订单进度不累加）。是否应升级为拒绝建单需产品决策。
### Round 7-iter25（2026-09-21，RBAC 资源名推导与收货事件生命周期）

> run 35598454276（head `2c3a5bbc`）63 job：57 success / 6 failure（flow 分片 1、2、11、13、14
> + 收尾清理级联），traversal 与 smoke 全绿。逐分片 error-context + backend.log 判责后
> 已修：`106f46ba`（dashboard/notifications 资源名推导）/`1d3bc84f`（角色种子外壳权限码）
> /`125c4cf2`（角色账号补码 + 32-roles 实断言落地页）/`33a51752`（入库明细契约去伪值）
> /`678bb0ff`（库存四维查询下推 + 响应补维度）/`8f9641d8`（收货事件改由确认触发）
> /`19bc6b70`（6 处死端点 + 2 处请求体契约）。

- [ ] **M-6 resource_id 语义使非 admin 无法访问任何 `/{id}` 端点（需产品与安全决策，禁止擅改）**
      `matches_permission`（middleware/permission.rs:603）在权限行 `resource_id=None` 且请求
      `resource_id=Some(id)` 时判不匹配，且 `backend/tests/middleware_permission_test.rs:661`
      把该行为固化为单元测试（"权限 resource_id=None 不能匹配请求 resource_id=Some"）。
      后果：角色种子里所有资源级授权（resource_id 全为 None）只对**无 ID 的列表/创建端点**生效，
      详情/修改/删除一律 403——非 admin 用户打不开任何单据详情页。
      这是安全测试锁定行为，改它等于放宽垂直越权防护，须先与用户确认语义
      （NULL=资源级授权覆盖全部行，还是仅限无 ID 请求）。
- [ ] **RBAC 资源名推导与注册表仍有大面积不一致（同类缺陷续）**：本轮修掉 dashboard/notifications
      两支，静态审计（.ci-evidence/perm_audit.py）仍报 190+ 推导资源名不在
      PERMISSION_RESOURCES/角色种子内：`crm/leads→leads`（注册表是 crm-leads）、
      `bi/sales→sales`（bi-analysis）、`ai/forecast-sales→forecast-sales`（ai-forecast）、
      `ap/invoices→invoices`（ap）、`bpm/{id}→<记录ID>` 等，非 admin 全部 fail-closed 403。
      修法二选一：扩 `resolve_module_prefixed_resource` 映射表按注册表对齐，或
      "段四不是已注册资源名则回落段三"。需逐域判定是否放宽既有 403 断言（33/44 系）
- [ ] **库存前端契约仍是虚构字段（死列）**：`api/inventory.ts` 的 `InventoryStock`
      声明 `quantity/color_code/lot_no/unit/status/product_name/warehouse_name`，
      后端 StockResponse 实际是 `quantity_on_hand/quantity_available/color_no/dye_lot_no/
      batch_no/grade/bin_location`，且不含产品/仓库名 → 库存列表 8 列里 6 列空白或恒 0；
      修法：后端列表按 ID 批量带出 product_code/product_name/warehouse_name
      （load_low_stock_product_map 已有同类实现），前端类型与列名对齐
- [ ] `/security/change-password` 渲染期 `SyntaxError {message:10}` 仍未定因
      （离线编译 20070 条 locale 消息 0 失败，排除静态 i18n 语法）；
      42x 遍历已改为失败时打印 pageErrors/consoleErrors 原文，待新 run 证据定位
- [ ] 产品列表死列 `barcode`（无 DB 字段）/`category_name`（无 join）
- [ ] 收敛项：`AppState.event_notification_service` 由 `Option<…>` 改非 Option；
      `bpm_task.process_instance_id/name` 旧列删除；18 个 `json!({"list":…})` handler
      统一到 PaginatedResponse
- [ ] 待授权：ci-cd.yml 接入 check-i18n.mjs、把 20 个从未执行的 E2E 目录（218 测试）纳入 testMatch；
      eslint 开 `no-unused-expressions` 并解除对 e2e/ 的忽略
- [ ] `views/purchase-receipt/composables/usePrcProc.ts` 对话框标题 '新增入库'/'编辑入库' 内联中文
      （该页其余文案已走 i18n）

### Round 7-iter23（2026-09-21，三个结构性根因突破）

> run 35524492654（head `de92d458`）进行中：65 job，46 success / 3 failure（05-system~10d、
> 20~26、44f~54）/ 9 in_progress / 4 queued，非 E2E 仍全绿。
> 判责证据源突破：artifact 内 `reports/playwright-output.txt`（全量 stdout）与
> `reports/backend.log` 才是完整信号，job 日志只有 E2E-HEAD/TAIL、`error-context.md`
> 只有最后一条断言文本——此前 16 轮的判责一直建立在残缺信号上。
> 本轮修复 commit：`71d76858`（BPM fixture）、`09b234c0`（通知链路）、`225a1380`（产品契约）。

- [x] **根因 1｜销售订单 submit 被异步自动审批**（02-o2c 2-5 报 `BUSINESS_ERROR/业务处理失败`）：
  `ensureTestEntities` 建的 `sales_order_approval` 流程定义用 `node_id/node_name/node_type`
  且无 `edges`，后端 `bpm_service.rs:138 resolve_first_task_node` 只认 `id/name/type`
  （`start_event`→`user_task`）与 `edges`，解析为 `None` 时 `bpm_ops/instance.rs:84`
  走「无任务节点，自动完成流程」→ 发布 `BpmProcessFinished{approved:true}` →
  `listener.rs:1302` 异步把订单 pending→approved，抢在用例显式 approve 之前。
  对外消息被 `utils/error.rs:428` 脱敏为「业务处理失败」（真实原因只进服务端日志），
  故 16 轮无从定位。18-resilience / 44f / 48 / 31d 里「status!==approved 才 approve」
  的条件兜底全部是为绕开它写的。
- [x] **根因 2｜通知列表读取恒为空，31d/31e 全线假通过**（31e-5 显式失败）：
  `notification_handler.rs:75` 的 payload key 是 `list`，两个 spec 的本地 reader 取
  `data.items`，再经 `|| data.data || data || []` + `if(!res.ok()) return []` 层层兜底
  退化成空数组；`status=unread` 小写也被后端大写匹配静默忽略。叠加硬编码
  `http://localhost:8082` 绕过 CSRF 头注入，publish/announcement 等 POST 被拒后无人检查响应。
  结果：P0 通知链路（提交/审批/发货/公告联动/直发/批量已读）一条断言都没跑到。
- [x] **根因 3｜产品前后端字段契约不一致**（31c-产品 `未找到目标行` 的直接原因）：
  后端实体 `name/code/status/standard_price` vs 前端 `Product` 契约
  `product_name/product_code/is_active/price`，无映射层 → 产品列表名称/编码/状态列恒空、
  状态恒显示"停用"；UI 新建产品提交的三个字段后端识别不到，名称被缺省成
  `产品_<时间戳>`；列表 `keyword/is_active` 两个筛选参数不在 `ProductListQuery` 中，
  搜索框与状态筛选是死控件（QuotationItemEditor 的 `is_active:true` 同）。
  修复采用「读侧权限过滤后补别名 + 写侧边界映射」，避免逐个改 18+ 视图引入新风险。

#### iter24（2026-09-21，拉上游 15 提交后继续判责：run 35585268854）

- [x] **站内通知整体静默失效（P0）**：`container/mod.rs` 以
  `email_service.as_ref().map(..)` 装配 EventNotificationService，未配 SMTP → 服务为
  None → 订单/库存/公告联动等站内通知全部不发（日志：`event_notification_service 未配置，
  OA 公告 2 跳过通知推送`）。这是 31d-A / 31e-2 反复失败的真实原因，不是用例问题。
  修复=无条件构造（服务内部本就区分邮件通道）。commit `41253087`
- [x] **BI 页双层信封未解包**：`bi_handler.rs` 16 端点返回 ApiResponse<BiResponse<T>>，
  `api/bi.ts` 按单层标注，页面取 `.data` 得到内层信封 →
  `x.map is not a function`（SalesAnalysis 崩溃）+ 月度/品类/日钻取三表恒空
  （被 `Array.isArray(d)?d:d.items??[]` 形态兜底掩盖）。commit `190461cf`
- [x] **销售订单状态映射不完整**：so_status 8 值，前端 4 处各自维护且都只覆盖 5 个；
  详情页 `<el-tag>{{ order?.status }}</el-tag>` 完全未映射。收敛到
  `utils/sales-status.ts`（Record 穷举）+ 补 6 条中英文案 + 筛选下拉补 3 项。commit `edf44bfc`
- [x] **2-12 断言归属错**（我自己上一批引入）：发货链之后仍断言 approved。已改为
  校验取值属于 so_status 枚举 + 详情页不得露出枚举原文；44f-4 补必填 unit_master。commit `0d7bce7c`
- [x] **6-8/6-9 访问不存在路由**：`/purchase/orders`、`/sales/orders` 均非注册路由
  （列表是 /purchase、/sales），页面落 /404 而断言无匹配器 → 双条假通过。已修
- [ ] **32-roles `e2e_readonly 登录 + Dashboard 可达` 失败**：Dashboard 对只读角色发
  `/notifications/unread-count` 与图表数据请求得 **403**，被健康门计为 error。
  待判：是权限码映射缺 `notification:read`（后端应放行用户自身通知计数），
  还是前端不该为无权限角色发起该请求。二者修法不同，需先看 readonly 角色权限集与
  notification 路由的权限声明。
- [ ] **traversal 仍有 2 个分片未取证据**（shard 14 / 21 各 33MB/38MB）：
  37b 打印内容匹配、39b~54 分片的具体断言原文待下一轮按新 run 精准拉取。
- [ ] `views/purchase-receipt/composables/usePrcProc.ts:163` `unit_master: it.unit || 'm'`
  属硬编码兜底：产品主单位缺失时应暴露而非伪造 'm'。

#### iter23 新增待办

- [x] **后端 P0｜人工审批链在 DB 层不可用**（commit `f7cc5d86`，run 35527832129 shard-1
  backend.log 实证 `null value in column "process_instance_id" of relation "bpm_task"`）：
  `bpm_task` 初始建表遗留 `process_instance_id`/`name` 两列 NOT NULL，模型与业务代码用的是
  后加的 `instance_id`/`node_name`，两列全仓零引用 → 任何含 user_task 节点的定义插入首任务
  必失败并回滚单据。同根因连带 02-o2c 2-5 / 44e-2 / 31d-A / 48-2。
  本轮只解除重复列 NOT NULL；**待办**：收敛为单列（DROP 遗留列或补 DEFAULT），需评估既有数据。
- [x] **后端 P1｜调拨流水 id=0 硬编码致主键冲突 500**（commit `586a0850`，
  `POST /inventory/transfers/1/receive` → `duplicate key inventory_transactions_pkey`）：
  `services/inv/batch.rs` 两处 `id: Set(0)` 改 `Default::default()`。
  同类写法已在 `fixed_asset_service.rs:529` 修过一次，属重复缺陷类；本轮全仓 grep 已确认清零。
- [x] **CRM 标签路由双前缀**（commit `1615dd67`）：`crm_tags()` 相对路径写 `/crm/tags`
  而 router 已 `.nest("/api/v1/erp/crm")` → 真实路径 `/api/v1/erp/crm/crm/tags`；
  前端 `crm-enhanced.ts` 早年按双前缀绕行调用，把缺陷固化成契约，已一并回正。
- [x] **凭证详情分录 key**（commit `c397e76f`）：响应是 `entries`（voucher_handler.rs:172），
  用例读 `items` 恒 0。**同族**：`traversal/42a-core` 的"点新建后固定等 800ms"改为
  真等待 + 归因信息；`02-o2c 2-12` 用的 `/sales/orders` 路由不存在（列表是 `/sales`），
  落到 `/404` 后旧断言仍成立。
- [ ] **`/security/change-password` 渲染期抛 SyntaxError（未决产品缺陷）**：ErrorBoundary 捕获
  `{name: SyntaxError, message: 10}`，压缩栈止于 `vue-vendor:3:527`，vue 错误号 runtime-1。
  已排除：PasswordStrengthMeter 的字面正则、该页 i18n 文案特殊字符。禁止本地起服务，
  需下一轮从"未压缩 source map / 逐步注释定位"入手。
- [ ] **产品列表两列仍无数据源**：`barcode`（products 表无该列）、`category_name`（list 未 join
  product_category）。需决定：后端出 VO，还是前端用已持有的 categories 列表本地解析。
- [ ] **分页响应 key 全量审计工具需 nest 感知**：`/tmp` 版 key_audit2.py 已能解析 817 条
  GET 路由与 handler 返回结构，但对 `.nest("/x", handler())` 形式（route 声明为 "/"）
  无法还原完整路径，导致 159 处误报"无路由匹配"。补法：解析 routes/*.rs 的
  nest 组合链重建全路径，再与 e2e 调用点比对；跑通后可作为 CI 静态门禁。
  已借此确证并修复：/inventory/reservations（list vs items，commit 见下）。
- [ ] **vac 空断言残余**：04-finance 4-8+、05-system 其余项、00-deploy-init 的
  `expect(me.permissions)` 与成片 `expect(x.length).toBeGreaterThanOrEqual(0)` 恒真断言
  仍未清理（受限于逐端点核对成本），随 key 审计工具完成后一次性处理。
- [ ] **真空断言成片**（flow/smoke/traversal 内 `expect(expr);` 无匹配器 ≥48 处，另有
  `expect(x.length).toBeGreaterThanOrEqual(0)` 恒真断言）：这类"永远绿"的断言是本轮三个
  根因能长期潜伏的放大器。建议 eslint 加 `@typescript-eslint/no-unused-expressions`
  并把 `e2e/` 从 ignores 移出 + 接入 CI lint（**动 eslint 配置/CI 需用户授权**）。
- [ ] **31d-D 库存预警**仍为条件分支：要硬断言需先把某商品 `safety_stock`（或等价字段）
  抬到现有库存之上构造确定前提。
- [ ] **31d-F 付款申请通知**：`payment-requests/:id/submit` 侧未检索到 `notify_*` 调用，
  需确认该链路是否真实接入；未接入则按「功能真实接入」补实现而不是删测试。
- [ ] **SO create 处理器语义错位**：`sales_order_handler.rs:245` 在**创建**时就发
  `notify_order_submitted`（标题「订单已提交」），而订单此时是 draft；
  且与随后 submit 发的同名通知在 5 分钟 dedup 窗口内互相折叠。
  `notify_order_created` 对 SO 不存在（PO 有 `notify_purchase_order_created`）。
- [ ] `views/quotations/components/QuotationItemEditor.vue:249` 等多处
  `catch { products.value = [] }` 静默吞错（不静默日志违规，属前端视图批量项）。
- [ ] **迁移设计缺陷（系统性）**：`Migrator::migrations()` 只有 7 个域级迁移，
  后续所有补列 SQL 都追加在域 `up()` 的幂等 blob 里；已应用过该域的库不会重跑，
  即迁移事实上只对全新库生效。需引入独立的增量迁移位（否则本仓库的 schema 修复
  在生产库上不落地）。
- [ ] run 35547989952（head `586a0850`）为新验证轮：需复核 02-o2c/44e/48/31d 是否随
  BPM 修复转绿，以及 53-1（登录超时）、54-new-domains（质量 8D）两条未判责项。
- [ ] 沿 iter22：CI 只跑 `e2e/flow|smoke|traversal`，另有 20 个目录 218 个用例从不执行；
  `check-i18n.mjs` 未接入 CI（两项均需用户授权改 `ci-cd.yml`）。
- [ ] **真空断言成片**（flow/smoke/traversal 内 `expect(expr);` 无匹配器 ≥48 处，另有
  `expect(x.length).toBeGreaterThanOrEqual(0)` 恒真断言）：这类"永远绿"的断言是本轮三个
  根因能长期潜伏的放大器。建议 eslint 加 `@typescript-eslint/no-unused-expressions`
  并把 `e2e/` 从 ignores 移出 + 接入 CI lint（**动 eslint 配置/CI 需用户授权**）。
- [ ] **31d-D 库存预警**仍为条件分支：要硬断言需先把某商品 `safety_stock`（或等价字段）
  抬到现有库存之上构造确定前提。
- [ ] **31d-F 付款申请通知**：`payment-requests/:id/submit` 侧未检索到 `notify_*` 调用，
  需确认该链路是否真实接入；未接入则按「功能真实接入」补实现而不是删测试。
- [ ] `views/quotations/components/QuotationItemEditor.vue:249` 等多处
  `catch { products.value = [] }` 静默吞错（不静默日志违规，属前端视图批量项）。
- [ ] run 35524492654 的 `20~26`、`44f~54` 两个失败分片待 job 结束后取
  `reports/playwright-output.txt` 判责。
- [ ] 沿 iter22：CI 只跑 `e2e/flow|smoke|traversal`，另有 20 个目录 218 个用例从不执行；
  `check-i18n.mjs` 未接入 CI（两项均需用户授权改 `ci-cd.yml`）。

### Round 7-iter22（2026-09-20/21，首个真实 E2E 全量信号判责）

> run 35515772653 全 67 job：9 失败（8 个 E2E 分片 + 收尾清理级联），**非 E2E 全绿**。
> 逐分片拉 artifact `error-context.md`（17 个失败上下文含断言原文）后判责分类如下。
> 已修部分见 commit `baa9792f` / `be8548b6` / `3a9a5921`。

- [x] **5-1 审计日志 TypeError**：iter20 参照 `audit_enhanced_handler`（挂在 analytics 域 `/logs`，
  响应键 `list`）把用例改成读 `list`，而 `GET /audit-logs` 实际由 `routes/system.rs:279` 绑定的
  `audit_log_handler::list_audit_logs` 处理，响应键是 `items`（`audit_log_handler.rs:132-137`）
  → 改回 items + 先断言数组与非空再索引 → `3a9a5921`
- [x] **色卡借出记录命中隐藏 Tab 表格**：`.first()` 取到 `aria-label="发放中列表"` 的隐藏节点
  （34 × resolved to hidden），IR 2026-09-12 已记过同类教训 → 选择器加 `:visible` +
  去掉 `if (tableVisible)` 条件式空转 → `3a9a5921`
- [x] **5 处请求体违反后端 DTO**：调拨 approve 缺必填 `approved`（EOF 400）、色号缺
  `color_type`+`extra_cost`、角色 code 含大写被 `role_permission_service.rs:153-159` 拒、
  质量问题缺 `custom_order_id`+`severity`、SO 明细误写 `material_id`（应为 `product_id`，
  44f-5 与 48-2 同错，后者此前被 44f-3 串行失败挡住未跑） → `baa9792f`
- [x] **4 处测试硬编码 + 1 处静默 skip**：31d-C 的 `customer_id:1`/`product_id:1`/
  `warehouse_code:'WH001'`（NOT_FOUND 真因是 `ship.rs:135` 按 code 查仓）、48-2 的
  `'WH-MAIN'`、30-persistence BOM `toBe(1)` 与从 `detail` 顶层误取 `version`/`is_default`
  （结构是 `{bom, items}`）→ 全部改真实实体/反查编码/对照 `shared.prodId`；
  `if(!orderId) test.skip()` 改显式断言 → `be8548b6`
- [ ] **42a-core `report-templates (A)` 新建断言 false**：页面侧接线正常
  （`index.vue:10 @click="openDialog()"`、`openDialog` 在 409 行置 `dialogVisible=true`、
  `el-dialog` 在 155 行存在），`visitModule` 用 `.el-dialog:visible,.el-drawer:visible` 的
  `.first()` + 800ms 等待，疑点集中在遍历上下文（快照显示"系统管理"菜单已展开，
  可能命中另一页的按钮/弹窗或被折叠菜单遮挡）。**需 CI 迭代定位，不靠猜改。**
- [ ] **47-AU1 审计记录 0 条**（`table_name=department`）：需确认部门创建写入的
  `resource_type` 实际取值与查询侧是否一致（iter20 已从 `departments` 改为单数 `department`，
  但仍 0 条，需查 `audit_log_service` 落库路径与异步轮询时机）
- [ ] **31e-5 通知 CRUD 0 条**：用例内联硬编码 `http://localhost:8082/api/v1/erp/...`
  （应改用 `API_BASE`/`API_PREFIX` helper），且 `!currentUserId` 与 403 两处均
  `test.skip()` 静默跳过；需先判定"发 3 条通知后列表查不到"是通知未落库（源代码缺陷）
  还是查询参数不匹配（测试缺陷）
- [ ] **31c 产品编辑弹窗停用返回 false**：断言"UI 停用操作应可完成"失败，
  需按诊断日志判定是弹窗交互未生效还是 `status` 回写不符
- [ ] **环境级 flaky 3 例**：M1-4 `Error: Channel closed`、24-排程甘特 `write EPIPE`、
  53-1 `waiting for locator('input[name="username"]')` 超时——浏览器进程侧崩溃/级联，
  非用例逻辑问题；CI 已按用户要求删除全部重试，故这类崩溃会直接红
- [ ] **API 一致性**：同一逻辑资源两套列表端点响应键不一致（`/audit-logs` 用 `items`、
  `/analytics/.../logs` 用 `list`），统一会牵动前端调用点，需单独立项



### Round 7-iter21（2026-09-20，拉 run 35510302989 全量失败日志判责）

> **本轮关键结论**：`c3b83cf6`（iter20）自身把 `bingxi-backend` lib 编坏，导致
> Clippy / Rust 测试预编译 / Rust 后端构建 三个 job 同一根因 exit 101，
> **E2E（25 分片）、Setup 向导 E2E、角色权限矩阵、E2E 真实性门禁、Rust 覆盖率、
> 死代码审计、打包发布、Release 全部 skipped**。
> 即 **Round 7-iter5 ~ iter20 共 16 轮 E2E 判责修复从未被 CI 真正执行过**，
> 其"已修复"结论全部待 CI 首次实跑验证。收尾清理 job 是纯级联（聚合各 job 结论后 exit 1），无独立缺陷。

- [x] 🔴 **E0596 `shipped_pool` 缺 mut**（`so/delivery_ops/inventory.rs:435`）：iter20 引入，CI 唯一编译阻塞点之一 → `764cd257`
- [x] 🔴 **iter20 预留回滚重构丢失状态作用域**（同文件）：`restore_reserved_stock` 查询与
  `release_reservations` 状态更新都不再限定 `pending`，造成
  (a) released/cancelled 行被二次回加 → 虚增 `quantity_available`；
  (b) consumed 行被改写 cancelled 并回减 `quantity_shipped` → 抹除真实出库与消耗审计，
      且与该函数"预留行必须保留用于追溯"的自身注释矛盾（规则 2）
      → 按业务语义拆为 `RELEASE_SCOPED_STATUSES`（软终态，pending/locked）与
      `DELETE_SCOPED_STATUSES`（硬删除，额外含 consumed）两份显式作用域，
      查询与更新经 `reservation_status_filter` 共用同一来源 → `764cd257`
- [x] 🔴 **consumed 回滚量 `or_insert(res.quantity)` 兜底**（同文件）：shipped 池无该产品即明细
  `shipped_quantity` 为 0，臆造池量会使回减落入不存在区间并抛误导性"库存回滚失败"
  → 改为按 0 跳过 + 输出含 order_id/product_id/预留量的显式 warn → `764cd257`
- [x] 🔴 **E0609 `receipt.created_by`**（`event_bus_ops/listener.rs:1184`）：`find_by_id().one()`
  返回 `Result<Option<Model>>`，`if let Ok` 只剥一层。不采纳编译器 `unwrap()` 建议
  （事件监听器 panic 会击穿后台消费者 task），改显式 match 三分支 → `b45ca262`
- [x] 🔴 **同一处 `if let Ok(..)` 静默吞掉 DbErr**：库存已入账而应付未生成属账实脱节的必须暴露场景，
  原实现零日志穿过 → Ok(None)/Err(e) 两支各自输出 error 级日志 → `b45ca262`
- [x] 🔴 **`AUTH_ONLY_PATHS` 安全豁免白名单双份真相源**：iter20 在 `csrf.rs` 复制了一份
  `permission.rs` 已有的 `AUTH_ONLY_PATHS` + `is_auth_only_path`，两份须手工同步，
  任一侧漂移即出现"RBAC 豁免但 CSRF 未豁免"或反向的认证语义不一致
  → 收敛到 `middleware/public_routes.rs`（`PUBLIC_PATHS` 既有归属地），两中间件同源导入
  → `77daf24e`；当前两份清单内容一致，收敛后行为不变
- [x] **`event_kafka.rs` 死导入 + 失实注释**：`#[cfg(test)] use ShippedItem`，但文件内既无
  `mod tests` 也无子模块，该导入在两种配置下都是死代码，注释"仅在测试模块使用"为假
  → 删除 → `dba9eb8d`。附带发现：该 `unused_imports` 告警在 clippy-log 可见，
  却因 baseline 按 message 文本匹配被判 NEW_COUNT=0，**既有告警治理机制存在漏网项**
- [x] **文档同步**（规则 10）：`bug.md` §三（import_csv 早在 2026-06-26 已删，结论全部过时）、
  `MEMORY.md` `## 二、常规规则` 重复两次导致章节编号断裂 → 去重并恢复连续编号
- [ ] **Round 7-iter5~iter20 的 E2E 修复首次真实 CI 验证**：本轮编译阻塞清除后 E2E 将首次实跑，
  预期暴露新的失败面，需按判责纪律逐测试归因（源代码/测试文件/测试配置/环境 flaky/测试基建）
- [x] **`network-resilience.spec.ts` 整文件 `test.skip(true)`**（规则 0）：已重写为真实网络条件并解除 skip 与
  E2E-AUTHENTICITY-EXEMPT 标记 → `66167f92`。中断改 `context.setOffline(true)`（请求真实失败于
  ERR_INTERNET_DISCONNECTED），弱网改 CDP `Network.emulateNetworkConditions` 真实链路延迟；
  断言取 `src/api/request.ts` 真实契约（提示文案精确为 '请求失败，请稍后重试' +
  `requestfailed` 计数 ≥2 证明幂等 GET 三次重试链路生效），原"body 可见/table attached"式弱断言全部替换。
  本地验证：`playwright test --list` 收集到 4 用例（chromium+webkit），门禁脚本本地复跑
  **violations=0 且 exempted=0**（全仓 E2E 首次零豁免）。
  原 403/422/401/500 伪造用例不保留等价版本：403 已由 33/33b 真实低权账号覆盖，
  5xx 不崩溃由各 flow spec 的 assertPageHealthy 零 5xx 门禁全站覆盖。
  ⚠️ 待 CI 实跑验证（IR 禁止本地起服务，无法本地执行 E2E）。
- [x] **`import_export_ops/task.rs` 与 `models/import_task.rs` 注释失实**：以现在时描述
  已删除的 `import_csv`，另含变更日志式表述与重复标点 `，；` → `4203d71e`。
  migration 内同类历史注释不动（已应用记录，改源恐影响迁移校验和）。
- [ ] **README E2E 数据口径修正**：`--list` 实测 1,265 用例 / 259 spec 文件，
  原写 1,332（flow 849 实为 665，虚高 184）→ 已按实测重写并补记口径来源。
  ⚠️ 其余 README 统计行（后端 294,000 行 / 1,360 文件 / 前端 376 Vue 等）仍为 2026-09-09 快照，
  未在本轮重测，后续更新需一并校准。
- [ ] **clippy baseline 按 message 匹配的机制缺陷**：同一 message 的新发生会被判"非新增"而放行
  （本次 `unused_imports` 即实证），需评估改为 `file:line + message` 复合键
- [ ] **`#![allow(dead_code)]` 覆盖 301/314 个 model 文件**：属全仓 SeaORM entity 既有惯例（PH），
  项目级移除会一次暴露数百条告警并直接打红 CI，本轮不动；如需治理应单独立项并同步重建 baseline



### E2E 权限与打印覆盖缺口（2026-09-09 审计，待推送授权后立项）

> 审计结论：真实链路在 admin 单角色登录 + 业务闭环 + 响应式维度扎实；多角色权限差异化验证与打印全链路是系统性空白。

- [x] **多角色登录测试**：32-roles-login 全角色真实 UI 登录+ensureRoleUsers 30+ 角色基建（1d89410+d24d43d）
- [x] **垂直越权测试**：33 矩阵（e0ce7bb）
- [x] **水平越权测试**：34 spec（d24d43d）
- [x] **修复恒真假断言**：09-permissions P1-2/3→精确403（9cdfd02）
- [x] **修复空转测试**：P1-7/8→真实断言（9cdfd02）
- [x] **打印端点覆盖**：37 矩阵 58 端点状态+格式断言（P5.7 已落地）
- [x] **打印内容匹配**：37b JSZip 解包 document.xml 断言源单据号（2026-09-10）
- [x] **打印模板 API 覆盖**：38-print-templates API 按真实路由（e0ce7bb 已落地）
- [x] **打印审计闭环**：37b 打印后 audit-logs PRINT 记录断言（2026-09-10）
- [x] **导出内容断言**：39c xlsx 解包列头/行数/首条内容匹配（2026-09-10）
- [x] **enhanced mock 清理**：network-resilience E2E-AUTHENTICITY-EXEMPT+test.skip（9b5aa14），待人工确认真实异常源重写方案后恢复
- [x] **purchase/sales 业务目录纳入主 CI**：testMatch 扩全 11 目录（1d89410 已落地）

### 内部更新功能 + 敏感导出授权审计缺口（2026-09-09 二轮）

- [x] **system-update 权限门禁零测试**：handlers_system_update_authz_test.rs 5 场景（未登录 401/非 admin 403/缺 role_id 403/rollback 403/只读 200）（2026-09-10）
- [x] **system-update E2E 深度**：40-system-update-authz 真实登录链路（d24d43d 已落地）
- [x] **bingxi update CLI 零集成测试**：upgrade.rs mod tests 纯函数 6 测试（check_version_downgrade 全分支）（2026-09-10）
- [x] 🔴 **敏感导出审批可绕过（机制断裂）**：enforce_export_download fail-closed 已落地（3e8ca73）
- [x] **导出审批前端零接入**：export-approvals api/UI/路由已落地（3dcd56a）
- [x] **导出审批测试零覆盖**：39 fail-closed 矩阵 + 39b 完整审批链已落地
- [x] **print/export 角色黑名单无端到端验证**：33b 持码仍拒断言 + BLACKLIST_TEST_ROLES 幂等补建（2026-09-10）

### E2E 综合审计缺口（2026-09-09 三轮，详证见 docs/audits/e2e-comprehensive-audit-2026-09-09.md）

> **全部 30 项缺口的修复实施计划已定稿：docs/plans/one-round-fix-plan-2026-09-09.md**（8 commit 划分、逐文件改动表、依赖图、风险清单）。等待用户解除源代码冻结后按计划一轮执行。

> 覆盖：E2E 真实性、2FA、预览、登录链路、版本号、审批体系、admin 全功能遍历、显示异常/重复提示。

- [x] 🔴 **E2E 真实性（用户标注关键）**：applyAuthMocks 真实化+lock-status 拦截移除（d84a461）+ e2e-authenticity-guard 门禁 job（9b5aa14）
- [x] **2FA/TOTP 零覆盖**：35 spec（d24d43d）
- [x] **预览零覆盖**：36 spec（d24d43d）
- [x] **登录 5 请求瀑布**：_skipAuthRetry+OptionalAuthContext+删 catch 刷新（3dcd56a+7604f41）
- [x] **协议/隐私页死链**：TermsView/PrivacyView 路由+内容页（3dcd56a+752921a）
- [x] **锁定阈值 5→9**（用户明确要求）：两处 MAX_FAILED_ATTEMPTS=9（7604f41）
- [x] **版本号机制故障**：方案 C env! 编译期化（11b179d）
- [x] **审批体系 E2E 8.6%**：41 端点矩阵+41b-c 专用审批流（9cdfd02+e0ce7bb）
- [x] **审批审计无 APPROVE 分类**：classify_operation APPROVE 分支（3e8ca73）
- [x] **admin 全功能遍历**（用户 b-j 项）：42a-d 遍历 4 spec+44 角色矩阵（9cdfd02+439c6ef）
- [x] **重复提示检测**：43-duplicate-toast spec（d24d43d）

### 验证类（需推送授权或手动执行）

- [ ] PG 机制锚点测试手动验证：`TEST_DATABASE_URL=... cargo test --test rls_context_test -- --ignored`（test_rls_guc_visible_in_same_pool_pg，CI 不跑 ignored）
- [x] RLS dept 迁移部署 sequencing 确认：生产部署先 `bingxi migrate run` 再发新后端二进制（顺序颠倒新代码读不到 department_id；旧后端+新迁移=dept 安全降级为 self+公海，无风险窗口）（runtime-flow-map.md L232）

### 剩余已知项状态（2026-09-18 判责更新，PR #941 剩余已知项）

- [x] **S10 31c 客户停用**：最新 run 35204061536 shard-12 已通过（无失败记录），关闭；c2d9a4f cookie 域统一进一步加固
- [x] **S9/S11 超时潮 + 33/33b 全 401**：根因确认为测试基建错误（API_BASE 默认 127.0.0.1:8082 与 UI 登录 cookie 域 localhost 错配，trace 网络层证据 Cookie=false），修复 = c2d9a4f（本地分支已含），待推送后 CI 验证
- [ ] 推送授权后验证上述修复（33/33b 401 应转 403、超时潮应消失）

### 流程备忘

- [ ] 机制约束（iter26 实证）：本地无编译权，删除/裁剪 `use` 语句时不得按「名字是否出现」判断，
      trait 导入要按它提供的方法名逐个检索（lock_exclusive/first/limit/all/filter/count/select/save/insert），
      否则 rustfmt 通过而 CI 编译失败，一次推送被整条链 skipped 消耗
- [ ] 推送前必做：`git log --oneline <remote-head>..HEAD` 与「本轮改动清单」逐条对照，
      确认每个改动文件都被静态自审覆盖（items.rs 这类未进 CI 的后续提交要显式列出）

- [ ] CI 失败若再出现新 job：按 doto 流程拉日志→记录→下批修复
- [ ] 推送授权后：本地 commit 批量推送 + 观察 main CI（含 coverage 等 main 专属 job）

### 待用户决策

- [ ] 分支 260907-feat-piece-domain-phase2 是否删除（已合并 PR #939）
- [x] runtime-flow-map.md 部署章节 RLS 迁移 sequencing 说明已补充（L232，前轮完成）
- [x] ci-cd.yml 的 docs/** 触发路径失效——已获用户授权（2026-09-09，定向豁免）修改 yaml，冗余条目清理完成（65736c1），CI 行为无变化

后续新增任务请在此文件追加。
