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

### Round 7-iter31（2026-09-22/23，CI 假绿清零：分片矩阵扩容 + 后端契约缺陷批量落地；**全部未经 CI 验证，推送冻结中**）

**本轮最重要的判责结论（推翻上一轮的"CI 回归"假设）**：run `35722450943`(4629) 的 7 个 flow 分片 `exit: 124`
**不是** 88f59a7c 那次 CI 复用改造引入的回归。逐 job 时长对照 4625 与 4629 两个 run：
同样这 7 个分片在 4625 里也是 26~27 分钟（= 撞上 `timeout 1500`），只是当时
`EXIT_CODE=$(tail -1 /tmp/shard-exit.txt)` 在包装进程被 timeout 杀掉后取到空串，
`exit $EXIT_CODE` 退化成 `exit`（=0），于是**七次超时被记成七次通过**。
我补的 `EXIT_CODE=${EXIT_CODE:-124}` 只是把假绿变成真红。
证据链：分片 2 的 `reports/playwright-output.txt` 有 49,950 行、进度标到 `[41/42]`、
`12:04:38` globalSetup 登录 200 OK、直到被杀输出持续增长 —— watchdog 的"停滞 180s"从未触发，
即**没有任何挂起证据**，纯是 1500s 上限小于 flow 分片真实耗时。
教训并入 MEMORY：判责"是不是我这次改动引入的"必须先看**上一轮同一 job 的时长与退出码链路**，
不能只看红/绿；红绿本身可能是被一个 `exit` 语义 bug 造出来的。

**CI 改动（`.github/workflows/ci-cd.yml`，均本地未验证）**：
- flow 分片 15 → 20（`--shard=k/20`），单轮上限 1500s → 2400s；20 分片下单片用例数下降，
  2400s 相对最慢分片仍有一倍以上余量。
- 分片 label 纠错：原 label 按 spec 号段命名（如 `flow: 11~22-returns/inventory/crm`），
  而 Playwright `--shard` 是按用例 hash 分配，**label 与实际跑的 spec 无关**——
  该分片实测跑的是 05-system/06-collaboration/07-fabric-four-dim/08-business-modes。
  按错 label 排查会把人引向完全无关的 spec。现 label 只标 `目录(片号/总片数)`。
- flow 与 traversal 两份逐字相同的 30 行 watchdog 合并为一个（参数集中派生）；
  traversal 的停滞告警原文写"kill 进程树后重跑"而代码根本不重跑，文案与行为对齐。
- 新增 extras 4 分片：`e2e/enhanced|purchase|sales|purchase-ext|quality|finance|crm|bpm`
  八个目录 + 根级 3 个 spec，共 **131 个用例 / 33 个文件**（`--list` 实测）此前
  **从未被任何分片执行**：testMatch 早就把它们纳入（注释还写着"去 mock 后全部进主 CI testMatch"），
  但分片命令只传 flow/smoke/traversal 三个目录，"纳入"从未变成"执行"。
  这八个目录 grep 零 `page.route()`/`vi.mock()`，是真实后端链路，故直接进矩阵不再拖延甄别。
- `package-release` 增加 `security-vulnerability-scan` 前置（用户批准的门禁加强）。
  核查 `88f59a7c^` 的 needs 清单证实：扫描类 job **从来不在**发布前置里，
  即高危依赖只会让 🛡️ job 自己红，拦不住 v* tag 产出 release 二进制。
- eslint：`e2e/` 与 `tests/` 从全局 ignores 移除（此前测试代码完全不受 lint 约束，
  `expect(x).toBe` 漏括号这类"看着在断言实则空转"一条都拦不住）；
  spec 文件里 `@typescript-eslint/no-unused-expressions` 关掉 `allowShortCircuit`/`allowTernary`
  （`ok && expect(x)` 在 ok 为假时一条断言都不执行却通过）。
  实测当前命中 0，故该规则是回归门禁而非清账。**遗留待清**：解 ignore 后暴露 28 个既有 error
  （`no-non-null-asserted-optional-chain` 18 / `prefer-const` 8 / `no-useless-catch` 2），
  不同步清掉 `ci-lint-fe` 会直接红 —— 已登记，属本轮必须收口的未完项。

**后端落地（子智能体 + 自审，均未编译验证）**：出库四维扣减与显式跨缸回退
（`services/inventory_deduction.rs` 纯函数 + 8 个单测；`so/delivery_ops/{inventory,ship,cancel}.rs`、
`inv/batch.rs` 接线；跨缸实扣缸号/批次如实写流水）、销售退货明细补齐 5 个 NOT NULL 列
（`sales_return_service.rs`，算法照抄采购退货/销售出库同族口径）、调拨明细出参补回
`color_no/dye_lot_no/batch_no` 三列、查询参数空串在 HTTP 边界归一为 None
（`utils/query_params.rs` 中间件 + 字段级 serde 原语，根治 `WHERE col = ''` 恒 0 行这一类）、
`products.barcode` 列与三列 OR 检索。
**自审抓到的一处致命点**：`plan_deduction` 的排序器写成 `a_exact.cmp(&b_exact)`，
Rust 里 `false < true`，于是"精确命中指定缸号"的行被排到**最后**——出库会先扣别的缸，
与函数头声明和用户拍板的口径完全相反；同文件两个单测（`test_exact_hit_deducts_only_requested_dye_lot`、
`test_partial_hit_triggers_deterministic_cross_dye_lot_fallback`）本来就会红，
已改为 `b_exact.cmp(&a_exact)`。**这正是"注释与代码各说各话"的样本**：
原作者在旁边写了 `// false < true：精确行排前`，推理本身是错的。

**需要用户动手的一项（本 token 无权限核查）**：分片 label 纠错 + `ci-audit`/`ci-deps` 合并会改变
GitHub 检查名（如 `🎭 E2E: flow: 05-system~10d-extended` → `🎭 E2E: flow: e2e/flow(2/20)`，
`🛡️ 依赖审计`/`📦 依赖图记录` 已随合并消失）。若分支保护把逐个检查名列为 Required，
真绿也仍会被挡在合并外：需要改成只把聚合 job `🧹 收尾清理` 设为必需，或按新名重新勾选。

**新闭上一个缺陷类别：库层 DEFAULT 取值越出该列状态词表**。全量枚举 108 个带 DEFAULT 的
状态/类型/级别 VARCHAR 列（不抽样），逐列按「词表常量 → 列上 CHECK → service 建单写值 → 前端筛选」
定取值域，查出 **9 列默认值根本不在自己的域内**并在 v15 尾部统一 `ALTER ... SET DEFAULT` + 回填：
`inventory_transfers/inventory_counts`(draft→pending)、`sales_delivery`(DRAFT→pending)、
`sales_contracts/purchase_contracts`(DRAFT→小写 draft)、`bpm_task`(PENDING→pending)、
`dye_batch`(pending→pending_schedule)、`sales_prices/purchase_prices`(ACTIVE→pending)。
危害机制：service 层建单都显式写值，所以默认值平时看不见；一旦有绕过 service 的写入
（导入/直连 SQL/以后新增的建单口），行就落进"界面筛不出、状态机不认"的死状态——
比常量写错更隐蔽，因为它不报错，只是行从此消失。`bpm_task` 那条还实证了另一半后果：
待办列表按小写 `pending` 过滤，大写默认值的任务**永远不会出现在任何人的待办里**。
登记同类未完项：另有 ~15 列"DEFAULT 合理但词表无对应常量"（裸字面量散布在 service 里），
以及 3 处需产品决策才能定回填值（purchase_orders 双状态列大小写矛盾、
color_cards 的 active 是 legacy 还是域内值、production_orders 词表缺项）。

**E2E 侧按取证清单修的根因（R1~R12，详见当轮判责记录）**：nest 前缀造成的假 404 被当成
"权限未拒"、裸 `Vec` 出参被按 `items` 断言、状态词表用错（`draft` 根本不存在于
transfer/count 词表）、报价 POST 缺 6 个必填字段（同一必填集还打死了 globalSetup 的前置数据）、
Decimal `DECIMAL(18,4)` 出参被按 `"200"` 字面量比较、CSRF 一次性消费恢复链的第一跳 403
被当成创建结果、断言方向反了（点"取消"后还在等 popconfirm 可见）等。
**已登记的真产品缺陷（不是测试问题）**：`BusinessError` 出参 message 恒为脱敏文案、
真文案只进 tracing（前端用户永远看不到具体原因）。

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

- [x] **台账状态筛选死控件（iter26 续，`22d4b9ba`+`081b95a6`+`16140087`）**：`GET /inventory/stock` 的
      `ListStockParams` 根本没有状态参数，前端提交的 `status=normal/warning/frozen` 既参数名不对、
      取值也不是后端主数据值（真实值是中文「正常/报废/已删除」），筛选从不生效；表格状态列文案映射
      也按英文键查表，对真实值永远走不到。修法：service 侧查询条件收敛为 `StockListFilter`（列表与
      导出共用，避免再加位置参数触发 `too_many_arguments` 新告警），新增 `stock_status` 精确匹配，
      未指定时排除软删除行（删除是 `stock_status=已删除` 的软删除，已删库存此前混在台账里被当成在库量）；
      状态值收进 `models/status/purchase_inventory::inventory_stock_status` 常量；前端以
      `constants/inventory-stock-status.ts` 为单一真相源，1-6 增加正反向断言。

- [x] **同一订单行被多条入库明细收货时确认失败（`7c820006`）**：`update_order_items_received_quantity` 逐条
      `map.remove(order_item_id)`，面料一张入库单常把同一订单行按缸号/批次拆成多行收货，
      第二条起查不到映射即报「订单明细不存在」——run 35633791612 的 1-4 NOT_FOUND 现形。
      改为按订单明细行汇总（BTreeMap 保序）后一次性更新。此缺陷由 `29b1f30a` 的挂接暴露
      （挂接前所有明细 order_item_id 为空，该分支从不被执行），属挂接修复的必要后续。

- [ ] **发货选行只按（产品 + 仓库），完全不校验订单行的色号/缸号/批次（`so/delivery_ops/inventory.rs:303-314`）**：
      出库流水记录的是「被扣库存行」自己的色号与缸号（`ship.rs:223-240` 取 `stock_color_no`），
      因此下单 RED-001 可以从 TEST-COLOR 的行里扣货，面料出库的维度追溯在发货侧断裂。
      2-7 的按固定色号断言正是被这个缺陷读成「查无数据」，已改为按产品累计出库量断言并逐行打印维度。
      修法是让 `reduce_inventory` 在订单行带色号/缸号时按维度选行、不足时报可出货量为 0 的业务错误，
      但这会连带要求全部 E2E fixture 建库存时使用与订单一致的色号/缸号（02-o2c/11/12/13/44f 等），
      属一次跨端改造，需单独一轮完成，不与本轮混批。

