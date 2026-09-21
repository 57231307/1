# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 2026-09-21

| PR | 一句话总结 |
|----|-----------|
| PR #941 | Round 7-iter23：定位并修掉潜伏 16 轮的三个结构性根因——① E2E 的 sales_order_approval 流程定义节点 schema（node_id/node_type、无 edges）与后端 resolve_first_task_node 需要的 id/type+edges 不符，后端走「无任务节点自动完成」异步回写 approved 抢跑用例的显式 approve（02-o2c 2-5），连带清掉 02-o2c/18/44f/48/31d 为绕开它写的条件兜底；② 通知列表读 data.items 而后端 key 是 data.list，叠加硬编码 localhost:8082 绕过 CSRF 头注入，31d/31e 全部 P0 通知链路 0 断言空转（31e-5 显式失败），改为 helpers.listNotifications + 固定标题硬断言 + 全量走 apiCall；③ 产品前后端字段契约不一致（name/code/status/standard_price vs product_name/product_code/is_active/price）致名称编码状态三列恒空、UI 建产品丢名字、keyword/is_active 两个筛选为死控件，修法为读侧权限过滤后补别名 + 写侧 toProductPayload 边界映射。判责证据源修正为 artifact 的 reports/playwright-output.txt + backend.log；登记 barcode/category_name 无数据源、真空断言成片、31d-D/F 前提缺失等 8 项 |

| PR #941 | Round 7-iter24：先拉取远端 15 个新提交并线性 rebase，再以 run 35585268854 的 error-context 原文逐条判责，修掉四个真实缺陷——① container 用 email_service 是否存在来决定 EventNotificationService 是否装配，未配 SMTP 时站内通知（订单/库存/公告联动）整体静默失效（31d-A/31e-2 之因），改为无条件构造、邮件通道内部自行降级；② bi_handler 16 端点是 ApiResponse<BiResponse<T>> 双层信封而前端按单层解包，致 SalesAnalysis 抛 map is not a function 且月度/品类/日钻取三表恒空（被 Array.isArray?:items??[] 形态兜底掩盖），新增 BiEnvelope/unwrapBi 统一解包；③ 销售订单 8 个状态在前端四处各自维护且都只覆盖 5 个、详情页干脆输出英文枚举，收敛为 utils/sales-status.ts 单一真相源并补 6 条中英文案与 3 个筛选项；④ 6-8/6-9 访问未注册的 /purchase/orders 与 /sales/orders 落 /404 而断言无匹配器。另纠正自己上一轮把 2-12 断言成 approved 的归属错误、为 44f-4 补齐必填 unit_master，并清零 12 个 flow spec 中 62 条永不失败空断言里的 49 条 |

