# v7 全项目严格复审报告

**审计基线**: main HEAD `1667134`（批次 24/25/26 已修复 P0+P1 共 70 项）
**审计日期**: 2026-06-29
**审计方式**: 5 并行子代理覆盖 16 维度，只读静态分析
**审计结论**: **不通过**，发现 P0=24 / P1=32 / P2=29 项新问题

---

## 一、已修复项回归验证结论

| 批次 | 修复范围 | 回归状态 |
|------|---------|---------|
| 批次 24 | v6 低难度高收益 P0 18 项 | 部分通过（vitest 升级遗漏 pnpm-lock.yaml；UserInfo 仅补 2/6 字段）|
| 批次 25 | 状态机 lock_exclusive P0 25 项 | 全部通过 |
| 批次 26 | 状态机 lock_exclusive P1 27 项 | 全部通过 |

---

## 二、按修复批次规划的问题清单

### 批次 27：状态机 P0 漏修 + P1 事务边界泄漏（13 项）

#### P0 状态机漏修（7 项）

| # | 文件 | 方法 | 类型 |
|---|------|------|------|
| 1 | services/ar/vfy.rs:568-599 | customer_confirm | 完全无 txn 无 lock |
| 2 | services/ar/vfy.rs:602-630 | customer_dispute | 完全无 txn 无 lock |
| 3 | services/ap_reconciliation_service.rs:113-153 | confirm_reconciliation | 有 txn 漏 lock |
| 4 | services/ap_reconciliation_service.rs:156-195 | dispute | 有 txn 漏 lock |
| 5 | services/color_card_crud_service.rs:135-170 | update | 完全无 txn 无 lock |
| 6 | services/color_card_crud_service.rs:172-190 | archive | 完全无 txn 无 lock |
| 7 | services/color_card_crud_service.rs:192-200 | mark_lost | 完全无 txn 无 lock |

**修复模式**：参照 `ar_invoice_service::mark_as_paid` + `ar/recon.rs::confirm/dispute/close`（批次 25 已建立模式）

#### P1 事务边界泄漏（6 项）

| # | 文件 | 方法 | 修复 |
|---|------|------|------|
| 8 | services/purchase_contract_service.rs:169-174 | execute | `&*self.db` → `&txn` |
| 9 | services/ar_collection_service.rs:50-56 | create_collection | 单号生成器移入 txn |
| 10 | services/ar/vfy.rs:82 | auto_match | 单号生成器移入 txn |
| 11 | services/ar/vfy.rs:458 | generate_reconciliation | 单号生成器移入 txn |
| 12 | services/sales_return_service.rs:273-276 | submit_return | `&*self.db` → `&txn` |
| 13 | services/purchase_return_service.rs:191-194 | approve_return | `&*self.db` → `&txn` |

---

### 批次 28：v7 安全敏感信息 P0（6 项）

| # | 文件 | 问题 | 修复 |
|---|------|------|------|
| 1 | frontend/scripts/comprehensive_test.cjs:8-9 | 硬编码生产 IP + admin/admin123 凭据 | 改环境变量 + git 历史清洗 |
| 2 | scripts/api-crud-test.sh:3 / scripts/fix-server-config.sh:15 / frontend/scripts/full_test.js / 生产服务器日志/README.md:11 / backend/config.yaml:60-62 | 生产 IP `111.230.99.236` 多处硬编码 | 统一 `${PROD_IP:?...}` fail-secure |
| 3 | backend/config.yaml | 被跟踪入 git 且含生产配置 | `git rm --cached` + 改用 config.yaml.example |
| 4 | backend/config.test.yaml | 被跟踪入 git 含弱密码 `bingxi123` | 加入 .gitignore + 提供 .example |
| 5 | 快速部署/install.sh:217 | 健康检查端点回归（应为 /health 而非 /api/v1/erp/health） | 改为 /health |
| 6 | scripts/fix-server-config.sh | 未对齐批次 24（保留 PROD_IP 默认值 + sshpass + StrictHostKeyChecking=no + /api/v1/erp/health/） | 按 deploy-latest.sh 批次 24 模式重写 |

---

### 批次 29：前后端类型契约 P0（8 项）