- [ ] **台账「冻结/待检」状态无任何写入口（功能缺失，非缺陷）**：全仓只有 正常/报废/已删除 三处写入，
      既没有冻结/解冻端点，也没有待检流转，因此前端不提供这两个筛选项。
      本轮先把"诱饵"拔掉：`models/inventory_stock.rs` 与 `inventory_stock_handler_dto.rs` 两处字段文档
      原文写的是 `库存状态（正常/冻结/待检）`——接口文档自己承诺了库里从不存在的取值，
      前端筛选栏当初就是照这份文档做出 冻结/待检 两项假选项的；现改为真实取值域并显式标注
      "本列从未有冻结/待检写入方，文档若写即为假文档"。是否补冻结/解冻能力（以及冻结后是否应
      排除在可用量之外）仍属功能范围决策，需用户确认后再做，不在本轮擅自实现。
- [x] **库存状态"字面量未收敛"的真实根因不是字面量，而是常量选错了域（本轮定位并修）**：
      挂账时以为剩下的只是 `inventory_stock_query.rs` / `inv/batch.rs` 的字面量一致性，核对代码发现
      那两处早已改用常量；真正的缺陷在另外三处，且都是"用另一个域的常量替换字面量"这类
      表面合规、实际更坏的写法：
      1. `POST /inventory/stock` 建单写 `stock_status = master_data::ACTIVE`（值为 `active`）、
         `quality_status = "qualified"`——两列的取值域分别是 正常/报废/已删除 与 合格/待检/不合格，
         这批行对所有按「正常 + 合格」过滤的可用量、可出库、缺料预警查询永久不可见（账上有货、界面与出库无货）；
      2. `dashboard_service.rs` 6 处按 `StockStatus.eq(master_data::ACTIVE)` 过滤库存：仪表盘的
         总库存量/低库存数/零库存数/仓库分布/低库存清单恒为 0，而它唯一能看见的恰好是缺陷 1 写坏的那些行
         （两个错误互相抵消，所以 16 轮 CI 无人发现）；
      3. `five_dimension_service.rs:80` 写字面量 `"ACTIVE"`（大写，连缺陷 1 的拼写都不匹配），
         五维统计对任何数据都恒返回空集。
      修法：三处一律改用本列常量（`inventory_stock_status::NORMAL` 等），`inventory_stock_status`
      与 `inventory_stock_quality_status` 各补 `ALL` 作为取值域单一来源；建单初始值抽成
      `initial_stock_statuses()` 以便被测试钉住；新增迁移 `m0057_normalize_stock_status_domain`
      把存量 `active/normal`、`qualified/pass` 归一（放在 production 域 m0056 之后，与既有
      `passed → 合格` 归一同域同位置，执行顺序已按建表域核对）。
      回归用例：`backend/tests/handlers_inventory_stock_status_test.rs`（词表、跨域写法拒绝、
      建单初始值在域内）+ `e2e/flow/01-p2p.spec.ts` 1-6（状态筛选真下推、越界值被 400 拒绝）。
      仍待产品确认：低库存/预警页是否应展示报废库存。

- [x] **库存列表关键词筛选是假控件（后端根本没有该入参）**：筛选栏有「关键词（产品编码/名称）」
      输入框，`getStockList` 也照实提交 `keyword`，但 `ListStockParams` 里没这个字段，axum 直接
      忽略未知查询参数 → 用户改了筛选、点了查询，列表一动不动，且 200 无任何告警。
      现补 `keyword` 入参并在 service 侧先按产品编码/名称取候选产品（LIKE 走 `safe_like_pattern`
      转义通配符）再把 product_id 集合下推；无产品命中时直接返回空集，不靠 `IN ()` 的边界行为。
      列表与导出共用 `stock_list_filter`，因此导出同步生效。
- [x] **库存导出的审计快照与列集与实际口径不符**：`build_stock_xlsx_table` 只输出 仓库ID/产品ID
      两列裸 ID，而 `attach_master_names` 已经按 ID 批量带出的产品编码/名称/仓库名称被丢弃，
      补货点/库存上限/库存状态/质量状态也没进文件（导出的 xlsx 人工不可读）；审计快照
      `after_snapshot` 只记 warehouse_id/product_id 两项，按四维或台账状态导出的记录在审计里
      看不出实际口径。现导出列与列表同口径（含编码/名称/状态/阈值），审计快照补全全部筛选条件。
- [x] **库存打印列里有不存在的字段**：`printJS({properties: [..., 'quantity']})`，后端字段名是
      `quantity_on_hand`，`quantity` 在纸上恒为空白列；表头还是英文字段名。现改为
      `properties` + `displayName`（print-js 的列定义项就叫 `properties`，写成 `columns` 会被
      类型检查与运行时双双忽略），列集与列表/导出一致，状态按主数据取值映射本地化文案。
      台账状态的取值→文案映射收进 `views/inventory/composables/invFmts.ts`，列表/详情/打印共用。
- [x] **质检结论被原样复制进入库单检验状态列（跨域写值，本轮修）**：
      `sync_receipt_inspection_status` 把 `quality_inspection_records.inspection_result` 的中文值
      （待检/合格/不合格）直接 `Set` 到 `purchase_receipt.inspection_status`，而后者取值域是大写码，
      全仓唯一的合法写入方是建单时的 `PENDING`（还是个裸字面量）——于是凡是回写过质检结论的入库单，
      该列都变成"没有任何读取方认识的取值"，按大写码判断的逻辑一律视为"从未检验"。
      修法：`purchase_receipt_inspection` 补齐 `PASSED/REJECTED` 与 `ALL`，并新增
      `from_inspection_result` 显式映射（词表外返回 None）；回写改走该映射，映射不到即报错，
      不再默认成某个值；建单裸字面量改引常量；同一条路径上原先"关联 ID 缺失就静默跳过、
      入库单查不到也什么都不做"两处静默分支改为显式报错（外键已破坏必须让调用方知道）；
      迁移 m0057 追加第三条 UPDATE 归一存量（`purchase_receipt` 在 business 域建表，
      production 域执行顺序晚于它，已核对）。核对过：全仓（含 e2e 与后端测试）没有任何地方
      读取或断言该列，因此不存在"测试为旧行为背书"的问题；也没有任何后端路径自动写
      `related_type=PURCHASE_RECEIPT`，这条同步只由调用方声明触发。
      回归用例：`handlers_quality_inspection_result_test.rs::test_receipt_inspection_status_maps_within_its_own_domain`。

- [ ] **入参取值域校验的"回显允许值"在对外响应里看不见（需安全/产品决策，禁止擅改）**：
      本轮把越界台账状态从"静默零命中"改成 400 并在 `AppError::validation` 里列出允许值，
      但 `utils/error.rs::public_message()` 对**所有**错误类型统一脱敏（漏洞 #4/#8/#12 修复），
      400 的响应体只有 `{code: "VALIDATION_ERROR", message: "请求参数验证失败", trace_id}`，
      允许值清单只进服务端 detail 日志。后果：前端/接口调用方拿到"参数验证失败"仍不知道该填什么，
      只能查文档。是否给参数校验类错误开一条"回显合法值"的口子（它不含内部实现细节，
      与 BusinessError 的脱敏目的不同）属安全策略决策，需用户确认。
      本轮只把断言口径改对：E2E 断稳定错误码 `VALIDATION_ERROR`，"必须列出合法值"由
      `backend/tests/handlers_*_test.rs` 的 `AppError::to_string()` 断言钉住；
      同时把 `e2e/smoke/quality-records-contract.smoke.spec.ts` 里两条
      `expect(body).toContain('合格'/'incoming')` 改掉了——那是上一轮按"错误信息回显允许值"
      写的断言，与脱敏策略冲突，照原样跑必红（本轮离线核对 `public_message()` 才发现）。
- [x] **run 4623（head bb028f30）真实失败的 5 个 job 逐条判责并修**（本轮把 4621/4622 那种"日志没读全就下结论"的毛病改掉：
      每个失败 job 日志与 artifact 内 `reports/playwright-output.txt` + `reports/backend.log` 全部下载并读完）：
      0. `📦 Rust 测试预编译` 与 `🔍 Rust Clippy` 双红是我上一批 `customer_name` 下推引入的
         `E0063 missing field`：`tests/handlers_sales_order_handler_test.rs` 两处
         `SalesOrderQuery { ... }` 初始化器没跟上新字段。这是「本地无编译权」同类问题第 5 次复发，
         自审清单补一条：**给 handler 的 Query/Request 结构体加字段，必须同步 grep tests/ 下该结构体的
         全部字面量初始化点**（本轮同时核了 `StockListFilter`（用 `..Default::default()` 安全）、
         `RecordQuery`、`ListStockParams` 三处，只有 SalesOrderQuery 中招）。
      1. `smoke/material-shortage-contract:116`：`level=critical` 期望被拒却拿到 200。判责为**测试判错**：
         后端 `validate_enum_param` 用 `eq_ignore_ascii_case` 匹配并回传常量本身（仓库既有入参约定），
         `critical` 与规范码 `Critical` 只差大小写，属归一接受；旧词表真正越界的是换了词根的
         high/medium/low。现改为断 high/medium/low/not_a_level/pending/notified 一律 4xx，
         并新增"归一必须与规范码返回同一结果集且行内 level 为 Critical"的断言（防止接受小写后出现第二套口径）。
      2. `smoke/material-shortage-contract:145`：状态下拉读出 4 个级别文案。逐层核对
         `MaterialShortageTable.vue` 两个 select 的选项源（`SHORTAGE_LEVEL_VALUES` 与
         `SHORTAGE_ALERT_STATUS_VALUES`）确认**页面是对的**，错在测试：Escape 后未等 teleport 到 body 的
         浮层真的收起就点第二个 select，点击落在未收起的浮层上，读到的还是级别面板。现加
         `toHaveCount(0)` 收起确认与"第二个面板必须是 5 项"的前置断言，使同类误判不可能静默复现。
      3. `flow/05-system:5-6`：断 `data.items` 为数组失败。核对 `query_user_tasks` 返回
         `PageResponse{total,page,page_size,total_pages,data}`，且前端 `api/bpm.ts` 早就按
         `{data,total}` 消费——**后端与前端一致，测试假设的字段名才是错的**。现按真实契约断言，
         并把状态取值域收窄到 `bpm_task` 模块的四个值（原用例还放行了库里不存在的 processing）。
         同因发现 `flow/10d` 的 A1-3 与 A1-5 是 `expect(expr)` 无匹配器的真空断言（所以"一直绿"），
         一并补成真实断言——这正是 doto 里"真空断言成片"那条的又一例证。
      4. `flow/08-business-modes:M1-6`：`GET /production/business-modes/rules` 返回 405。
         核对 `routes/production.rs`：该路径只注册了 `post(create_rule)`，规则的全局列表端点
         **不存在**（只有 `GET /rules/by-mode/{mode_id}`），`flow-steps` 同形。改用真实入口
         （先列模式取 outsourcing 的 id，再按模式取规则并断 mode_id 归属、rule_code 非空、
         rule_type 在 required/optional/forbidden 内）。登记待决：是否需要业务模式规则的
         全局分页列表端点（该域前端零调用方，唯一消费者是 E2E），需产品确认后再补，不擅自建端点。
      5. `flow/01-p2p:1-8 付款`：400「应付单 API…未付金额为 0，申请金额 56500 超过未付金额」
         （对外文案脱敏，真因在 backend.log 的 detail 里）。核对 `ap_invoice_ops/receipt.rs`
         确认后端两处生成逻辑都对（采购收货单 unpaid=amount，退货红字单 unpaid=0 且有意为之），
         根因在测试：1-7 查 `/ap/invoices` **不带 supplier_id**，取 `invoiceList[0]` 即库里任意一张
         （多半是已付清或红字单），再写死 56500 申请付款。现改为按本用例供应商查询、优先挑
         `unpaid_amount > 0` 的那张、并断其 supplier_id 归属；1-8 的申请金额取该单真实未付金额
         （去掉 56500 硬编码），两处 `test.skip()` 前置兜底改为显式断言。