| PR #941 | Round 7-iter25：以 run 35598454276 的 6 个失败分片逐条判责，修掉五个结构性缺陷——① 权限中间件把 segment3 命中模块前缀的路径用 segment4 当资源名，dashboard/notifications 的子路径（overview/sales-stats/unread-count/记录 ID）因此推导出 sales-stats、unread-count 等在注册表与角色种子中都不存在的资源名，全站仪表板与铃铛对任何非 admin 角色恒 403（fail-closed 静默失效），改为按注册表资源名归类；② 角色种子只有 4 个角色含 dashboard:read、1 个含 notifications:read，而 / 强制重定向 /dashboard、MainLayout 对全部用户拉未读数，其余角色登录后停在 /403，补 SHELL_PERMISSIONS 并让 32-roles 真断言落地路由（原来"页面有字"即通过）；③ 库存 /inventory/stock 只按仓库+产品过滤，色号/缸号参数被静默忽略且 StockResponse 不返回 batch_no/color_no/dye_lot_no/grade/已发货量，"产品→色号→缸号→匹号"四维聚合在接口层不成立，过滤下推 SQL 并补齐维度与计量字段（含导出列）；④ 采购入库 PurchaseReceiptCompleted 事件在创建草稿时即发布，监听器当场加库存并把单据置 COMPLETED，使"确认入库"端点对任何新单必报状态不允许（端点形同虚设）且库存无凭据入账，事件改由确认触发，01-p2p 补确认步骤并修掉误用的 purchase_order_id 字段名；⑤ 入库明细前端另起 product_code/product_name/unit/price 一套名字，提交时以 P{id}/物料{id}/'m' 伪造主数据、编辑回显与详情列全空，改为按后端 material_*/unit_master/unit_price 建模并从产品档案带入，缺主数据直接拦下报错。另纠正 6 处调用未注册端点（/system/bpm/*、/iam/role-change-approvals、/system/audit-logs、/system/health）、委外单创建缺 issue_date 等必填、销售列表 V2Table 选择器缺失，并登记 M-6 resource_id 语义使非 admin 无法访问任何 /{id} 端点等 4 项待决 |
| PR #941 | Round 7-iter26：以 run 35613422400 的分片 artifact 全量判责（3 个 flow 分片 + Rust 测试分区 7），把采购收货从两套实现收敛为一套——确认事务原先已按入库明细写库存并推进订单，PurchaseReceiptCompleted 事件的 receive_order 又按订单明细再收一次：订单被确认推到 COMPLETED 后事件侧状态门必失败并回滚（44f-4 入库单永不到终态），订单仍可收时两条路径抢同一库存行版本触发并发冲突（1-4 之因），且事件侧口径整体丢失入库明细的缸号/批次/匹号；现统一到确认事务（内含库存维度建键、乐观锁数量与版本回写、订单终态推进），事件只做收货终态核对与应付补偿，按订单明细收货的 receive_order 及其专属 helper 一并删除。补齐两处结构性缺口：入库明细 order_item_id 全靠调用方传，缺失即订单收货进度永不累加，改由后端按产品与剩余可收量挂载并拒绝挂错单；系统没有默认会计科目表且凭证生成路径写死未登记科目，收入/成本/应付类自动凭证被预校验整体拒绝，补默认科目种子（一二级分两条语句，避免同语句读不到刚插入的父行）并把应收科目 1131 更正为 1122。另修 8D 列表解包层级错误导致接口有数据而表格恒空、产品列表分类名无数据源、53 用不存在的 name 选择器登录、入库弹窗标题硬编码中文，并把 1-5 从「只查色号 + 缸号负向对照」收紧为按入库维度正向命中并断言同维度两行累加。登记 system-update 资源未注册致全角色 403、出库侧库存行仍按产品+色号匹配、入库明细与订单产品不匹配只记日志的策略待决 3 项 |
| PR #941 | Round 7-iter26 续：推送后 run 35631938195 只有 4 个非绿 job 且全部指向同一根因——我在删除按订单明细收货的实现时按「trait 名是否还出现」裁剪 sea_orm 导入，误删提供 lock_exclusive() 的 QuerySelect，8 个 E0599/E0282 使 lib 编译失败并连带 Clippy/测试预编译/整条 E2E skipped（rustfmt 只解析语法不解析路径，本地无编译权，此类错误只能靠导入裁剪纪律避免），恢复导入并把入库明细端点（POST/PUT /purchase/receipts/{id}/items）补上订单明细挂接、同时修掉明细更新静默丢弃 12 个 DTO 字段的假保存 |
| PR #941 | Round 7-iter26 续二：清掉台账状态的假控件——`GET /inventory/stock` 的 ListStockParams 根本没有状态参数，前端提交的 status=normal/warning/frozen 参数名与取值两头都不对（后端真实值是中文 正常/报废/已删除），筛选从不生效、状态列文案映射也永远走不到；service 侧把查询条件收敛为 StockListFilter（列表与导出共用，且不再加位置参数以免触发 too_many_arguments 新告警）并新增 stock_status 精确匹配，未指定时排除软删除行（删除是 stock_status=已删除 的软删除，已删库存此前混在台账里被当成在库量），状态值收进 models/status 常量、前端以 constants/inventory-stock-status.ts 为单一真相源并对未知值告警，同时移除库存弹窗里后端并不接受的状态字段（改了也不生效的假编辑），1-6 补状态筛选的正反向断言。登记冻结/待检状态无任何写入口属功能缺失、汇总与低库存仍各自写字面量两项 |
| PR #941 | Round 7-iter26 续三：run 35633791612 判责（68 job，49 success / 仅 1 个 E2E 分片失败 = 1-4 与 2-7，其余全绿，Rust 构建/Clippy/10 个测试分区首次全通过）。1-4 的 NOT_FOUND 是我上一批挂接修复暴露的既有缺陷——确认入库更新订单已收数量时逐条 map.remove(order_item_id)，面料把同一订单行按缸号拆成多行收货时第二条起查不到映射即失败（即便查到也会互相覆盖），改为按订单明细行汇总后一次性更新并让报错带入库行号。2-7 的「查无数据」则是把真实缺陷读成假失败：发货选行只按（产品+仓库），完全不校验订单行色号/缸号，出库流水记录的是被扣行自己的色号，故按固定色号过滤必然落空；断言改为取该产品全部库存行、逐行打印维度并核对累计已发货量不少于 500+300，同时登记「发货侧维度校验缺失需跨端单独一轮改造」 |
| PR #941 | Round 7-iter26 续四：run 35633791612 的 60 success / 3 个 E2E 分片失败逐条判责修尽——44f-5 发货硬编码 WH-MAIN 仓库编码致 404（该用例此前被串行失败挡住从未跑到）、48-2 的收入凭证其实已创建并过账但断言在凭证里找订单 ID（凭证按发货单挂账，永远命不中）、53-1 的二级审批人被逐用例 CLEANUP 在 afterEach 删号致登录 401（跨用例共享账号改由 afterAll 清理）、质量 8D 前端阶段值 d0~d8 与后端 d0_plan/d1_team/... 两套命名使「推进下一阶段」对任何真实报告都取不到边、界面 8D 走不动（改按后端真实状态建推进边表并对未知值告警），另给发货含税金额为 0 时跳过收入凭证的静默分支补 warn |
| PR #941 | Round 7-iter27：run 35645936703 首个失败为前端测试 job（8D 阶段单测仍按已被替换的前端简写词表断言 d8，改为锁定后端 11 态完整序列），随后把物流运单整页的六处假功能与一处状态机旁路一次修尽——列表接口原先不读任何查询参数、不分页、返回裸数组（筛选栏与分页控件全假），PUT 只认 {status} 而前端「编辑运单」提交的是运单字段（必然反序列化失败）且「发货」写入后端从不认的 shipped，状态更新端点对任意字符串都不校验（脏值一落库就同时击穿电子签收的状态门与删除守卫），前端词表 pending/shipped/in_transit/delivered/cancelled 加上 camelCase 的 i18n 键使状态标签恒显原值、统计卡恒 0、操作按钮按不存在状态渲染而整体隐藏，表内 waybill_no 与 order_no 两列后端从不返回恒为空白，建单「预计到达」以 YYYY-MM-DD 提交给 DateTime<Utc> 字段（选了就 400）、关联订单下拉是两条硬编码假数据；现后端补真实筛选与分页并回查订单号、状态按状态机校验且 SIGNED 只能经签收接口写入（否则绕过应收确认）、字段编辑限在运输中阶段、删除规则从「禁删在途误建单却放行删已签收凭证」纠正为反向，前端状态值收进 constants/waybill-status.ts 单一真相源、补上后端 P0-B13 电子签收端点的前端接线（此前零调用）、日期改日期粒度、订单下拉改真实查询，并以 e2e/smoke/logistics-contract.smoke.spec.ts 用「分状态计数之和=不分片总数」这一与数据量无关的恒等式锁定筛选真生效（替换 e2e/logistics/ 下两个整文件 if(isVisible) 空转且从未进 testMatch 白名单的死 spec）。另发现并解决 runtime-flow-map.md 被带着未解决冲突标记提交（§3.5/§8 两处 <<<<<<< HEAD，iter24 rebase 残留），项目介绍文档中此前一直含冲突垃圾内容 |
| PR #941 | Round 7-iter28：拉 run 35645936703 全部失败证据逐条判责并修尽（该 run 68 job 中仅 3 红：前端测试 + flow 分片 1 + flow 分片 14）。前端测试红在 8D 阶段单测仍按已被替换的前端简写词表断言 d8，改为锁定后端 11 态完整序列。flow 分片暴露三个真实缺陷：① 入库明细的映射用 ..Default::default() 收尾，把 CreateReceiptItemRequest 的色号/缸号/批次/等级/克重/幅宽/库位/包号/生产日期/保质期十个字段整批丢弃，而确认入库正是按「产品+色号+缸号+批次+等级」定位或新建库存行，缺维度即写出一条四维查询永远检索不到的库存行（1-5 之因），追加明细入口还额外丢行号/物料编码/名称/单位，故抽出 build_receipt_item_active_model 让建单与追加共用唯一映射；② 验布/委外/工资三域资源注册在 nest("/api/v1/erp/production") 的 router 内，前端三个 api 文件却按裸路径调用，47 个请求恒 404、三个页面自始无数据（run 4610 验布用例的网络层 404 + Vue 渲染中断为实证），按仓内惯例补前缀并核对权限推导不变；③ 收货完成事件无条件调用应付生成，而确认事务内已生成，于是每张正常入库单都刷「补偿失败需人工补生成」告警（告警恒真等于淹没真故障），改为先按 source_type+source_id 判定、查询失败单独出 error，并把该文件 4 处字面量收进常量。另修三处按错误契约书写的用例：48-2 用 apiCall 却直接读 .list/.items（信封层未剥）、53 二级审批请求无 body 被 axum 反序列化 400 挡在业务判定之前且清理用了不存在的 DELETE 路由（405）、1-4/1-5 用 ctx.productIds[0] || 1 静默回退掩盖装置缺失 |
| PR #941 | Round 7-iter28 续：把库存两张状态列的取值口径彻底收敛——台账状态（正常/报废/已删除）此前仍有 8 处代码各自写字面量（调拨建行、告警判定与汇总过滤、库存服务四处建改、出库流水建行、缺料可用量过滤），改一处必漏其余，已全部改引 `inventory_stock_status` 常量；质量状态列更严重：可用量、缺料预警、可出库筛选四处一律按中文「合格」过滤，而验布放行与批色放行两处写入的是另一个检验域的英文 `passed` 且未用任何常量，于是这批已放行的库存在所有可用性查询中永久不可见（账上有货、界面显示缺货、预警误报，且全程零日志），现建 `inventory_stock_quality_status`（合格/待检/不合格）常量、11 处写入与筛选改引常量，并加迁移 m0056 幂等归一存量 passed 行——只改代码不改历史数据的话，缺陷对既有行继续生效。委外回仓单的 quality_status 属另一张表的独立取值域（前端 qualified、E2E passed、后端确认时映射合格），未随本项一并改动，需单独核对。另登记台账状态「冻结/待检」仍无任何写入口的功能缺失判断。 |
| PR #941 | Round 7-iter28 续二：把状态词表核对方法用到生产订单页并证实整页失效——后端 production_orders.status 是大写下划线值（PUT /status 白名单也是大写），前端字典却是 draft/planned/in_production 小写名，于是筛选项 5 个值一个都命中不了（planned 后端不存在，待审批/已审批/已驳回无处可选）、状态标签与配色退化、操作按钮里新加的大写分支与永不生效的小写分支并存，导致草稿单无编辑/删除、已审批无排产、已排产无开工、生产中无完工，生产计划主流程在界面上根本无法推进，建单还自造 status:'draft' 提交；已重建字典（含取值派生与 i18n 文案八项）、文本配色统一出口并对状态机外取值告警、按钮按真实状态机重排、去掉自造字段与旧文案键 |

