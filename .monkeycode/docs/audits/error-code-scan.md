# 错误码与错误信息规范性扫描报告

- 扫描日期：2026-08-22
- 扫描范围：`backend/src/`
- 关键文件：`backend/src/utils/error.rs`、`backend/src/utils/messages.rs`
- 扫描方法：静态 `grep` + 人工抽检

## 1. AppError variant 清单与错误码体系

`AppError` 定义于 `backend/src/utils/error.rs:12-26`，共 **10 个 variant**：

| Variant | 携带字段 | HTTP 状态码 | error_code（对外稳定码） | public_message 脱敏文案 |
|---|---|---|---|---|
| `DatabaseError(String)` | message | 500 INTERNAL_SERVER_ERROR | `DATABASE_ERROR` | 数据库错误 |
| `ValidationError(String)` | message | 400 BAD_REQUEST | `VALIDATION_ERROR` | 请求参数验证失败 |
| `NotFound(String)` | message | 404 NOT_FOUND | `NOT_FOUND` | 资源未找到 |
| `BusinessError(String)` | message | 400 BAD_REQUEST | `BUSINESS_ERROR` | 业务处理失败 |
| `Unauthorized(String)` | message | 401 UNAUTHORIZED | `UNAUTHORIZED` | 未授权 |
| `InternalError(String)` | message | 500 INTERNAL_SERVER_ERROR | `INTERNAL_ERROR` | 服务器内部错误 |
| `BadRequest(String)` | message | 400 BAD_REQUEST | `BAD_REQUEST` | 请求参数错误 |
| `PermissionDenied(String)` | message | 403 FORBIDDEN | `FORBIDDEN` | 无权限 |
| `NotImplemented(String)` | message | 501 NOT_IMPLEMENTED | `NOT_IMPLEMENTED` | 功能未实现 |
| `TooManyRequests { retry_after, message }` | retry_after + message | 429 TOO_MANY_REQUESTS | `TOO_MANY_REQUESTS` | 请求过于频繁，请稍后重试 |

### 错误码体系评估

- 已建立**统一错误码体系**：`error_code()` 返回稳定字符串枚举（`error.rs:406-420`），对外通过 `ErrorResponse.code` 暴露。
- 已建立**脱敏机制**：`public_message()`（`error.rs:423-436`）按 variant 返回固定脱敏文案，HTTP 响应不再泄露 SQL/路径/堆栈；原始 message 仅进 tracing 日志。
- HTTP 状态码映射在 `error_status_and_type()`（`error.rs:119-132`）中集中维护，无散落映射。
- 日志分类（severity/action_required）在 `error_severity_and_action()`（`error.rs:135-148`）中集中维护。

## 2. 调用频次统计

### 2.1 辅助构造函数调用频次（项目主用方式）

| 构造函数 | 调用次数 |
|---|---|
| `AppError::not_found` | 741 |
| `AppError::business` | 701 |
| `AppError::validation` | 455 |
| `AppError::internal` | 415 |
| `AppError::bad_request` | 214 |
| `AppError::permission_denied` | 125 |
| `AppError::database` | 122 |
| `AppError::unauthorized` | 21 |
| `AppError::too_many_requests` | 5 |
| `AppError::not_implemented` | 0（无辅助调用，仅 variant 直接构造 7 次） |
| **合计** | **2799** |

### 2.2 variant 直接构造调用频次（少量场景）

| Variant | 直接构造次数 |
|---|---|
| `TooManyRequests` | 14 |
| `BadRequest` | 12 |
| `NotFound` | 11 |
| `DatabaseError` | 10 |
| `ValidationError` | 9 |
| `BusinessError` | 9 |
| `Unauthorized` | 8 |
| `PermissionDenied` | 8 |
| `InternalError` | 8 |
| `NotImplemented` | 7 |
| **合计** | **96** |

观察：项目以辅助构造函数（`not_found`/`business` 等）为主入口，variant 直接构造集中在 `error.rs` 内部的 `From` 实现与少量 handler。

## 3. 硬编码消息数 vs 常量化消息数

### 3.1 消息来源分类统计

| 消息来源 | 数量 | 占比 | 规范性 |
|---|---|---|---|
| `format!` 动态拼接（含 `{}` 占位符） | 1002 | — | 不规范（拼接底层错误细节） |
| `format!` 动态拼接（无占位符，纯模板） | 541 | — | 半规范 |
| **`format!` 小计** | **1543** | — | **硬编码** |
| 纯静态字符串字面量（非 `err_msg::`） | 801 | — | **硬编码** |
| `err_msg::` 常量引用 | 4 | <0.1% | 规范 |
| **硬编码消息总计** | **2344** | — | — |