- [x] **验布管理页建单/编辑提交的是后端不存在的字段（run 4623 的 54 验布用例暴露）**：
      `views/fabric-inspections/index.vue` 的建单与编辑表单提交 `fabric_batch_no` 与
      `total_length_m`——`CreateInspectionRequest` 与 `fabric_inspection_record` 都没有这两列，
      而该 DTO 唯一的必填项 `inspection_date`（NaiveDate）表单里根本没有，于是每次新建都被
      `422 missing field inspection_date` 拒掉（首轮还叠加一次 CSRF 403，重试后才是真正的拒因）。
      表象是 E2E「等不到 .el-message--success」，实质是页面进了 ErrorBoundary
      （快照显示"页面加载出错"且顶栏仍是"未登录"）。
      修法：表单字段重建为后端真实契约（验布日期必填、按当天预填；缸号/色号/验布员/机台号/
      评分制式/门幅英寸/备注），编辑态把 `UpdateInspectionRequest` 不接受的三项
      （日期/缸号/色号）置灰——不能让用户填一个改了也不会生效的输入框；详情面板去掉两个死字段，
      改列出色号/验布日期/机台/评分制式/门幅/总扣分/每百平方码分数；api 层的
      `Record<string, unknown>` 换成 `CreateFabricInspectionPayload` / `UpdateFabricInspectionPayload`
      两个与后端字段一一对应的类型，字段写错从此由类型检查拦住；评分制式取值收进
      `constants/fabric-scoring.ts`（four_point/ten_point，与后端 `fabric_scoring` 同源），
      提交码值而不是文案。
- [x] **run 4623 其余三条 E2E 失败的判责与修复**：
      ① 44f-7 大货处方审核报「业务处理失败」，backend.log detail 是「审核前处方明细不能为空」——
      后端约束正确（`CreateProductionRecipeRequest.recipe_detail` 就是业务必填的处方明细），
      用例建了张空处方去审核，现补一条染料明细；② 53-2 断"无敏感角色"，实际是 `GET /roles`
      出参为 `RoleListResponse{roles,total}`（全量返回、无分页），用例按 `items` 取值恒 undefined，
      现按真实契约断言并去掉误导性的 page/page_size 参数；③ 48-3 断"AR 列表应可达"拿到 403，
      根因是路径写错——真实路由是 `/ar/invoices`（`finance.rs:847`），不存在的端点被权限层
      先拒成 403；同文件 48-2 早已为 AP 踩过并写下注释，AR 这条却仍在用 `/ar-invoices`。
- [x] **库存预警页整页不可用：`/inventory/stock/alerts` 出参是 `{list,total}` 且行内没有产品/仓库名称，
      前端却按 `StockAlert[]` 直接赋给数组并按不存在的字段渲染**（本轮定位，未在本批动，
      需与列表口径一起单独成轮，避免与已提交的库存改动混在一次推送里无法判责）：
      后端 `inventory_stock_query.rs:320-381` 返回 `json!({"list":..., "total":...})`，
      行内字段是 `id/product_id/warehouse_id/quantity_on_hand/quantity_available/quantity_reserved/
      reorder_point/max_stock_point/expiry_date/last_movement_date/stock_status/alert_type`；
      而 `api/inventory.ts` 声明的是 `StockAlert{id,product_name,product_code,warehouse_name,
      current_quantity,min_quantity,unit,alert_level:'warning'|'danger'}`——
      产品名/产品编码/仓库名/当前量/最低量/单位/告警级别七项在后端行内都不存在，
      `current_quantity` 实为 `quantity_on_hand`、`min_quantity` 实为 `reorder_point`、
      `alert_level` 实为 `alert_type`（取值是 normal/low_stock/out_of_stock/over_stock/
      slow_moving/expiring/discrepancy 七种，不是 warning|danger）。
      更直接的是 `views/inventory/index.vue` 的 `alerts.value = res.data || []`：`data` 是对象不是数组，
      于是预警 tab 表格恒空（与此前修掉的库位对话框同一型错误）。
      修法应是：后端把该端点从裸 `serde_json::Value` 改为typed 结构并回传主数据名称
      （`attach_master_names` 已有同类实现，且该查询已 inner_join products/warehouses），
      前端 `fetchAlerts` 按分页包装取数、`StockAlert` 与 `alert_type` 词表收进 constants、
      预警 tab 列名改真实字段；同时该端点的 page/page_size 目前被完全忽略
      （`10a-extended-inventory-approval.spec.ts:152` 传了 `page=1&page_size=5` 却拿全量），
      本轮修完：后端把该端点从裸 `serde_json::Value` 改为 `StockAlertRow` + `StockAlertQuery`，
      分页真实生效（page/page_size 不再被忽略），产品编码/名称/单位/仓库名称按 ID 批量带出
      （主数据缺失记 error 并留空，不用 ID 拼假名称）；出参统一为 `PaginatedResponse{items,total,
      page,page_size}`，错误不再一律压成 internal。前端 `StockAlert` 改为与后端逐字段一致
      （数量类是 Decimal 字符串序列化）、预警告警级别词表收进 `constants/stock-alert-type.ts`
      （normal/low_stock/out_of_stock/over_stock/slow_moving/expiring/discrepancy，
      取代界面自造的 `alert_level: warning|danger`），tab 列名改真实字段；
      `views/inventory/index.vue` 与 `store/inventory.ts` 两处 `alerts.value = res.data || []`
      的"把分页对象当数组"错误都改为按 `items` 取数，形状不符直接抛错而不是静默变空，
      取满一屏仍有剩余时 logger.warn 说明截断条数。`StockAlert↔StockAlertRow` 已纳入
      `check-contract.mjs`（现 11 组映射 / 146 字段 / 0 挂账），`10a` 的 L1-10 也从
      "真空断言 + 失败时改查 /material-shortage 顶包"改成真实契约断言。
- [x] **验布建单/编辑入参纳入契约门禁**：`scripts/check-contract.mjs` 的 TS→Rust 对照表补
      `CreateFabricInspectionPayload → CreateInspectionRequest` 与
      `UpdateFabricInspectionPayload → UpdateInspectionRequest` 两组（现 7 组映射 / 63 字段 / 0 挂账），
      同类漂移从此在提交前静态拦截，不必再等 E2E 撞出 422。
      登记一条限制：`api/inventory.ts` 的 `InventoryQueryParams` 被多个端点共用
      （/inventory/stock、/inventory/batches、低库存与预警各自的 DTO 字段集都不同），
      直接映射到 `ListStockParams` 会把 `low_stock` 这类"别的端点才有的字段"误判成幽灵字段；
      要纳入门禁需先按端点拆分入参类型，属另一轮工作。
- [ ] **库存详情弹窗字段不全（本轮只统一了状态取值口径）**：详情行只列 编码/名称/仓库/批次/色号/
      缸号/米数/公斤/状态/库位，后端出参里已有的 等级、质量状态、可用量、预留量、补货点、库存上限
      一项都不显示——降级后的等级与质检结论在界面上看不到，只剩列表与导出可见。本轮把详情里的
      `stock_status` 改走与列表/打印同一份取值→文案映射（英文界面不再直出中文码），补齐上述字段
      需新增文案键并决定展示顺序，登记待做，不在本轮顺手加半套。
- [ ] **库存页「采购」按钮跳转带的 `product_name` 无人消费（死参数）**：
      `views/inventory/index.vue:580` `router.push({name:'Purchase', query:{product_name}})`，
      而 `views/purchase/` 全目录没有一处读 `route.query`（grep 无命中），参数到即丢。
      要真实生效需给采购订单列表补"按产品筛选"（采购主表按订单行组织，需 join `purchase_order_items`
      或在明细侧过滤），属跨端改造，需单独一轮完成，不与本轮混批。

- [x] **调拨单状态筛选假控件**：后端 `inventory_transfer` 只有 pending/approved/rejected/shipped/
      completed，前端筛选项却是 pending/approved/executed/cancelled——executed、cancelled 库里从不
      写入，选中即零命中，同时三个真实状态筛不出来；表单类型联合与「可编辑判定」还用了不存在的
      received。已按后端取值统一（含中英文案补齐与未知值告警）（`a5c3a2d7`）。

