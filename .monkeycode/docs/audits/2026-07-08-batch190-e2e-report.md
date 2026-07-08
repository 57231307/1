# 批次 190 E2E 加强测试报告

> **规则 5 首次执行**（2026-07-08 追加）：每 10 个批次必须完整跑完 E2E 测试一次并给出报告。
> **CI Run**: 28912297000（commit `36d818c7`）
> **E2E Job ID**: 85772198946
> **执行时间**: 2026-07-08 02:09:03Z ~ 02:39:16Z（30 分钟，job timeout 超时终止）

## 一、测试执行总览

| 指标 | 数值 |
|------|------|
| 测试总数 | 95 |
| 失败数 | 88 |
| 通过数 | 0 |
| 未跑完（超时终止） | 7 |
| 结论 | ❌ 全部失败，job 因 30 分钟 timeout 被强制取消 |

## 二、失败原因分类

### 类别 A：环境变量缺失（24 个测试，占 27%）

**错误信息**：
```
Error: E2E 测试需要环境变量 TEST_USERNAME / TEST_PASSWORD（fail-secure 模式，对齐批次 28 P0-1）
```

**影响文件**：
- `e2e/color-card.spec.ts`（5 个测试）
- `e2e/color-price.spec.ts`（3 个测试）
- `e2e/custom-order.spec.ts`（4 个测试）
- 其他使用真实登录的 spec（12 个测试）

**根因**：这些 spec 使用真实登录流程（`login()` 函数），需要 `TEST_USERNAME` / `TEST_PASSWORD` 环境变量，但 CI 中未设置。
**违反规则**：规则 0（未真实接入）+ 规则 5（E2E 必须完整跑完）

### 类别 B：页面元素 Timeout（64 个测试，占 73%）

**错误信息**：
```
Error: Timed out 5000ms waiting for expect(locator).toBeVisible()
Error: Timed out 5000ms waiting for expect(locator).toBeAttached()
Error: Timed out 5000ms waiting for expect(locator).toContainText(expected)
```

**影响文件**：
- `e2e/smoke/*.spec.ts`（5 个 smoke 套件）
- `e2e/sales/01-07.spec.ts`（7 个销售流程套件）
- `e2e/purchase/01-07.spec.ts`（7 个采购流程套件）

**根因**：
1. `playwright.config.ts` 的 `webServer` 仅启动前端 dev server，不启动后端服务
2. `e2e/smoke/_helpers.ts` 的 `mockBusinessApi` 对所有业务 API 返回空数据 `{ items: [], total: 0 }`
3. 空数据导致页面元素（表格、按钮、分页器）不渲染，断言超时

**违反规则**：规则 0（占位符式 mock）+ 规则 2（未完全实现）+ 规则 5（禁止以"已知设计缺陷"推迟修复）

### 类别 C：未跑完（7 个测试，job 超时终止）

**根因**：95 个测试 × 单测试 30s 超时 = 最长 47.5 分钟，超过 job `timeout-minutes: 30`。
最后跑到的测试：`sales/07-report.spec.ts`。

## 三、E2E 配置缺陷分析（违反规则 0/2/5/6）

### 缺陷 1：`continue-on-error: true`（ci-cd.yml:1124）
- E2E 失败不阻塞 CI，违反规则 5"E2E 必须完整跑完"
- 注释中明确写"批次 23 完成真实测试改造后移除"，属于推迟修复（违反规则 0）

### 缺陷 2：`webServer` 仅启动前端（playwright.config.ts:52-59）
- 不启动后端服务，导致真实登录测试和业务流程测试无法运行
- 注释中写"批次 23 真实测试改造后，改为 webServer 数组同时启动前端 + 后端"，属于占位符（违反规则 0/2）

### 缺陷 3：`mockBusinessApi` 返回空数据（_helpers.ts:81-100）
- 对所有业务 API 返回 `{ items: [], total: 0 }`，导致页面元素不渲染
- mock 数据硬编码在函数体内（违反规则 6）

### 缺陷 4：`reporter: 'line'`（playwright.config.ts:35）
- 不生成 HTML 报告，CI 中 `playwright-report/` 目录为空
- artifact 上传警告"No files were found"，无法下载报告分析（违反规则 5）

### 缺陷 5：mock 数据硬编码（_helpers.ts 全文）
- `generateFakeJwt()`、`mockAuthMe()`、`mockInitStatus()` 中的 mock 数据直接内联
- 违反规则 6"测试 mock 数据禁止硬编码"

### 缺陷 6：CI 未设置 TEST_USERNAME/TEST_PASSWORD
- ci-e2e job 未注入 `TEST_USERNAME` / `TEST_PASSWORD` 环境变量
- 导致使用真实登录的 24 个测试全部失败

## 四、修复优先级评估

| 优先级 | 修复项 | 影响测试数 | 违反规则 |
|--------|--------|-----------|---------|
| P0 | 修复 `playwright.config.ts`：reporter 改 html + webServer 改数组（前端+后端） | 95 | 0/2/5 |
| P0 | 修复 `ci-cd.yml` ci-e2e：添加 PostgreSQL service + 编译后端 + 设置环境变量 + 移除 continue-on-error | 95 | 0/2/5 |
| P1 | 设置 CI 环境变量 TEST_USERNAME/TEST_PASSWORD（或 seed 数据库初始化用户） | 24 | 0 |
| P1 | 移除 `mockBusinessApi`（让业务 API 走真实后端） | 64 | 0/2 |
| P2 | 按规则 6 提取 mock 数据到 `e2e/fixtures/` | 全部 | 6 |
| P2 | 优化 timeout 配置（单测试 60s + job timeout 60min） | 7 | 5 |

## 五、修复计划（后续批次）

### 批次 190a（本批次后续提交）
1. 修复 `playwright.config.ts`：`reporter: [['html'], ['line']]` + `webServer` 数组
2. 修复 `ci-cd.yml` ci-e2e job：PostgreSQL service + Rust 编译 + 环境变量 + 移除 continue-on-error + timeout 60min
3. 修复前端单元测试 `tests/components/v2-table.test.ts`（规则 6 fixtures）
4. 提取 mock 数据到 `e2e/fixtures/auth.ts`（规则 6）

### 批次 191-195（E2E 测试文件修复）
1. 移除 `mockBusinessApi`，让业务 API 走真实后端
2. 修复 color-card/color-price/custom-order spec 的真实登录（seed 数据库用户）
3. 修复 smoke/sales/purchase spec 的断言（适配真实后端数据）

### 批次 196-200（E2E 完整验证）
1. E2E 完整跑完，所有测试通过
2. 更新本报告 v2

## 六、结论

批次 190 E2E 测试**全部失败**（0 通过 / 88 失败 / 7 未跑完），根因是 E2E 配置存在 6 类缺陷，全部违反规则 0/2/5/6。按规则 5，这些缺陷必须真实修复，禁止以"已知设计缺陷"推迟。