| # | 问题 | 文件 |
|---|------|------|
| 1 | pnpm-lock.yaml 残留 vitest 2.1.9（CVSS 9.8） | frontend/pnpm-lock.yaml |
| 2 | RefreshTokenResponse.token 仍回传 access_token（与批次 24 矛盾） | backend/src/handlers/auth_handler_misc.rs:24-29 |
| 3 | LoginRequest TOTP 字段名 totp_code vs totp_token 不一致 | 前端 types/api.ts:6 + 后端 auth_handler.rs:41 |
| 4 | UserInfo 缺 is_totp_enabled（前端 2FA 检测恒 false） | backend/src/handlers/auth_handler.rs:60-72 |
| 5 | UserInfo 缺 real_name/phone/avatar/department_id/department_name | 同上 |
| 6 | auth_flow.rs 集成测试形同虚设 | backend/tests/integration/auth_flow.rs |
| 7 | Login.test.ts 测的是 Mock 组件 | frontend/tests/unit/Login.test.ts |
| 8 | E2E 仅 page.goto 导航无业务流程 | frontend/e2e/color-card.spec.ts |

---

### 批次 30：分页输入验证（涉及 30+ 文件）

- P1-3：22+ service 用 `fetch_page(page - 1)` 而非 `saturating_sub(1)`
- P1-7：25+ handler page_size 无上限 `.clamp(1, 100)`
- P1-4：omni_audit_handler.rs:128 total = items.len()（应为 COUNT(*)）
- P1-5：inventory_batch_handler.rs:93-94 同上

---

### 批次 31：N+1 查询 + DTO 输入验证

- P1-1：ap_payment_request_service.rs:50-72 N+1 查询
- P1-2：color_price_batch_service.rs:63-114 3N+1 查询
- P1-6：35+ DTO 缺 #[validate] 注解

---

### 批次 32：i18n + OpenAPI 文档（高工作量）

- P0-1：i18n 形同虚设，仅 1 个页面接入（30160 处硬编码中文）
- P0-2：ElMessage 硬编码中文 291 处
- P0-3：OpenAPI 覆盖率 2.6%（20/771 端点）

---

## 三、其他 P1/P2 问题（后续迭代）

### P1 重要问题（共 32 项，分布）

- 维度 9: P1-1 optional 不对齐 / P1-2 RefreshTokenResponse 缺 ToSchema / P1-3 Decimal 序列化字符串 vs number
- 维度 10: 无新发现 P1
- 维度 11: P1-4 状态机无并发测试 / P1-5 CSRF 无并发测试 / P1-6 测试密钥弱熵源 / P1-7 api_routes 字符串匹配
- 维度 12: P1-1 locales key 漂移 / P1-2 后端 AppError 硬编码中文 163 处
- 维度 13: P1-3 缓存层仅接入 2/150 服务 / P1-4 Vite 无分包策略 / P1-5 虚拟列表仅 4 页
- 维度 15: P1-6 OpenAPI 标题不一致

### P2 中等优先级（共 29 项，详见各子代理报告）

主要包括：
- 跨服务级联无补偿机制（7 项，需 Saga 模式）
- 长事务死锁风险（2 项）
- 测试相关（多处）
- Cargo.toml 依赖版本宽松约束
- Dockerfile 用户权限
- CI 权限最小化

---

## 四、修复优先级建议

按以下顺序修复，每批次完整流程：修复分支 → push → CI 全绿 → 合并 → 删除分支 → 下一批

1. **批次 27**（状态机 + 事务边界，13 项 P0+P1）：技术债务最高，但修复模式已建立，CI 风险低
2. **批次 28**（敏感信息 6 项 P0）：安全风险最高，但部分需 git 历史清洗需谨慎
3. **批次 29**（类型契约 8 项 P0）：影响功能正确性，前后端协同
4. **批次 30**（分页输入验证，30+ 文件）：修复模式统一，工作量大但机械
5. **批次 31**（N+1 + DTO 验证）：性能与安全
6. **批次 32**（i18n + OpenAPI）：高工作量，长期迭代

**目标**：所有 P0 修复完成后再进行 v8 复审，循环直到无 P0 问题。