- [x] **物流运单整页假功能与状态机旁路（本轮定位到根因并修）**：核对后端 `logistics_waybill`
      状态机（大写 IN_TRANSIT/DELIVERED/SIGNED，`models/status/bpm_crm_contract.rs`）后确认，
      物流页此前有六处与后端不符，且相互放大成「整页不可用」：
      1. `GET /inventory/logistics` 完全不读查询参数、不分页、返回裸数组，而前端按
         `{items,total}` 消费——筛选栏与分页控件都是假的；
      2. `PUT /inventory/logistics/:id` 只接受 `{status}`，前端「编辑运单」提交的却是运单字段
         （无 status），反序列化即失败；「发货」按钮写入的 `shipped` 后端从不认；
      3. `update_waybill_status` 对状态值不做任何校验，任何字符串都能落库——脏值一旦写入，
         电子签收（要求 DELIVERED）与「在途/已送达不可删」判定同时失效；
      4. 前端状态词表（pending/shipped/in_transit/delivered/cancelled）与 i18n 键（camelCase）
         双重的不匹配，使状态标签恒显示原始值、统计卡恒 0、操作按钮按不存在状态渲染而全部隐藏；
      5. 表内 `waybill_no`/`order_no` 两列后端从不返回，恒为空白；
      6. 建单表单「预计到达」以 `YYYY-MM-DD` 提交给 `DateTime<Utc>` 字段，选了就 400；
      「关联订单」下拉是硬编码的两条假数据。
      修法：后端补真实筛选（状态/物流公司/关键字/创建时间区间）+ 分页 + order_no 回查，状态入参
      按状态机校验且 SIGNED 只能经签收接口写入（否则绕过应收确认），字段编辑限定在运输中阶段，
      删除规则改为「已送达/已签收不可删」（原规则禁止删除唯一可撤销的在途误建单，却放行删除
      已签收的财务凭证，方向反了）；前端状态值收进 `constants/waybill-status.ts` 单一真相源，
      补电子签收接线（后端 P0-B13 端点此前前端零调用）、日期改日期粒度、订单下拉改真实查询。
      验证用例：`e2e/smoke/logistics-contract.smoke.spec.ts`。

- [ ] **运单号 `waybill_no` 需 schema 迁移才能真实化**：`logistics_waybills` 表根本没有该列
      （m0011 建表 SQL 可证），前端却长期显示空白的「运单号」列/详情项。本轮先撤掉假显示
      （列表列与详情项改列真实字段），要补真实单据号需新增迁移 + 建单时按编号规则生成 +
      历史行回填，属独立立项。

- [x] **物流公司取值跟随界面语言**：已按「库里在用的中文公司名即稳定值」收敛，详见下文
      「译文当业务值剩余 13 处」一条（含 v15 存量归一与筛选等值匹配的后果）。
      当时判断「要修需先决定是否建物流公司字典」——字典仍未建，但不建字典也能先把
      提交值固定下来，界面语言不再改写业务数据。
- [ ] **模板/列定义里直接写中文字面量（i18n 门禁的盲区，规模已量化）**：
      `src/views` 下 55 个视图文件在 `label=` / `title=` / `placeholder=` / `content=` 属性上
      直接写中文，另有 `views/sales/composables/useOlv.ts` 的表格列 `title` 也是中文字面量
      （销售订单列表的表头因此不随语言切换）。`check-i18n.mjs` 只校验「键是否被引用、引用是否缺失」，
      不看有没有人绕过 i18n 写字面量，所以这些永不显红。本轮物流运单详情的 12 处已改走文案键；
      余下部分要一次成体系做（否则改一个文件没意义），并且**不能直接把门禁加上**——
      门禁上线当日即 55 文件全红，需要按 Clippy baseline 那套「存量挂账 + 增量阻塞」的机制先行。

- [ ] **`playwright.config.ts` 的 testMatch 未覆盖的业务目录 = 死用例集**：testMatch 白名单里
      没有 `logistics/inventory/mrp/production/ai/dashboard/fabric/quotations/sales-ext/system`
      等目录，落在其中的 spec 永不执行（且 CI 分片命令是按目录显式传参，改 testMatch 也不生效）。
      本轮已把 `e2e/logistics/` 下两个「整文件 if(isVisible) 空转 + 假词表」的 spec 删掉并按真实
      契约重写为 smoke 用例；其余目录需逐个甄别「仍有价值→迁 flow/smoke / 已过时→删」，
      逐个判责需要一轮专门排查，禁止一次性删目录。
      测量陷阱（本轮实证）：Windows 上 `playwright test --list` 不带路径参数时会把这些目录
      也收进来（`^[^/]*\.spec\.ts$` 分支因反斜杠路径误匹配，本地多计 80 个用例），
      所以「本地 --list 数字」与「CI 实跑数字」不等价，README 已改为按 CI 口径分目录实测。


- [x] **库存台账状态字面量全仓收敛**：`models/status::inventory_stock_status` 落地后仍有 8 处
      代码各自写 `"正常"/"报废"`（`inv/batch.rs`、`inventory_stock_query.rs` 判定与过滤、
      `inventory_stock_service.rs` 四处建/改、`inventory_stock_txn.rs` 流水建行、
      `material_shortage_service.rs` 缺料可用量过滤），已全部改引常量，
      `inventory_stock_service.rs` 里原先的函数内 `use` 与三处全限定路径也一并收敛为模块级导入。

- [x] **`inventory_stocks.quality_status` 口径已裁决并归一**：确认该列合法值为
      合格/待检/不合格（中文），四处筛选一律按「合格」，而验布放行与批色放行两处
      写入的是另一域的 `passed`，导致已放行的库存在可用量、缺料预警、可出库筛选中
      永久不可见（账上有货、界面缺货，且无日志）。已建
      `inventory_stock_quality_status` 常量、11 处写入/筛选全部改引常量，
      并以迁移 m0056 幂等归一存量 passed 行。委外回仓单的 quality_status 是
      另一张表的独立取值域（前端 qualified / E2E passed / 后端确认时映射合格），
      未随本项改动，需单独核对那张表的口径。

- [x] **生产订单状态词表与按钮全假**：后端写入大写下划线值（DRAFT/PENDING_APPROVAL/APPROVED/
      REJECTED/SCHEDULED/IN_PROGRESS/COMPLETED/CANCELLED，PUT /status 白名单同为大写），
      前端字典却是 draft/planned/in_production 一套小写名——筛选项 5 个值全都命中不了任何行
      （planned 后端根本不存在，待审批/已审批/已驳回无处可选），标签配色查不到字典，
      操作按钮同时留着新加的大写分支与永不生效的小写分支，
      结果是草稿单没有编辑/删除、已审批没有排产、已排产没有开工、生产中没有完工，
      生产计划主流程在界面上无法推进；建单还把 status:'draft' 当字段提交（后端固定写 DRAFT）。
      已按后端取值重建字典并派生筛选列表、文本与配色统一出口且未知值告警、
      按钮按状态机重排（目标值取后端白名单）、去掉自造的 status 提交与旧文案键（`0e67faf8`）。

- [x] **物料缺料页整套取值是编造的**：后端缺料状态只写在 `material_shortage_alerts`
      （identified→purchase_request→purchase_order→received→resolved，`update_status` 已按这五个值校验），
      级别是 `ShortageLevel` 的 Critical/Severe/Warning/Normal；前端却自造 severity
      critical/high/medium/low + status pending/notified/resolved，列表接口返回的是实时检测项
      （无 status、无单号），于是状态列恒空、`row.status === 'pending'` 的行内按钮永不出现、
      「标记解决」把不存在的 `row.id` 拼成 `/material-shortage/undefined/status`，
      统计卡读的 total_shortage_count/high_count/last_check_time 三个字段后端从不返回。
      已把值收进 `constants/shortage.ts` 单一真相源（未知值告警）、列表改为
      `ShortageAlertView`（实时缺料 + 未解决预警的 alert_no/status/identified_at）、
      level 与 status 入参先校验取值域再过滤、触发检查补请求体、月报与汇总计数改读真实字段，
      并新增 `e2e/smoke/material-shortage-contract.smoke.spec.ts` 钉住取值（`2b5a27c6`+`95674ecb`）。
      同轮修掉两处：列表分页页码二阶换算（翻第二页回到第一页）、
      `from_deficit_rate` 写死 100/50 导致 `/threshold` 保存的阈值永不参与定级（`1399de43`）。

- [ ] **缺料预警不自动解除**：`persist_alerts` 只在检测到缺料时插入/刷新未解决预警，
      库存补足后该行的 status 仍是 identified，只有人工置 resolved 才关闭。列表按实时集合
      过滤所以看不到陈旧行，但 `update_status`（按 material_id 找未解决预警）会命中它们，
      月报 `get_monthly_report` 也是按落库行统计，长期偏高。修法要定口径：
      是 detect 之后对不再缺料的物料自动置 resolved（推荐，与「缺料」语义一致），
      还是保留人工解除并把月报改成按实时快照统计——属业务决策，不顺手改。

- [ ] **缺料预警的采购关联两列恒空**：`material_shortage_alerts.purchase_request_id /
      purchase_order_id` 只在建单时写 None，状态推进到 purchase_request/purchase_order
      不回填真实单据 ID，所谓「识别→采购申请→采购订单→入库→解除」闭环目前只是状态字符串。
      需要先定方向：由采购申请/订单侧在创建时反查未解决预警回写，还是状态推进接口要求传单据 ID
      并校验归属；两者都涉及跨域调用与权限，不能挑简单的一半凑数。

- [ ] **`safety_factor` 阈值项无数据源**：`ShortageThresholdConfig.safety_factor` 的语义是
      「低于安全库存 × 倍率即预警」，但 `products` 表没有 safety_stock 列（只有
      chemical_master / greige_fabric 有），缺料检测只用「需求 > 可用」判定。
      要真正接入需先加列并定义维护入口（物料主数据页），属 schema 变更 + 产品口径，单独立项。
      另外 `/material-shortage/threshold` 目前无前端页面，配置只能经接口保存。

- [x] **AI 质量预测的字段词表核对**：该资源的可筛选状态本来就对齐（risk_level 落库
      low/medium/high、is_acknowledged 布尔，四个筛选参数都真实下推 SQL，migration m0044 还有
      CHECK 约束钉住取值），但查出并修掉三处别的断层（`f03fbd23`、`4d90bd9e`）：
      POST/批量创建把 AI 侧的中文标签（高/中/低、上升/平稳/下降/无数据）原样回传，而列表与详情
      返回库中的英文小写值，同一字段两套词表，前端按同一字典取名导致创建成功提示里风险等级恒为
      undefined；风险映射还写着 `_ => "low"`，未知标签会被静默标成无风险；source 字段落库有
      history/fallback/degraded 三种值、响应里另有 degraded 布尔，但前端既不展示来源也没有
      degraded 标签，服务降级与正常预测在界面上无从分辨。现已统一为库内词表、越界标签直接报错、
      列表补数据来源列（降级标红）。状态取值核对方法到此已在调拨/物流/生产订单/缺料/质量预测五处用完。