---

## 2026-09-20

| PR | 一句话总结 |
|----|-----------|
| PR #941 | Round 7-iter22：拉 run 35515772653 全 67 job（9 失败=8 E2E 分片+收尾级联，非 E2E 全绿）与 17 个 error-context 判责；修 5 处请求体违反后端 DTO（调拨 approve 缺 approved、色号缺 color_type/extra_cost、角色 code 大写被拒、质量问题缺 custom_order_id/severity、SO 明细 material_id 应为 product_id 且 44f-5 同错潜伏）、4 处测试硬编码与 1 处静默 skip（31d-C customer/product/warehouse_code、48-2 WH-MAIN、30-persistence BOM toBe(1) 与 version 取自 detail 而非 bom）、5-1 响应层级（iter20 参照错 handler 把 items 改成不存在的 list）、色卡用例命中隐藏 Tab 表格改 :visible 并去条件式空转；登记 42a-core/47-AU1/31e/31c 待 CI 迭代与环境级 flaky 3 例 |

| PR #941 | Round 7-iter21：拉 run 35510302989 全量失败日志判责，确认 iter20 自身把 bingxi-backend lib 编坏（E0596 shipped_pool 缺 mut + E0609 receipt Option 误取字段），致 Clippy/测试预编译/后端构建三 job 同根因 exit 101、E2E 与覆盖率/发布全链 skipped，即 iter5~iter20 共 16 轮 E2E 修复从未被 CI 实跑；同时修掉该重构丢失预留状态作用域导致的 released/cancelled 二次回加虚增可用库存、consumed 行被改写 cancelled 并回减 shipped 抹除真实出库与消耗审计，removed consumed 回滚量 or_insert 兜底改显式跳过+告警；listener 补偿应付不采纳编译器 unwrap 建议改 match 三分支并消除 if let Ok 对 DbErr 的零日志静默；AUTH_ONLY_PATHS 安全豁免白名单由 csrf/permission 两份手工同步副本收敛到 public_routes 单一真相源；删除 event_kafka 无 mod tests 支撑的死导入（并暴露 clippy baseline 按 message 匹配致新发生漏网）；文档同步 bug.md §三 import_csv 结论全部过时、MEMORY.md 常规规则章节重复致编号断裂 |

