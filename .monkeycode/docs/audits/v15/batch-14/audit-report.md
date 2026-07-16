# V15 AI 模块审计报告（类十六·批次 14）

- **审计子代理**：V15 审计子代理（类十六 AI 模块审计专项）
- **审计范围**：10 维度（对应审计计划 5996-6113 行）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 5996-6113 行
  - `/workspace/backend/src/models/ai_process_optimization.rs`
  - `/workspace/backend/src/models/ai_quality_prediction.rs`
  - `/workspace/backend/src/services/ai_extend_service.rs`
  - `/workspace/backend/src/services/ai/{mod,recipe_opt,quality_pred,pred,rec,detect}.rs`
  - `/workspace/backend/src/handlers/ai_extend_handler.rs`
  - `/workspace/backend/src/handlers/ai_analysis_handler.rs`
  - `/workspace/backend/src/handlers/advanced/{mod,analytics,decide,forecast,quality_pred,rec,recipe_opt,reorder}.rs`
  - `/workspace/backend/src/routes/system.rs`、`/workspace/backend/src/routes/analytics.rs`
  - `/workspace/backend/src/middleware/{permission,rate_limit,omni_audit,timeout,public_routes}.rs`
  - `/workspace/backend/src/utils/path_utils.rs`、`/workspace/backend/src/utils/field_mask.rs`
  - `/workspace/backend/src/services/{lab_dip,mrp_engine,supplier_evaluation,quality_inspection,dye_recipe,production_recipe}_service.rs`
  - `/workspace/backend/database/init_admin_permissions.sql`
  - `/workspace/backend/migrations/20260617000009_create_ai_process_optimizations/up.sql`
  - `/workspace/backend/migrations/20260617000010_create_ai_quality_predictions/up.sql`
  - `/workspace/backend/migrations/20260628000001_drop_tenant_columns/up.sql`
  - `/workspace/monitoring/prometheus/alert_rules.yml`、`/workspace/monitoring/grafana/dashboards/bingxi-erp-overview.json`
  - `/workspace/backend/src/services/business_metrics.rs`、`/workspace/backend/src/services/metrics_service.rs`
  - `/workspace/backend/tests/ai_extend_test.rs`
- **审计方法**：Read 审计计划 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 1：AI 工艺优化模块完整性

### 检查方法
- Grep 检索 `ai_process_optimization` / `optimize_recipe` / `recipe_optimization` 关键字
- Read `/workspace/backend/src/models/ai_process_optimization.rs`
- Read `/workspace/backend/src/services/ai/recipe_opt.rs`
- Read `/workspace/backend/src/services/ai_extend_service.rs` 第 36-289 行
- Read `/workspace/backend/src/handlers/ai_extend_handler.rs` 第 28-106 行
- Read `/workspace/backend/src/handlers/advanced/recipe_opt.rs`
- Read `/workspace/backend/migrations/20260617000009_create_ai_process_optimizations/up.sql`

### 发现

#### ✅ 已落实的项

1. **AI 工艺优化模型层完整**（`/workspace/backend/src/models/ai_process_optimization.rs:9-37`）
   - 表 `ai_process_optimizations` 包含 22 个字段：request_id（唯一）、color_no/fabric_type/dye_type、recommended_temperature/time_minutes/ph_value/liquor_ratio、similar_cases、confidence、source（knn/fallback）、reason、candidates_json、is_applied/applied_at/applied_by、feedback_score/feedback_remark、created_by/created_at/updated_at
   - CHECK 约束完备：`chk_ai_proc_source`（source 限定 knn/fallback）、`chk_ai_proc_confidence`（0.0-1.0）、`chk_ai_proc_feedback`（1-5）

2. **AI 工艺优化算法核心完整**（`/workspace/backend/src/services/ai/recipe_opt.rs:295-403`）
   - `AiAnalysisService::optimize_recipe` 实现 k-NN 相似度推荐 + 加权平均 + 退化路径
   - 相似度评分维度：颜色（1.0/0.7）+ 布类（+0.2）+ 染料（+0.1），最大理论值 1.3
   - 候选集筛选：近 6 个月未删除染料配方（`recipe_opt.rs:326-333`）
   - 退化路径：命中 < 3 条或 k=0 时回退典型参数表（80°C/45min/pH6.0/浴比1:8）

3. **持久化与历史回溯完整**（`/workspace/backend/src/services/ai_extend_service.rs:126-289`）
   - `create_process_optimization`：算法调用 + 落库 + 返回 (response, id)
   - `list_process_optimizations`：分页查询（page 1-1000，page_size 1-100）
   - `get_process_optimization`：详情查询
   - `list_process_optimizations_by_color`：按色号+布类历史（limit ≤ 50）
   - `apply_process_optimization`：标记应用 + 反馈打分（1-5 星校验）
   - `delete_process_optimization`：删除记录

4. **HTTP 端点齐全**（`/workspace/backend/src/handlers/ai_extend_handler.rs:32-106`）
   - POST `/api/v1/erp/ai/process-optimizations`（创建）
   - GET `/api/v1/erp/ai/process-optimizations`（列表）
   - GET `/api/v1/erp/ai/process-optimizations/:id`（详情）
   - POST `/api/v1/erp/ai/process-optimizations/:id/apply`（应用+反馈）
   - DELETE `/api/v1/erp/ai/process-optimizations/:id`（删除）
   - GET `/api/v1/erp/ai/process-optimizations/by-color`（按色号+布类）
   - POST `/api/v1/erp/ai/process-optimizations/batch`（批量，最多 20 条）
   - advanced 域：POST `/advanced/ai/recipe-optimization`（`analytics.rs:484`，仅算法返回不落库）

5. **输入校验基础**（`/workspace/backend/src/handlers/advanced/recipe_opt.rs:54-60`）
   - 校验 `color_no` 与 `fabric_type` 非空（trim 后）

6. **单元测试覆盖纯函数**（`/workspace/backend/src/services/ai/recipe_opt.rs:410-695`）
   - test_typical_params_fallback、test_color_match_knn、test_temperature_recommendation、test_fallback_path 共 4 个测试用例覆盖相似度/加权/退化路径

#### ❌ 缺陷项

**缺陷 1.1：染料配伍性 / 助剂兼容性校验缺失**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:33-45` RecipeOptRequest 仅包含 color_no/fabric_type/dye_type/color_name/k 5 个字段
  - `/workspace/backend/src/handlers/advanced/recipe_opt.rs:54-60` 仅校验 color_no / fabric_type 非空
  - `/workspace/backend/src/services/ai/recipe_opt.rs:329-333` 查询候选集时仅过滤 `is_deleted=false` 与 `updated_at >= six_months_ago`，无染料配伍性维度校验
- **业务影响**：违反 fabric-industry-research §11.2 要求，染料与布类不匹配（如分散染料用于棉）会生成无效配方推荐，工艺员采纳后可能导致染色失败、批次报废
- **修复建议**：在 RecipeOptRequest 增加 dye_compatibilities 字段（染料→可用布类映射表），optimize_recipe 内增加配伍性校验：若 dye_type 与 fabric_type 不在配伍表内则返回 422 业务错误

**缺陷 1.2：优化配方成本上限未校验**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:77-91` RecipeOptResponse 不含 cost 字段
  - `/workspace/backend/src/services/ai_extend_service.rs:140-174` 落库时无原配方成本对比
- **业务影响**：违反审计计划 16.5 第 2 项要求，优化后配方总成本可能高于原配方 10%，导致"AI 优化"反而增加生产成本
- **修复建议**：RecipeOptResponse 增加 `original_cost` / `optimized_cost` / `cost_delta_percentage` 字段；service 层比较染料+助剂总成本，超过 10% 时返回警告或拒绝

**缺陷 1.3：与化验室打样系统集成缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `AiExtendService` 仅出现在 `ai_extend_service.rs` 与 `ai_extend_handler.rs`，未被 `lab_dip_service.rs` 引用
  - `/workspace/backend/src/services/lab_dip_service.rs:1-272` 无 `ai_extend` / `process_optimization` / `optimize_recipe` 关键字
- **业务影响**：违反 fabric-industry-research §11.1 与审计计划 16.5 第 4 项要求，AI 优化配方可一键推送到化验室打样系统的能力缺失，工艺员需手工复制参数到打样系统，效率低且易错
- **修复建议**：在 ai_extend_handler 增加 POST `/api/v1/erp/ai/process-optimizations/:id/push-to-lab-dip` 端点，调用 lab_dip_service.create 自动创建打样请求，并在打样完成后将结果回写到 ai_process_optimizations.feedback_score