- [ ] **AI 质量预测的三处功能缺口**：其一 `ai_quality_predictions.model_version_id` 全仓无任何
      写入点（建单以 `..Default::default()` 收尾），预测记录无法追溯到具体模型版本，而
      `ai_model_versions` 表已存在——需先确定推理时该取哪个版本（当前生效版本还是请求指定版本）。
      其二 `/ai/quality-predictions/{id}/actual-result` 与 `/actual-grade` 两个端点前端零调用，
      `actual_risk_level / actual_grade` 直接把用户提交的字符串落库且不做取值校验，
      「预测 vs 实际」的准确率闭环在界面上走不通；其三该页两个操作按钮用的是
      `ai_quality_prediction:approve` / `ai_quality_prediction:delete`，而注册表里的资源名是
      `ai-quality-pred`，动作也没有 approve 一档——权限码不匹配时前端 fail-closed 隐藏按钮，
      与已登记的 M-6 resource_id / 角色权限矩阵项属同一决策面，需一并定口径后再改，不单点放行。

- [x] **委外收回单质检结论四套写法混用，`passed` 的收回单在确认时被判成不合格**：
      `outsourcing_receipt.quality_status` 无 CHECK 也无 DEFAULT（v15/mod.rs:707），历史上同时收过
      四种写法——收回单界面提交 `qualified/concession/unqualified`、E2E 用例提交 `passed`、
      模型注释写的是 `pending/passed/failed`（染化料来料检验域），而 confirm 只把字面量
      `qualified` 视为接收（`receipt.rs` 原 440-452 行），其余一律落到「不合格」分支：
      于是 `passed` 的收回单确认时生成一条「不合格」质检记录、合格数量记 0，
      与同一单声明的 A 级自相矛盾，且只有一条 warn；结论为空(NULL)的行更被
      `unwrap_or_else(|| "qualified")` 直接默认成合格，等于伪造质检结论。
      已建 `outsourcing_receipt_quality_status`（pending/qualified/concession/unqualified）常量、
      建单与改单入口按取值域校验（别域同义写法一律 400 并报合法值，不做大小写或跨域宽容）、
      确认改为按四值显式判定（让步接收计入接收、待检与取值域外拒绝确认、NULL 要求补录），
      并以v15 域内归一语句处理存量（passed→qualified、failed→unqualified、中文→对应小写、NULL→pending）；
      前端取值收进 `constants/outsourcing-quality.ts`（列表原来直接把英文码展示给用户），
      另补 `tests/handlers_outsourcing_receipt_test.rs` 钉住取值域。
      遗留：本列仍无质检结论筛选（前后端都没有），`print_service.rs:2340` 的收回单打印数据仍输出
      原始英文码（未接中文标签），且 `grade` 缺省仍写死 "B"、
      让步接收是否强制 B 级未定，见下一条与库存项。

- [x] **`GET /inventory/batches` 七个筛选字段一个都不用**：`inventory_batch_handler.rs:25` 的
      BatchListQuery 声明 product_id/batch_no/color_no/grade/warehouse_id/start_date/end_date，
      `:94` 却只把 page/page_size 传给 service，筛选栏整体是假的；前端
      `BatchListTab.vue` 还按 camelCase 发 `batchNo/colorNo`，名称也与后端不符。
      已加 `BatchListFilter` 下推 SQL（批次号/色号模糊、等级/产品/仓库精确、创建时间区间），
      分页改用 `paginate_with_total` 并像台账一样排除软删除行；顺带修掉等级取值把译文当业务值的问题
      （el-option 的 label 与 value 同为译文，英文界面会把 "First Grade" 发去筛选甚至写进库，
      且界面把库里不存在的「三等品」当选项、没有「等外品」，列表标签也靠比对译文上色），
      等级收进 `constants/stock-grade.ts`（`dbba88ec`）。
      遗留：`bulk_color_approval_service` 的降级规则仍以中文等级字面量写死（一等品→二等品→等外品），
      未随本轮改引后端常量；`inventory_batch` 页其余 tab 的等级下拉同样未收敛。

- [x] **27 处 `:value="t('...')"` 把界面译文当业务值提交**（同一类缺陷的批量清单）：
      本轮先按台账给的判据逐处回查后端真实落库值，再收敛，剩 13 处见下一条。已收敛的三组：
      ① `AdvancedQualityPanel.vue`（检验类型）——质量预测按 `quality_inspection_records.inspection_type`
      做等值筛选（`services/ai/quality_pred.rs:450`），而该列的写入端 `views/quality/index.vue:214-229`
      用的是 incoming/process/finished/outgoing 四个码；界面此前提交「进货检验」等译名，
      任何选择都筛不到数据。收进 `constants/quality-inspection-type.ts`。
      注意另一套码：`ai_quality_predictions.inspection_type` 的 CHECK 允许
      all/incoming/inprocess/final/outgoing（`production/m0044`），两张表的词表不可互抄。
      ② `AdvancedRecipePanel.vue`（布类）——`dye_recipe.fabric_type` 是自由文本、库里存中文布类名，
      后端配伍表按 `cotton/棉/棉布`、`涤纶`、`丝绸/真丝`、`羊毛` 比对（`services/ai/recipe_opt.rs:141-150`），
      故 value 取中文稳定名本身（`constants/recipe-fabric-type.ts`），label 才走 i18n；
      「化纤」不在任何配伍列表内（选中即 422 或不参与配伍判定），已从选项中去掉并登记见下条。
      ③ `ai-extend/process-optimization.vue`（染料类型）——入口白名单同时收英文码与中文别名
      （`handlers/ai_extend_handler.rs:87-108`，reactive/活性 … cationic/阳离子、sulfur/硫化），
      界面提交 "Reactive Dye"/"活性染料" 一律 422。收进 `constants/dye-type.ts`（英文码），
      并补上白名单里有、界面上没有的阳离子/硫化两个选项（新增 `aiExtend.process.dyeCationic|dyeSulfur` 文案）。
      同时补 `tests/unit/translated-value-select.test.ts` 作门禁：
      一是扫描 `src/**/*.vue` 的 `:value="t('…')"` 残留并与挂账清单逐文件逐数比对（修完不减清单也失败），
      二是校验常量表里的文案键在 zh-CN/en-US 中真实存在——`scripts/check-i18n.mjs` 只识别
      `t('字面量')` 调用，键名以常量字段存放时它看不见，这条缺口由该用例补上。

- [x] **译文当业务值剩余 13 处**：已按「自由文本列以库里在用的中文名当稳定值」收敛，判据与
      续四那三组一致，只多了一次存量归一。物流公司（`logistics_waybills.logistics_company` 是
      VARCHAR 自由文本、无字典表，后端筛选按该列 `eq()` 精确匹配）与报价行单位
      （`sales_quotation_items.unit` VARCHAR(20) 同样无字典）此前把译文当 value，英文界面写入
      "SF Express"/"Meter" 后，中文界面按「顺丰速运」「米」就筛不到那些单，反之亦然，同一实体在
      库里裂成两套值。现两处各建常量（value 为中文稳定名、label 走既有 i18n 键），报价单位另把
      此前硬编码成 "kg" 的第四个选项并进来（同一列不再同时存在三种语言写法），v15 末尾对两列做
      一次归一 UPDATE（两表分别由 business 与 sales_crm 域创建，都早于 v15，落点符合上轮教训）；
      `tests/unit/translated-value-select.test.ts` 的挂账清单已清空，即全仓 `:value="t('…')"`
      形态归零，此后新增任何一处都会让该用例失败。
      仍未解决的是数据模型层面：物流公司要支持任意承运商需正式字典表与编码；报价行单位本应跟随
      所选产品的单位，且实务还要用 码/条/吨 等，编辑器只给四项属功能限制。两者都是新增能力而非
      把错值改对，已登记不随本轮改动。
- [x] **列表端点收参数却不 filtering（假控件）批量核对**：已核实并修尽的四处——
      ① `GET /inventory/batches` 七个筛选字段全不下推、前端还按 camelCase 发 `batchNo/colorNo`
      （补 `BatchListFilter` 真实下推并排除软删除行，等级同时收进 `constants/stock-grade.ts`）；
      ② `GET /purchase/orders` 没有 keyword 字段而界面有关键字框（补 keyword 下推为订单号/供应商名
      LIKE，出参改 `PaginatedResponse` 回传真实 total，`c4c732dc`）；
      ③ `GET /warehouses`：后端的 `search` 本身是生效的（名称+编码模糊），但列表页发的是 `keyword`，
      而类型下拉发的 `warehouse_type` 后端 DTO 根本没有 —— 关键字与类型两个筛选端到端都不生效；
      导出路径虽映射了 search 却漏了类型，导出与列表不同口径。现后端补 `warehouse_type` 精确筛选
      （该列存 greige/raw/finished/semi/return 码，与前端下拉同源），前端按后端契约发 search，
      导出补同一类型条件，导出审计快照一并记录类型筛选（否则事后无法解释导出行数）；
      ④ `PUT /sales/orders/{id}` 把客户端传入的任意 status 字符串直接写库（`so/order_crud.rs:626`
      原先只判「已发货/已完成不许改」而不判取值域），一条脏值即可让状态机、列表筛选与按状态统计
      同时失真；现按 `sales_order::ALL` 白名单拒绝并报出允许值。
      注：本轮只收紧取值域，不代表允许任意跳转——状态流转仍应走工作流端点，状态机收敛另计。

- [x] **`/sales/orders` 的 `customer_name` 是死筛选**：列表页 `useOlv.ts:188` 一直在发这个参数，
      而后端 `SalesOrderQuery` 根本没有该字段（serde 静默丢弃），客户名搜索恒返回全量。
      现补进查询结构并下推——查询条件收敛为 `SalesOrderFilter`（列表与导出共用，避免再加一个
      位置参数就把函数推到 clippy 的 too_many_arguments，这条教训来自库存台账那一处），
      名称走已左连接的 customers 表做转义后的 LIKE 模糊匹配，导出端点同口径。

- [ ] **假控件余下实例（各自都需要配套改造，不是删一行字段就能收）**：
      `GET /budgets`（`budget_management_handler.rs:478` 收裸 JSON 且只读 item_type/status，前端
      `BudgetListTab.vue:273-275` 发 budget_no/name；更根本的是该页展示 plans 而后端查 items，
      口径要先定）；
      `GET /warehouses/locations` 的 `search` 已按「不假装功能」删除（库位列表只在仓库详情弹窗内
      按 warehouse_id 展示，没有任何调用方发 search；要支持库位搜索得连弹窗的搜索框与分页控件
      一起加，该端点出参本就是分页对象）。顺带修掉同一弹窗的一处取数错位：后端返回
      `{items,total,page,page_size}`，前端却按 `WarehouseLocation[]` 直接赋给表格，
      库位对话框此前恒为空；现按分页结构取数，并在弹窗（无分页控件）取满一页上限 100 条而
      仍有剩余时显式提示被截断，不静默少显示；
      `wage-records/{id}/details` 的 flow_card_id 前端零调用（`api/wage.ts:95` 不带参数）；
      ~~`production/quality-inspection/records` 一处三病~~（已修：记录列表改用独立的
      `RecordListParams`，`inspection_type`/`inspection_result`/`product_id`/`batch_no` 四项都真正
      下推，两个枚举入参越界一律拒绝并列出允许值；参数名由 `batch_number` 回到 `batch_no`，
      `status: None` 的误用随独立结构消失；导出端点共用同一构造函数，前端补出对应的筛选栏与
      `e2e/smoke/quality-records-contract.smoke.spec.ts` 契约用例）；
      `supplier-evaluations/ratings` 是端点级错位——复用 `EvaluationRecordQuery`（承诺
      supplier_id/period）却返回 `supplier_evaluation_indicator`（指标定义表，无这两列），
      前端与 E2E 均不调用，属孤儿端点，改为返回真实评级或撤掉，是 API 设计决策；
      api-gateway keys 的 method 与未知 status 静默忽略；色卡分析/AR 报表/资金/BPM 等报表端点
      同样不收日期与人员筛选。