## 2026-09-05

| PR | 一句话总结 |
|----|-----------|
| PR #937 | fix: 修复 CI #4409 全部 14 失败——后端 CSRF 强制轮换恢复失效语义（用户索引改 token 集合）+ omni_audit 模块推断跳过 /api/v1/erp 前缀 + 公开路径测试对齐 init/status 设计；前端 401 刷新请求自等待死锁白屏（_skipAuthRetry）+ PermissionTab 信封直赋表格整页崩溃 + 全站金额 toFixed 按数字解析（后端 Decimal 为字符串，共享 formatCurrency/局部 fmt）+ 守卫 init 检查去 3s 硬超时 + Setup 健康检查走 /api 代理；E2E 修正 production/会计期间/产品色号错误路径、verifyAuditLog 查 omni 真实管道并对齐 resource_type 真实值、必填校验改 footer 主按钮（按钮文案差异）、Tab 切换可见表格、定制订单断言创建页、日期测试自建数据、登录响应状态日志 |

## 2026-08-21

| PR | 一句话总结 |
|----|-----------|
| PR #920 | ci: setup-rust action 智能探测可用内存设 cargo 并行度（防 OOM，5GB/普通+7GB/测试编译区分系数，不设上限） |
| PR #921 | ci: 合并 E2E 与死代码审计进主 CI 作为发布门禁，main 全绿（含 E2E+死代码）后才允许发布成品包；PR push 不跑这两个避免拖慢迭代 |
| PR #922 | ci: E2E 复用 ci-build-rust 预编译产物（download-artifact），不再重复编译，E2E 90min→30min |
| 文档 | README 项目数据更新至 2026-08-21 九轮审计封板数据；doto.md 纳入代码深挖审计 25 项问题（A.1-A.25），未完成 72→97 项 |