> 注：`err_msg::` 模块共定义 **72 个常量**（`messages.rs`），但仅被 `AppError` 构造直接引用 **4 次**；绝大多数常量服务于 `Display` 前缀、`public_message` 脱敏、`log_meta`、`classify_db_*` 等内部逻辑，handler/service 层几乎未采用常量化错误消息。

### 3.2 常量化程度评估

- `messages.rs` 常量体系设计完善（72 常量覆盖 Display 前缀、脱敏文案、日志标签、修复建议、DB 分类、action_required 等）。
- 但 handler/service 层 2799 次构造调用中，**仅 4 次**引用 `err_msg::` 常量，常量化覆盖率 **<0.1%**。
- `biz_msg` 模块（5 个成功消息常量）主要服务于 `ApiResponse::success_with_message`，不参与错误消息。

## 4. 不规范的错误消息示例

### 4.1 英文消息与项目中文风格不一致（最多 10 个）

项目错误消息以中文为主，以下英文消息与整体风格不一致：

| # | 文件:行 | 消息 | 问题 |
|---|---|---|---|
| 1 | `services/fund_management_service.rs:536` | `"From account not found"` | 英文，应为中文 |
| 2 | `services/fund_management_service.rs:539` | `"Insufficient balance"` | 英文，应为中文 |
| 3 | `services/fund_management_service.rs:557` | `"To account not found"` | 英文，应为中文 |
| 4 | `services/purchase_return_service.rs:910` | `"Return not found"` | 英文，应为中文 |
| 5 | `services/audit_log_service.rs:168` | `"Entity has no primary key"` | 英文，应为中文 |
| 6 | `services/report/job.rs:41` | `"cron 候选日期时分秒非法"` | 中英混用 |
| 7 | `services/wage_ops/rate.rs:242` | `"A 级等级系数必须在 [0, 1] 范围内"` | 半英变量名 |
| 8 | `services/ai_extend_service.rs:374` | `"feedback_score 必须在 1-5 范围内"` | 字段英文名混入中文 |
| 9 | `services/finance_report_service.rs:813` | `"period 格式必须为 YYYY-MM"` | 字段英文名混入中文 |
| 10 | `services/report/job.rs:113` | `"cron 候选时间时分秒非法"` | 中英混用 |

### 4.2 同义不同文案（重复定义）

同一语义的错误在多处使用不同文案，缺乏统一常量：

| 语义 | 文案变体 | 出现情况 |
|---|---|---|
| 客户 ID 无效 | `"客户 ID 无效"` vs `"客户ID无效"` | 2 种写法 |
| 年月参数无效 | `"无效的年月"` vs `"无效的年月参数"` | 2 种写法 |
| 资源不存在 | `"X不存在"` 模式散落 25+ 处（BOM/订单/客户/发票/对账单...） | 每处独立硬编码 |
| 状态不允许操作 | `"当前状态不允许此操作"` / `"状态不允许审批"` / `"已关闭状态不可取消"` 等 20+ 种写法 | 无统一模板 |
| 序列化失败 | `"序列化失败"` 出现 9 次，另有 `"销售价格序列化失败：期望 JSON 对象"` 等 | 无常量 |

### 4.3 format! 拼接底层错误细节（潜在信息泄露风险）

以下将底层错误 `e` 直接拼入 `InternalError` 消息。虽 `public_message` 已脱敏不会进 HTTP 响应，但日志中仍含底层细节，且文案不规范：

| # | 文件:行 | 消息模板 |
|---|---|---|
| 1 | `routes/search_api.rs:90` | `format!("搜索销售订单失败: {}", e)` |
| 2 | `routes/search_api.rs:116` | `format!("搜索客户失败: {}", e)` |
| 3 | `routes/search_api.rs:141` | `format!("搜索产品失败: {}", e)` |
| 4 | `utils/number_generator.rs:48` | `format!("开始事务失败: {:?}", e)` |
| 5 | `utils/number_generator.rs:66` | `format!("提交事务失败: {:?}", e)` |
| 6 | `utils/xlsx_export.rs:75` | `format!("xlsx 工作表名称错误: {}", e)` |
| 7 | `utils/xlsx_export.rs:85` | `format!("xlsx 冻结首行失败: {}", e)` |
| 8 | `utils/import_export.rs:109` | `format!("第 {} 行解析失败: {}", row_idx + 2, e)` |
| 9 | `utils/xlsx_export.rs:91` | `format!("xlsx 保存失败: {}", e)` |
| 10 | `utils/export_concurrency.rs:31` | `AppError::too_many_requests(format!(...))` |