- [x] **质检记录页与后端请求契约整体错位（页面级改造，勿零碎改）**：
      核对结论：`POST/PUT /production/quality-inspection/records` 的
      `CreateInspectionRecordRequest` 要求 `inspection_no / inspection_type / product_id /
      inspection_date / total_qty / inspected_qty / inspection_result` 全部必填（非 Option），
      而 `views/quality/index.vue` 的表单字段是 `record_no / result / inspector`（人名文本）加一个
      后端根本没有的 `product_name`，也完全没有两个必填数量项 —— 界面新建一条质检记录必然 400，
      更新同理；列表侧又读 `row.result`（后端出参字段是 `inspection_result`），
      `product_name / inspector / record_no` 三列在模型里同样不存在，7 列里有 4 列恒空。
      已修：表单模型与字段名改为与后端契约一致（产品/检验人改成按主数据选择的 product_id /
      inspector_id，补 total_qty / inspected_qty 必填数字项，结论文案走
      `constants/quality-inspection-record.ts`），提交前做表单校验、不再发必然被拒的请求；
      列表列名改为真实字段，产品与检验人经 `useQualityLookups` 按主数据翻名称（查不到则告警并
      显示 ID，不猜名称），打印行与列表同口径；`api/quality.ts` 的 `QualityRecord` 类型改为
      与后端 Model 一一对应（含 Decimal 序列化为字符串）；后端新增
      `quality_inspection_result`（待检/合格/不合格）常量与入口校验，委外回仓自动写入方改引常量，
      v15 迁移末尾把历史 pass/fail/pending 归一为中文结论（同前一轮教训：归一 SQL 必须落在
      建表域之内或之后），并补 `handlers_quality_inspection_result_test.rs` 钉住词表与越界拒绝。
      仍待处理：`inspection_no` 目前由客户端提交（应由后端用单据号生成器产生，界面上把单号
      做成必填输入并不合理）；列表未展示送检数/合格数/合格率与等级列，supplier_id/customer_id
      与缸号色号等字段界面仍不收集（后端为 Option，不影响主流程）；
      `sync_receipt_inspection_status` 会把该结论复制到 `purchase_receipt.inspection_status`，
      入库单侧的取值口径需与库存质量状态域一并核对（属另一条已登记项）。

- [x] **委外打印数据仍输出英文码**：`print_service.rs:2340` 把 `quality_status` 原值
      （pending/qualified/concession/unqualified）直接打进打印件，用户看到的是码不是文案；
      打印由服务端渲染成中文文档，转文案应落在 print_service（与同文件既有中文文档类型名一致），
      不能只依赖前端常量。已实现：文案映射放进 `outsourcing_receipt_quality_status::label`
      （与入参校验同一模块，避免两处字典），词表外存量值原样带出以便发现脏数据。
      同批改掉销售侧两处重复字典：`views/sales/composables/olvFmts.ts` 另存了一份只覆盖 5 个状态
      的中文硬编码映射（draft/partial_shipped/rejected 在列表里露出英文枚举，且英文界面下文案仍
      是中文），现统一走 `utils/sales-status`（与后端 `sales_order` 常量一一对应）并按当前语言取
      文案，配色返回类型收窄为 `SalesTagType` 以免退化成 string；`api/sales.ts` 的
      `SalesDelivery.status` 声明了该表根本不存在的 draft/delivered，已改为 pending/shipped/cancelled。

- [x] **物流轨迹事件的两处欠账**：`event_type` 此前是自由字符串、后端不判取值域
      （`logistics_service.rs` 直接 Set），界面四个选项（pickup/in_transit/arrived/delivered）
      虽核实为稳定码，但绕过界面即可写任意词，轨迹展示与按事件推进的判断都会失真——现已建
      `logistics_event_type` 常量并在写入入口拒绝越界值（报出允许值），前端建
      `constants/logistics-event-type.ts` 供下拉与列表共用，并以用例钉住「事件类型（小写码）与
      运单主状态（大写码）两域不重叠」，防止把 IN_TRANSIT/DELIVERED 当成事件写进轨迹。
      `LogisticsDetail.vue` 的轨迹表格与两个对话框里 12 处模板硬编码中文已改为
      `logistics.detail.events.*` 文案键（i18n 门禁只校验键的引用与缺失，抓不到不走 i18n 的
      字面量，所以英文界面下整块轨迹是中文这件事一直不显红）。
      已复核该页其余组件（index/filter/form/table/stat）的属性级中文直写字面量为 0，
      即这一页的 i18n 缺口只在轨迹区（本轮已补）；全仓其余视图未做同类扫描。

- [x] **验布/委外/工资三域接口路径缺 `/production` 前缀（47 个请求恒 404）**：这三组资源注册在
      `routes/production.rs`，而该 router 挂在 `nest("/api/v1/erp/production")` 下，前端
      `api/fabric-inspection.ts`/`outsourcing.ts`/`wage.ts` 全部按裸路径调用，页面自始拿不到数据
      （run 4610 验布用例的 404 与 Vue 渲染中断为实证）。已按仓内既有惯例补前缀
      （`c2a6de1e`），并核对四组资源名均已在 PERMISSION_RESOURCES/path_utils 已知资源表内，
      补前缀不改变权限推导结果。

- [ ] **前端调用路径与后端 nest 前缀的一致性缺静态门禁**：本轮靠 CI 失败才暴露 47 个 404，
      同类问题（尤其 inventory/sales 两域的子 router 前缀）仍可能潜伏。doto 既有的
      「路由-返回结构审计脚本」需扩展为同时校验 nest 组合后的完整路径，
      把「FE 调用路径 ∈ 后端注册路径全集」做成 CI 静态门禁；
      脚本需处理 `.nest("/", …)` 与多级 nest 组合，不能按文件内首段路径近似判断
      （本轮试写的近似版本对 `/ap/*` 一类"前缀即资源名"路径大量误报，未采信）。

- [ ] **处方/染整配方审核人身份取自请求体，可由调用方任意指定**：
      `POST /production-recipes/{id}/approve` 与染整配方的同名端点都是
      `Json<ApproveRecipeRequest{approved_by: i32}>`，审批人来自请求体而非 AuthContext，
      等于任何有 approve 权限的账号都能把 approved_by 写成他人 ID 伪造审批足迹，
      双人约束在本域形同虚设。修法是把身份改为从 AuthContext 取、请求体仅保留备注，
      牵动前端调用点与既有 44f 用例，属安全语义变更需单独一轮确认。

- [ ] **入库单/审批等「补偿型」日志必须区分已存在与真失败**：本轮修掉收货完成事件的恒真告警，
      同类模式（补偿前先判存在）需作为惯例检查其余事件分支，
      避免"告警恒真 → 真故障被噪音淹没"再次出现。

- [ ] **`system-update` 资源未注册且被当模块前缀，系统更新页对所有角色 fail-closed**：
      `path_utils.rs:24` 把 `system-update` 列为模块前缀，`/system-update/version` 因此推导出
      资源名 `version`；而 `PERMISSION_RESOURCES` 里根本没有 `system-update`（grep 无命中），
      两条路都拿不到权限码。run 35613422400 分片 13 `ui-failures.md` 实证
      `API GET /system-update/version failed: code=403 message=权限不足`（33/40 系用例 catch 住
      才未翻红）。修法要先定授权模型：注册资源 + 角色种子 + 前端权限矩阵三处同步，
      属于「谁能操作系统更新/回滚」的安全决策，禁止顺手放行。
- [x] **run 35633791612 剩余 3 个 flow 分片的逐条判责与修复**：
      ① 44f-4 之后的 44f-5 销售发货硬编码 `WH-MAIN` 仓库编码 → 404（该用例此前被串行失败挡住从未执行），
         改为按仓库 ID 取真实编码 + ensureStockInWarehouse 保底有货（`6886327d`）；
      ② 48-2 的收入凭证其实已生成（日志 `凭证创建成功：no=ZZ20260921001` 且过账），
         断言却在凭证 JSON 里找订单 ID——凭证挂的是发货单，永远命不中，改为按发货单号定位（`8f3f0e66`）；
      ③ 53-1 的二级审批人被 53-0 放进逐用例 CLEANUP，afterEach 就删号 → 登录 401，
         跨用例共享账号改由 afterAll 清理（`76843636`）；
      ④ 质量 8D 前端阶段值写的是 d0~d8，后端是 d0_plan/d1_team/...，
         「推进下一阶段」对任何真实报告都取不到推进边，界面上 8D 根本走不动（CI 表现为等不到输入框超时），
         改为按后端真实状态值建推进边表并给未知值告警（`44dead67`）；
      ⑤ 发货含税金额为 0 时跳过收入凭证此前完全静默，补 warn 日志（`f5d606d7`）。

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
- [x] **库存前端契约曾是虚构字段（死列）**：挂账描述的 `quantity/color_code/lot_no` 那套类型
      已在前一轮改掉，本轮逐处核对确认已闭环：`api/inventory.ts` 的 `InventoryStock` 现与
      `StockResponse` 字段一一对应（`quantity_on_hand/quantity_available/color_no/dye_lot_no/
      batch_no/grade/bin_location` + `product_code/product_name/warehouse_name`），后端
      `attach_master_names` 在详情/新建/更新/列表/批次/导出 6 个出口都调用，主数据缺失时
      记 error 并留空，不用 ID 拼假名称；`InventoryStockTab` 的 9 列列名全部取真实字段。
      本轮补的是这条挂账没覆盖的三处出口不一致（导出丢主数据名称、打印列里有不存在的
      `quantity`、`api/inventory.ts` 仍把 stock_status 取值域注释成 正常/冻结/待检），见下。