## 2026-08-20

| PR | 一句话总结 |
|----|-----------|
| PR #917 | 方案A稳健版：实现 parse_bullets 修复 CI Release；修 m0044/m0029 顺序+m0032 表存在性保护+m0044 补 notes 列修复 E2E 迁移中断；逐表列对比后删除纯重复迁移 m0017/m0018（19 表与原迁移列一致）；8 文件 +110 -721 |

## 2026-08-14

| PR | 一句话总结 |
|----|-----------|
| PR #907 | 测试代码编译修复：批量修复 50+ 处符号路径错误、6 处 Model 字段名、3 处顶层模块导入；CI 添加 --keep-going；规则 18 并入规则 6、规则 19 并入规则 21 |

## 2026-08-09

| PR | 一句话总结 |
|----|-----------|
| PR #878 | P3 批量任务第三轮：composables try/catch + dashboard store 错误提示 + 结账日志 + system_version 接入 + Alertmanager 启用 + RTL 支持 + 企业微信/钉钉渠道 + 供应商评估 model 重命名；8 文件 +80 行 |

## 2026-08-08

| PR | 一句话总结 |
|----|-----------|
| PR #877 | P3 批量任务第二轮：Retry-After HTTP 头 + 成本归集 event_retry + CSRF 提示 + env.d.ts 类型声明 + 迁移跳跃检测 + no-v-html 规则 + v-html 安全注释；7 文件 +80 行 |
| PR #876 | P3 批量任务：拆匹号改进 + 甘特图增强 + AUDIT_RETENTION_DAYS 纳入 AppSettings + HTTP OPTIONS/HEAD + AI 操作审计 + 慢查询阈值 + ARIA 标签 + 按钮 loading；8 文件 +120 行 |
| PR #875 | batch-18 P3 任务：拆匹号改进 + 甘特图拖拽增强 + AUDIT_RETENTION_DAYS 纳入 AppSettings + HTTP OPTIONS/HEAD 映射；2 文件 +60 行 |
| PR #871 | 批量实现 4 个 P2 任务：B12-P2-1 权限码命名规范 + B12-P2-5 流式导出 + batch-13 P2 供应商余额+异常订单 + batch-16 P2-3 通知模板模型；10 文件 +275 行 |
| PR #870 | 前端 18 dynamic_router 实现 + batch-12 P2-9 权限测试 + B04-P2-3 月末分摊测试：EndpointCache + 动态路由中间件 + test_data_permission.rs 6 个测试 + test_energy_allocation.rs 12 个测试；4 文件 +221 -3 |
| PR #869 | batch-18 P2-6 调拨在途库存独立核算 + batch-12 P2-9 权限测试 + B04-P2-3 月末分摊测试：inv/batch.rs 更新 quantity_incoming + test_data_permission.rs 6 个测试 + test_energy_allocation.rs 12 个测试；5 文件 +282 行 |
| PR #868 | batch-18 P2-2 委外加工费按缸号/匹号核算：outsourcing_order_item 添加 processing_fee/freight_fee 字段 + migration + DTO 更新；7 文件 +48 -1 |
| PR #867 | batch-18 P2-4 瓶颈识别扩产/外包建议：BottleneckSuggestion + generate_suggestions + overview 自动生成建议；3 文件 +115 -3 |
| PR #866 | batch-18 P2-5 排程重复录入校验：apply_schedule_details_to_orders 添加状态校验（仅 DRAFT/SCHEDULED）和日期保护（None 不覆盖）；3 文件 +31 -11 |