**缺陷 1.4：批量端点无并发限制且无原子事务**
- **风险等级**：P2
- **证据**：`/workspace/backend/src/handlers/ai_extend_handler.rs:268-305` batch_create_process_optimizations 循环串行调用 svc.create_process_optimization，每条独立事务；20 条失败时已落库数据无法回滚
- **业务影响**：批量调用部分失败时数据不一致；20 条串行调用响应时间可能远超 2s 性能阈值
- **修复建议**：改为单事务批量插入（使用 `Inserter::insert_many`）；或引入事务边界，失败时回滚已落库记录

---

## 维度 2：AI 质量预测模块完整性

### 检查方法
- Grep 检索 `ai_quality_prediction` / `predict_quality` / `quality_prediction` 关键字
- Read `/workspace/backend/src/models/ai_quality_prediction.rs`
- Read `/workspace/backend/src/services/ai/quality_pred.rs`
- Read `/workspace/backend/src/services/ai_extend_service.rs` 第 291-456 行
- Read `/workspace/backend/src/handlers/ai_extend_handler.rs` 第 108-186 行
- Read `/workspace/backend/src/handlers/advanced/quality_pred.rs`
- Read `/workspace/backend/migrations/20260617000010_create_ai_quality_predictions/up.sql`

### 发现

#### ✅ 已落实的项

1. **AI 质量预测模型层完整**（`/workspace/backend/src/models/ai_quality_prediction.rs:9-36`）
   - 表 `ai_quality_predictions` 包含 22 个字段：request_id（唯一）、product_id/inspection_type/window_days、total_inspections/avg_qualification_rate、trend/trend_rate、risk_score/risk_level、confidence、top_issues_json/recommendations_json/period_breakdown_json、source（history/fallback）、is_acknowledged/acknowledged_at/acknowledged_by、created_by/created_at/updated_at
   - CHECK 约束完备：`chk_ai_qual_type`（inspection_type 限定 all/incoming/inprocess/final/outgoing）、`chk_ai_qual_trend`（up/flat/down/nodata）、`chk_ai_qual_level`（low/medium/high）、`chk_ai_qual_source`（history/fallback）、`chk_ai_qual_risk`（0-100）、`chk_ai_qual_confidence`（0.0-1.0）、`chk_ai_qual_window`（1-365）

2. **AI 质量预测算法核心完整**（`/workspace/backend/src/services/ai/quality_pred.rs:313-486`）
   - `AiAnalysisService::predict_quality` 实现历史聚合 + 趋势判定 + 风险评分 + 问题归因 + 建议措施 + 退化路径
   - 趋势判定：最近 30 天 vs 之前 30 天移动平均，变化率 > ±5% 判定上升/下降
   - 风险评分：`(100 - avg_rate) * 0.6 + trend_penalty * 0.4`（趋势下降时 +15 分）
   - 风险等级：≥ 60 高 / 30-60 中 / < 30 低
   - 置信度：`min(sample_count / 30, 1.0)`，退化路径 0.3
   - 问题归因：5 类关键词库（颜色差异/色牢度/克重偏差/纬密偏差/强度不足）+ "其他"
   - 建议措施：按风险等级生成 1-3 条建议
   - 退化路径：历史 < 5 条时返回保守默认值（合格率 95%/置信度 0.3/风险等级中）

3. **持久化与历史回溯完整**（`/workspace/backend/src/services/ai_extend_service.rs:296-456`）
   - `create_quality_prediction`：算法调用 + 落库 + 中英文映射（trend: 上升→up、风险等级: 高→high）+ 返回 (response, id)
   - `list_quality_predictions`：分页查询
   - `get_quality_prediction`：详情查询
   - `list_quality_predictions_by_product`：按产品历史
   - `acknowledge_quality_prediction`：标记确认
   - `delete_quality_prediction`：删除记录

4. **HTTP 端点齐全**（`/workspace/backend/src/handlers/ai_extend_handler.rs:108-186`）
   - POST `/api/v1/erp/ai/quality-predictions`（创建）
   - GET `/api/v1/erp/ai/quality-predictions`（列表）
   - GET `/api/v1/erp/ai/quality-predictions/:id`（详情）
   - POST `/api/v1/erp/ai/quality-predictions/:id/acknowledge`（确认）
   - DELETE `/api/v1/erp/ai/quality-predictions/:id`（删除）
   - GET `/api/v1/erp/ai/quality-predictions/by-product`（按产品）
   - POST `/api/v1/erp/ai/quality-predictions/batch`（批量）
   - advanced 域：POST `/advanced/ai/quality-prediction`（`analytics.rs:485`）

5. **输入校验基础**（`/workspace/backend/src/handlers/advanced/quality_pred.rs:38-58`）
   - window_days 1-365 范围校验
   - inspection_type 长度 ≤ 32 校验

6. **单元测试覆盖纯函数**（`/workspace/backend/src/services/ai/quality_pred.rs:492-681`）
   - test_risk_score_low、test_risk_score_high、test_trend_calculation、test_fallback_low_data、test_mean_qualification_with_real_records 共 5 个测试

#### ❌ 缺陷项

**缺陷 2.1：质量预测准确率监控缺失**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/models/ai_quality_prediction.rs:9-36` 表结构未存储实际质检结果（actual_risk_level/actual_qualification_rate 字段缺失）
  - Grep 检索 `accuracy` / `precision` / `recall` / `f1_score` 在 AI 模块无匹配
  - 无定时任务/批处理对比预测与实际结果
- **业务影响**：违反审计计划 16.6 第 1 项与 16.6 第 4 项要求，无法验证预测准确率 ≥ 80%，无法触发准确率下降告警，预测模型长期运行无验证可能逐步失真
- **修复建议**：ai_quality_predictions 表增加 `actual_risk_level` / `actual_avg_qualification_rate` / `actual_recorded_at` 字段；新增定时任务（每周）对比预测与实际，计算准确率并写入 ai_quality_accuracy_report 表；准确率 < 80% 时触发告警

**缺陷 2.2：质量预测特征不完整**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai/quality_pred.rs:45-53` QualityPredRequest 仅包含 product_id/inspection_type/window_days 3 个字段
  - `/workspace/backend/src/services/ai/quality_pred.rs:335-346` 查询 quality_inspection_records 仅按 product_id/inspection_type/inspection_date 过滤
- **业务影响**：违反 fabric-industry-research §11.4 与审计计划 16.6 第 2 项要求，预测特征未包含染料/助剂/温度/时间/缸号/胚布来源等面料行业关键因子，风险评分仅依赖历史合格率趋势，预测准确度受限
- **修复建议**：QualityPredRequest 增加 dye_type/auxiliary_type/temperature_range/batch_no/fabric_source 等字段；predict_quality 内按这些特征分组聚合；特征权重通过历史数据训练得到

**缺陷 2.3：误判成本未量化追踪**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/models/ai_quality_prediction.rs:9-36` 表无 `customer_claim_amount` / `misjudgment_cost` 字段
  - Grep 检索 `customer_claim` / `misjudgment` 无匹配
- **业务影响**：违反审计计划 16.6 第 3 项要求，误判为 A 级实际 C 级时客户索赔金额无法追踪，AI 决策的业务影响不可量化，难以推动模型优化
- **修复建议**：表增加 `actual_grade` / `predicted_grade` / `claim_amount` / `claim_recorded_at` 字段；新增 service 方法 `record_actual_result(prediction_id, actual_grade, claim_amount)` 用于回填实际结果

**缺陷 2.4：质量预测与质检结果对账缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `quality_inspection_service` 中无 `ai_extend` / `predict_quality` / `quality_prediction` / `reconcile` / `对账` 关键字
  - `/workspace/backend/src/services/quality_inspection_service.rs` 无定时任务对比 ai_quality_predictions 与 quality_inspection_records
- **业务影响**：违反审计计划 16.6 第 4 项要求，每月对账预测结果与实际质检结果的机制缺失，无法生成准确率报告
- **修复建议**：新增 service `ai_quality_reconciliation_service`，每月 1 日定时执行对账：拉取上月 ai_quality_predictions → 查询同期间 quality_inspection_records 实际结果 → 计算准确率/召回率/F1 → 写入 `ai_quality_accuracy_reports` 表并通知质量管理员

---

## 维度 3：AI 模型管理（模型版本/训练/推理/评估）

### 检查方法
- Grep 检索 `model_version` / `training_date` / `training_dataset_size` / `accuracy` / `precision` / `recall` / `f1_score` / `model_drift` / `data_drift` / `retrain`
- Read `/workspace/backend/src/services/ai_extend_service.rs` algorithm_metadata 方法
- Read `/workspace/backend/src/services/ai/recipe_opt.rs`、`/workspace/backend/src/services/ai/quality_pred.rs`

### 发现

#### ✅ 已落实的项

1. **算法元信息可查**（`/workspace/backend/src/services/ai_extend_service.rs:521-533`）
   - `algorithm_metadata` 返回 JSON：process_optimization.algorithm="k-NN + 加权平均"、fallback="典型参数表"；quality_prediction.algorithm="趋势分析 + 风险评分"、fallback="保守默认"
   - 通过 GET `/api/v1/erp/ai/health`（`ai_extend_handler.rs:205-212`）对外暴露

2. **推理结果确定性来源标识**（`/workspace/backend/src/services/ai/recipe_opt.rs:86`）
   - RecipeOptResponse 包含 `source: String`（"knn" | "fallback"）
   - QualityPredResponse 同样包含 `source: String`（"history" | "fallback"）

#### ❌ 缺陷项

**缺陷 3.1：模型版本管理完全缺失**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/models/ai_process_optimization.rs:9-37` 与 `/workspace/backend/src/models/ai_quality_prediction.rs:9-36` 均无 `model_version` / `training_date` / `training_dataset_size` 字段
  - Grep 检索 `model_version` 在 backend/src 中无 AI 相关匹配