- [ ] `/security/change-password` 渲染期 `SyntaxError {message:10}` 仍未定因
      （离线编译 20070 条 locale 消息 0 失败，排除静态 i18n 语法）；
      42x 遍历已改为失败时打印 pageErrors/consoleErrors 原文，待新 run 证据定位。
      本轮再排除一条：该页 `security.changePassword.*` 两门语言文案里不含
      `@ | % [ ] { }` 等 vue-i18n 消息语法字符，故不是链接消息/复数分支导致的编译期
      SyntaxError（与此前「离线编译 20070 条消息 0 失败」互相印证）；
      下一步只能从 CI 产物的 consoleErrors 原文或未压缩 source map 入手，不猜测
- [ ] 产品列表死列 `barcode`（无 DB 字段）/`category_name`（无 join）
- [ ] 收敛项：`AppState.event_notification_service` 由 `Option<…>` 改非 Option；
      `bpm_task.process_instance_id/name` 旧列删除；18 个 `json!({"list":…})` handler
      统一到 PaginatedResponse
- [ ] 待授权：ci-cd.yml 接入 check-i18n.mjs、把 20 个从未执行的 E2E 目录（218 测试）纳入 testMatch；
      eslint 开 `no-unused-expressions` 并解除对 e2e/ 的忽略
- [x] `views/purchase-receipt/composables/usePrcProc.ts` 对话框标题内联中文：预生成单号那条
      标题是模板串 `新增入库（预生成单号 ${no}）`，改用文案键 `addReceiptTitleWithNo`（带 {no}
      插值，中英双备）；`usePrc.ts` 里 `dialogTitle` 的初始值 `'新增入库'` 也改成
      `msg.translate('addReceiptTitle')`，与打开对话框时的赋值同源，避免英文界面下初始标题仍是中文

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
- [x] `views/purchase-receipt/composables/usePrcProc.ts` 曾用 `unit_master: it.unit || 'm'`
      兜底伪值：现该文件已无此写法（明细主数据缺失会被拦下并 logger.error 指出行号与缺失字段，
      不再用 P{id}/物料{id}/'m' 伪值提交），本轮复核确认
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
- [x] **产品列表 `category_name` 无数据源**：已按当页 `category_id` 批量查主数据回填
      `category_name`，主数据缺失按行记错误日志（`12657627`）。

- [ ] **产品条码 `barcode` 全链路缺列（假保存，需一整轮完成）**：前端表单有「条码」输入、
      列表有 `barcode` 列（ProductFormDialogTab.vue:87、ProductListTab.vue:190），
      `api/product.ts` 的 `toProductPayload` 用 `...rest` 原样透传，因此用户填的条码
      被后端 serde 静默丢弃——列恒空、表单改了不生效。products 表根本没有该列。
      实施点位（已核对：`product::Model` 字面量 0 处、`product::ActiveModel` 5 处全部带
      `..Default::default()`，新增 Option 字段无编译连锁）：
      1) `migration/src/domain/system/mod.rs`（products ALTER 段，299-300 行附近）加
         `ADD COLUMN IF NOT EXISTS "barcode" VARCHAR(100)` 与 `idx_products_barcode`；
      2) `models/product.rs` 加 `pub barcode: Option<String>`；
      3) `handlers/product_handler.rs` 的 Create/UpdateProductRequest 各加 `barcode`，
         两处 request→args 映射（379、426 行附近）带上；
      4) `services/product_service.rs` 的 CreateProductArgs/UpdateProductArgs 加字段，
         `product_ops/crud.rs` 的 create 解构与 ActiveModel 赋值、update 应用各落一处；
      5) 关键词检索加入条码（`product_ops/crud.rs:72-79` 的 Name/Code 两支），
         面料行业扫码查找是条码的真实用途，只存不查等于半个功能；
      6) E2E 锁层：建产品带条码 → 详情返回该值 → 用条码作 keyword 能搜到（缺任何一层即红）。