## 2026-08-07

| PR | 一句话总结 |
|----|-----------|
| PR #865 | batch-18 P2-7 缺料月报能力：material_shortage_handler.rs get_monthly_report + service get_monthly_report + 路由注册；3 文件 +156 行 |
| PR #863 | P2 快速修复 + 导出技术债：B12-P2-2 字段级权限推广 + B12-P2-3 权限审计日志接口 + batch-12 P2-8 审计日志保留调度 + batch-11 P2-6 打印水印 + T1/T2/T3 CSV 中转去除；18 文件 +572 -258 |
| PR #862 | A1-A4 完成：57 个新 docx 打印端点（纺织专用 9 + P0 16 + P1 25 + P2 6），63 个 get_*_print_data + 63 个 handler，覆盖纺织专用/P0/P1/P2 全部未实现打印场景；CI 全绿 |
| A0b | `ExportService::export_pdf` 由纯文本改写 printpdf 真实 PDF，修复 `report_enhanced` `POST /export/pdf` 与 `export_template` pdf 分支以 PDF 名义交付文本的规则 3 硬违规 |
| PR #859 | A0 完成：6 个原 HTML 打印场景改为 docx 成品（接入 generate_docx）+ 会计凭证 `/vouchers/:id/print` 路由 + 删除 generate_pdf/escape_html 死代码 + 模板数据驱动改造；CI 全绿 |

## 2026-08-05

| PR | 一句话总结 |
|----|-----------|
| PR #854 | batch-21 部署升级：端口冲突 + .env 600 + 断点续传 + 版本降级 + API 兼容 + 配置迁移 + 日志持久化 + draining + 升级监控告警 + 多租户残留；11 文件 |
| PR #853 | P2-Batch-32：胚布追溯字段 + 拆匹强校验 + 告警去重 + 在途采购 + 排程冲突告警 + 负荷告警 + SPT 调度；7 文件 |
| PR #852 | P2-Batch-31 续作：慢查询告警/优化追踪 + 通知订阅调度 + 权限合规 + 供应商评估 + recipe_opt + PII脱敏 + 存货跌价 + 部门服务 |
| PR #848 | P2-Phase-9 CRM 数据权限+数据流转：客户字段权限配置 + 客户操作审计日志 + 转化数据双向同步 + 客户主数据关系 + 客户 CLV；5 文件 |
| PR #847 | P2-Phase-8 CRM 商机+公海管理增强：阶段停留时长 + 商机竞争对手 + 商机跟进记录 + 回收规则跟进/成交周期 + 回收规则部门差异化 + 公海客户保护机制；6 文件 |
| PR #846 | P2-Phase-7 CRM 线索管理增强：线索来源 ROI 跟踪 + 线索分配规则 + 线索培育流程；3 文件 |
| PR #844 | P2-Phase-6C 调拨审批流 + 资金日报/月报：按金额分级审批 + 资金日报/月报接口；2 文件 |
| PR #842 | P2-Phase-6B 预算版本管理 + 资产减值测试 + 折旧政策变更：预算版本管理 + 资产减值测试 + 折旧政策变更 + m0099 migration；3 文件 |
| PR #840 | P2-Phase-6 现金流比率 + 趋势分析增强：现金流比率（OPERATING_CF_RATIO/SALES_CF_RATIO/CF_ADEQUACY_RATIO）+ 趋势分析增强（线性回归+移动平均+趋势方向）；2 文件 |
| PR #839 | P2-Phase-5 预算科目-会计科目映射 + 资产分类管理：budget_items.account_subject_id + asset_categories 表 + CRUD + fixed_assets.asset_category_id + m0098 migration；2 文件 |
| PR #838 | P2-Phase-4 辅助核算余额增强+账龄业务员维度+穿透查询：期初/期末余额计算 + 账龄按 salesperson_id GROUP BY + 穿透查询总账到辅助明细；3 文件 |
| PR #836 | P2-Phase-3.5 接入未实现的修复项：m0094 processor_type 筛选接入 + m0095 sales_contract_items service/handler/route + m0096 period_report_snapshot service/handler + m0097 aging_alert_rules service/handler + mask_fields 接入 customer_handler + record_actual_grade handler 端点；6 文件 |
| PR #835 | P2-Phase-3 DB migration：m0093 suppliers category_id FK + m0094 is_processor+processor_type + m0095 sales_contract_items + m0096 period_report_snapshot + m0097 aging_alert_rules；5 文件 |
| PR #834 | P2-Batch-31 全域 P2 审计修复：慢查询告警/优化追踪 + 通知订阅调度 + 权限合规 + 供应商评估 + recipe_opt + PII脱敏 + 存货跌价 + 部门服务；20 文件 |