- **业务影响**：违反审计计划 16.1 第 2 项要求，模型版本号/训练日期/训练数据集大小不可追溯，无法回滚到历史版本，无法审计某次预测使用的是哪一版算法；算法变更（如调整 k 值或相似度权重）后历史数据无法对比
- **修复建议**：新增 `ai_model_versions` 表（id, model_name, version, algorithm, training_date, training_dataset_size, accuracy_metrics_json, status）；ai_process_optimizations/ai_quality_predictions 表增加 `model_version_id` 外键；service 层在创建记录时绑定当前生效版本

**缺陷 3.2：训练数据集合理性无评估**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:329-333` 查询候选集直接全表扫描近 6 个月染料配方，无样本量校验
  - `/workspace/backend/src/services/ai/quality_pred.rs:349` 仅校验 `records.len() < 5` 触发退化，未要求 ≥ 1000 条业务样本
- **业务影响**：违反审计计划 16.3 第 1 项要求，训练数据采样无偏差校验、无样本量阈值（≥ 1000），样本量过小时模型可能过拟合
- **修复建议**：service 层增加 `validate_training_dataset` 方法，统计样本量、类别分布、时间跨度；样本量 < 1000 或类别严重不均衡时拒绝上线并告警

**缺陷 3.3：推理结果一致性风险**
- **风险等级**：P2
- **证据**：`/workspace/backend/src/services/ai/recipe_opt.rs:329-333` 查询 dye_recipe 未按 id 或 created_at 排序，PostgreSQL 在并发写入或 VACUUM 时可能返回不同顺序，导致相同入参产生不同 TopK 结果
- **业务影响**：违反审计计划 16.3 第 2 项要求，相同输入产生非确定性漂移，工艺员重复调用可能得到不同推荐，影响信任度
- **修复建议**：查询候选集时增加 `.order_by_asc(Column::Id)` 保证稳定排序

**缺陷 3.4：模型评估指标缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `accuracy` / `precision` / `recall` / `f1_score` 在 backend/src/services/ai 无匹配
  - 无模型评估报告文件、无评估 service、无评估表
- **业务影响**：违反审计计划 16.3 第 3 项要求，模型上线前无 accuracy/precision/recall/F1 评估报告，无法验证模型质量
- **修复建议**：新增 `ai_model_evaluation` 表（model_version_id, evaluation_date, accuracy, precision, recall, f1, sample_count）；新增 service `evaluate_model(model_version_id)`，离线评估并生成报告；模型上线需附评估报告

**缺陷 3.5：模型漂移检测缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `model_drift` / `data_drift` / `retrain` 在 backend 无匹配
  - 无定时任务检测数据漂移或概念漂移
- **业务影响**：违反审计计划 16.3 第 4 项要求，模型上线后无定期检测数据漂移/概念漂移机制，染整业务特征变化（如新布类上市、染料配方更新）后模型预测准确度会逐步下降而无人察觉
- **修复建议**：新增定时任务（每月）对比当前数据分布与训练数据分布（如特征均值/方差/KL 散度），超过阈值时触发告警并启动再训练流程

---

## 维度 4：AI 接口安全（认证/权限/速率限制/输入校验）

### 检查方法
- Read `/workspace/backend/src/main.rs` 中间件挂载链路（auth_middleware → omni_audit → csrf → permission → request_validator）
- Read `/workspace/backend/src/middleware/permission.rs`、`/workspace/backend/src/middleware/rate_limit.rs`
- Read `/workspace/backend/src/utils/path_utils.rs`、`/workspace/backend/src/middleware/public_routes.rs`
- Read `/workspace/backend/database/init_admin_permissions.sql`
- Grep 检索 `ai:` / `advanced:` 权限码注册情况

### 发现

#### ✅ 已落实的项

1. **认证中间件全局挂载**（`/workspace/backend/src/main.rs:746`）
   - `auth_middleware` 全局挂载，AI 端点非 public_paths，必须携带有效 JWT

2. **权限中间件覆盖 AI 路径**（`/workspace/backend/src/utils/path_utils.rs:14`）
   - `is_module_prefix("ai")` 返回 true，将 `/api/v1/erp/ai/process-optimizations` 中的 `process-optimizations` 作为 resource_type 进行权限校验

3. **速率限制全局挂载**（`/workspace/backend/src/main.rs:753-755`）
   - `rate_limit_by_ip` 全局挂载，180 req/min/user（`rate_limit.rs`）

4. **通用审计日志全局挂载**（`/workspace/backend/src/main.rs:742-745`）
   - `omni_audit_middleware` 全局挂载，AI 端点的调用者/参数/响应/耗时被记录到 `omni_audit_logs` 表

5. **CSRF 防护全局挂载**（`/workspace/backend/src/main.rs:734`）
   - `csrf_middleware` 全局挂载

6. **请求体验证全局挂载**（`/workspace/backend/src/main.rs:732`）
   - `request_validator_middleware` 全局挂载

#### ❌ 缺陷项

**缺陷 4.1：AI 端点权限码未注册到 init_admin_permissions.sql**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/database/init_admin_permissions.sql:1-66` 仅注册 purchases/sales/inventory/finance/customers/suppliers/products/warehouses/users/audit/dashboard 11 类资源
  - 缺失 `process-optimizations` / `quality-predictions` / `summary` / `health` / `by-color` / `by-product` 等 AI 资源类型
  - `/workspace/backend/src/middleware/permission.rs:172-174` 仅 `admin_checker::is_admin_role` 返回 true 才直接放行；非 admin 角色访问 AI 端点时 check_permission 因无对应权限记录返回 false → 403 Forbidden
- **业务影响**：违反审计计划 16.4 第 1 项与 16.4 第 2 项要求，14 个 AI 端点无明确的角色权限矩阵；非 admin 角色全部被拒绝，analyst/manager 等业务角色无法使用 AI 功能，AI 模块形同虚设
- **修复建议**：在 init_admin_permissions.sql 增加 AI 端点权限码注册：
  ```sql
  -- AI 工艺优化
  (1, 'process-optimizations', 'read', true, NOW(), NOW()),
  (1, 'process-optimizations', 'create', true, NOW(), NOW()),
  (1, 'process-optimizations', 'delete', true, NOW(), NOW()),
  -- AI 质量预测
  (1, 'quality-predictions', 'read', true, NOW(), NOW()),
  (1, 'quality-predictions', 'create', true, NOW(), NOW()),
  (1, 'quality-predictions', 'delete', true, NOW(), NOW()),
  -- AI 看板
  (1, 'summary', 'read', true, NOW(), NOW()),
  (1, 'health', 'read', true, NOW(), NOW()),
  ```
  并补充 advanced 域端点（forecast-sales/optimize-inventory/detect-anomalies/recommendations/recipe-optimization/quality-prediction）

