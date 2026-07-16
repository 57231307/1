# V15 测试体系审计报告（类六·批次 06）

- **审计子代理**：V15 审计子代理（类六 测试体系审计类）
- **审计范围**：7 维度（单元测试覆盖率 / 集成测试执行率 / E2E 测试 / mock 数据 fixtures / 测试质量 / 性能基准测试 / 覆盖率报告）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md`（第 953-1060 行 类六测试体系审计类）
  - `/workspace/.monkeycode/docs/audits/2026-06-28-strict-reaudit-v4.md`（v4 维度 12 P0 28 项基线）
  - `/workspace/.monkeycode/docs/audits/2026-07-08-batch190-e2e-report.md`（批次 190 E2E 报告）
  - `/workspace/.github/workflows/ci-cd.yml`（主 CI 工作流）
  - `/workspace/.github/workflows/e2e-batch.yml`（E2E 工作流）
  - `/workspace/frontend/playwright.config.ts`
  - `/workspace/frontend/e2e/fixtures/auth.ts`
  - `/workspace/frontend/tests/unit/*.test.ts`
  - `/workspace/backend/Cargo.toml`
  - `/workspace/backend/src/services/*.rs`（含 `#[cfg(test)]` 的 service 文件）
  - `/workspace/backend/tests/*.rs`（41 个集成测试文件）
  - `/workspace/backend/.test-baseline.txt`（测试 baseline）
- **审计方法**：Grep 检索关键测试模式（`#[test]` / `#[tokio::test]` / `mockBusinessApi` / `fixtures` / `criterion` / `tarpaulin`）+ Glob 查找测试文件 + Read 关键测试文件 + 对照 v4 维度 12 P0 修复保持核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码；测试体系缺陷按 P0/P1/P2/P3 风险分级

---

## 维度 1：单元测试覆盖率

### 检查方法
1. `Grep "#\[tokio::test\]|#\[test\]"` 在 `/workspace/backend/src/services` 共找到 79 个文件含测试代码
2. `wc -l` 统计 13 个核心 service 总行数 + `grep -c` 统计测试数
3. `Read` 关键测试函数命名（dye_batch_state_machine_service.rs / inventory_stock_service.rs / voucher_service.rs）
4. 对照审计计划 6.1 检查要点：核心 service 100% 覆盖 + 测试函数名清晰描述场景

### 发现

#### ✅ 已落实的项

1. **测试函数中文命名规范已部分落实**（v4 维度 12 P0 修复保持）：
   - `/workspace/backend/src/services/voucher_service.rs:1244-1807`：33 个测试，全部使用中文命名（如 `测试_凭证状态常量_值正确性`、`测试_借贷平衡校验_借方等于贷方通过`、`测试_状态校验_仅草稿可更新`）
   - `/workspace/backend/src/services/inventory_stock_service.rs:487-612`：6 个测试，中文命名（如 `测试_calculate_quantity_kg_克重和幅宽齐全走转换器`）
   - `/workspace/backend/src/services/wage_service.rs:1084-1455`：21 个测试，中文命名（如 `测试_合格率计算_正常情况`、`测试_工资计算_计件_A级全额`）
   - `/workspace/backend/src/services/energy_service.rs:1665-1775+`：18 个测试，中文命名（如 `测试计算消耗量_正常`、`测试超基准判断_超出阈值`）
   - `/workspace/backend/src/services/dye_batch_state_machine_service.rs:1191-1450+`：31 个测试，中文命名（如 `测试校验生命周期状态_合法`、`测试状态流转_合法流转`）

2. **测试函数总数较 v4 基线大幅提升**：
   - `Grep "#\[tokio::test\]|#\[test\]"` 在 `backend/src/services` 共 874 个匹配（v4 基线 586 个，增长 49%）
   - 79 个 service 文件含测试代码（v4 基线 46 个）

3. **核心 service 测试覆盖已补全**（v4 P0 修复保持）：
   - `voucher_service.rs`：33 测试 / 1847 行（v4 基线为 0 测试）
   - `dye_batch_state_machine_service.rs`：31 测试 / 1510 行（v14 批次 432 新增）
   - `dye_recipe_service.rs`：22 测试 / 586 行（v14 批次 432 新增）
   - `outsourcing_service.rs`：21 测试 / 1782 行
   - `wage_service.rs`：21 测试 / 1507 行
   - `energy_service.rs`：18 测试 / 1800 行
   - `chemical_service.rs`：18 测试 / 1676 行
   - `business_mode_service.rs`：28 测试 / 1674 行
   - `production_order_service.rs`：55 测试

4. **测试夹具宏使用规范**（`voucher_service.rs:1620-1647`）：
   ```rust
   fn 测试_decs_夹具宏可用性() { ... }
   fn 测试_ymd_夹具宏可用性() { ... }
   ```
   自定义 `decs!()` 和 `ymd!()` 宏，统一 Decimal 和日期构造。

#### ❌ 缺陷项 1：核心 service 100% 覆盖未达成（quotation_service / purchase_receipt_service 零测试）

**风险等级：P0**（核心业务 service 零单元测试）

**证据**：
- `/workspace/backend/src/services/quotation_service.rs`：549 行，14 个 `pub async fn` 方法（`create_draft` / `list` / `get_by_id` / `update` / `cancel` 等），**0 个 `#[test]` / `#[tokio::test]`**
  ```bash
  $ grep -c '#\[tokio::test\]\|#\[test\]' src/services/quotation_service.rs
  0
  ```
- `/workspace/backend/src/services/purchase_receipt_service.rs`：677 行，14 个 `pub async fn` 方法（`create_receipt` / `update_receipt` / `delete_receipt` / `confirm_receipt` / `add_receipt_item` 等），**0 个 `#[test]` / `#[tokio::test]`**
- `/workspace/backend/src/services/purchase_receipt_private.rs`：243 行，0 测试
- `/workspace/backend/src/services/purchase_receipt_dto.rs`：137 行，0 测试
- `/workspace/backend/src/services/quotation_pricing_service.rs`：228 行，0 测试

**业务影响**：
- v4 维度 12 P0 第 1 项要求"voucher_service / inventory_stock_service / quotation_service / purchase_receipt_service / sales_order_service"100% 覆盖未达成
- 报价单与采购收货是 ERP 核心 CRUD 流程，零单元测试意味着任何状态机/校验逻辑变更无回归保护
- 审计计划 6.1 检查要点 2 明确要求这些核心 service 100% 覆盖

#### ❌ 缺陷项 2：inventory_stock_service 测试覆盖率严重不足

**风险等级：P1**（核心库存 service 测试覆盖核心方法比例低）

**证据**：
- `/workspace/backend/src/services/inventory_stock_service.rs`：613 行，仅 6 个测试
- 测试分布：
  - 4 个测试 `calculate_quantity_kg`（纯函数工具方法）
  - 1 个测试 `库存硬编码状态字符串常量值正确性`（仅断言常量字符串）
  - 1 个测试 `服务实例化_SQLite内存数据库`（仅验证 `new()` 不触发 DB）
- **核心 CRUD 方法零覆盖**：`create_stock` / `list` / `update` / `delete` / `adjust_quantity` / `transfer_stock` 等业务方法无测试

**业务影响**：
- 库存是面料 ERP 最核心模块，金额/数量计算/双单位换算等关键逻辑缺少回归保护
- 与 v4 维度 12 P0-021 "库存调整 quantity_kg 计算逻辑修正" 相关，修复后无回归测试保护

#### ❌ 缺陷项 3：染整相关 service 测试函数名不符合中文规范

**风险等级：P3**（测试命名规范违反）

**证据**：
- `/workspace/backend/src/services/dye_recipe_service.rs:414-500+`：22 个测试使用英文命名（`test_generate_recipe_no_auto` / `test_status_transition_draft_to_approved`）
- `/workspace/backend/src/services/lab_dip_service.rs:1048-1110`：7 个测试使用英文命名（`test_label_from_seq` / `test_request_status_transition_valid`）
- `/workspace/backend/src/services/production_recipe_service.rs:815-1095`：12 个测试使用英文命名（`test_generate_recipe_no` / `test_parse_liquor_ratio`）
- `/workspace/backend/src/services/flow_card_service.rs:1153-1263`：7 个测试使用英文命名（`test_generate_card_no_format` / `test_validate_status_transition_normal`）
- `/workspace/backend/src/services/fabric_inspection_service.rs:779-907`：10 个测试使用英文命名（`test_calculate_four_point_points_normal`）
- `/workspace/backend/src/services/quotation_approval_service.rs:435-469`：4 个测试使用英文命名
- `/workspace/backend/src/services/quotation_convert_service.rs:226-252`：2 个测试使用英文命名

**业务影响**：
- 违反项目规范"测试函数名应该清晰描述测试场景（中文描述测试目的）"
- 与项目规则第二章 §3.3"测试命名：使用中文描述测试目的"不一致
- voucher_service / inventory_stock_service / wage_service / energy_service / dye_batch_state_machine_service 已使用中文命名，证明规范可执行

#### ❌ 缺陷项 4：service 模块覆盖率未达 80% 目标

**风险等级：P1**（覆盖率目标未达成）

**证据**：
- v4 维度 12 P0 第 1 项要求"后端 service 模块覆盖率从 38% → 80%+"
- v4 报告第 476 行：v4 基线为"120 个 service 模块，含 `#[cfg(test)]` 仅 46 个，覆盖率约 38%"
- v15 审计：`Grep` 在 `backend/src/services` 找到 79 个文件含 `#[cfg(test)]`（约 65% service 文件含测试代码）
- 但"含 `#[cfg(test)]`" 不等于"覆盖率 80%"——测试数量与源代码行数比例不均：
  - 高覆盖：voucher_service (33/1847=1.8%)、dye_batch_state_machine_service (31/1510=2.1%)、dye_recipe_service (22/586=3.8%)
  - 低覆盖：inventory_stock_service (6/613=1.0%)、lab_dip_service (7/1118=0.6%)、flow_card_service (7/1271=0.6%)
  - 零覆盖：quotation_service / purchase_receipt_service / purchase_receipt_private / purchase_receipt_dto / quotation_pricing_service
- 项目无 cargo tarpaulin / cargo llvm-cov 集成（详见维度 7），无法量化覆盖率，仅能定性评估

### 修复建议
1. **P0 紧急**：为 `quotation_service.rs`（14 个方法）和 `purchase_receipt_service.rs`（14 个方法）补单元测试，参考 voucher_service 模式（mock DatabaseConnection + 测试 create/list/update/cancel 状态机校验）
2. **P1**：为 `inventory_stock_service.rs` 核心 CRUD 方法补集成测试（使用 SQLite 内存数据库 + sea-orm schema）
3. **P3**：将 `dye_recipe_service` / `lab_dip_service` / `production_recipe_service` / `flow_card_service` / `fabric_inspection_service` 等的测试函数名改为中文（保持向后兼容，逐批重命名）
4. **P1**：CI 集成 cargo tarpaulin（详见维度 7），量化覆盖率目标 80%+

---

## 维度 2：集成测试执行率

### 检查方法
1. `Read /workspace/.github/workflows/ci-cd.yml`（第 833-1050 行 ci-test-rust job）
2. `ls /workspace/backend/tests/`（41 个集成测试文件）
3. `Grep "production_order|purchase_receipt|sales_delivery|付款全流程|染整全流程|lab_dip|dye_batch"` 在 `backend/tests/`
4. 对照 v4 维度 12 P0-1 修复保持

### 发现

#### ✅ 已落实的项

1. **CI 不再 `--lib` 跳过集成测试**（v4 P0-1 修复保持）：
   - `/workspace/.github/workflows/ci-cd.yml:904`：
     ```yaml
     cargo test --jobs 1 -- --test-threads=1 2>&1 | tee reports/cargo-test-output.txt
     ```
   - 注释明确说明（第 843 行）："批次 21 修复：移除 --lib 跳过 47 个集成测试的问题"
   - 已配置 PostgreSQL service container + DATABASE_URL（第 844-863 行）

2. **集成测试文件数量增长**：
   - `/workspace/backend/tests/` 共 41 个集成测试文件（v4 基线为 47 个，部分伪测试已清理）
   - 测试函数总数 198 个（基于 `grep -c` 统计）

3. **PostgreSQL service container 配置完整**（`ci-cd.yml:844-857`）：
   ```yaml
   services:
     postgres:
       image: postgres:16-alpine
       env:
         POSTGRES_USER: bingxi
         POSTGRES_PASSWORD: bingxi_test
         POSTGRES_DB: bingxi_test
       ports:
         - 5432:5432
   ```

4. **测试 baseline 机制已建立**（`ci-cd.yml:920-950`）：
   - 自动建立失败测试 baseline
   - 仅"新增失败"阻塞 CI，历史失败渐进清理

#### ❌ 缺陷项 1：关键业务路径无真实集成测试

**风险等级：P0**（审计计划 6.2 检查要点 3 完全未达成）

**证据**：
- `Grep "production_order|purchase_receipt|sales_delivery|付款全流程|染整全流程|lab_dip|dye_batch|production_recipe|flow_card|fabric_inspection|wage|energy"` 在 `/workspace/backend/tests/`：
  - **生产订单全流程**：无匹配（仅 `test_generate_no_endpoints.rs` 测试单据号格式）
  - **采购收货全流程**：无匹配（仅 `test_generate_no_endpoints.rs` 测试 RK 前缀）
  - **销售发货全流程**：无匹配
  - **付款全流程**：无匹配
  - **染整全流程**：无匹配（V15 新增要求）
  - **打样全流程**：无匹配（V15 新增要求）
  - **大货处方全流程**：无匹配（V15 新增要求）

- 现有集成测试主要覆盖：
  - 报价单（`quotation_*_test.rs` 6 个文件，含 35 个测试，但多伪测试）
  - 色卡（`color_card_*_test.rs` 7 个文件，32 个测试）
  - 色价（`color_price_*_test.rs` 5 个文件，22 个测试）
  - 定制订单（`custom_order_*_test.rs` 5 个文件，18 个测试）
  - 其他工具/中间件测试（`test_cache.rs` / `test_capacity.rs` / `test_csrf_middleware.rs` 等）

**业务影响**：
- 7 项关键业务路径全部无真实端到端集成测试覆盖
- 任何业务流程变更（如采购收货确认后自动生成 AP 发票）无回归保护
- v4 维度 12 P0-1 修复仅解决了"`--lib` 跳过"问题，但集成测试内容仍以工具/中间件为主，缺少业务流程闭环测试

#### ❌ 缺陷项 2：CI baseline 机制可能掩盖编译失败

**风险等级：P0**（CI 有效性问题）

**证据**：
- `/workspace/backend/.test-baseline.txt`：**空文件**（0 字节）
- `/workspace/backend/tests/bi_analysis_test.rs`：16 个测试全部使用过时的 API：
  ```rust
  // 测试中（第 12-15 行）：
  assert!(BiAnalysisService::kpi_summary(0).await.is_err());  // 静态调用 + i32 参数
  // 但源码中（bi_analysis_service.rs:657）：
  pub async fn kpi_summary(&self) -> Result<KpiSummary, AppError>  // 实例方法 + 无参数
  ```
- 所有 16 个测试均存在相同问题：`BiAnalysisService::sales_by_customer(1, 2)` vs 源码 `pub async fn sales_by_customer(&self, limit: i64)`、`BiAnalysisService::slice(1, "customer", ...)` vs 源码 `pub async fn slice(&self, ...)`
- CI 工作流逻辑（`ci-cd.yml:1017-1020`）：
  ```bash
  if [ "$NEW_FAILED" -gt "0" ]; then
    exit 1
  fi
  exit 0
  ```
  仅"新增失败"阻塞 CI。如果 baseline 是空的且当前测试也是空跑（因编译失败无 "test ... FAILED" 行），NEW_FAILED = 0，CI 通过

**业务影响**：
- bi_analysis_test.rs 16 个测试自 v9 批次 130 重构 BiAnalysisService 后即与源码 API 脱节，但因 baseline 机制被掩盖
- 任何开发者修改 BiAnalysisService 时不会发现这些测试已损坏
- 真实的"集成测试 100% 执行率"未达成——这些测试实际从未运行

#### ❌ 缺陷项 3：集成测试断言不充分（多伪测试）

**风险等级：P1**（违反审计计划 6.2 检查要点 5）

**证据**：
- `/workspace/backend/tests/color_card_e2e_test.rs:50-65` `test_lost_compensation_workflow`：仅断言常量字符串相等，不调用任何业务代码
  ```rust
  let mut status = "borrowed";
  assert_eq!(status, "borrowed");  // 自相等伪测试
  status = "lost";
  let compensation: f64 = 500.0;
  assert!(compensation > 0.0);  // 显然为真
  assert_eq!(status, "lost");  // 自相等
  let card_status = "lost";
  assert_eq!(card_status, "lost");  // 自相等
  ```
- `/workspace/backend/tests/quotation_e2e_test.rs:42-67` 多个伪测试：
  - `test_approved_quotation_cannot_update`：仅断言本地常量数组不包含某些字符串
  - `test_convert_only_works_on_approved`：同上
  - `test_quotation_no_format`：仅断言本地 `format!` 字符串
  - `test_order_no_format`：同上
  - `test_quotation_state_machine`：仅断言本地数组 contains
- `/workspace/backend/tests/test_inventory_count.rs`：整个文件为骨架（仅注释，0 测试），原 3 个伪测试已于批次 65 删除，但未补真实测试
  ```rust
  //! InventoryCountService 当前为占位模块（见 services/inv/count.rs），
  //! 尚未实现任何业务方法，无法在无 DB 环境下编写真实单元测试。
  //! 待 Service 实现后，在此添加真实业务方法测试。
  ```

**业务影响**：
- 集成测试断言仅检查 status code / 常量字符串相等，未真实验证业务流程
- 违反 v4 维度 12 P0 第 2 项"测试必须调用真实生产代码"

### 修复建议
1. **P0 紧急**：为 7 项关键业务路径补真实集成测试（生产订单/采购收货/销售发货/付款/染整/打样/大货处方全流程），每流程至少 1 个测试用例覆盖：创建 → 提交 → 审核 → 完成 → 状态校验
2. **P0 紧急**：修复 bi_analysis_test.rs 的 16 个测试，将 `BiAnalysisService::method(args)` 静态调用改为 `BiAnalysisService::new(db).method(args)` 实例调用，并补 `&self` 参数
3. **P0**：评估 CI baseline 机制，对编译失败的测试不应纳入 baseline（应直接 CI 阻塞）
4. **P1**：删除 color_card_e2e_test.rs / quotation_e2e_test.rs 中的伪测试，替换为真实业务代码调用
5. **P1**：为 inventory_count_service 实现真实业务方法后补测试（当前为占位模块）

---

## 维度 3：E2E 测试完整通过

### 检查方法
1. `Read /workspace/.github/workflows/e2e-batch.yml`（E2E 工作流，337 行）
2. `Read /workspace/frontend/playwright.config.ts`（60 行）
3. `Read /workspace/frontend/e2e/fixtures/auth.ts`（mock 工具，140 行）
4. `Grep "mockBusinessApi|applyAuthMocks"` 在 `/workspace/frontend/e2e/`
5. 对照批次 190 E2E 报告 + 规则 5

### 发现

#### ✅ 已落实的项

1. **E2E 工作流独立 + 每 30 批次触发**（`e2e-batch.yml:1-25`）：
   ```yaml
   name: E2E 批次测试（每 30 批次）
   on:
     workflow_dispatch:
       inputs:
         batch_number:
           description: '批次编号（如 270、300、330，30 的倍数）'
   ```

2. **playwright.config.ts 修复**（v4 P0 修复保持）：
   - `reporter: [['html'], ['line']]`（第 25 行）✅ 生成 HTML 报告
   - `timeout: 60_000`（第 27 行）✅ 单测试 60s（非 30s）

3. **PostgreSQL service + 真实后端启动**（`e2e-batch.yml:70-83, 194-214`）：
   ```yaml
   services:
     postgres:
       image: postgres:16-alpine
       ...
   - name: 启动后端服务
     run: |
       ./target/release/server > /tmp/e2e-logs/backend.log 2>&1 &
       # 等待后端端口就绪
       for i in $(seq 1 60); do
         if curl -s http://localhost:8082/health > /dev/null 2>&1; then
           break
         fi
         sleep 0.5
       done
   ```

4. **CI 环境变量 TEST_USERNAME/TEST_PASSWORD 已设置**（`e2e-batch.yml:93-94`）：
   ```yaml
   TEST_USERNAME: e2e_admin
   TEST_PASSWORD: E2e@TestPassword2026!
   ```

5. **E2E 不 continue-on-error: true**（v4 P0 修复保持）：
   - `e2e-batch.yml:255-298` 运行 E2E 测试步骤无 continue-on-error
   - `exit $EXIT_CODE` 直接传递失败码

6. **数据库迁移 + 系统初始化已集成**（`e2e-batch.yml:189-253`）：
   ```yaml
   - name: 运行数据库迁移
     run: ./target/release/bingxi migrate run
   - name: 初始化系统（创建 E2E 测试管理员）
     run: |
       curl -s -X POST http://localhost:8082/api/v1/erp/init/initialize \
         -H "X-Init-Token: ${INIT_TOKEN}" \
         -d "{\"admin_username\":\"${TEST_USERNAME}\",\"admin_password\":\"${TEST_PASSWORD}\"}"
   ```

#### ❌ 缺陷项 1：mockBusinessApi 未移除（违反规则 5）

**风险等级：P0**（规则 5 明确要求"移除 mockBusinessApi（让业务 API 走真实后端）"未达成）

**证据**：
- `/workspace/frontend/e2e/fixtures/auth.ts:100-117`：`mockBusinessApi` 函数仍存在
  ```typescript
  export async function mockBusinessApi(context: BrowserContext): Promise<void> {
    await context.route('**/api/v1/erp/**', (route) => {
      // ...
      return route.fulfill({
        status: 200,
        body: JSON.stringify({ ...EMPTY_PAGINATION, ... }),
      })
    })
  }
  ```
- `/workspace/frontend/e2e/fixtures/auth.ts:122-127`：`applyAuthMocks` 自动包含 `mockBusinessApi`
  ```typescript
  export async function applyAuthMocks(context: BrowserContext): Promise<void> {
    await injectAuthToken(context)
    await mockAuthMe(context)
    await mockInitStatus(context)
    await mockBusinessApi(context)  // ← 自动应用
  }
  ```
- `Grep "applyAuthMocks|mockBusinessApi"` 在 `frontend/e2e/` 共找到 60 处使用：
  - **smoke 测试**（5 个 spec）：production / inventory / quality / sales / quotation.smoke.spec.ts
  - **sales 流程测试**（7 个 spec）：01-create-quotation / 02-create-order / 03-approve / 04-ship / 05-ar-invoice / 06-payment / 07-report.spec.ts 全部使用 applyAuthMocks
  - **purchase 流程测试**（7 个 spec）：01-create-po / 02-approve / 03-receipt / 04-inspection / 05-ap-invoice / 06-payment / 07-supplier-report.spec.ts 全部使用 applyAuthMocks
  - **enhanced 测试**（3 个 spec）：multi-role-collaboration / network-resilience / rpa-data-extraction.spec.ts

**业务影响**：
- 22+ 个 E2E spec 实际上走 mock 而非真实后端，违反规则 5"移除 mockBusinessApi（让业务 API 走真实后端）"
- 批次 190 E2E 报告显示 95 个测试中 88 个失败（73% 因 mockBusinessApi 返回空数据导致页面元素 timeout）
- 批次 191-200 计划"移除 mockBusinessApi"未执行

#### ❌ 缺陷项 2：playwright.config.ts webServer 不是数组

**风险等级：P1**（规则 5 要求"webServer 数组（同时启动前端+后端）"）

**证据**：
- `/workspace/frontend/playwright.config.ts:36-43`：
  ```typescript
  webServer: {
    command: 'npm run dev',
    url: 'http://localhost:3000',
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    stdout: 'pipe',
    stderr: 'pipe',
  },
  ```
- 仅启动前端 dev server，后端由 CI job 独立启动进程（`e2e-batch.yml:194-214`）
- 注释明确说明（第 34-35 行）："CI 中自动启动前端 dev server（后端由 CI job 独立启动进程）"

**业务影响**：
- 虽然后端通过 CI job 启动进程实现了等价效果，但偏离规则 5 明确要求的"webServer 数组"配置
- 本地运行 E2E 时开发者需手动启动后端，与"一键启动"目标不一致

#### ❌ 缺陷项 3：E2E 通过率 < 90%（批次 190 基线 0% 通过）

**风险等级：P0**（规则 5 要求 E2E 通过率 ≥ 90%）

**证据**：
- 批次 190 E2E 报告（`/workspace/.monkeycode/docs/audits/2026-07-08-batch190-e2e-report.md:10-16`）：
  ```
  | 测试总数 | 95 |
  | 失败数 | 88 |
  | 通过数 | 0 |
  | 未跑完（超时终止） | 7 |
  | 结论 | ❌ 全部失败，job 因 30 分钟 timeout 被强制取消 |
  ```
- 无更新的 E2E 报告（仅 `2026-07-08-batch190-e2e-report.md` 一份）
- 批次 191-200 修复计划（报告第 105-115 行）：
  ```
  ### 批次 191-195（E2E 测试文件修复）
  1. 移除 `mockBusinessApi`，让业务 API 走真实后端
  2. 修复 color-card/color-price/custom-order spec 的真实登录
  3. 修复 smoke/sales/purchase spec 的断言
  ```
  截至本次审计，mockBusinessApi 仍存在（见缺陷 1）

**业务影响**：
- E2E 通过率 0%（远低于规则 5 要求的 90%）
- 7 个测试因 job timeout 30 分钟未跑完（job timeout 现已调整为 90 分钟，部分缓解）
- 失败用例未按 P0/P1/P2 优先级纳入后续批次（违反规则 5 第 6 项）

#### ❌ 缺陷项 4：E2E 报告未保存到 docs/audits/

**风险等级：P2**（违反规则 5 第 7 项"E2E 报告保存到 docs/audits/"）

**证据**：
- E2E 报告实际保存位置：`/workspace/.monkeycode/docs/audits/2026-07-08-batch190-e2e-report.md`
- 规则 5 要求路径：`/workspace/docs/audits/`
- `/workspace/docs/audits/` 目录下无 E2E 报告（仅有 P1/P2/P3 审计 / v5 / v11 等 reaudit 报告）
- E2E 工作流（`e2e-batch.yml:308-318`）将报告作为 artifact 上传，未持久化到代码库 `docs/audits/`

#### ❌ 缺陷项 5：E2E 失败用例未按 P0/P1/P2 优先级纳入后续批次

**风险等级：P1**（违反规则 5 第 6 项）

**证据**：
- 批次 190 E2E 报告第 88-97 行有修复优先级表，但未明确按"P0/P1/P2"标注每个失败用例
- 批次 190 报告创建于 2026-07-08，截至本次审计（2026-07-16）8 天，未发现后续批次的 E2E 修复进展
- 批次 191-200 修复计划未明确转化为 doto.md 任务

### 修复建议
1. **P0 紧急**：移除 `mockBusinessApi` 函数及 `applyAuthMocks` 中的调用，让 sales/* 和 purchase/* E2E 走真实后端
2. **P0**：将 playwright.config.ts 的 webServer 改为数组配置（前端 dev server + 后端二进制），实现本地+CI 一致启动
3. **P0**：跑一次完整 E2E，将 95 个测试通过率提升至 ≥ 90%
4. **P1**：将批次 190 失败用例按 P0/P1/P2 优先级拆解到 `.monkeycode/doto.md`，逐批修复
5. **P2**：CI 工作流完成后将 E2E 报告 copy 到 `/workspace/docs/audits/` 目录持久化

---

## 维度 4：测试 mock 数据 fixtures 化

### 检查方法
1. `ls /workspace/frontend/tests/fixtures/` 和 `ls /workspace/frontend/e2e/fixtures/`
2. `Read` 关键 fixtures 文件（v2-table.ts / auth.ts / multi-context.ts）
3. `Grep "fixtures|createXxxMock"` 在 `frontend/tests/unit/`
4. 对照规则 6 检查要点

### 发现

#### ✅ 已落实的项

1. **fixtures 目录已建立**：
   - `/workspace/frontend/tests/fixtures/v2-table.ts`（48 行，V2Table 测试 mock 数据）
   - `/workspace/frontend/e2e/fixtures/auth.ts`（140 行，E2E auth mock）
   - `/workspace/frontend/e2e/fixtures/multi-context.ts`（多上下文隔离工具）
   - `/workspace/frontend/e2e/fixtures/network.ts`（网络拦截工具）
   - `/workspace/frontend/e2e/fixtures/rpa.ts`（RPA 数据提取工具）

2. **smoke 测试 mock 已抽取到 fixtures**（v4 P0 修复保持）：
   - `/workspace/frontend/e2e/smoke/_helpers.ts:5-13` 仅做再导出：
     ```typescript
     export {
       generateFakeJwt,
       injectAuthToken,
       mockAuthMe,
       mockInitStatus,
       mockBusinessApi,
       applyAuthMocks,
       waitForPageReady,
     } from '../fixtures/auth'
     ```

3. **fixtures 文件附中文注释**：
   - `/workspace/frontend/tests/fixtures/v2-table.ts:1-4`：
     ```typescript
     /**
      * V2Table 测试 mock 数据夹具
      * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures
      */
     ```
   - `/workspace/frontend/e2e/fixtures/auth.ts:1-7`：附中文注释说明用途

4. **v2-table.ts 使用 const 命名导出**（部分实现工厂函数模式）：
   ```typescript
   export const singleRow: TestRow[] = [{ id: 1, name: 'A' }]
   export const dualRows: TestRow[] = [...]
   export const fullColumns: ColumnDef<TestRow>[] = [...]
   ```

#### ❌ 缺陷项 1：缺少按业务域组织的 fixtures 文件

**风险等级：P1**（规则 6 检查要点 2 未达成）

**证据**：
- 规则 6 要求按业务域组织：
  - `fixtures/sales.ts`
  - `fixtures/user.ts`
  - `fixtures/dyeing.ts`（V15 新增）
  - `fixtures/color_card.ts`（V15 新增）
  - `fixtures/production_order.ts`
- 实际只有：
  - `tests/fixtures/v2-table.ts`（仅 V2Table 组件测试）
  - `e2e/fixtures/auth.ts`（仅 auth mock）
  - `e2e/fixtures/multi-context.ts`（多角色工具）
  - `e2e/fixtures/network.ts` / `e2e/fixtures/rpa.ts`（工具类）
- **缺**：`fixtures/sales.ts` / `fixtures/user.ts` / `fixtures/dyeing.ts` / `fixtures/color_card.ts` / `fixtures/production_order.ts` 均不存在

**业务影响**：
- 销售域、用户域、染整域、色卡域、生产订单域的测试数据分散在各个测试文件中
- 无法集中维护业务对象工厂函数，mock 数据变更需逐文件修改

#### ❌ 缺陷项 2：未使用 createXxxMock(overrides?) 工厂函数模式

**风险等级：P2**（规则 6 检查要点 3 未达成）

**证据**：
- `/workspace/frontend/tests/fixtures/v2-table.ts:16-25`：使用 const 常量导出，非工厂函数
  ```typescript
  export const singleRow: TestRow[] = [{ id: 1, name: 'A' }]
  export const dualRows: TestRow[] = [
    { id: 1, name: 'Item 1' },
    { id: 2, name: 'Item 2' },
  ]
  ```
- `Grep "createXxxMock|create.*Mock.*overrides"` 在 `frontend/tests` 和 `frontend/e2e`：无匹配
- 规则 6 推荐模式（未实现）：
  ```typescript
  export function createSalesOrderMock(overrides?: Partial<SalesOrder>): SalesOrder {
    return { id: 1, customer_id: 1, status: 'draft', ...overrides }
  }
  ```

**业务影响**：
- 测试用例无法通过 `overrides` 灵活定制 mock 数据
- 每个 spec 需重复定义完整 mock 对象

#### ❌ 缺陷项 3：现有测试仍存在内联硬编码 mock JSON

**风险等级：P2**（规则 6 检查要点 4 未达成）

**证据**：
- `/workspace/frontend/tests/unit/inventory-store.test.ts:45-48`：mock 数据内联
  ```typescript
  const mockStocks = [
    { id: 1, product_name: '面料A', quantity: 100 },
    { id: 2, product_name: '面料B', quantity: 200 },
  ]
  ```
- `/workspace/frontend/tests/unit/user-store.test.ts:42-45`：mock 响应内联
  ```typescript
  const mockResponse: LoginResponse = {
    user: { id: 1, username: 'admin', role: 'admin' } as UserInfo,
    permissions: [],
  }
  ```
- `/workspace/frontend/tests/unit/login.test.ts:26-46`：mock 函数 + 路由对象内联
- `/workspace/frontend/e2e/sales/01-create-quotation.spec.ts:40-56`：表单填写逻辑内联（数量/单价等）

**业务影响**：
- 违反规则 6 检查要点 4"禁止内联硬编码 mock JSON"
- mock 数据变更需逐文件修改

### 修复建议
1. **P1**：新建按业务域的 fixtures 文件：
   - `frontend/tests/fixtures/sales.ts`（销售订单/报价单/发货单 mock 工厂）
   - `frontend/tests/fixtures/user.ts`（用户/角色/权限 mock 工厂）
   - `frontend/tests/fixtures/dyeing.ts`（染整/染缸/染化料 mock 工厂）
   - `frontend/tests/fixtures/color_card.ts`（色卡 mock 工厂）
   - `frontend/tests/fixtures/production_order.ts`（生产订单 mock 工厂）
2. **P2**：将 v2-table.ts 的 const 常量改为 `createXxxMock(overrides?)` 工厂函数
3. **P2**：扫描所有 `frontend/tests/unit/*.test.ts`，将内联硬编码 mock JSON 迁移到 fixtures
4. **P2**：扫描 `frontend/e2e/sales/*` 和 `frontend/e2e/purchase/*` spec，将表单填写数据迁移到 fixtures

---

## 维度 5：测试质量（禁止伪测试）

### 检查方法
1. `Read /workspace/backend/tests/color_card_e2e_test.rs` / `quotation_e2e_test.rs` / `bi_analysis_test.rs` / `test_inventory_count.rs`
2. `Read /workspace/frontend/tests/unit/login.test.ts` / `utils.test.ts`
3. `Grep "assert_eq!\".*\", *\".*\""` 在 `backend/tests/`
4. 对照 v4 维度 12 P0 28 项

### 发现

#### ✅ 已落实的项

1. **80+ 伪测试已大量清理**（v4 P0 修复保持）：
   - v4 报告第 455-465 行：v4 基线 80+ 伪测试，分布在 `p9_5_ar_extra_tests.rs` / `p9_5_inventory_extra_tests.rs` / `p9_5_sales_extra_tests.rs` / `p9_5_purchase_extra_tests.rs` / `tests/integration/sales_flow.rs` / `tests/integration/auth_flow.rs` / `tests/integration/api_routes.rs`
   - v15 审计：`Grep "p9_5|tests/integration"` 在 `backend/`：**无匹配**
   - 上述伪测试文件均已删除（清理完成）

2. **tests/unit/Login.test.ts 测真实 Login.vue**（v4 P0 修复保持）：
   - `/workspace/frontend/tests/unit/login.test.ts:101`：
     ```typescript
     import Login from '@/views/Login.vue'
     ```
   - 第 6 行注释明确说明："批次 29 v7 P0-7 修复：原测试用 LoginMock 自定义组件，未测试真实 Login.vue。改为 mount 真实 Login.vue"
   - 8 个测试用例覆盖：渲染 / 表单校验 / 登录成功跳转 / 登录失败不跳转 / Open Redirect 防护 / checkLockStatus 预检查

3. **tests/unit/utils.test.ts import 真实 utils**（v4 P0 修复保持）：
   - `/workspace/frontend/tests/unit/utils.test.ts:2`：
     ```typescript
     import { formatCurrency, formatDate, debounce } from '@/utils'
     ```
   - 测试覆盖：formatCurrency 金额格式化 / formatDate 日期格式化 / debounce 防抖函数

4. **bi_analysis_test.rs 的 e2e 测试用 #[ignore] 标注**（部分修复）：
   - `/workspace/backend/tests/bi_analysis_test.rs:128`：
     ```rust
     #[tokio::test]
     #[ignore = "需要 PostgreSQL + ETL 数据 + axum server"]
     async fn test_e2e_etl_to_aggregation() { ... }
     ```

#### ❌ 缺陷项 1：tests/unit/Login.test.ts 仍有内联硬编码 mock 数据

**风险等级：P2**（v4 P0 修复保持但部分违反规则 6）

**证据**：
- `/workspace/frontend/tests/unit/login.test.ts:26-46`：mock 函数 + 响应对象内联在 `vi.hoisted()` 中
  ```typescript
  const { mockLogin, mockCheckLockStatus, pushSpy, routeRef } = vi.hoisted(() => ({
    mockLogin: vi.fn().mockResolvedValue(undefined),
    mockCheckLockStatus: vi.fn().mockResolvedValue({
      data: { is_locked: false, failed_attempts: 0, ... },
    }),
    ...
  }))
  ```
- 第 104-126 行：i18n messages 全部内联在测试文件中
- 第 218-232 行：`routeRef.query = { redirect: '/dashboard' }` / `routeRef.query = { redirect: '//evil.com' }` 内联

**业务影响**：
- 虽然测试用真实 Login.vue（v4 P0 修复保持），但 mock 数据未抽取到 fixtures
- 与维度 4 缺陷项 3 关联

#### ❌ 缺陷项 2：color_card_e2e_test.rs 仍含伪测试

**风险等级：P1**（v4 P0 修复未完全保持）

**证据**：
- `/workspace/backend/tests/color_card_e2e_test.rs:11-29` `test_full_workflow`：
  ```rust
  let card_no = "E2E-TEST-001";
  assert!(!card_no.is_empty());  // 显然为真
  let _color_code = "18-1664 TPX";
  let hex = rgb_to_hex(220, 50, 50);
  assert_eq!(hex, "#DC3232");  // 工具函数测试，非业务流程
  let borrow_status = "borrowed";
  assert_eq!(borrow_status, "borrowed");  // 自相等伪测试
  let return_status = "returned";
  assert_eq!(return_status, "returned");  // 自相等伪测试
  ```
- 第 32-46 行 `test_scan_workflow`：仅测试 `rgb_to_lab` 和 `delta_e_76` 工具函数
- 第 49-64 行 `test_lost_compensation_workflow`：仅断言常量字符串和 `compensation > 0.0`（显然为真）

**业务影响**：
- 文件名为 "color_card_e2e_test"，但实际未端到端测试色卡业务流程
- 不调用 ColorCardCrudService / ColorCardBorrowService 等业务方法
- 违反 v4 维度 12 P0 第 2 项"测试必须调用真实生产代码"

#### ❌ 缺陷项 3：quotation_e2e_test.rs 多个伪测试

**风险等级：P1**（v4 P0 修复未完全保持）

**证据**：
- `/workspace/backend/tests/quotation_e2e_test.rs:42-56` `test_approved_quotation_cannot_update`：仅断言本地常量数组
  ```rust
  let allowed_for_update = ["draft", "rejected"];
  for status in ["approved", "pending_approval", "converted", "cancelled", "expired"] {
      assert!(!allowed_for_update.contains(&status), ...);
  }
  ```
- 第 58-68 行 `test_convert_only_works_on_approved`：同上伪测试
- 第 70-78 行 `test_quotation_no_format`：仅断言本地 `format!` 字符串
  ```rust
  let no = format!("QT{}{:04}", today, 1);
  assert!(no.starts_with("QT"));  // 显然为真
  assert!(no.len() >= 14);  // 显然为真
  ```
- 第 80-88 行 `test_order_no_format`：同上
- 第 90-114 行 `test_quotation_state_machine`：仅断言本地数组 contains
- 第 116-143 行 `test_create_quotation_dto_required_fields`：仅断言 DTO 字段值（不调用 service）
- 第 145-177 行 `test_quotation_response_serialize`：仅断言 DTO 序列化（不调用 service）
- 第 179-218 行 `test_quotation_model_default`：仅构造 Model 实例并断言字段值

**业务影响**：
- 文件 9 个测试中 8 个为伪测试，仅 `test_full_workflow_amount_tier_logic` 调用真实 `ApproverRole::from_amount`
- 文件名为 "e2e"，但完全不调用 QuotationService 任何业务方法

#### ❌ 缺陷项 4：bi_analysis_test.rs 16 个测试 API 与源码脱节

**风险等级：P0**（测试代码与生产代码脱节，编译失败）

**证据**：
- `/workspace/backend/tests/bi_analysis_test.rs:12-15` 测试调用：
  ```rust
  assert!(BiAnalysisService::kpi_summary(0).await.is_err());
  assert!(BiAnalysisService::kpi_summary(-1).await.is_err());
  assert!(BiAnalysisService::kpi_summary(1).await.is_ok());
  ```
- `/workspace/backend/src/services/bi_analysis_service.rs:657` 源码签名：
  ```rust
  pub async fn kpi_summary(&self) -> Result<KpiSummary, AppError>
  ```
  测试用静态调用 + i32 参数，源码是实例方法 + 无参数
- 16 个测试全部存在相同问题：
  - `BiAnalysisService::kpi_summary(0/1/-1)` → 应为 `BiAnalysisService::new(db).kpi_summary()`
  - `BiAnalysisService::sales_by_customer(1, 2)` → 应为 `service.sales_by_customer(2)`
  - `BiAnalysisService::sales_by_product(1, 1)` → 应为 `service.sales_by_product(1)`
  - `BiAnalysisService::drilldown_year_to_month(1, 2026)` → 应为 `service.drilldown_year_to_month(2026)`
  - `BiAnalysisService::slice(1, "customer", &json)` → 应为 `service.slice("customer", &json)`
  - `BiAnalysisService::rollup(1, "day", "month")` → 应为 `service.rollup("day", "month")`
  - `BiAnalysisService::pivot(1, "time", "product", "amount")` → 应为 `service.pivot("time", "product", "amount")`

**业务影响**：
- bi_analysis_test.rs 必然编译失败（自 v9 批次 130 重构 BiAnalysisService 后即脱节）
- 这些测试实际从未运行成功，但因 CI baseline 机制被掩盖（详见维度 2 缺陷项 2）
- 违反 v4 维度 12 P0 第 2 项"测试必须调用真实生产代码"

#### ❌ 缺陷项 5：test_inventory_count.rs 整个文件为空骨架

**风险等级：P2**（v4 P0 部分修复保持）

**证据**：
- `/workspace/backend/tests/test_inventory_count.rs`：整个文件仅注释，0 测试
  ```rust
  //! 库存盘点模块测试骨架（P1 批 65 测试资产清理）
  //! 原文件含 3 个伪测试（测算术减法 / 测 Vec 遍历 / 测本地 is_valid_transition 函数），
  //! 均不调用任何 Service 方法，已于批次 65 删除。
  //! InventoryCountService 当前为占位模块（见 services/inv/count.rs），
  //! 尚未实现任何业务方法，无法在无 DB 环境下编写真实单元测试。
  //! 待 Service 实现后，在此添加真实业务方法测试。
  ```

**业务影响**：
- 3 个伪测试已删除（v4 P0 修复保持），但未补真实测试
- InventoryCountService 是否仍为占位模块需进一步核实（services/inv/count.rs 存在但内容未审计）

### 修复建议
1. **P0 紧急**：修复 `bi_analysis_test.rs` 的 16 个测试 API 调用，将静态方法调用改为 `BiAnalysisService::new(db).method(args)` 实例方法调用
2. **P1**：删除 `color_card_e2e_test.rs` 中 3 个伪测试（test_full_workflow / test_scan_workflow / test_lost_compensation_workflow），改为调用 ColorCardService 真实方法的集成测试
3. **P1**：删除 `quotation_e2e_test.rs` 中 8 个伪测试，仅保留 `test_full_workflow_amount_tier_logic`，并补真实 QuotationService 集成测试
4. **P2**：将 `tests/unit/Login.test.ts` 的内联 mock 数据抽取到 `tests/fixtures/login.ts`
5. **P2**：核实 `services/inv/count.rs` 是否已实现业务方法，如已实现则在 `test_inventory_count.rs` 补真实测试

---

## 维度 6：性能基准测试

### 检查方法
1. `Grep "criterion|cargo bench|benches/"` 在 `/workspace/`
2. `Glob /workspace/backend/benches/**/*.rs`
3. `Read /workspace/backend/Cargo.toml`（查 `[[bench]]` 段和 `criterion` 依赖）
4. `Read /workspace/frontend/scripts/p2-3-perf-test.mjs`（前端性能测试）
5. `Grep "perf-baseline|perf-test|性能基准"` 在 `/workspace/`

### 发现

#### ✅ 已落实的项

1. **前端 V2Table 性能测试脚本已建立**（部分实现）：
   - `/workspace/frontend/scripts/p2-2-perf-baseline.mjs`：性能基线脚本
   - `/workspace/frontend/scripts/p2-3-perf-test.mjs`：性能测试脚本（Playwright + chromium）
   - `/workspace/frontend/scripts/poc-perf-test.cjs`：POC 性能测试
   - `/workspace/frontend/scripts/p2-3-perf-report.md`：性能测试报告（已生成）

2. **前端性能测试有合理阈值**（`p2-3-perf-test.mjs:5`）：
   ```javascript
   // 验证：4 页面（inventory/sales/production/quality）的 TTI < 1.5s / FPS > 50 /
   // renderCell 计数 = 可见行数 × 列数
   ```

3. **前端性能测试覆盖 4 个核心页面**（`p2-3-perf-test.mjs:19-24`）：
   ```javascript
   const PAGES = [
     { name: 'inventory', url: '/inventory', expectedRows: 10000, rowHeight: 40 },
     { name: 'sales', url: '/sales', expectedRows: 5000, rowHeight: 56 },
     { name: 'production', url: '/production', expectedRows: 2000, rowHeight: 48 },
     { name: 'quality', url: '/quality', expectedRows: 2000, rowHeight: 44 }
   ]
   ```

#### ❌ 缺陷项 1：后端关键 service 性能基准测试完全缺失

**风险等级：P0**（审计计划 6.6 检查要点 1 完全未达成）

**证据**：
- `Glob /workspace/backend/benches/**/*.rs`：**无文件**
- `Grep "criterion|cargo bench|criterion_group|criterion_main"` 在 `/workspace/backend/`：**无匹配**
- `/workspace/backend/Cargo.toml`：无 `[[bench]]` 段、无 `criterion` 依赖
- 审计计划 6.6 检查要点 1 要求的关键 service 性能基准测试：
  - **库存核算**：无性能基准
  - **凭证生成**：无性能基准
  - **染整成本归集**（V15 新增）：无性能基准
  - **产量工资计算**（V15 新增）：无性能基准

**业务影响**：
- 4 项关键 service 性能无基准保护
- 任何性能回归（如 N+1 查询、循环内 DB 调用）无法被 CI 检测
- 大数据量场景（如 10 万条凭证生成 / 1000 个缸号成本归集）性能无监控

#### ❌ 缺陷项 2：cargo bench 配置完全缺失

**风险等级：P1**（审计计划 6.6 检查要点 2 未达成）

**证据**：
- `/workspace/backend/Cargo.toml` 无 `[[bench]]` 段
- 无 `benches/` 目录
- `[dev-dependencies]` 段（第 140-148 行）无 `criterion` 依赖：
  ```toml
  [dev-dependencies]
  tokio-test = "0.4"
  mockall = "0.12"
  tower = "0.5"
  sea-orm = { version = "1.1.20", features = ["sqlx-sqlite", "runtime-tokio"] }
  rust_decimal_macros = "1.34"
  ```

#### ❌ 缺陷项 3：性能回归 CI 监控缺失

**风险等级：P1**（审计计划 6.6 检查要点 3 未达成）

**证据**：
- `/workspace/.github/workflows/ci-cd.yml`：无性能测试 job
- `/workspace/.github/workflows/e2e-batch.yml`：无性能测试 job
- 前端性能测试脚本 `p2-3-perf-test.mjs` 不在 CI 工作流中执行
- 无性能基线对比机制（基线 `p2-2-perf-baseline.mjs` 仅手动运行）

#### ❌ 缺陷项 4：性能报告生成不完整

**风险等级：P2**（审计计划 6.6 检查要点 5 部分达成）

**证据**：
- 仅 `/workspace/frontend/scripts/p2-3-perf-report.md` 一份性能报告（2026-06-16 生成）
- 后端无性能报告
- CI 不自动生成性能报告

### 修复建议
1. **P0 紧急**：为 4 项关键 service 添加性能基准测试：
   - `benches/inventory_calculation_bench.rs`：库存核算性能（含大数据量场景）
   - `benches/voucher_generation_bench.rs`：凭证生成性能（含批量场景）
   - `benches/dye_cost_collection_bench.rs`：染整成本归集性能（V15 新增）
   - `benches/wage_calculation_bench.rs`：产量工资计算性能（V15 新增）
2. **P1**：在 `backend/Cargo.toml` 添加：
   ```toml
   [dev-dependencies]
   criterion = { version = "0.5", features = ["async_tokio"] }

   [[bench]]
   name = "inventory_calculation"
   harness = false
   ```
3. **P1**：在 CI 工作流新增 `ci-perf-bench` job，运行 `cargo bench --bench xxx` 并对比基线
4. **P2**：将前端 `p2-3-perf-test.mjs` 纳入 E2E 工作流，定期生成性能报告

---

## 维度 7：覆盖率报告生成

### 检查方法
1. `Grep "coverage|tarpaulin|llvm-cov|codecov"` 在 `/workspace/`
2. `Read /workspace/frontend/vitest.config.ts`
3. `Grep "coverage|--coverage"` 在 `frontend/package.json` 和 `ci-cd.yml`
4. `Glob /workspace/docs/2026-06-17-p4-5-coverage-report*`（README 提到的覆盖率报告）

### 发现

#### ✅ 已落实的项

1. **前端 vitest 覆盖率配置已建立**（`/workspace/frontend/vitest.config.ts:20-31`）：
   ```typescript
   coverage: {
     provider: 'v8',
     reporter: ['text', 'json', 'html'],
     reportsDirectory: './coverage',
     include: ['src/**/*.{ts,vue}'],
     exclude: [
       'src/types/**',
       'src/**/*.d.ts',
       'src/main.ts',
       'src/App.vue',
     ],
   },
   ```

2. **前端 package.json 配置 test:coverage 脚本**（`/workspace/frontend/package.json:12`）：
   ```json
   "test:coverage": "vitest run --coverage"
   ```
   依赖 `@vitest/coverage-v8@^4.1.8`（第 39 行）

3. **README 文档说明覆盖率工具使用**（`/workspace/README.md:573-586`）：
   ```markdown
   ### 覆盖率报告
   ```bash
   # 后端
   cd backend
   cargo install cargo-tarpaulin
   cargo tarpaulin --out Html --output-dir coverage/
   # 前端
   cd frontend
   npm run test:coverage
   ```
   ```

#### ❌ 缺陷项 1：CI 不集成 cargo tarpaulin / cargo llvm-cov + codecov

**风险等级：P0**（审计计划 6.7 检查要点 1 完全未达成）

**证据**：
- `/workspace/.github/workflows/ci-cd.yml`：`Grep "tarpaulin|llvm-cov|codecov|coverage"` 无匹配
- `ci-test-rust` job（第 833-1050 行）仅运行 `cargo test --jobs 1 -- --test-threads=1`，无覆盖率收集
- README 第 578-579 行说明手动安装 `cargo-tarpaulin`，但 CI 未集成
- v4 报告第 469 行：v4 基线"CI 不生成覆盖率报告（无 cargo tarpaulin / cargo llvm-cov / codecov）"——未修复

**业务影响**：
- 后端测试覆盖率无量化数据（v4 估计 38%，v15 无准确数据）
- 无法监控覆盖率趋势、无法设置门槛、无法下降告警
- 违反审计计划 6.7 检查要点 1

#### ❌ 缺陷项 2：前端 CI 不运行 --coverage

**风险等级：P1**（审计计划 6.7 检查要点 2 部分未达成）

**证据**：
- `/workspace/.github/workflows/ci-cd.yml:1085` `ci-test-fe` job：
  ```yaml
  npx vitest run --reporter=default --reporter=junit --outputFile.reporter=junit=reports/vitest-junit.xml
  ```
  仅运行测试，不收集覆盖率
- 应改为 `npx vitest run --coverage --reporter=default` 或新增 `ci-coverage-fe` job

**业务影响**：
- 前端覆盖率配置存在但 CI 不执行，无法生成报告
- 违反审计计划 6.7 检查要点 2

#### ❌ 缺陷项 3：覆盖率门槛未配置

**风险等级：P1**（审计计划 6.7 检查要点 3 未达成）

**证据**：
- 审计计划 6.7 检查要点 3 要求：
  - 核心模块 80%+
  - 全项目 60%+
- `/workspace/frontend/vitest.config.ts`：无 `thresholds` 配置
- 后端无任何覆盖率门槛配置
- 无 codecov.yml 或 .github/coverage.yml 配置文件

**业务影响**：
- 覆盖率下降无告警
- 核心模块覆盖率不达标不阻塞 CI

#### ❌ 缺陷项 4：覆盖率趋势监控缺失

**风险等级：P2**（审计计划 6.7 检查要点 4 未达成）

**证据**：
- 无 codecov 集成（无 `codecov.yml`）
- CI 不上传覆盖率报告到 codecov/coveralls
- 无覆盖率趋势图表
- 无 PR 评论覆盖率变化

#### ❌ 缺陷项 5：README 引用的覆盖率报告不存在

**风险等级：P3**（文档失效）

**证据**：
- `/workspace/README.md:586`：
  ```markdown
  详细覆盖率见 [docs/2026-06-17-p4-5-coverage-report.md](docs/2026-06-17-p4-5-coverage-report.md)。
  ```
- `Glob /workspace/docs/2026-06-17-p4-5-coverage-report*`：**无文件**
- `Glob /workspace/**/coverage-report*`：**无文件**

**业务影响**：
- 文档引用失效，开发者无法查看历史覆盖率报告
- 可能是文件已删除但 README 未同步更新

### 修复建议
1. **P0 紧急**：在 `ci-cd.yml` 新增 `ci-coverage-rust` job：
   ```yaml
   ci-coverage-rust:
     runs-on: ubuntu-latest
     steps:
       - uses: actions/checkout@v5
       - name: 安装 tarpaulin
         run: cargo install cargo-tarpaulin
       - name: 运行覆盖率
         working-directory: backend
         run: cargo tarpaulin --out Xml --output-dir coverage/
       - name: 上传到 codecov
         uses: codecov/codecov-action@v4
         with:
           file: backend/coverage/cobertura.xml
           fail_ci_if_error: false
   ```
2. **P1**：修改 `ci-test-fe` job 添加 `--coverage` 参数，并上传到 codecov
3. **P1**：配置覆盖率门槛：
   - 创建 `codecov.yml` 设置核心模块 80%+ / 全项目 60%+
   - 在 vitest.config.ts 添加 `coverage.thresholds` 配置
4. **P2**：建立覆盖率趋势监控（codecov PR 评论 + 状态徽章）
5. **P3**：修复 README 中的失效链接，或重新生成 `docs/2026-06-17-p4-5-coverage-report.md`

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 6.1 单元测试覆盖率 | 1 | 2 | 0 | 1 | 4 | 8 |
| 6.2 集成测试执行率 | 2 | 1 | 0 | 0 | 4 | 7 |
| 6.3 E2E 测试完整通过 | 2 | 1 | 1 | 0 | 6 | 10 |
| 6.4 测试 mock 数据 fixtures 化 | 0 | 1 | 2 | 0 | 4 | 7 |
| 6.5 测试质量（禁止伪测试） | 1 | 2 | 2 | 0 | 4 | 9 |
| 6.6 性能基准测试 | 1 | 2 | 1 | 0 | 3 | 7 |
| 6.7 覆盖率报告生成 | 1 | 2 | 1 | 1 | 3 | 8 |
| **合计** | **8** | **11** | **7** | **2** | **28** | **56** |

## 修复优先级队列

### P0 级（阻塞，8 项）

1. **维度 6.1 缺陷 1**：`quotation_service.rs`（549 行 14 个方法）和 `purchase_receipt_service.rs`（677 行 14 个方法）零单元测试，需补核心 CRUD 测试
2. **维度 6.2 缺陷 1**：7 项关键业务路径（生产订单/采购收货/销售发货/付款/染整/打样/大货处方全流程）无真实集成测试
3. **维度 6.2 缺陷 2**：CI baseline 机制可能掩盖编译失败（bi_analysis_test.rs 16 个测试 API 与源码脱节）
4. **维度 6.3 缺陷 1**：`mockBusinessApi` 未移除，22+ 个 E2E spec 走 mock 而非真实后端（违反规则 5）
5. **维度 6.3 缺陷 3**：E2E 通过率 0%（远低于规则 5 要求的 90%），批次 190 后无更新
6. **维度 6.5 缺陷 4**：`bi_analysis_test.rs` 16 个测试 API 与源码脱节，必然编译失败
7. **维度 6.6 缺陷 1**：4 项关键 service（库存核算/凭证生成/染整成本归集/产量工资计算）性能基准测试完全缺失
8. **维度 6.7 缺陷 1**：CI 不集成 cargo tarpaulin / cargo llvm-cov / codecov，后端覆盖率无量化数据

### P1 级（高，11 项）

1. **维度 6.1 缺陷 2**：`inventory_stock_service.rs` 核心 CRUD 方法零覆盖（仅 6 个工具方法测试）
2. **维度 6.1 缺陷 4**：service 模块覆盖率未达 80% 目标（v4 基线 38%，无量化数据）
3. **维度 6.2 缺陷 3**：集成测试断言不充分（color_card_e2e_test.rs / quotation_e2e_test.rs 多伪测试）
4. **维度 6.3 缺陷 2**：playwright.config.ts webServer 不是数组（仅启动前端，后端由 CI 独立进程）
5. **维度 6.3 缺陷 5**：E2E 失败用例未按 P0/P1/P2 优先级纳入后续批次
6. **维度 6.4 缺陷 1**：缺少按业务域组织的 fixtures 文件（sales.ts/user.ts/dyeing.ts/color_card.ts/production_order.ts）
7. **维度 6.5 缺陷 2**：`color_card_e2e_test.rs` 3 个伪测试未删除
8. **维度 6.5 缺陷 3**：`quotation_e2e_test.rs` 8 个伪测试未删除
9. **维度 6.6 缺陷 2**：cargo bench 配置完全缺失（无 [[bench]] 段、无 criterion 依赖）
10. **维度 6.6 缺陷 3**：性能回归 CI 监控缺失（无性能测试 job）
11. **维度 6.7 缺陷 2**：前端 CI 不运行 `--coverage`（vitest.config.ts 已配置但 CI 不执行）
12. **维度 6.7 缺陷 3**：覆盖率门槛未配置（核心模块 80%+ / 全项目 60%+）

### P2 级（中，7 项）

1. **维度 6.3 缺陷 4**：E2E 报告未保存到 `docs/audits/`（实际在 `.monkeycode/docs/audits/`）
2. **维度 6.4 缺陷 2**：未使用 `createXxxMock(overrides?)` 工厂函数模式
3. **维度 6.4 缺陷 3**：现有测试仍存在内联硬编码 mock JSON（inventory-store / user-store / login.test.ts）
4. **维度 6.5 缺陷 1**：`tests/unit/Login.test.ts` 仍有内联硬编码 mock 数据
5. **维度 6.5 缺陷 5**：`test_inventory_count.rs` 整个文件为空骨架
6. **维度 6.6 缺陷 4**：性能报告生成不完整（仅前端 V2Table 一份，后端无报告）
7. **维度 6.7 缺陷 4**：覆盖率趋势监控缺失（无 codecov 集成）

### P3 级（低，2 项）

1. **维度 6.1 缺陷 3**：染整相关 service 测试函数名不符合中文规范（dye_recipe / lab_dip / production_recipe / flow_card / fabric_inspection / quotation_approval / quotation_convert）
2. **维度 6.7 缺陷 5**：README 引用的覆盖率报告 `docs/2026-06-17-p4-5-coverage-report.md` 不存在

## 审计结论

V15 类六（测试体系审计类）7 维度审计完成。项目测试体系在 v4 维度 12 P0 28 项基础上已部分修复，但仍存在系统性问题：

### 已落实的核心修复（28 项）

1. **CI 不再 `--lib` 跳过集成测试**（v4 P0-1 修复保持）
2. **80+ 伪测试已大量清理**（p9_5_*.rs / tests/integration/* 已删除）
3. **tests/unit/Login.test.ts 测真实 Login.vue**（v4 P0-7 修复保持）
4. **tests/unit/utils.test.ts import 真实 utils**（v4 P0 修复保持）
5. **测试函数中文命名规范已部分落实**（voucher_service / inventory_stock_service / wage_service / energy_service / dye_batch_state_machine_service）
6. **测试函数总数较 v4 基线大幅提升**（586 → 874，增长 49%）
7. **E2E 工作流独立 + 每 30 批次触发**（v4 P0 修复保持）
8. **playwright.config.ts 修复**（reporter html + 单测试 timeout 60s）
9. **PostgreSQL service + 真实后端启动 + 数据迁移 + 系统初始化**（e2e-batch.yml 完整配置）
10. **CI 环境变量 TEST_USERNAME/TEST_PASSWORD 已设置**
11. **E2E 不 continue-on-error: true**
12. **fixtures 目录已建立**（v2-table.ts / auth.ts / multi-context.ts / network.ts / rpa.ts）
13. **smoke 测试 mock 已抽取到 fixtures**
14. **前端 vitest 覆盖率配置已建立**（provider: v8）

### 未达成的核心缺陷（28 项）

1. **核心 service 单元测试覆盖率未达成**（quotation_service / purchase_receipt_service 零测试）
2. **关键业务路径集成测试完全缺失**（7 项关键流程无真实集成测试）
3. **mockBusinessApi 未移除**（22+ 个 E2E spec 走 mock）
4. **E2E 通过率 0%**（远低于 90% 要求）
5. **bi_analysis_test.rs 16 个测试 API 与源码脱节**（编译失败被 baseline 掩盖）
6. **后端性能基准测试完全缺失**（4 项关键 service 无基准）
7. **CI 不集成覆盖率工具**（cargo tarpaulin / codecov 缺失）
8. **fixtures 未按业务域组织**（缺 sales.ts / user.ts / dyeing.ts / color_card.ts / production_order.ts）

### 核心问题聚类

1. **测试与生产代码脱节**（P0，影响维度 2/5）：
   - bi_analysis_test.rs 16 个测试 API 不匹配源码
   - CI baseline 机制掩盖编译失败
   - **建议作为最高优先级修复**

2. **E2E 测试体系未真正落地**（P0，影响维度 3）：
   - mockBusinessApi 仍存在，22+ spec 走 mock
   - E2E 通过率 0%
   - **建议系统性移除 mockBusinessApi，重写 E2E spec**

3. **核心业务 service 测试覆盖空白**（P0，影响维度 1/2）：
   - quotation_service / purchase_receipt_service 零测试
   - 7 项关键业务路径无集成测试
   - **建议按业务域补全单元测试和集成测试**

4. **性能与覆盖率监控空白**（P0/P1，影响维度 6/7）：
   - 后端无 cargo bench / cargo tarpaulin 集成
   - 4 项关键 service 无性能基准
   - 覆盖率无量化数据
   - **建议在 CI 中集成 cargo bench + cargo tarpaulin + codecov**

5. **测试数据 fixtures 未按业务域组织**（P1/P2，影响维度 4/5）：
   - 缺少 sales.ts / user.ts / dyeing.ts 等业务域 fixtures
   - 现有测试仍存在内联硬编码 mock JSON
   - **建议按业务域补全 fixtures 文件，迁移内联 mock 数据**

建议按 P0 → P1 → P2 → P3 优先级依次修复，每个修复需经 CI/CD 验证全绿后方可合并。特别关注 bi_analysis_test.rs API 脱节问题（P0），这是测试体系有效性的根本问题。

---

**审计报告完成时间**：2026-07-16
**审计子代理**：V15 审计子代理（类六 测试体系审计类）
**报告路径**：`/workspace/.monkeycode/docs/audits/v15/batch-06/audit-report.md`