## 2026-08-04

| PR | 一句话总结 |
|----|-----------|
| PR #833 | P2-Batch-30 Nginx gzip + 移动端触屏按钮：gzip 压缩 + Touch targets 44px CSS；2 文件 |
| PR #832 | CI Release 清理排序修复：sort -V 混合段数版本号排序错误，改用 --order asc 按创建时间排序 |
| PR #831 | P2-Batch-29 WebSocket 心跳超时断开：30s Ping + 60s 超时断开；1 文件 |
| PR #830 | P2-Batch-28 角色命名校验 + is_system 约束 + 报表参数 Validate：角色编码规范 + admin 约束 + Validate 派生；3 文件 |
| PR #829 | P2-Batch-27 报表元数据 refresh/cache + AI 速率限制：refresh_strategy/cache_ttl_seconds 字段 + AI 端点专用速率限制 (10 req/min/user)；2 文件 |
| PR #827 | P2-Batch-25/26 前端优化 + 后端超时/事务/账龄基准日：visualizer + persistedstate + lazy loading + alt prop + baseline_date + batch atomicity + OTel 10% + manager_id + supplier qual CRUD + BI/dashboard timeout；14 文件 |
| PR #826 | P2-Batch-24 CI Release 清理修复：修复 --cleanup-tag 不生效 + 清理无 Release 旧 tag（保留 100 个）；1 文件 |
| PR #824 | P2-Batch-23 部署变更文件记录：部署时记录变更文件列表到 deploy-changes.log；1 文件 |
| PR #823 | P2-Batch-22 AI explanation + 前端性能/可访问性/权限缓存：explanation 字段 + 错误去重 + 焦点重置 + 懒加载 + 权限缓存 + 路由预取；6 文件 |
| PR #822 | P2-Batch-19 售后退货类型 + incoterms 责任划分：issue_type 增加 return_goods（前后端）+ incoterms cost_bearer/清关责任接入报价构成；2 文件 |
| PR #821 | P2-Batch-21 部署脚本加固：日志持久化 + 配置权限600 + 健康检查database + CLI权限/确认/校验/回退 + 回滚验证；10 文件 |
| PR #820 | P2-Batch-08 角色校验 + 通配匹配 + 测试：is_system/admin 校验 + matches_permission 通配 + require_admin_role 测试 + 文档单复数；4 文件 |
| PR #819 | P2-Batch-07 AI 输入校验 + 降级 + 推理耗时：create_process_optimization 长度/枚举校验 + anomaly_detection 降级 + 错误文案 + inference_latency_ms；4 文件 |
| PR #817 | P2-Batch-06 权限 fail-closed + PII 脱敏 + CRUD 审计：extract_resource_info unknown fail-closed + 手机号/身份证脱敏 + CRUD 审计；3 文件 |
| PR #815 | P2-Batch-05 导出审计 + 打印水印：3 个导出端点补 Export 审计 + 打印IP水印 + rate_limit确认全局挂载；3 文件 |
| PR #814 | P2-Batch-04 硬编码 role_id==1 修复 + v-role 指令删除；2 文件 |