### 4.4 硬编码消息集中的文件（Top 15）

| 文件 | 直接字面量消息数 |
|---|---|
| `services/fund_management_service.rs` | 21 |
| `services/ar_invoice_service.rs` | 18 |
| `services/role_change_approval_service.rs` | 14 |
| `services/wage_ops/rate.rs` | 13 |
| `services/fabric_inspection_service.rs` | 13 |
| `services/finance_report_service.rs` | 12 |
| `services/chemical_ops/master.rs` | 12 |
| `handlers/auth_handler_misc.rs` | 12 |
| `handlers/user_handler.rs` | 11 |
| `services/inv/inventory_move.rs` | 10 |
| `services/business_mode_service.rs` | 10 |
| `services/report_subscription_service.rs` | 9 |
| `services/budget_management_service.rs` | 9 |
| `services/account_subject_service.rs` | 9 |
| `services/totp_service.rs` | 8 |

## 5. 建议：应提取为错误码常量的内容

### 5.1 优先提取的高频重复文案

1. **"X 不存在"模板**：建议在 `err_msg` 增加 `not_found_with(name)` 辅助函数或 `NOT_FOUND_PREFIX` 模板，统一 25+ 处 `"BOM不存在"`/`"订单不存在"` 等写法。
2. **"当前状态不允许此操作"**：出现 6+ 次，另有 20+ 种状态校验文案变体。建议提取 `STATE_NOT_ALLOWED` 常量 + `state_not_allowed(entity, action)` 辅助函数。
3. **"无效的年月"/"无效的日期参数"**：校验类高频消息，建议提取 `INVALID_PERIOD`、`INVALID_DATE_PARAM` 常量。
4. **"序列化失败"**：出现 9 次，建议提取 `SERIALIZE_FAILED` 常量，配合 `format!` 拼接实体名。
5. **"金额必须大于零"/"发出数量不能为负"/"单位成本不能为负"**：数值校验类，建议提取 `AMOUNT_MUST_POSITIVE`、`QTY_CANNOT_NEGATIVE`、`COST_CANNOT_NEGATIVE` 常量。
6. **"失效日期必须晚于生效日期"**：出现 4 次，建议提取 `EXPIRE_AFTER_EFFECTIVE` 常量。

### 5.2 英文消息中文化

`fund_management_service.rs`、`purchase_return_service.rs`、`audit_log_service.rs` 中的 5 条英文消息应统一为中文，或确认是否为国际化场景预留。

### 5.3 format! 拼接底层错误的重构

`routes/search_api.rs`、`utils/number_generator.rs`、`utils/xlsx_export.rs`、`utils/import_export.rs` 中 1002 处 `format!` 含 `{}` 占位符，拼接底层 `e`。建议：
- 对内部错误（`InternalError`/`DatabaseError`）：保留 `format!` 但统一模板为 `format!("{}：{e}", err_msg::X_FAIL)`，前缀走常量。
- 对面向用户错误（`ValidationError`/`BusinessError`）：底层 `e` 不应拼入，改用稳定常量文案。

### 5.4 扩展 `err_msg` 常量覆盖面

当前 `err_msg` 72 常量主要服务 `error.rs` 内部逻辑。建议新增 handler/service 层常用业务消息常量子模块（如 `err_msg::biz`），覆盖：
- 资源不存在模板（`not_found(entity)`）
- 状态校验模板（`state_not_allowed(entity, action)`）
- 数值校验（`must_positive`、`cannot_negative`）
- 日期校验（`invalid_period`、`invalid_date`）

目标：将 handler/service 层 2344 处硬编码逐步迁移至常量引用，提升文案一致性与可维护性。

## 6. 扫描结论汇总

| 指标 | 数值 |
|---|---|
| AppError variant 数 | **10** |
| 统一错误码（error_code）数 | 10 |
| err_msg 常量数 | 72 |
| 辅助构造函数调用总数 | 2799 |
| variant 直接构造总数 | 96 |
| **硬编码消息总数** | **2344**（format! 1543 + 静态字面量 801） |
| 其中 format! 含底层错误拼接 | 1002 |
| 常量化消息引用数 | 4 |
| 英文/中英混用消息（抽样） | 10+ |
| 同义不同文案重复组 | 5+ |

**核心判断**：错误码体系（variant + error_code + public_message 脱敏）设计规范、分层清晰；但 handler/service 层错误消息常量化覆盖率 <0.1%，2344 处硬编码消息存在文案不一致、中英混用、底层细节拼接等问题，建议按 §5 优先级分批提取常量。
