# 构建验证错误报告

**日期**：2026-06-20
**状态**：仅记录，未修复

---

## 一、后端构建错误

### 1. Cargo.toml 重复键错误

**错误信息**：
```
error: duplicate key
   --> Cargo.toml:122:1
    |
122 | redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }
    | ^^^^^
```

**影响**：
- `cargo fmt --check` 失败
- `cargo clippy --all-targets -- -D warnings` 失败
- `cargo build` 失败

**原因**：`Cargo.toml` 文件中第 122 行的 `redis` 依赖重复定义

---

## 二、前端构建错误

### 1. npm ci 失败

**错误信息**：
```
npm error
```

**原因**：依赖安装失败，具体原因需要查看日志文件 `/root/.npm/_logs/2026-06-20T01_43_03_666Z-debug-0.log`

### 2. Vite 构建失败

**错误信息**：
```
[vite]: Rollup failed to resolve import "vue-i18n" from "/workspace/frontend/src/i18n/index.ts".
```

**原因**：`vue-i18n` 模块未安装或未在 `package.json` 中声明

### 3. TypeScript 类型检查失败

**错误总数**：422 行

**主要错误类型**：

#### 3.1 模块导入错误（TS2305/TS2307/TS2613）
- `src/api/ai-extend.ts(5,10)`: Module '"./index"' has no exported member 'request'
- `src/api/bi.ts(5,10)`: Module '"./index"' has no exported member 'request'
- `src/api/color-card.ts(5,10)`: Module '"./index"' has no exported member 'request'
- `src/api/color-price.ts(6,21)`: Cannot find module '@/utils/request'
- `src/api/custom-order.ts(5,10)`: Module '"./index"' has no exported member 'request'
- `src/api/failover.ts(2,8)`: Module has no default export
- `src/api/index.ts(92,1)`: Module './currency' has already exported a member named 'Currency'
- `src/i18n/index.ts(2,28)`: Cannot find module 'vue-i18n'

#### 3.2 隐式 any 类型错误（TS7053）
- `src/components/AfterSalesPanel.vue(18,22)`: Element implicitly has an 'any' type
- `src/components/AfterSalesPanel.vue(25,16)`: Element implicitly has an 'any' type
- `src/components/ProcessFlow.vue(25,28)`: Element implicitly has an 'any' type
- `src/components/ProcessFlow.vue(26,18)`: Element implicitly has an 'any' type
- `src/components/QualityCheck.vue(20,26)`: Element implicitly has an 'any' type
- `src/components/QualityCheck.vue(21,16)`: Element implicitly has an 'any' type

#### 3.3 未使用变量/导入错误（TS6133）
- `src/components/QualityCheck.vue(112,21)`: 'ElMessageBox' is declared but its value is never read
- `src/components/V2Table/index.vue(55,1)`: 'CellRendererParams' is declared but its value is never read
- `src/utils/websocket.ts(85,11)`: 'baseUrl' is declared but its value is never read
- `src/utils/websocket.ts(86,11)`: 'token' is declared but its value is never read
- `src/views/advanced/index.vue(108-112)`: 5 个组件导入未使用
- `src/views/ai-extend/quality-prediction.vue(22,7)`: 'router' is declared but its value is never read
- `src/views/api-gateway/index.vue(94-101)`: 6 个组件导入未使用

#### 3.4 属性不存在错误（TS2339/TS2551）
- `src/components/LanguageSwitcher.vue(28,28)`: Module '"@element-plus/icons-vue"' has no exported member 'Globe'
- `src/views/advanced/tabs/AdvancedAiTab.vue(27,76)`: Property 'emit' does not exist (should be '$emit')
- `src/views/advanced/tabs/AdvancedReportTab.vue(10,41)`: Property 'emit' does not exist (should be '$emit')
- `src/views/advanced/tabs/AdvancedTenantTab.vue(10,41)`: Property 'emit' does not exist (should be '$emit')
- `src/views/api-gateway/index.vue(16-40)`: 多个属性不存在（endpoints, endpointLoading, endpointTotal, endpointQuery, viewKeyDetail, handleToggleKey）

---

## 三、错误汇总

| 模块 | 错误类型 | 数量 | 严重程度 |
|------|----------|------|----------|
| 后端 Cargo.toml | 重复键 | 1 | 高 |
| 前端 npm ci | 依赖安装失败 | 1 | 高 |
| 前端 Vite | 模块未找到 | 1 | 高 |
| 前端 TypeScript | 模块导入错误 | 8 | 中 |
| 前端 TypeScript | 隐式 any 类型 | 6 | 低 |
| 前端 TypeScript | 未使用变量 | 15+ | 低 |
| 前端 TypeScript | 属性不存在 | 10+ | 中 |

---

## 四、修复优先级建议

### P0（必须修复）
1. 修复 `Cargo.toml` 中的重复 `redis` 依赖
2. 安装 `vue-i18n` 依赖
3. 修复 `npm ci` 失败问题

### P1（建议修复）
1. 修复模块导入错误（request 导出）
2. 修复属性不存在错误（api-gateway）
3. 修复 emit 语法错误（AdvancedAiTab 等）

### P2（可选修复）
1. 清理未使用变量/导入
2. 修复隐式 any 类型错误

---

## 五、相关文件

- `backend/Cargo.toml` - 后端依赖配置
- `frontend/package.json` - 前端依赖配置
- `frontend/src/i18n/index.ts` - i18n 配置
- `frontend/src/api/index.ts` - API 导出配置
- `frontend/src/api/request.ts` - 请求工具