## 2026-08-03

| PR | 一句话总结 |
|----|-----------|
| PR #812 | CI Cargo.toml SemVer 兼容：TAG/Release 保持 4 段式 YYYY.M.D.HHMM，Cargo.toml 转为 3 段式 YYYY.MDHHMM |
| PR #811 | CI 版本号格式修复：日期分隔 YYYY.MMDD.HHMM → YYYY.M.D.HHMM |
| PR #810 | CI Release 流程修复：用 gh CLI 替代 softprops/action-gh-release，添加三重验证 |
| PR #809 | CI 发布说明调试：添加发布说明生成调试输出和错误处理 |
| PR #808 | CI 改进：clippy 日志化 + fmt 自动修正 + 消除重复检查 |
| PR #807 | CI clippy 新增警告修复：修复 18 条 clippy 警告（11 条代码修复 + 7 条 dead_code 恢复 baseline） |

## 2026-08-02

| PR | 一句话总结 |
|----|-----------|
| PR #803 | P2-Batch-03 类八法律合规剩余 + 类九色卡发放：跨境合规 + 商检/产地证 + 色卡报表/成本/预警/统计 12 端点接入路由；75 文件 +2322 -40 |
| PR #801 | P2-Batch-02 类五运行闭环：反馈闭环 + 重染补染 + 告警死信 + 色卡状态 + CancellationToken + 染缸占用 + 设备连接 + 人工成本归集 + 能耗凭证归集 + 期末调整；46 文件 +3001 -51 |
| PR #799 | P2-Batch-01b 续作：Cookie 双写 + 缓存一致性 + SQL 参数化 + 表重叠 + 测试补齐 + service 拆分 + 差异化 TTL；34 文件 +1799 -848 |
| PR #797 | P2-Batch-01a 首批快速修复：CSP+Argon2+魔法数字+TODO+i18n 注释；9 文件 |

## 2026-07-31

| PR | 一句话总结 |
|----|-----------|
| PR #795 | P0 缺陷 10-4 审计日志导出二次审计机制：新建 audit_log_export_log 防篡改表 + BEFORE UPDATE/DELETE 触发器禁止篡改 + 导出文件 SHA256 指纹留存 + /audit-logs/export-logs 查询端点；CI 全绿合并 main 7b18573 |
| PR #793 | P1 后续 #2 业务追溯 producer 接入：record_purchase_receipt 接入采购收货创建后、record_sales_delivery 接入销售发货后；best-effort 集成不阻塞主流程；CI 全绿合并 main 8fa619e5 |
| PR #791 | 缺陷 9-2 染色批次导出全量查询：导出查询加 .limit(10000) + QuerySelect trait 导入；CI 全绿合并 main b2e7b419 |
| PR #785 | P1 预留服务路由接入消除 174 个 dead_code 警告：为 14 个 P1 预留服务创建 handler 和 route 文件并注册路由；37 文件 +2093 -11 行 |
| PR #783 | Clippy runner shutdown (exit 143) 修复 + Release 变更说明模板 |
| PR #777 | 彻底移除 Docker/K8s 引用，对齐 systemd 直部署；11 文件 -130 行 |
| PR #776 | CHANGELOG/doto 文档同步 PR #775 合并记录 |
| PR #775 | P1-batch11 缺陷 2-3 遗留修复：补齐 4 个前端页面导出/打印按钮 v-permission 指令 |
| PR #771 | P1-batch02+03 通用代码质量+安全性：9 项 P1 全部完成 |

## 2026-07-30

| PR | 一句话总结 |
|----|-----------|
| PR #790 | P1 主线八维后续修复：盘点契约 P0-1 前端契约对齐 + API 网关 PATCH rate_limit 范围校验；CI 全绿合并 main 85aec7de |
| PR #788 | P1 委外收货主链路统一：confirm 收敛为唯一事务主链路 + OutsourcingOrderCompleted 事件 + workflow tests；CI 全绿合并 main |
| PR #786 | V15 主线八维审计 + 快速修复 P0/P2 批次：P0 全部 11 项 + P2 全部 3 项；21 文件 +989/-229；CI 全绿合并 main 8cd956d |