- [ ] **分页响应 key 全量审计工具需 nest 感知**：`/tmp` 版 key_audit2.py 已能解析 817 条
  GET 路由与 handler 返回结构，但对 `.nest("/x", handler())` 形式（route 声明为 "/"）
  无法还原完整路径，导致 159 处误报"无路由匹配"。补法：解析 routes/*.rs 的
  nest 组合链重建全路径，再与 e2e 调用点比对；跑通后可作为 CI 静态门禁。
  已借此确证并修复：/inventory/reservations（list vs items，commit 见下）。
- [ ] **vac 空断言残余**：04-finance 4-8+、05-system 其余项、00-deploy-init 的
  `expect(me.permissions)` 与成片 `expect(x.length).toBeGreaterThanOrEqual(0)` 恒真断言
  仍未清理（受限于逐端点核对成本），随 key 审计工具完成后一次性处理。
- [ ] **E2E 空断言 / 响应键错位清单（2026-09-22 重新扫描，Round 7-iter30）**：
  扫描口径是「`expect(` 的配对右括号后首个非空字符不是 `.`」，实测 flow/smoke/traversal/enhanced
  共 40 处 `expect(Array.isArray(x.items), msg);` 无匹配器行（此前记的「≥48 处」含注释行误报）。
  **本轮已修**：01-p2p 4 处、02-o2c 1 处、09-permissions 1 处补 `.toBe(true)`
  （`/inventory/stock`、`/audit-logs` 已确证是 PaginatedResponse{items,total,page,page_size}）；
  05-2 / 6-4 / 6-5 的 `roles.items` 改为 RoleListResponse{roles,total} 真实键
  （`GET /roles` 不分页，page/page_size 会被 serde 忽略），并删掉 6-4「取不到角色就 return」的静默跳过；
  6-5 的 `GET /roles/{id}/permissions` 按裸数组 Vec<PermissionResponse> 断言；
  4-10 `/finance/accounting-periods`、4-11 `/vouchers` 出参同为裸数组，已去掉 `.items` 并逐行校验字段。
  **本轮（续）把该类清零**：自写 nest 感知解析器（`routes/*.rs` 的 `.route()` 字面量 +
  `mod.rs` 的 `.nest("/api/v1/erp/<域>", <域>::routes())` + 域内二级 nest 与根路径 `route("/")`）
  逐端点确证出参键后，`expect(Array.isArray(x.items))` 无匹配器一类**已全部清零（0 处）**。
  确证结论（含被证实为"端点根本不存在"的三处，已改挂真实端点）：
  `/purchase/orders`、`/sales/orders`、`/ap/invoices`、`/purchase/receipts`、
  `/production/lab-dip/requests`、`/bulk-color-approvals`、`/financial-analysis/reports`、
  `/role-change-approvals`、`/inventory/stock`、`/audit-logs` = `items/total`；
  `/inventory/counts` = `{counts,total,page,page_size}`；`/inventory/adjustments` = `{adjustments,total,…}`；
  `/departments` = `{list,total}`；`/inventory/transfers`、`/data-permissions`、
  `/production/lab-dip/samples/by-request/{id}`、`/production/cost-collections`、
  `/production/dye-batch-lifecycle-logs/by-batch/{id}`、`/vouchers`、`/finance/accounting-periods`、
  `/roles/{id}/permissions` = 裸数组；`/roles` = `{roles,total}` 不分页。
  **三处用例原本在调不存在的路径**（此前因 expect 无匹配器而"绿"）：
  `GET /business-trace`、`GET /ai-models/quality-predictions`、`GET /ai-models/process-optimizations`
  以及 `GET /production/process-nodes`、`/production/process-logs`、`/production/lab-dip/samples`
  均无路由；已分别改挂 `/business-trace/forward?supplier_id&batch_no`（TraceListResponse{traces,total}）、
  `/ai/quality-predictions`、`/ai/process-optimizations`（routes/system.rs:366/399）、
  `/custom-orders/{id}/nodes`（ProcessTimeline 含节点与节点日志）、
  `/production/lab-dip/samples/by-request/{request_id}`。
  **判定键名的唯一可靠依据是 service 的返回类型**：handler 签名常是
  `ApiResponse<serde_json::Value>`，在其函数体附近抓 `json!` 键会把同文件其他函数的键
  误当成该端点的键——本批据此把 `/departments`、`/warehouses` 误判为 `{list,total}`
  （实为 `department_service::list` / `warehouse_service::list` 返回
  `PaginatedResponse{items,…}`），已在下一 commit 纠正回 items 并补真实断言。
  自写解析器只能用于定位 handler，出参键必须读到 service 层为止。
  **恒真断言一类本轮继续收敛**（已修 20 处）：00-deploy-init 的 10 处
  `expect(x.length ?? 0).toBeGreaterThanOrEqual(0)`、04-finance 科目表、06-collaboration 建用户、
  09-permissions 的 CSRF/缓存/拒绝审计三处（已确认 `permission.rs:147-195` 确实以
  `resource_type=permission_denied` 落审计，故按"真实越权 → 轮询审计命中"重写）、
  10e 的 6 处 `expect(page.url()).toBeTruthy()`（改为断言未被重定向出目标路由）与
  2 处状态显示占位断言（改为校验 `.el-tag` 存在且不是后端枚举原值）。
  **仍待处理**：`14-costing-period` 的
  `by-batch` 出参键未确证（1 处）与折旧用例的 try/catch 兜底把"折旧被状态机拒绝"当作正常路径
  （2 处 `records.items?.length >= 0`）、
  `rpa-data-extraction` 的 `elapsed >= 0`（弱断言，可改为与阈值比较），
  （教训补一条：确证路由时 `grep | head -N` 会截断，`cost-collections` 与
  `dye-batch-*` 的 analysis/by-batch 路由都在同一 route 块的 8 行之后；
  判"端点不存在"前必须看完整 route 块，且要记住 apiCall 对 404 是直接抛错、
  用例能绿就说明端点存在。）
  以及全库 79 处条件 `test.skip()`（多为"前置数据缺失即跳过"，需改为造前置数据后硬断言）。
  工艺节点日志只有 POST 写入端点（`/{id}/nodes/{nid}/logs`）没有 GET 列表端点，
  若要独立校验日志需在时间线出参之外补端点（属产品决策，未擅自新建）。
  **剩余 22 处需逐端点确证响应键后再补匹配器**（不得凭印象加 `.toBe(true)`，键错位会把绿变成真红）：
  01-p2p:488 `/purchase/orders`；02-o2c:401 `/sales/orders`；05-system:60 `/departments`、
  69 `/data-permissions`、180 `/bulk-color-approvals`、201 `/business-trace`、
  210 `/ai-models/process-optimizations`、219 `/ai-models/quality-predictions`；
  06-collaboration:212 `/purchase/orders`；10a:70 purchase-receipts、78 `/ap/invoices`、
  98 `/inventory/counts`（`CountListResponse`，键名待核）、113 `/inventory/transfers`、
  128 `/inventory/adjustments`（`AdjustmentListResponse`，键名待核）；
  10b:64 `/financial-analysis/reports`；
  10c:123/138 lab-dip requests/samples、153/175 `/bulk-color-approvals`、
  185 `/analytics/business-trace`、192 `/business-trace`、202 `/production/process-nodes`、
  208 `/production/process-logs`；10d:29 `/role-change-approvals`。
  **本轮另修两处已确证的错位**：03-production:339 的
  `list_lifecycle_logs_by_batch` 返回 `Vec<dye_batch_lifecycle_log::Model>`（裸数组），
  `logs.items` 恒 undefined，已改数组遍历并校验 `to_status/transition_code/batch_id`；
  10b F2-2 的 `list_collections` 同样返回裸 `Vec<cost_collection::Model>`，且带一个
  「失败就改查 /cost」的 catch 顶包，已去掉顶包并按 id/collection_no 断言。
  **另两类同族问题**：一是 try/catch 顶包剩 1 处（10c:185-192 analytics/business-trace 失败后改查
  `/business-trace`）需按真实端点重写；
  二是恒真比较（09-permissions:139 `expect(denied.length >= 0)`——且 `permission_denied` 是否
  真作为 audit `resource_type` 落库尚未确认，需先看拒绝审计写入点；10e:33/41；
  全库 79 处条件 `test.skip()`）。根治手段是给 `e2e/` 开
  `@typescript-eslint/no-unused-expressions` 并接入 CI lint（**动 eslint 配置/CI 需用户授权**）。
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
- [ ] **CI 工作流剩余可继续合并项（2026-09-22 本轮只做低风险部分）**：本轮已把
  `ci-fmt-fe`+`ci-contract-align`+`ci-i18n-check` 合并为 `ci-static-checks`，并新增
  `.github/actions/e2e-runtime` 让 32 个 E2E 族 job 实例改为下载 `frontend-dist` 而非各自重建前端。
  下一轮可继续：① `ci-audit` + `security-vulnerability-scan` + `ci-deps` 三个 job 都在跑
  cargo-audit / npm audit / 依赖图，可并为一个供应链 job（省一次 `cargo install cargo-audit` 全量编译）；
  ② `ci-coverage-rust` 是第 5 次全量编译，应复用 `ci-build-test-artifacts` 的 nextest 插桩产物或并入
  `ci-test-rust` 分区 1；③ `ci-dead-code-audit` 与 `dead-code-audit.yml` 的 `audit` job 几乎逐行重复，
  且自身又跑一次 `cargo check --all-targets`（`ci-lint-rust` 已编译过），应删其一或改 `workflow_call`；
  ④ `e2e-batch.yml` 是 `ci-e2e` 的近似克隆且使用未固定 SHA 的 `dtolnay/rust-toolchain@master`
  （违反本仓库"工具链引用固定 SHA"的供应链策略）。
  ⑤ `ci-fmt-rust`/`ci-build-rust` 会向 PR 分支 push 提交，在 `cancel-in-progress` 下自触发取消，
  需加 `[skip ci]` 与 paths 过滤后再启用（本轮未动）。
  ⑥ 分片→spec 的映射靠 `matrix.shard` 算术推导（2441-2491 与 2694/2732/2742），顺序敏感，
  调整分片数必须同步改三处，建议改为显式 spec 列表。
- [ ] **业务模式配置无法做「每轮建实例再删」的删除矩阵**：`mode_code` 是
  `validate_mode_code` 的封闭词表且同代码唯一，6 行由 v15 迁移种子写入并被 08 spec 只读依赖，
  因此 31b 删除矩阵的该用例已改覆盖子资源（流程节点创建→删除→按模式回读消失），
  `business_mode_config` 自身的 DELETE 端点目前无 E2E 覆盖。要恢复覆盖需产品决策：
  放开词表（允许企业自定义模式代码）或提供「克隆模式」端点；在此之前不要往 31b 塞回原用例。
  同类残留：`/business-modes/rules`、`/business-modes/flow-steps` 无全局列表端点（只能按模式查），
  `GET /vouchers`、`GET /finance/accounting-periods` 收 page/page_size 却不分页。
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

- [ ] 推送机制（两次实证后的正确做法）：本机 git 配了 `credential.helper=manager` + `store`，
      GCM 会先接管认证并弹交互框，在无交互 shell 里表现为 `git push` 静默挂起十余分钟无输出；
      而把脚本路径给 `GIT_ASKPASS` 也不行——Git for Windows 无法直接 spawn `.sh`，
      报 `cannot spawn ...: Exec format error`。可行做法是用 shell 型凭据助手：
      `GIT_TERMINAL_PROMPT=0 git -c credential.helper= -c credential.helper='!/绝对路径.sh' push ...`
      （前者清空 helper 列表，后者由 sh 调用，输出 protocol/host/username/password 四行），
      令牌仍只存仓库外文件、不进命令行与 .git/config；推送完立即删除该脚本。
      挂起时先 TaskStop 再重试，不要并发第二个 push。（流程备忘，非待修项）

- [ ] 本地无编译权的第二道约束（run 4611 实证）：新写函数签名若返回裸 `&str`/`&T`，
      且入参有两个以上引用，必须显式写生命周期（否则 E0106，rustfmt 与 --check 都发现不了，
      同类自查清单（sea-orm 2.0 实测，run 4613 二次实证）：
      `paginate()` 由 **`PaginatorTrait`** 提供，1.x 时代的 `QuerySelect` 已不再提供它
      （只导 QuerySelect 会同时得到 E0599 与 unused import 两条反馈）；
      `lock_exclusive/lock_shared/limit/offset/into_model` 才属于 `QuerySelect`；
      `order_by_desc`、`all`、`one` 是 `Select` 固有方法，`filter` 来自 `QueryFilter`。
      写新查询必须按「用到的方法 → 对应 trait」逐个对照，不按印象、也不按名字是否出现裁剪。
      （流程备忘，非待修项）
- [ ] CI 失败若再出现新 job：按 doto 流程拉日志→记录→下批修复
- [ ] 推送授权后：本地 commit 批量推送 + 观察 main CI（含 coverage 等 main 专属 job）

### 待用户决策

- [ ] 分支 260907-feat-piece-domain-phase2 是否删除（已合并 PR #939）
- [x] runtime-flow-map.md 部署章节 RLS 迁移 sequencing 说明已补充（L232，前轮完成）
- [x] ci-cd.yml 的 docs/** 触发路径失效——已获用户授权（2026-09-09，定向豁免）修改 yaml，冗余条目清理完成（65736c1），CI 行为无变化

后续新增任务请在此文件追加。

- [ ] **自审清单一处补充：把 `format!("{:?}", x)` 换成 `x.as_str()` 时，必须逐点看赋值目标类型**。
      run `35666231768`（4616）在 Clippy/构建/测试预编译三个 job 上报
      `material_shortage_service.rs:309 mismatched types [E0308]`：
      `BusinessEvent::MaterialShortageAlert.shortage_level` 是 `String`
      （`services/event_bus.rs:142`），我把 `format!("{:?}", level)` 换成 `level.as_str()` 时
      只看了「取值不变」，没看目标字段类型，`&'static str` 赋给 `String` 直接编译失败，
      整条流水线 28 个 job 又被 skipped。改法是赋值点补 `.to_string()`；
      自查要点：凡改「字符串生成方式」，要同时看接收方是 `String`、`&str` 还是列类型。

- [ ] **迁移域顺序是新迁移最常见的自审漏项（本轮实证）**：`migration/src/lib.rs` 的模块顺序是
      production → v15（v15 在其后），而很多业务表恰恰由 v15 脚本创建。把归一/回填语句放进
      production 域时，全新库上会报 `relation "xxx" does not exist`，迁移中断 → 后端起不来 →
      12 个 flow 分片 + 5 个角色矩阵 job 一起红（run `35676525961` 实证，`da227298` 已改到 v15 末尾）。
      自审清单加一条：新增涉及某表的 SQL 迁移前，先 `grep` 该表的 CREATE TABLE 落在哪个域，
      迁移必须排在该域之后；无法确定时优先复用该域的脚本尾部而不是新开编号。
- [ ] **iter31 新增待用户决策（已在会话内提问，未决前不擅自选边）**：
  1. 销售订单**明细编辑器**没有"色号"选择能力：后端 `sales_order_item` 已有
     `color_no/gram_weight/...`（`models/sales_order_item.rs:33-45` + 表列齐），
     但 `src/views/sales/` 下只有 `DeliveryDialog.vue`/`useOlv.ts` 出现 color_no，
     建单明细表格没有该列 → E2E `21a-fabric-sales-order.spec.ts:194` 等
     `.el-dialog:has-text("色号")` 3s 超时。是"补建单明细的色号选择"还是"该用例本就不该要求 UI 有这一列"？
  2. 退货明细的 `color_no/dye_lot_no/batch_no` 仍落 `''`（DB 有 DEFAULT ''，不违反 NOT NULL），
     于是**退货入库在四维口径下会命中空色号行**。是否要求从关联销售订单/出库明细回写这三列？
  3. `BusinessError` 出参 message 恒为脱敏文案（`utils/error.rs:94-99,423-435`），
     真文案只进 tracing → 前端用户与所有断言都看不到拒绝原因。保持脱敏（用例改断 code，本轮已这么做）
     还是给"可安全外显的业务文案"开一个字段？
  5. 调拨明细的「白坯布」判定口径在两条路径上不一致（本轮新发现，未擅自统一）：
     `inv/inventory_move.rs:244-247`（建单/重建明细，3 处）把 **color_no 为空** 也当作白坯布放行，
     而 `inv/batch.rs:1079`（向已有调拨单追加明细，iter31 新写）要求色号必填、
     仅"色号含白/等于 white"才算白坯布。同一张调拨单因此可能"建单时允许空色号、
     追加明细时被拒"。候选口径：(a) 白坯布应以物料/库存类型判定（grey vs dyed），
     空色号一律拒绝——最贴用户拍板的四维口径，但需要 products/inventory 上有可靠的类型来源；
     (b) 沿用"空色号即白坯布"的现状并把 batch 放宽对齐——兼容坯布经销流程，
     但"空字符串 ≡ 白色"本身是把缺数据当成业务取值。未选定前两边都保持现状，仅登记。
  4. 出库 `check_inventory` 的预留分支（`so/delivery_ops/inventory.rs:146-155`）不校验四维即
     `continue`，而 `reduce_inventory_four_dim` 要求四维候选非空——预留行与四维行不一致时
     运行期才报错。是否要求预留在建单期即按四维登记？