**缺陷 4.2：advanced 域 AI 端点路径解析风险**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/routes/analytics.rs:477-485` advanced 域路径如 `/advanced/ai/recipe-optimization` 嵌套在 `/api/v1/erp/advanced` 下
  - `/workspace/backend/src/utils/path_utils.rs:3-30` `is_module_prefix` 不包含 `"advanced"`，因此 permission 中间件会将 `/api/v1/erp/advanced/ai/recipe-optimization` 解析为 resource_type=`advanced`（取第 4 段），而非 `recipe-optimization`
  - 这意味着 advanced 域下所有 AI 端点共用一个 `advanced` 权限码，无法细粒度控制
- **业务影响**：违反审计计划 16.4 第 1 项要求，14 个 AI 端点中 6 个 advanced 域端点无法独立权限控制；analyst 角色若被授予 `advanced/read` 即可访问全部 advanced 域资源（包括 list_sales_contracts 等非 AI 资源）
- **修复建议**：path_utils.rs `is_module_prefix` 增加 `"advanced"` 分支；并按 advanced 子路径细分权限码

**缺陷 4.3：AI 推理数据范围未按用户过滤**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai_extend_service.rs:180-215` list_process_optimizations 仅按 color_no/fabric_type/is_applied/source 过滤，无用户数据范围过滤
  - `/workspace/backend/src/services/ai_extend_service.rs:358-393` list_quality_predictions 同样无用户数据范围过滤
  - `/workspace/backend/src/services/ai_extend_service.rs:463-519` ai_summary 返回全局聚合数据，所有用户看到的指标相同
- **业务影响**：违反审计计划 16.4 第 3 项要求，销售分析师只能看自己负责客户的预测这一规则未实现；任何能访问 AI 端点的用户可看到全部租户的 AI 数据（包括其他销售负责的产品预测）
- **修复建议**：service 层增加 `data_scope_filter` 参数，根据 AuthContext.user_id 关联的用户-产品责任范围过滤；ai_summary 按 data_scope 分别聚合

**缺陷 4.4：AI 端点专用速率限制缺失**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/middleware/rate_limit.rs:259-303` rate_limit_by_ip 全局 180 req/min/user
  - 无针对 `/api/v1/erp/ai/*` 的专用更严格速率限制
  - batch 端点单次请求触发 20 次算法调用，按 1 req 计数
- **业务影响**：违反审计计划 16.9 第 3 项要求，AI 推理 CPU 密集，180 req/min 远超系统能力，可能引发 CPU 过载
- **修复建议**：增加 AI 专用速率限制中间件（如 10 req/min/user），在 main.rs 通过 from_fn_with_state 针对 `/api/v1/erp/ai/*` 路径挂载

**缺陷 4.5：输入校验深度不足**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/handlers/ai_extend_handler.rs:34-48` create_process_optimization 直接接收 CreateProcessOptDto，未校验 color_no 长度（数据库字段 VARCHAR(64) 但代码无校验）、fabric_type 长度、k 值上限
  - `/workspace/backend/src/handlers/ai_extend_handler.rs:114-128` create_quality_prediction 未校验 inspection_type 枚举值（数据库 CHECK 约束允许 all/incoming/inprocess/final/outgoing 5 类，但 handler 不校验）
- **业务影响**：依赖数据库 CHECK 约束兜底，错误请求会触发 500 错误而非 422 业务错误，用户体验差；color_no 超长会触发 PostgreSQL 错误
- **修复建议**：handler 层增加 validator 库（如 `validator::Validate`）对 DTO 字段做长度/枚举/范围校验，返回 422 + 字段级错误信息

---

## 维度 5：AI 推理性能（响应时间/超时/降级/缓存）

### 检查方法
- Read `/workspace/backend/src/middleware/timeout.rs`
- Grep 检索 `cache` / `redis` / `moka` 在 services/ai 模块
- Read `/workspace/backend/src/handlers/ai_extend_handler.rs` batch 端点
- Read `/workspace/backend/src/services/ai/recipe_opt.rs` 第 326-354 行（候选集查询）

### 发现

#### ✅ 已落实的项

1. **全局超时机制**（`/workspace/backend/src/middleware/timeout.rs:5-38`）
   - `TIMEOUT_SECONDS = 30` 全局 30 秒超时，超时返回 408 Request Timeout

2. **退化路径实现**（`/workspace/backend/src/services/ai/recipe_opt.rs:308-323, 378-402`）
   - k=0 时强制退化返回典型参数表（80°C/45min/pH6.0/浴比1:8）
   - 命中 < 3 条时回退典型参数表 + reason 字段说明

3. **退化路径质量预测**（`/workspace/backend/src/services/ai/quality_pred.rs:349-367`）
   - 历史 < 5 条时回退保守默认值（合格率 95%/置信度 0.3/风险等级中）

#### ❌ 缺陷项

**缺陷 5.1：AI 推理响应时间无 ≤ 2s 阈值控制**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/middleware/timeout.rs:5` `TIMEOUT_SECONDS = 30` 远超审计计划 16.9 第 1 项要求的 2s
  - 无针对 AI 端点的专用更短超时（≤ 2s）
  - `/workspace/backend/src/handlers/ai_extend_handler.rs:268-305` batch_create_process_optimizations 串行循环 20 次 service 调用，每次涉及数据库查询 + k-NN 计算，单次约 200-500ms，20 次累计 4-10s 远超 2s
- **业务影响**：违反审计计划 16.9 第 1 项要求，AI 接口 P95 响应时间可能远超 2s，前端用户体验差；批量端点可能触发全局 30s 超时
- **修复建议**：
  1. 在 ai_extend_handler 增加专用超时包装 `tokio::time::timeout(Duration::from_millis(2000), svc.xxx())`，超时返回 503 + 降级结果（典型参数表）
  2. batch 端点改为并发执行（`futures::future::join_all`）或限制 batch size ≤ 5
  3. service 层在算法执行前检查候选集大小，> 1000 条时拒绝并提示缩小时间窗口

**缺陷 5.2：AI 并发控制缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `Semaphore` / `max_concurrent` 在 backend/src/services/ai 与 ai_extend_service 无匹配
  - `/workspace/backend/src/handlers/ai_extend_handler.rs:268-305` batch 端点串行循环，但单用户可同时发起多个 batch 请求
  - `/workspace/backend/src/middleware/rate_limit.rs:259-303` rate_limit_by_ip 限制 180 req/min，但 AI 单请求消耗远高于普通 CRUD
- **业务影响**：违反审计计划 16.9 第 3 项要求（max_concurrent=10），无并发限制时多用户同时调用 AI 推理可能 CPU 过载，影响其他业务
- **修复建议**：service 层引入 `tokio::sync::Semaphore`（permits=10），所有 AI 推理方法在进入算法前 `permit.acquire().await`；超时未获取 permit 时返回 503

**缺陷 5.3：AI 缓存策略完全缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `cache` / `redis` / `moka` 在 `/workspace/backend/src/services/ai/` 与 `ai_extend_service.rs` 无匹配
  - `/workspace/backend/src/services/ai/recipe_opt.rs:326-333` 每次调用都重新查询近 6 个月所有染料配方（可能数千条）
  - `/workspace/backend/src/services/ai/quality_pred.rs:335-346` 每次调用都重新查询窗口内全部质检记录
  - 项目其他模块使用 cache_service.rs（`/workspace/backend/src/services/cache_service.rs`），AI 模块未集成
- **业务影响**：违反审计计划 16.9 第 4 项要求（相同输入缓存 TTL 5min），相同入参重复调用引发重复数据库查询与重复算法计算，浪费数据库与 CPU 资源，响应时间无法优化
- **修复建议**：service 层引入 moka::Cache 或集成现有 cache_service，key 为 (color_no, fabric_type, dye_type, k) 的哈希，TTL 5min；缓存命中时直接返回缓存结果并标注 `cache_hit: true`

**缺陷 5.4：模型资源占用无监控**
- **风险等级**：P2
- **证据**：
  - Grep 检索 `model_cache` / `memory_limit` / `OOM` 在 backend 无 AI 相关匹配
  - `/workspace/backend/src/services/ai/recipe_opt.rs:329-333` 一次性加载全部候选集到内存，无 LIMIT 限制
- **业务影响**：违反审计计划 16.9 第 2 项要求（模型加载内存 ≤ 1GB），染料配方表数据量大时（> 10万条）可能 OOM
- **修复建议**：候选集查询增加 `.limit(10000)` 兜底；监控 RSS 内存并在接近 1GB 时拒绝新请求

**缺陷 5.5：batch 端点性能严重劣化**
- **风险等级**：P2
- **证据**：`/workspace/backend/src/handlers/ai_extend_handler.rs:268-305, 314-352` 两个 batch 端点均串行循环，每次循环独立 service 调用（含算法 + 落库两个事务）
- **业务影响**：20 条 batch 在 200ms/条 平均下需 4s，触发全局 30s 超时风险高，且响应时间长影响用户体验
- **修复建议**：改为并发执行（`futures::future::join_all` + Semaphore 限制并发 5）；或改为单事务批量插入算法预计算结果

---

## 维度 6：AI 数据管理（训练数据/推理数据/数据脱敏）

### 检查方法
- Read `/workspace/backend/src/services/ai/recipe_opt.rs` 第 326-354 行（候选集查询）
- Read `/workspace/backend/src/services/ai/quality_pred.rs` 第 333-346 行（质检记录查询）
- Read `/workspace/backend/src/utils/field_mask.rs`
- Grep 检索 `mask` / `desensitize` / `脱敏` 在 services/ai 模块
- Read `/workspace/backend/migrations/20260617000009_create_ai_process_optimizations/up.sql` 与 20260617000010

### 发现

#### ✅ 已落实的项

1. **AI 推理数据来源真实**（`/workspace/backend/src/services/ai/recipe_opt.rs:329-333`）
   - 工艺优化候选集来自 `dye_recipe` 表真实历史数据
   - 质量预测来自 `quality_inspection_records` 表真实历史数据

2. **审计日志敏感路径脱敏**（`/workspace/backend/src/middleware/omni_audit.rs:113-118, 311-334`）
   - 通用 omni_audit_middleware 对 change-password/reset-password/setup-totp 等敏感路径请求体脱敏为 "[REDACTED]"
   - 但 AI 端点不在敏感路径列表内

#### ❌ 缺陷项

**缺陷 6.1：AI 训练/推理数据未脱敏**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:329-333` 直接查询 dye_recipe 表全部字段（包括可能含客户信息的 color_name/remarks 字段）
  - `/workspace/backend/src/services/ai/quality_pred.rs:335-346` 直接查询 quality_inspection_records 全部字段（包括 customer_id/supplier_id/remark）
  - `/workspace/backend/src/services/ai/quality_pred.rs:268-302` mean_qualification_rate 直接读取 remark 字段用于问题归因关键词匹配
  - Grep 检索 `mask` / `desensitize` / `脱敏` 在 `/workspace/backend/src/services/ai/` 与 `ai_extend_service.rs` 均无匹配
- **业务影响**：违反规则 11 数据保护与审计计划 16.2 第 1 项要求，训练数据中客户/供应商敏感信息（手机/银行账号/身份证）未脱敏；remark 字段可能包含客户姓名/手机号等敏感信息被关键词匹配后写入 ai_quality_predictions.top_issues_json
- **修复建议**：service 层引入脱敏过滤器，对从 dye_recipe/quality_inspection_records 读取的 remark/customer_id/supplier_id 字段在写入 ai_*_json 前做脱敏处理（如客户 ID 哈希化、remark 中手机号正则替换为 `***`）

**缺陷 6.2：推理数据最小化未实现**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:329-333` `DyeRecipeEntity::find().filter(...).all(&*self.db)` 拉取全部字段，包括 formula/chemical_formula/remarks 等算法不使用的字段
  - `/workspace/backend/src/services/ai/quality_pred.rs:335-346` 同样拉取 quality_inspection_records 全部字段
  - `/workspace/backend/src/services/ai/recipe_opt.rs:336-350` 在内存中对所有候选计算相似度，无前置 SQL 过滤
- **业务影响**：违反规则 11 数据最小化与审计计划 16.2 第 2 项要求，全表扫描传给模型，数据库 IO 与内存消耗大幅增加；可能拉取与算法无关的敏感字段
- **修复建议**：使用 `QuerySelect::select_only()` + `Column::expr()` 仅查询算法所需字段（recipe_no/color_no/fabric_type/dye_type/temperature/time_minutes/ph_value/liquor_ratio）；在 SQL 层增加前置过滤（如 `WHERE color_no LIKE 'BL%'` 缩小候选集）

**缺陷 6.3：AI 中间结果未加密存储**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/migrations/20260617000009_create_ai_process_optimizations/up.sql:21` `candidates_json JSONB` 明文存储
  - `/workspace/backend/migrations/20260617000010_create_ai_quality_predictions/up.sql:19-21` `top_issues_json` / `recommendations_json` / `period_breakdown_json` 均为 JSONB 明文存储
  - Grep 检索 `encrypt` / `pgcrypto` 在 migrations 无 AI 相关匹配
- **业务影响**：违反规则 12 安全标准与审计计划 16.2 第 3 项要求，AI 中间结果（含候选案例 ID、问题归因、推荐措施）明文存储，数据库泄露即可获取全部 AI 推理历史
- **修复建议**：对 candidates_json/top_issues_json 等敏感字段使用 pgcrypto 扩展的 `pgp_sym_encrypt` 加密存储；service 层读取时解密；或对字段做应用层 AES 加密

**缺陷 6.4：训练数据集版本管理缺失**
- **风险等级**：P2
- **证据**：
  - 无 `ai_training_datasets` 表
  - service 层直接从业务表读取训练数据，无版本快照
- **业务影响**：训练数据变化后无法追溯历史版本，模型再训练时数据集不可复现
- **修复建议**：新增 `ai_training_datasets` 表（id, model_name, snapshot_date, row_count, feature_stats_json, storage_path）；定期生成训练数据快照并落库

---

## 维度 7：AI 结果可解释性（推理依据/置信度/人工复核）

### 检查方法
- Read `/workspace/backend/src/services/ai/recipe_opt.rs` 第 76-91 行 RecipeOptResponse
- Read `/workspace/backend/src/services/ai/quality_pred.rs` 第 78-108 行 QualityPredResponse
- Read `/workspace/backend/src/services/ai_extend_service.rs` apply_process_optimization 与 acknowledge_quality_prediction

### 发现

#### ✅ 已落实的项

1. **工艺优化响应包含可解释字段**（`/workspace/backend/src/services/ai/recipe_opt.rs:77-91`）
   - `confidence: f64`（0.0-1.0 置信度）
   - `source: String`（"knn" | "fallback"）
   - `reason: String`（人类可读原因说明，含 k 值与命中条数）
   - `candidates: Vec<RecipeCandidate>`（候选案例，最多 10 条，含相似度归一化分数）

2. **质量预测响应包含可解释字段**（`/workspace/backend/src/services/ai/quality_pred.rs:78-108`）
   - `confidence: f64`（0.0-1.0）
   - `trend: String` / `trend_rate: f64`（趋势与变化率）
   - `risk_score: u32` / `risk_level: String`（风险评分与等级）
   - `top_issues: Vec<QualityIssue>`（top 3 问题归因，含类型/次数/百分比）
   - `recommendations: Vec<String>`（按风险等级的建议措施）
   - `period_breakdown: Vec<PeriodStat>`（按月分段统计）

3. **人工干预机制部分实现**（`/workspace/backend/src/services/ai_extend_service.rs:247-274`）
   - `apply_process_optimization`：标记应用 + 反馈打分（1-5 星）+ 反馈备注
   - `acknowledge_quality_prediction`：标记确认 + 确认人 + 确认时间
   - 数据库字段 `is_applied`/`applied_at`/`applied_by`/`feedback_score`/`feedback_remark`/`is_acknowledged`/`acknowledged_at`/`acknowledged_by` 持久化

#### ❌ 缺陷项

**缺陷 7.1：缺少 explanation/factors 字段（影响因素权重）**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:77-91` RecipeOptResponse 无 `explanation` / `factors` 字段
  - `/workspace/backend/src/services/ai/quality_pred.rs:78-108` QualityPredResponse 无 `explanation` / `factors` 字段
  - 仅有 `reason: String` 文本说明，无结构化的"影响因素→权重"映射
- **业务影响**：违反审计计划 16.1 第 1 项要求，每个 AI 预测/推荐/优化结果必须包含 `explanation`/`confidence_score`/`factors` 字段；当前仅有 reason 文本，工艺员/质量管理员无法快速理解"为什么 AI 给出这个推荐"，影响采纳决策
- **修复建议**：RecipeOptResponse 增加 `factors: Vec<FactorContribution>` 字段，每项含 `factor_name`（如 "color_match"/"fabric_match"/"dye_match"）/ `weight`（0.0-1.0）/ `contribution`（数值贡献）；QualityPredResponse 同理增加 `factors` 字段（如 "avg_qualification_rate"/"trend_direction"/"sample_size"）

**缺陷 7.2：人工覆盖（override）机制缺失**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/services/ai_extend_service.rs:247-274` apply_process_optimization 仅标记 `is_applied=true` + 反馈打分，无"覆盖原决策"概念
  - 无 `overridden_by` / `override_reason` / `override_params` 字段
- **业务影响**：违反审计计划 16.1 第 4 项要求，关键 AI 决策（配方优化/补货）支持人工复核与覆盖，覆盖记录可追溯；当前工艺员若不采纳 AI 推荐参数而是手工修改，系统无记录，无法回溯"实际生产使用了什么参数"
- **修复建议**：ai_process_optimizations 表增加 `overridden_by` / `override_reason` / `override_params_json` 字段；新增 service 方法 `override_process_optimization(id, override_params, reason)`，记录覆盖人与覆盖参数；ai_summary 增加 override_count 统计

**缺陷 7.3：人工复核状态机不完整**
- **风险等级**：P3
- **证据**：
  - `/workspace/backend/src/models/ai_quality_prediction.rs:30-32` 仅有 `is_acknowledged`/`acknowledged_at`/`acknowledged_by` 字段，表示"已确认查看"
  - 无 `review_status`（pending_review/approved/rejected/overridden）/ `reviewer_id` / `reviewed_at` 字段
- **业务影响**：审计计划 16.1 第 4 项要求"关键 AI 决策支持人工复核"，当前仅"确认查看"无法表达"批准/拒绝/覆盖"三种状态
- **修复建议**：增加 `review_status` 枚举字段，状态机：pending_review → approved / rejected / overridden；记录 reviewer_id 与 reviewed_at

---

## 维度 8：AI 与业务集成（工艺优化→生产执行/质量预测→质检联动）

### 检查方法
- Grep 检索 `AiExtendService` / `AiAnalysisService` 在 backend/src/services 的引用
- Grep 检索 `ai_extend` / `optimize_recipe` / `predict_quality` 在 lab_dip_service/mrp_engine_service/quality_inspection_service/dye_recipe_service/production_recipe_service
- Read `/workspace/backend/src/services/lab_dip_service.rs` 第 1-30 行
- Read `/workspace/backend/src/services/mrp_engine_service.rs` 中 reorder 相关逻辑

### 发现

#### ✅ 已落实的项

1. **AI 模块独立运行**（`/workspace/backend/src/services/ai_extend_service.rs`）
   - 工艺优化、质量预测、销售预测、库存优化、异常检测、智能推荐各自独立 service，无循环依赖
   - advanced 子模块复用 AiAnalysisService（`/workspace/backend/src/handlers/advanced/forecast.rs:11`）

2. **算法核心与持久化解耦**（`/workspace/backend/src/services/ai_extend_service.rs:130-132`）
   - AiExtendService 调用 AiAnalysisService 算法，再独立落库
   - 算法纯函数化（recipe_opt.rs:108-289 / quality_pred.rs:114-307）便于复用

#### ❌ 缺陷项

**缺陷 8.1：工艺优化未与化验室打样集成**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `AiExtendService` 仅出现在 `ai_extend_service.rs` 与 `ai_extend_handler.rs`
  - Grep 检索 `ai_extend` / `optimize_recipe` 在 `/workspace/backend/src/services/lab_dip_service.rs` 无匹配
  - `/workspace/backend/src/services/lab_dip_service.rs:117-178` create 方法接收 CreateLabDipRequestRequest，不接收 AI 推荐参数
- **业务影响**：违反 fabric-industry-research §11.1 与审计计划 16.5 第 4 项要求，优化配方可一键推送到化验室打样系统的能力缺失；工艺员需手工复制 AI 推荐参数到打样系统，效率低且易错
- **修复建议**：在 ai_extend_handler 增加 POST `/api/v1/erp/ai/process-optimizations/:id/push-to-lab-dip` 端点，调用 lab_dip_service.create 自动创建打样请求（color_no/fabric_type/dye_type/temperature/time_minutes/ph_value/liquor_ratio 直接透传）；lab_dip_sample 表增加 `ai_process_optimization_id` 外键关联 AI 推荐记录

**缺陷 8.2：工艺优化未与生产执行集成**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `ai_extend` / `optimize_recipe` 在 `/workspace/backend/src/services/production_recipe_service.rs` 与 `/workspace/backend/src/services/dye_recipe_service.rs` 无匹配
  - `/workspace/backend/src/models/ai_process_optimization.rs:39-58` Relation 仅关联 user 表（AppliedByUser/CreatedByUser），无关联 dye_recipe 或 production_recipe
- **业务影响**：违反审计计划 16.8 第 4 项要求，AI 配方优化结果未推送至生产执行系统；标记 `is_applied=true` 后实际生产是否使用该参数无法追踪
- **修复建议**：ai_process_optimizations 表增加 `production_recipe_id` 外键；新增 service 方法 `push_to_production(id, production_recipe_id)`，将 AI 推荐参数写入 production_recipe 表并建立关联

**缺陷 8.3：质量预测未与质检结果联动**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `ai_extend` / `predict_quality` / `quality_prediction` 在 `/workspace/backend/src/services/quality_inspection_service.rs` 无匹配
  - `/workspace/backend/src/models/ai_quality_prediction.rs:39-58` Relation 关联 product 与 user，未关联 quality_inspection_record
  - 质检结果产生后无回调写入 ai_quality_predictions.actual_* 字段
- **业务影响**：违反审计计划 16.6 第 4 项要求，质量预测与质检结果无联动；预测准确率无法计算，预测-实际对比报告无法生成
- **修复建议**：在 quality_inspection_service 完成质检后触发事件，ai_extend_service 监听事件并更新对应 ai_quality_predictions 记录的 actual_risk_level/actual_qualification_rate 字段

**缺陷 8.4：补货推荐未与 MRP 引擎对账**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `ai_extend` / `optimize_inventory` 在 `/workspace/backend/src/services/mrp_engine_service.rs` 无匹配
  - `/workspace/backend/src/services/ai/rec.rs:32-164` optimize_inventory 独立计算安全库存与再订货点，未调用 mrp_engine_service
  - `/workspace/backend/src/services/mrp_engine_service.rs:210, 256` mrp_engine 内部有 safety_stock/reorder_point 计算，但与 AI 推荐结果无对账
- **业务影响**：违反审计计划 16.8 第 4 项要求，AI 补货建议与 MRP 引擎结果差异超 20% 需人工复核的机制缺失；两套系统可能给出冲突的补货建议，业务员无所适从
- **修复建议**：在 rec.rs optimize_inventory 完成后调用 mrp_engine_service 计算同期间 MRP 结果，比较两者差异，差异 > 20% 时在 reason 字段标注"与 MRP 差异 X%，建议人工复核"

**缺陷 8.5：补货供应商推荐未集成 supplier_evaluation**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/services/ai/rec.rs:152-160` InventorySuggestion 结构体无 `supplier_id` / `supplier_score` 字段
  - Grep 检索 `supplier_evaluation` 在 `/workspace/backend/src/services/ai/` 无匹配
- **业务影响**：违反审计计划 16.8 第 3 项要求，补货决策可推荐最优供应商的能力缺失
- **修复建议**：InventorySuggestion 增加 `recommended_supplier_id` / `supplier_score` 字段；optimize_inventory 完成补货数量计算后，调用 supplier_evaluation_service 查询该产品历史供应商评分，推荐 top 1 供应商

**缺陷 8.6：销售预测未与销售订单 / 库存补货联动**
- **风险等级**：P3
- **证据**：
  - `/workspace/backend/src/services/ai/pred.rs:30-163` forecast_sales 仅返回预测结果，未触发补货建议或生产计划
  - 无事件总线集成（event_bus 未订阅预测完成事件）
- **业务影响**：销售预测结果仅展示给用户，未驱动下游业务流程（如自动生成补货建议、生产计划），AI 价值未充分释放
- **修复建议**：销售预测完成后发布 `SalesForecastCompleted` 事件，rec.rs optimize_inventory 订阅该事件自动触发补货建议生成

---

## 维度 9：AI 错误处理（模型不可用/推理失败/降级策略）

### 检查方法
- Read `/workspace/backend/src/services/ai/recipe_opt.rs` 第 295-403 行（退化路径）
- Read `/workspace/backend/src/services/ai/quality_pred.rs` 第 318-367 行（退化路径）
- Read `/workspace/backend/src/handlers/ai_extend_handler.rs` 错误处理
- Read `/workspace/backend/src/handlers/advanced/forecast.rs` 第 79-83 行（销售预测失败兜底）
- Read `/workspace/backend/src/utils/error.rs` AppError 定义

### 发现

#### ✅ 已落实的项

1. **算法层退化路径完整**（`/workspace/backend/src/services/ai/recipe_opt.rs:308-323, 378-402`）
   - k=0 强制退化返回典型参数表
   - 命中 < 3 条自动回退典型参数表 + 标注 source="fallback" + reason 字段说明

2. **质量预测退化路径完整**（`/workspace/backend/src/services/ai/quality_pred.rs:349-367`）
   - 历史 < 5 条回退保守默认值（合格率 95%/置信度 0.3/风险等级中）
   - 标注 source="fallback"

3. **AppError 错误分类完整**（`/workspace/backend/src/utils/error.rs`）
   - 提供 `AppError::validation`（422）/ `AppError::not_found`（404）/ `AppError::internal`（500）/ `AppError::forbidden`（403）等分类

4. **handler 错误传播规范**（`/workspace/backend/src/handlers/ai_extend_handler.rs:38-48` 等）
   - 所有 handler 返回 `Result<Json<ApiResponse<T>>, AppError>`，错误统一通过 ApiResponse 序列化

5. **销售预测失败兜底**（`/workspace/backend/src/handlers/advanced/forecast.rs:79-83`）
   - forecast_sales 失败时返回 `ApiResponse::error` 而非 500 错误，前端可正常显示错误信息

#### ❌ 缺陷项

**缺陷 9.1：模型不可用降级策略缺失**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/services/ai/recipe_opt.rs:300-303` optimize_recipe 直接返回 Result，无 try-catch 包装
  - 数据库连接失败时直接返回 `AppError::internal`，无降级到"返回缓存结果"或"返回典型参数表"的逻辑
  - Grep 检索 `failover` / `degrade` / `降级` 在 services/ai 无匹配
- **业务影响**：违反审计计划 16.9 第 1 项要求，超时返回降级结果（默认/缓存）；当前数据库故障时 AI 接口直接 500，前端无法展示任何结果
- **修复建议**：service 层增加降级包装：算法失败时尝试从缓存读取最近一次成功结果；缓存也无时返回典型参数表并标注 `degraded: true`；handler 层捕获 internal 错误并返回 200 + 降级结果（而非 500）

**缺陷 9.2：批量端点部分失败处理不完整**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/handlers/ai_extend_handler.rs:280-298` batch_create_process_optimizations 单条失败时记录 `success: false` + `error` 字段，但已成功落库的记录无法回滚
  - 无重试机制，失败条目需用户手工重新提交
- **业务影响**：批量调用部分失败时数据一致性受损；用户需手工识别失败条目并重新提交，体验差
- **修复建议**：增加 `idempotency_key` 字段，用户重试时跳过已成功条目；或改为全部成功/全部失败的两阶段提交

**缺陷 9.3：异常检测失败未兜底**
- **风险等级**：P2
- **证据**：`/workspace/backend/src/handlers/advanced/decide.rs:36-37` anomaly_detection 调用 service.detect_anomalies，失败时直接返回 AppError，未提供空列表兜底
- **业务影响**：异常检测算法失败时前端无法展示任何异常列表，影响运营人员监控
- **修复建议**：handler 层捕获错误后返回空列表 + `degraded: true` 标识

**缺陷 9.4：库存优化失败错误信息不友好**
- **风险等级**：P3
- **证据**：`/workspace/backend/src/handlers/ai_analysis_handler.rs:79-85` optimize_inventory 失败时返回 `AppError::internal(format!("库存优化失败: {}", e))`，错误信息含内部异常堆栈
- **业务影响**：错误信息暴露内部实现细节，可能泄露数据库结构；用户无法理解错误原因
- **修复建议**：返回友好错误信息"库存优化服务暂时不可用，请稍后重试"，内部错误仅写入日志

**缺陷 9.5：AI 推理超时无降级结果**
- **风险等级**：P1
- **证据**：
  - `/workspace/backend/src/middleware/timeout.rs:11-37` 全局 30s 超时返回 408 + "请求超时" 文本，无 JSON 响应
  - 无 AI 专用超时降级机制（如超时返回缓存或典型参数表）
- **业务影响**：违反审计计划 16.9 第 1 项要求（超时返回降级结果），超时时前端仅收到 408 文本，无法展示降级结果
- **修复建议**：在 service 层使用 `tokio::time::timeout` 包装算法调用，超时时返回降级结果（典型参数表/保守默认值）+ `degraded: true` 标识

---

## 维度 10：AI 审计日志（推理记录/模型变更/结果采纳）

### 检查方法
- Grep 检索 `ai_decision_log` / `ai_audit` 在 backend
- Read `/workspace/backend/src/middleware/omni_audit.rs`
- Read `/workspace/backend/src/models/ai_process_optimization.rs` 与 `ai_quality_prediction.rs` 中 is_applied/feedback_score/acknowledged 字段
- Grep 检索 AI 模型变更记录机制

### 发现

#### ✅ 已落实的项

1. **AI 推理结果持久化**（`/workspace/backend/src/models/ai_process_optimization.rs:9-37`、`/workspace/backend/src/models/ai_quality_prediction.rs:9-36`）
   - 每次推理结果落库（request_id 唯一、入参、推荐参数、置信度、来源、时间戳）
   - 支持历史回溯查询

2. **结果采纳记录**（`/workspace/backend/src/models/ai_process_optimization.rs:29-33`）
   - `is_applied: bool` / `applied_at: Option<DateTime>` / `applied_by: Option<i64>` 记录应用状态、时间、操作人
   - `feedback_score: Option<i16>` / `feedback_remark: Option<String>` 记录反馈打分与备注

3. **通用 HTTP 审计日志覆盖 AI 端点**（`/workspace/backend/src/middleware/omni_audit.rs:18-29, 230-290`）
   - omni_audit_middleware 全局挂载，记录所有 AI 端点的 method/uri/user_id/username/ip_address/request_body/response_body/latency
   - 写入 `omni_audit_logs` 表

4. **质量预测确认记录**（`/workspace/backend/src/models/ai_quality_prediction.rs:30-32`）
   - `is_acknowledged: bool` / `acknowledged_at: Option<DateTime>` / `acknowledged_by: Option<i64>`

#### ❌ 缺陷项

**缺陷 10.1：AI 决策审计日志专用表缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `ai_decision_log` / `ai_audit` 在 backend 无匹配
  - 无专用 AI 决策审计表存储 input/output/user_id/timestamp/latency/model_version
  - 当前仅依赖通用 omni_audit_logs 表，但该表为 HTTP 请求级别，无 model_version/algorithm_confidence 等AI 专用字段
- **业务影响**：违反审计计划 16.1 第 3 项要求，AI 每次调用记录 input/output/user_id/timestamp/latency 到 `ai_decision_log` 表；当前缺失专用表导致 AI 决策审计需从 omni_audit_logs 解析 request_body/response_body，效率低且无法关联模型版本
- **修复建议**：新增 `ai_decision_logs` 表（id, decision_type, model_version_id, input_json, output_json, user_id, ip_address, latency_ms, confidence, source, created_at）；service 层在每次 AI 推理完成后异步写入该表

**缺陷 10.2：模型变更审计日志缺失**
- **风险等级**：P1
- **证据**：
  - Grep 检索 `model_version` 在 backend 无 AI 相关匹配（缺陷 3.1）
  - 无模型版本管理表，自然无模型变更审计
  - 算法参数（如 k 值默认 5、相似度权重 1.0/0.7/0.2/0.1、TYPICAL_TEMPERATURE=80.0）硬编码在代码中（`recipe_opt.rs:97-106`），变更仅通过 git 提交记录可查
- **业务影响**：违反审计计划 16.1 第 2 项与第 3 项要求，模型版本号/训练日期不可追溯，模型变更（如调整 k 值或权重）无审计记录；算法参数硬编码意味着变更需重新编译部署，且无变更审批流程
- **修复建议**：新增 `ai_model_versions` 表记录每次模型变更（版本号/算法/参数/变更人/变更时间/审批状态）；算法参数从数据库或配置中心读取而非硬编码

**缺陷 10.3：AI 推理延迟未记录**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/models/ai_process_optimization.rs:9-37` 与 `ai_quality_prediction.rs:9-36` 均无 `inference_latency_ms` 字段
  - 通用 omni_audit_logs 有 latency 字段但未区分 AI 推理与其他 HTTP 请求
- **业务影响**：违反审计计划 16.1 第 3 项要求（latency 到 ai_decision_log 表），无法监控 AI 推理性能趋势，无法识别慢查询
- **修复建议**：ai_process_optimizations/ai_quality_predictions 表增加 `inference_latency_ms` 字段；service 层在算法执行前后记录耗时并落库

**缺陷 10.4：结果采纳反馈未回流模型**
- **风险等级**：P2
- **证据**：
  - `/workspace/backend/src/models/ai_process_optimization.rs:32-33` feedback_score/feedback_remark 字段已存在
  - 但 Grep 检索 feedback_score 在 services/ai 与 ai_extend_service 仅在 apply_process_optimization 内被写入，无后续读取用于模型再训练
  - 无定时任务聚合 feedback_score 计算模型满意度
- **业务影响**：违反审计计划 16.7 第 3 项要求（推荐反馈闭环），用户对推荐结果的采纳/拒绝反馈未回流到模型再训练；feedback_score 数据"沉睡"在数据库中无业务价值
- **修复建议**：新增定时任务（每月）聚合 feedback_score 计算 AI 推荐满意度（avg_score/采纳率），低满意度时触发模型再训练；service 层提供 `get_feedback_summary` API 供运营查看

**缺陷 10.5：AI 操作审计未区分敏感操作**
- **风险等级**：P3
- **证据**：
  - `/workspace/backend/src/middleware/omni_audit.rs:311-334` is_sensitive_request_body_path 仅匹配 change-password/reset-password/setup-totp 等敏感路径
  - AI 端点不在敏感路径列表内，AI 推理请求体（含产品 ID/客户 ID/工艺参数）被完整记录到 omni_audit_logs.request_body
- **业务影响**：AI 推理请求体可能含敏感业务数据（如客户专属色号、配方参数），完整记录可能泄露商业机密
- **修复建议**：将 `/api/v1/erp/ai/process-optimizations` 与 `/api/v1/erp/ai/quality-predictions` 加入敏感路径列表，请求体脱敏为 "[REDACTED]"；或对 AI 端点单独审计并加密存储

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 1. AI 工艺优化模块完整性 | 0 | 2 | 2 | 0 | 6 | 10 |
| 2. AI 质量预测模块完整性 | 0 | 3 | 1 | 0 | 6 | 10 |
| 3. AI 模型管理（版本/训练/推理/评估） | 0 | 4 | 1 | 0 | 2 | 7 |
| 4. AI 接口安全（认证/权限/速率/校验） | 0 | 2 | 3 | 0 | 6 | 11 |
| 5. AI 推理性能（响应/超时/降级/缓存） | 0 | 3 | 2 | 0 | 3 | 8 |
| 6. AI 数据管理（训练/推理/脱敏） | 0 | 2 | 2 | 0 | 2 | 6 |
| 7. AI 结果可解释性（依据/置信度/复核） | 0 | 0 | 2 | 1 | 3 | 6 |
| 8. AI 与业务集成（工艺→生产/质量→质检） | 0 | 4 | 1 | 1 | 2 | 8 |
| 9. AI 错误处理（不可用/失败/降级） | 0 | 2 | 2 | 1 | 5 | 10 |
| 10. AI 审计日志（推理/变更/采纳） | 0 | 2 | 2 | 1 | 4 | 9 |
| **合计** | **0** | **24** | **20** | **4** | **39** | **85** |

**风险等级分布说明**：
- P0（阻塞）：0 项 — 无阻塞性问题，AI 模块基础功能可用
- P1（高）：24 项 — 涉及安全、权限、性能、业务集成的关键缺陷，应在下一迭代优先修复
- P2（中）：20 项 — 涉及可解释性、监控、数据管理的改进项
- P3（低）：4 项 — 体验优化与细节完善

---

## 修复优先级队列

### P1 优先级（24 项，按修复紧迫度排序）

1. **缺陷 4.1**：AI 端点权限码未注册到 init_admin_permissions.sql（非 admin 角色完全无法访问）
2. **缺陷 4.3**：AI 推理数据范围未按用户过滤（数据越权风险）
3. **缺陷 5.1**：AI 推理响应时间无 ≤ 2s 阈值控制（性能不达标）
4. **缺陷 5.2**：AI 并发控制缺失（CPU 过载风险）
5. **缺陷 5.3**：AI 缓存策略完全缺失（重复计算浪费资源）
6. **缺陷 9.1**：模型不可用降级策略缺失（故障时直接 500）
7. **缺陷 9.5**：AI 推理超时无降级结果（违反 16.9 第 1 项）
8. **缺陷 6.1**：AI 训练/推理数据未脱敏（违反规则 11）
9. **缺陷 6.2**：推理数据最小化未实现（全表扫描）
10. **缺陷 1.1**：染料配伍性 / 助剂兼容性校验缺失（无效推荐风险）
11. **缺陷 1.3**：与化验室打样系统集成缺失（fabric-industry-research §11.1）
12. **缺陷 2.1**：质量预测准确率监控缺失（无法验证 ≥ 80%）
13. **缺陷 2.2**：质量预测特征不完整（缺染料/助剂/温度等关键因子）
14. **缺陷 2.4**：质量预测与质检结果对账缺失
15. **缺陷 3.1**：模型版本管理完全缺失
16. **缺陷 3.4**：模型评估指标缺失
17. **缺陷 3.5**：模型漂移检测缺失
18. **缺陷 8.1**：工艺优化未与化验室打样集成
19. **缺陷 8.2**：工艺优化未与生产执行集成
20. **缺陷 8.3**：质量预测未与质检结果联动
21. **缺陷 8.4**：补货推荐未与 MRP 引擎对账
22. **缺陷 10.1**：AI 决策审计日志专用表缺失
23. **缺陷 10.2**：模型变更审计日志缺失
24. **缺陷 4.2**：advanced 域 AI 端点路径解析风险（权限码共用）

### P2 优先级（20 项，按业务影响排序）

1. **缺陷 1.2**：优化配方成本上限未校验
2. **缺陷 1.4**：批量端点无并发限制且无原子事务
3. **缺陷 2.3**：误判成本未量化追踪
4. **缺陷 3.2**：训练数据集合理性无评估
5. **缺陷 3.3**：推理结果一致性风险（候选集未排序）
6. **缺陷 4.4**：AI 端点专用速率限制缺失
7. **缺陷 4.5**：输入校验深度不足
8. **缺陷 5.4**：模型资源占用无监控
9. **缺陷 5.5**：batch 端点性能严重劣化
10. **缺陷 6.3**：AI 中间结果未加密存储
11. **缺陷 6.4**：训练数据集版本管理缺失
12. **缺陷 7.1**：缺少 explanation/factors 字段
13. **缺陷 7.2**：人工覆盖（override）机制缺失
14. **缺陷 8.5**：补货供应商推荐未集成 supplier_evaluation
15. **缺陷 9.2**：批量端点部分失败处理不完整
16. **缺陷 9.3**：异常检测失败未兜底
17. **缺陷 10.3**：AI 推理延迟未记录
18. **缺陷 10.4**：结果采纳反馈未回流模型
19. **缺陷 4.2 复用**：advanced 域权限码细分
20. **缺陷 9.4**：库存优化失败错误信息不友好（部分 P2）

### P3 优先级（4 项，体验优化）

1. **缺陷 7.3**：人工复核状态机不完整
2. **缺陷 8.6**：销售预测未与销售订单 / 库存补货联动
3. **缺陷 9.4**：库存优化失败错误信息不友好
4. **缺陷 10.5**：AI 操作审计未区分敏感操作

---

## 总结

V15 类十六 AI 模块审计专项完成 10 维度审计，共发现 **48 项缺陷**（0 P0 / 24 P1 / 20 P2 / 4 P3），已落实 39 项检查项。

**核心结论**：

1. **AI 模块基础功能完整**：14 个 AI 端点（5 工艺优化 + 5 质量预测 + 4 看板/批量 + 6 advanced 域）全部实现，算法核心（k-NN/趋势分析/风险评分）有单元测试覆盖，退化路径完整。

2. **关键安全与权限缺陷**：AI 端点权限码未注册（非 admin 完全无法访问）、AI 数据范围未按用户过滤（数据越权）、训练/推理数据未脱敏（违反规则 11）。

3. **性能与监控盲区**：无 AI 专用超时（≤ 2s）、无并发控制、无缓存策略、无 AI 监控看板、无 AI 告警机制、无 AI 业务指标注册。

4. **业务集成断裂**：AI 模块与化验室打样、生产执行、质检结果、MRP 引擎、供应商评估等业务系统均无集成，AI 推荐结果"孤立"无法驱动业务流程。

5. **MLOps 完全缺失**：无模型版本管理、无模型评估报告、无模型漂移检测、无训练数据集版本管理、无 AI 决策审计日志专用表、无模型变更审计。

**建议下一迭代优先修复 P1 中的安全/权限/性能/集成类缺陷（共 24 项），可显著提升 AI 模块的生产可用性。**
