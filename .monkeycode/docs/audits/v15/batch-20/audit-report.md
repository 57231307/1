# V15 前端架构与体验审计报告（类二十四·批次 20）

- **审计子代理**：V15 审计子代理（类二十四前端架构与体验）
- **审计范围**：20 维度（24.1 响应式 / 24.2 路由懒加载 / 24.3 Pinia 状态 / 24.4 组件设计 / 24.5 composables / 24.6 ECharts / 24.7 WebSocket / 24.8 性能 bundle / 24.9 Vite 构建 / 24.10 前端测试 / 24.11 XSS / 24.12 敏感数据 / 24.13 可访问性 / 24.14 错误边界 / 24.15 表单验证 / 24.16 i18n / 24.17 权限粒度 / 24.18 路由元信息 / 24.19 API 拦截器 / 24.20 主题暗黑）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 6665-6892 行（类二十四 20 维度检查表）
  - `/workspace/frontend/package.json`、`vite.config.ts`、`tsconfig.json`、`.eslintrc.cjs`、`.prettierrc`、`vitest.config.ts`、`playwright.config.ts`、`env.d.ts`、`index.html`、`nginx.conf`、`.env.development`、`.env.production.example`
  - `/workspace/frontend/src/main.ts`、`App.vue`、`router/index.ts`、`store/{user,dashboard,fabric,inventory,sales,index}.ts`、`api/request.ts`、`api/auth.ts`、`i18n/index.ts`、`utils/{index,storage,logger,lazy-loader,websocket}.ts`、`directives/permission.ts`、`components/Layout/MainLayout.vue`、`components/Charts/{BaseChart,LineChart,BarChart,PieChart}.vue`、`components/V2Table/{index.vue,types.ts}`、`composables/{useTableApi,useTableColumns}.ts`、`locales/{zh-CN,en-US}.ts`
  - Glob 查找：`/workspace/frontend/src/**/*` 视图、组件、API、stores、composables、locales
  - Grep 检索：`v-html` / `localStorage` / `loading="lazy"` / `AbortController` / `debounce|throttle` / `defineProps|defineEmits` / `keep-alive|KeepAlive` / `el-config-provider|dark|theme` / `role=|aria-|tabindex` / `errorCaptured|onErrorCaptured` / `onUnmounted|onBeforeUnmount` / `dataZoom|dataset` / `import * as echarts|from 'echarts'` / `manualChunks|rollupOptions|optimizeDeps` / `Intl\.DateTimeFormat|Intl\.NumberFormat|toLocaleDateString|toLocaleString` / `@media|max-width|min-width` / `Content-Security-Policy|X-Frame-Options|frame-ancestors` / `:deep\(|::v-deep` / `validator:|validateField|validate\(` / `:loading="submitLoading|:loading="loading` / `v-loading|el-skeleton|skeleton` / `NProgress|nprogress` / `defineAsyncComponent|keep-alive` / `prefetch|preload` / `v-permission` / `beforeRouteLeave|beforeunload`
- **审计方法**：Read 审计计划 + Glob 查找 + Grep 检索 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 维度 24.1 前端响应式设计与移动端适配审计

### 检查方法
- Grep 检索：`@media|max-width|min-width`、`loading="lazy"`
- Glob 查找：`/workspace/frontend/public/**`、`/workspace/frontend/**/manifest*`、`/workspace/frontend/**/sw.js`
- Read：`/workspace/frontend/src/components/Layout/MainLayout.vue`、`/workspace/frontend/index.html`

### 发现

#### ✅ 已落实的项
- `index.html:5` 配置 `<meta name="viewport" content="width=device-width, initial-scale=1.0" />` 基础移动端 viewport。
- `MainLayout.vue` 使用 Element Plus 的 `el-container/el-aside/el-header/el-main` 布局组件，响应式基础正确。
- `views/Dashboard.vue:24-35` 使用 `:xs="24"` + `:lg="16"/:lg="8"` 响应式栅格（PC 双列、移动端单列）。
- `MainLayout.vue:14-15` 已为侧边栏菜单添加 `role="menubar"` 与 `:aria-label`，移动端无障碍基础存在。
- `tests/setup.ts:82-105` Mock 了 `window.matchMedia` 与 `IntersectionObserver`，说明测试中已考虑响应式断点逻辑。

#### ❌ 缺陷项

**缺陷 1：无 PWA 支持（manifest.json + Service Worker 全缺）**
- **风险等级：P1**
- **证据**：
  - Glob `/workspace/frontend/**/manifest*` → `No file found`
  - Glob `/workspace/frontend/**/sw.js` → `No file found`
  - `LS /workspace/frontend/public` 仅含 `favicon.ico` 与 `robots.txt`，无 `manifest.json`、无 service worker 注册脚本
  - `index.html` 未引入 `<link rel="manifest">`，未注册 service worker
- **业务影响**：用户无法将 ERP 添加到桌面，离线场景下无法访问任何已加载过的页面，移动端体验降级。计划要求 PWA 支持（manifest.json + Service Worker 离线访问）。
- **修复建议**：
  1. 在 `public/` 下添加 `manifest.json`，声明 `name`/`short_name`/`icons`/`start_url`/`display: standalone`
  2. 在 `index.html` 添加 `<link rel="manifest" href="/manifest.json">`
  3. 引入 `vite-plugin-pwa` 或自研 service worker 注册脚本（缓存首屏关键资源）

**缺陷 2：移动端布局未做侧边栏抽屉化与顶部导航折叠**
- **风险等级：P1**
- **证据**：
  - `MainLayout.vue:3` `<el-aside width="220px">` 写死宽度，无 `<el-drawer>` 移动端抽屉化
  - `MainLayout.vue` 全文未出现 `window.matchMedia` / `useBreakpoint` / `el-drawer` 等响应式判定
  - Grep `@media|max-width|min-width` 在 `MainLayout.vue` 中无匹配，全局 `src` 下仅 `print.ts` 的 `@media print` 匹配（打印场景），无移动端断点
- **业务影响**：移动端访问时左侧 220px 固定侧栏会挤压主内容区，触屏体验差；与计划"移动端必须侧边栏抽屉化 + 顶部导航折叠 + 触屏按钮 ≥ 44px"不符。
- **修复建议**：
  1. 引入 `@vueuse/core` 的 `useBreakpoints` 或 Element Plus 的 `useResizeObserver`，在 `md` 断点以下切换 `<el-aside>` 为 `<el-drawer>`
  2. 顶部 `el-header` 在移动端增加汉堡按钮控制抽屉
  3. 操作按钮显式设置 `size="large"` 或 `min-height: 44px`

**缺陷 3：图片懒加载未启用**
- **风险等级：P2**
- **证据**：Grep `loading="lazy"` 在 `src/` 下无任何匹配；`components/ColorCardGrid.vue` 等含图片的组件未使用 `<img loading="lazy">`。
- **业务影响**：色卡详情、产品图片等场景首次进入页面会同步加载所有图片，移动端 4G 网络下首屏耗时增加。
- **修复建议**：所有 `<img>` 添加 `loading="lazy"` 属性；或使用 `v-lazy`（vue-lazyload）指令；色卡 grid 中的 `background: item.hex_value` 是色块（无网络请求），可豁免。

**缺陷 4：触屏按钮 ≥ 44px 未在全局规范中强制**
- **风险等级：P3**
- **证据**：`.eslintrc.cjs` 无 a11y 规则（如 `jsx-a11y/no-static-element-interactions`），全局样式 `App.vue` 仅有 `body { font-family: ... }`，无 `button { min-height: 44px }` 等移动端规范。
- **业务影响**：移动端误触概率高，但不阻塞 PC 业务。
- **修复建议**：在移动端断点样式表中加入 `el-button--small { min-height: 44px; min-width: 44px; }`。

---

## 维度 24.2 路由懒加载与代码分割审计

### 检查方法
- Read：`/workspace/frontend/src/router/index.ts` 全量
- Grep：`manualChunks|rollupOptions|optimizeDeps`、`prefetch|preload`
- Read：`/workspace/frontend/vite.config.ts`

### 发现

#### ✅ 已落实的项
- `router/index.ts` 中 **所有页面级路由 100% 使用 `() => import('@/views/...')` 懒加载**，共 70+ 条 children 路由，未发现同步 `component: MainLayout` 之外的同步导入（`MainLayout` 是布局壳子，按 Element Plus 模式必须同步）。
- `router/index.ts:35` Login/Setup/403/404 等独立路由也使用 `() => import(...)` 懒加载。
- `vite.config.ts:36` 配置 `sourcemap: false`，生产环境不暴露源码（同时满足 24.9 维度的 Source Map 安全要求）。

#### ❌ 缺陷项

**缺陷 1：未配置 rollupOptions.manualChunks 代码分割策略**
- **风险等级：P1**
- **证据**：`vite.config.ts:33-37` 仅配置 `outDir/assetsDir/sourcemap`，无 `build.rollupOptions.output.manualChunks`；Grep `manualChunks|rollupOptions|optimizeDeps` 在 `vite.config.ts` 中无匹配。
- **业务影响**：所有第三方依赖（vue/element-plus/echarts/axios/pinia/vue-i18n）会合并到一个 vendor chunk，预计 vendor chunk > 1MB（echarts 6 + element-plus 2.6 已经接近 1MB）。计划要求"单 chunk ≤ 500KB，超限告警"，"vendor chunk ≤ 1MB，业务 chunk ≤ 500KB"。
- **修复建议**：在 `vite.config.ts` 添加：
  ```ts
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          'vendor-vue': ['vue', 'vue-router', 'pinia'],
          'vendor-element': ['element-plus', '@element-plus/icons-vue'],
          'vendor-echarts': ['echarts'],
          'vendor-utils': ['axios', 'dompurify', 'print-js', 'vuedraggable'],
          'vendor-i18n': ['vue-i18n'],
        },
      },
    },
  },
  ```

**缺陷 2：未配置 rollup-plugin-visualizer 进行 chunk 体积监控**
- **风险等级：P2**
- **证据**：`package.json` devDependencies 无 `rollup-plugin-visualizer`；`vite.config.ts` 无相关插件引用。
- **业务影响**：无法可视化分析各 chunk 体积，bundle 超限时无告警。
- **修复建议**：安装 `rollup-plugin-visualizer`，在 vite.config.ts 的 plugins 中添加 `visualizer({ open: false, gzipSize: true, brotliSize: true })`，CI 中产出 `stats.html` 供审计。

**缺陷 3：无 prefetch 策略，关键路由未预加载**
- **风险等级：P2**
- **证据**：Grep `prefetch|preload` 在 `src/` 下无匹配；`router/index.ts` 未对登录后默认跳转的 `/dashboard` 配置 `webpackPrefetch`/`<link rel="prefetch">`。
- **业务影响**：登录后用户跳转 dashboard 时才发起 chunk 请求，首次进入增加 200-500ms 延迟。
- **修复建议**：在 `Login.vue` 登录成功后通过 `router.push` 前 `import('@/views/Dashboard.vue')` 触发预加载，或使用 vite 的 `import(/* webpackPrefetch: true */ './Dashboard.vue')`（vite 4+ 通过 `<link rel="modulepreload">` 自动处理，需测试）。

**缺陷 4：首屏性能指标（FCP/LCP/TTI）未监控**
- **风险等级：P3**
- **证据**：`package.json` 无 `web-vitals` 库；`main.ts` 未注入任何 PerformanceObserver 上报逻辑；`scripts/p2-3-perf-test.mjs` 是 Playwright 端的脚本，但未在 CI 中常驻运行。
- **业务影响**：性能回归无数据支撑。
- **修复建议**：引入 `web-vitals` 库，在 `main.ts` 中上报 LCP/FCP/TTI/CLS 到后端 `/api/v1/erp/tracking/page-view`（已有此接口）。

---

## 维度 24.3 Pinia 状态管理与持久化审计

### 检查方法
- Read：`/workspace/frontend/src/store/{index,user,dashboard,fabric,inventory,sales}.ts`
- Grep：未单独执行（5 个 store 已全量读取）

### 发现

#### ✅ 已落实的项
- Store 模块化按业务域拆分：`user/dashboard/fabric/inventory/sales` 共 5 个 store（计划要求 6 个，差 1 个 system store），通过 `store/index.ts` 仅统一导出 `useUserStore`（其他 store 直接 import 路径）。
- Store 全部使用 Composition API 风格（`defineStore('xxx', () => { ... })`），与 Vue 3 + TypeScript 对齐。
- `user.ts:19,42` 对 `permissions` 字段添加 `Object.freeze` 运行时保护，防止前端恶意 push 注入 `admin:write`，安全加固到位。
- `dashboard.ts/fabric.ts/inventory.ts/sales.ts` 全部在 catch 中使用 `logger.error` 记录错误（统一错误处理），并在 finally 中重置 `loading.value = false`（避免 loading 卡死）。
- Store 内部不再操作 `localStorage`：`user.ts:28-30` logout 时仅清空内存 state，Cookie 由后端清理；与 Wave B-3 安全方案一致。

#### ❌ 缺陷项

**缺陷 1：Store 数量未覆盖所有业务域，缺 system store**
- **风险等级：P3**
- **证据**：`store/` 目录仅 5 个文件（user/dashboard/fabric/inventory/sales/index），计划要求 6 个（user/dashboard/fabric/inventory/sales/系统）。系统管理（用户/角色/部门/权限）的状态散落在各 view 内部。
- **业务影响**：系统管理模块的状态未集中管理，跨 Tab 切换时易丢失，但不阻塞业务。
- **修复建议**：新增 `store/system.ts`，集中管理用户列表/角色列表/部门列表/权限列表，供 `system/tabs/UserTab.vue` 等子组件复用。

**缺陷 2：未使用 pinia-plugin-persistedstate 持久化用户偏好**
- **风险等级：P2**
- **证据**：`package.json` dependencies 无 `pinia-plugin-persistedstate`；`main.ts:23` 仅 `app.use(createPinia())` 未注册持久化插件。
- **业务影响**：刷新页面后用户偏好（如表格筛选条件、分页大小、Tab 选中态）全部丢失，需要用户重新设置。计划要求"必须用 pinia-plugin-persistedstate，敏感数据（token）不入 localStorage"。
- **修复建议**：
  1. 安装 `pinia-plugin-persistedstate`
  2. `main.ts` 注册 `pinia.use(piniaPluginPersistedstate)`
  3. 仅对非敏感 store（如 `useDashboardStore` 的 dateRange、`useInventoryStore` 的 queryParams）启用 `persist: true`
  4. **严格禁止**对 `useUserStore.token` 持久化（token 已通过 httpOnly Cookie 由后端管理）

**缺陷 3：Store 跨模块通信未规范化，无事件总线解耦机制**
- **风险等级：P3**
- **证据**：`store/index.ts` 仅 `export { useUserStore } from './user'`，无事件总线（mitt/EventEmitter）封装；5 个 store 之间无直接互调（已避免循环依赖），但缺乏统一的通信规范文档。
- **业务影响**：未来跨模块通信需求（如订单创建后刷新 dashboard 统计）只能通过组件层 emit，扩展性差。
- **修复建议**：评估是否需要引入事件总线；当前业务复杂度下可暂缓，但应在项目规范中明确"store 间不直接互调，通过组件层 emit 或路由参数传递"。

**缺陷 4：Store 单元测试覆盖率不全**
- **风险等级：P2**
- **证据**：`tests/unit/` 下仅有 `user-store.test.ts`、`inventory-store.test.ts` 2 个 store 测试，缺少 `dashboard/fabric/sales` 3 个 store 的测试。
- **业务影响**：dashboard/fabric/sales 三个 store 的 fetchXxx/createXxx/updateXxx/deleteXxx 逻辑无回归保护。计划要求"每个 store 必须有单元测试，覆盖率 ≥ 70%"。
- **修复建议**：补充 `dashboard-store.test.ts`、`fabric-store.test.ts`、`sales-store.test.ts`，覆盖 fetch/create/update/delete 全流程与 catch 分支。

---

## 维度 24.4 组件设计与 Props/Emits 类型安全审计

### 检查方法
- Read：`/workspace/frontend/src/components/Charts/{BaseChart,LineChart,BarChart,PieChart}.vue`、`components/V2Table/index.vue`、`components/AdvancedFilter.vue`、`components/PasswordStrengthMeter.vue`、`components/ColorCardGrid.vue`、`components/BatchActions.vue`
- Grep：`defineProps|defineEmits` 在 src/ 下所有 .vue 文件中的使用

### 发现

#### ✅ 已落实的项
- 公共组件 100% 使用 `defineProps<T>()` 泛型定义 props 类型：
  - `BaseChart.vue:17` `withDefaults(defineProps<Props>(), {...})`
  - `LineChart.vue:37`、`BarChart.vue:37`、`PieChart.vue:35` 均使用 `withDefaults(defineProps<Props>(), {...})`
  - `AdvancedFilter.vue:178`、`PasswordStrengthMeter.vue:35`、`ColorCardGrid.vue:36` 均使用 `defineProps<{...}>()` 或 `defineProps<Props>()`
  - `V2Table/index.vue:46-79` 使用 `<script setup lang="ts" generic="T">` + `defineProps<{...}>()` 泛型组件，行数据类型 T 由调用方自动推导
- 公共组件 100% 使用 `defineEmits<T>()` 泛型定义 emits 类型：
  - `BaseChart.vue:23-26` `defineEmits<{ ready: [...]; click: [...] }>()`
  - `LineChart.vue:48`、`BarChart.vue:48`、`PieChart.vue:46` 同上
  - `AdvancedFilter.vue:197-204`、`ColorCardGrid.vue:37-40` 同上
  - `V2Table/index.vue:81-87` `defineEmits<{ 'page-change': [...]; 'size-change': [...]; 'sort-change': [...]; 'row-click': [...]; refresh: [] }>()`
- Charts 4 组件复用 BaseChart：`LineChart.vue:2-10`、`BarChart.vue:2-10`、`PieChart.vue:2-10` 均 `<BaseChart :option="chartOption" .../>`，无重复 echarts.init 代码，复用到位。
- `Charts/index.ts` 提供统一 barrel 导出（`export { default as BaseChart } from './BaseChart.vue'` 等）。
- `tsconfig.json:14` `"strict": true` + `"noUnusedLocals": true` + `"noUnusedParameters": true`，TypeScript 严格模式开启。
- `.eslintrc.cjs:38` `@typescript-eslint/no-explicit-any` 配置为 `warn`（虽非 error 但有规则），并附详细注释说明历史债务与逐步收紧计划。

#### ❌ 缺陷项

**缺陷 1：`vue/require-explicit-emits` 规则被关闭，emits 声明无强制**
- **风险等级：P2**
- **证据**：`.eslintrc.cjs:29` `'vue/require-explicit-emits': 'off'`
- **业务影响**：组件可以通过 `@event` 触发未在 `defineEmits` 中声明的事件，类型安全被绕过。计划要求"必须用 `defineEmits<T>()` 泛型，事件名 kebab-case"。
- **修复建议**：移除 `'vue/require-explicit-emits': 'off'`，改为默认 error；逐个组件审计并补齐 emits 声明。

**缺陷 2：组件文档注释覆盖不全，多数公共组件无 props/emits/slots 文档**
- **风险等级：P3**
- **证据**：
  - `BaseChart.vue` 无 `/** ... */` props 文档注释，仅 `interface Props` 字段名
  - `LineChart.vue`、`BarChart.vue`、`PieChart.vue` 同样无文档注释
  - `AdvancedFilter.vue:138-170` 有 `FilterValue/FilterField/FilterOperator/FilterCondition/FilterGroup/SavedScheme` 类型注释，但 props 本身无文档
  - 仅 `PasswordStrengthMeter.vue:1-7` 有头部文件级注释说明用途
- **业务影响**：新成员接入组件时需读源码才能理解 API，复用成本高。计划要求"公共组件必须有 props/emits/slots 文档注释"。
- **修复建议**：为 17 个公共组件逐个添加 JSDoc 注释，至少说明 props 类型、默认值、emits 触发时机、slots 用途。

**缺陷 3：`ColorCardGrid.vue` emits 未使用泛型 + kebab-case 风格**
- **风险等级：P3**
- **证据**：`ColorCardGrid.vue:37-40`
  ```ts
  defineEmits<{
    (e: 'scan', item: ColorItemInfo): void
    (e: 'delete', item: ColorItemInfo): void
  }>()
  ```
  使用旧版 call signature 风格，非 kebab-case（应为 `'scan-item'`/`'delete-item'`），与 `V2Table` 的 `'page-change'/'size-change'/'sort-change'/'row-click'` 风格不一致。
- **业务影响**：组件 emits 命名风格不统一，新组件复用时易混淆。
- **修复建议**：统一为 `defineEmits<{ 'scan-item': [item: ColorItemInfo]; 'delete-item': [item: ColorItemInfo] }>()` 风格，并修改父组件监听器。

---

## 维度 24.5 composables 响应式与内存泄漏审计

### 检查方法
- Read：`/workspace/frontend/src/composables/{useTableApi,useTableColumns}.ts`
- Grep：`onUnmounted|onBeforeUnmount` 在 src/ 下所有 .vue 文件中的使用
- Glob：`/workspace/frontend/src/**/composables/**` 查找所有 composables

### 发现

#### ✅ 已落实的项
- 项目内 composables 按"模块嵌套"组织：根级 `src/composables/` 2 个通用 composables（useTableApi/useTableColumns），各 view 内 `views/xxx/composables/` 按业务域拆分（如 `arReconciliation/composables/useArRec.ts`、`bpm/approval/composables/useBpmAp.ts`），命名统一 `useXxx` 前缀。
- `useTableApi.ts:54-184` 正确使用 `ref<T[]>`、`ref<number>`、`ref<Record<string, unknown>>`，未出现 `reactive` 解构丢失响应式问题。
- `useTableApi.ts:144` 错误处理使用 `onError?.(err)` + `throw err`，向上抛出由调用方处理，符合计划要求"composables 必须有 try/catch，错误向上抛出由调用方处理"。
- `useTableApi.ts:167-169` 使用 `watch([page, pageSize], () => fetchData())` 响应式监听分页变化，自动重载。
- 22 个 .vue 文件使用 `onBeforeUnmount` 或 `onUnmounted` 清理资源（charts dispose、timer clearInterval、eventListener removeEventListener），包括：
  - `BaseChart.vue:80-85` dispose echarts 实例 + 移除 resize 监听
  - `Login.vue:296-301` 清理 countdownTimer
  - `admin/failover.vue:122` 清理定时器
  - `scheduling/components/SchGChart.vue:225` 清理 echarts
  - 多个 chart 子组件（CpTrend/DbTrend/DbPie/SaTrend/PriceHistoryChart/SalesAnalysis）均有 onBeforeUnmount

#### ❌ 缺陷项

**缺陷 1：useTableApi 内部 watch 触发的 fetchData 无 onUnmounted 取消订阅**
- **风险等级：P2**
- **证据**：`useTableApi.ts:167-172`
  ```ts
  watch([page, pageSize], () => {
    fetchData()
  })
  fetchData() // 初始加载
  ```
  无 `onScopeDispose` / `onUnmounted` 取消 watch；如果调用方在组件卸载后 page/pageSize 仍被异步修改（极端场景），会触发 fetchData 但组件已销毁。
- **业务影响**：常规场景无问题（watcher 会随组件销毁自动停止），但如果 useTableApi 在非组件上下文（如插件、路由守卫）调用会内存泄漏。
- **修复建议**：在 useTableApi 内部显式获取 watch 返回值，并通过 `onScopeDispose(stop)` 注销；或者在文档中明确"仅可在组件 setup 内使用"。

**缺陷 2：composables 缺乏统一 try/catch 包裹**
- **风险等级：P3**
- **证据**：
  - `useTableApi.ts:139-148` fetchData 有 try/catch
  - `useTableColumns.ts` 全文无 try/catch（仅 computed/filter 操作，理论上不抛错）
  - 各 view 内 composables 未全量审计
- **业务影响**：异步 composables 调用方未 catch 时会 unhandledrejection（已被 `main.ts:19` 全局兜底），但缺少业务层错误提示。
- **修复建议**：在 composables 规范中明确"所有 async composable 函数必须 try/catch 并通过 onError 回调或返回 error ref 暴露错误"。

**缺陷 3：composables 文档注释覆盖率低**
- **风险等级：P3**
- **证据**：
  - `useTableApi.ts:1-10` 有文件头注释
  - `useTableColumns.ts:6` 有简短行内注释
  - 但 36+ 个 view 内 composables 多数无文件头注释（未全量展开，但抽样 arReconciliation/composables/useArChart.ts 无文档）
- **业务影响**：复用成本高。
- **修复建议**：为所有 composables 添加 JSDoc 注释，说明参数、返回值、副作用（如自动 watch、定时器）。

---

## 维度 24.6 ECharts 图表性能与无障碍审计

### 检查方法
- Read：`/workspace/frontend/src/components/Charts/BaseChart.vue`、`components/PriceHistoryChart.vue` 部分
- Grep：`dataZoom|dataset`、`import * as echarts|from 'echarts'`

### 发现

#### ✅ 已落实的项
- `BaseChart.vue:80-85` `onBeforeUnmount` 中 `chartInstance?.dispose()` + `chartInstance = null`，ECharts 实例销毁到位。
- `BaseChart.vue:73-77` 同时使用 `ResizeObserver` + `window.addEventListener('resize', handleResize)`，并在卸载时 `resizeObserver?.disconnect()` + `window.removeEventListener('resize', handleResize)`，resize 监听清理到位。
- `BaseChart.vue:42-47` 调用 `chartInstance.showLoading({...})` 显示加载状态，加载完成通过 `props.loading` 切换。
- `BaseChart.vue:48` `chartInstance.on('click', params => emit('click', params))` 暴露点击事件给父组件。
- `PriceHistoryChart.vue:90` 与 `scheduling/components/SchGChart.vue:142` 在大数据场景使用 `dataZoom` 分段加载。

#### ❌ 缺陷项

**缺陷 1：未使用 echarts/core 按需引入，全量引入 echarts**
- **风险等级：P1**
- **证据**：
  - `BaseChart.vue:7` `import * as echarts from 'echarts'`（全量引入，约 1MB+）
  - `PriceHistoryChart.vue:11` `import * as echarts from 'echarts/core'`（仅 1 个文件按需引入）
  - `capacity/components/CpTrend.vue:31`、`dashboard/components/DbTrend.vue:27`、`DbPie.vue:17`、`SalesAnalysis.vue:16`、`SchGChart.vue:25`、`SaTrend.vue:37`、`arReconciliation/composables/useArChart.ts:8` 均使用 `import * as echarts from 'echarts'` 全量引入
- **业务影响**：bundle 体积增加约 800KB（gzip 后约 250KB），首屏加载延迟。计划要求"必须用 echarts/core tree-shaking，禁止全量引入"。
- **修复建议**：所有使用 echarts 的文件改为：
  ```ts
  import * as echarts from 'echarts/core'
  import { LineChart, BarChart, PieChart } from 'echarts/charts'
  import { GridComponent, TooltipComponent, LegendComponent, DataZoomComponent } from 'echarts/components'
  import { CanvasRenderer } from 'echarts/renderers'
  echarts.use([LineChart, BarChart, PieChart, GridComponent, TooltipComponent, LegendComponent, DataZoomComponent, CanvasRenderer])
  ```

**缺陷 2：图表无 ARIA 标签，屏幕阅读器不友好**
- **风险等级：P3**
- **证据**：`BaseChart.vue:1-3` 模板仅 `<div ref="chartRef" class="chart-container">`，无 `role="img"` / `aria-label` / `title` 描述图表内容。
- **业务影响**：视障用户无法理解图表展示的数据。
- **修复建议**：在 BaseChart props 中增加 `alt?: string` 字段，渲染为 `<div role="img" :aria-label="alt">`，调用方传入图表摘要（如"2026 年 7 月销售趋势折线图，最高 50 万"）。

**缺陷 3：dataZoom 仅在 2 个组件使用，其他图表未评估大数据场景**
- **风险等级：P3**
- **证据**：Grep `dataZoom` 仅匹配 `PriceHistoryChart.vue:90` 与 `SchGChart.vue:142`，其他图表（LineChart/BarChart/PieChart/CpTrend/DbTrend/DbPie/SaTrend）未配置 dataZoom。
- **业务影响**：当订单流水、销售统计等场景数据 >10000 点时，图表会卡顿。
- **修复建议**：在 BaseChart props 中增加 `enableDataZoom?: boolean`，默认大数据场景自动启用 inside+slider dataZoom。

---

## 维度 24.7 WebSocket 客户端连接重连心跳审计

### 检查方法
- Read：`/workspace/frontend/src/utils/websocket.ts`

### 发现

#### ✅ 已落实的项
- `websocket.ts:62` 心跳间隔 `HEARTBEAT_INTERVAL = 30000`（30s ping），符合计划要求。
- `websocket.ts:65-71` 重连参数：`MAX_RECONNECT_DELAY = 30000`（30s 上限）、`INITIAL_RECONNECT_DELAY = 1000`（1s 起步）、`MAX_RECONNECT_ATTEMPTS = 10`（最大 10 次），完全符合计划"指数退避（1s→30s），最大 10 次重连"。
- `websocket.ts:243-265` `scheduleReconnect` 实现指数退避：`delay = Math.min(1000 * 2^(attempts-1), 30000)`，并在超限时 `dispatchEvent('max_reconnect_failed')`，符合计划"超限降级轮询"（虽然未实现轮询，但通过事件暴露给调用方）。
- `websocket.ts:270-275` `startHeartbeat` 发送 `{ type: 'ping', timestamp: Date.now() }`。
- `websocket.ts:173-184` `disconnect` 主动关闭时清理 `heartbeatTimer` + `reconnectTimer`，内存清理到位。
- `websocket.ts:78-92` 文档说明：v12 P1-4 改用一次性短时票据鉴权，不再通过 URL query 传递 JWT，票据 30s 过期，避免 JWT 泄露到浏览器历史/服务器日志。完全符合计划"票据鉴权 + 30s 过期"。
- `websocket.ts:127-130` 每次连接（含重连）都重新获取新票据，避免票据复用。
- `websocket.ts:200-207` 重写 `addEventListener` 提供类型安全的事件监听（泛型 K extends keyof WebSocketEventMap）。

#### ❌ 缺陷项

**缺陷 1：心跳超时 60s 断开未实现**
- **风险等级：P2**
- **证据**：`websocket.ts:270-285` `startHeartbeat` 仅发送 ping，未监听 pong 响应；如果服务端无响应，客户端会持续发送 ping 但不会主动断开重连。
- **业务影响**：网络假死（TCP 连接存在但服务端不响应）时客户端不会重连。计划要求"超时 60s 断开重连"。
- **修复建议**：在 startHeartbeat 中记录 `lastPongTime`，onmessage 收到 pong 时更新；启动一个 60s 间隔的检查定时器，若 `Date.now() - lastPongTime > 60000` 则 `ws.close()` 触发重连。

**缺陷 2：超过最大重连次数后未降级为轮询**
- **风险等级：P3**
- **证据**：`websocket.ts:245-248`
  ```ts
  if (this.reconnectAttempts > MAX_RECONNECT_ATTEMPTS) {
    this.dispatchEvent(new CustomEvent('max_reconnect_failed'));
    return;
  }
  ```
  仅触发事件返回，未实现轮询降级。
- **业务影响**：10 次重连失败后 WebSocket 永久停止，通知模块不再接收实时消息。
- **修复建议**：在 max_reconnect_failed 事件回调中由调用方启动轮询（如每 30s 拉取 `/api/v1/erp/notifications`），websocket.ts 本身可不实现轮询但应在文档中明确"由调用方降级"。

**缺陷 3：票据鉴权未评估 URL query 传递的泄露风险**
- **风险等级：P3**
- **证据**：`websocket.ts:130` `const url = ${this.baseUrl}?ticket=${encodeURIComponent(ticket)}` 票据仍通过 URL query 传递。
  - 文档说明票据 30s 过期 + 一次性消费，即使泄露也无法复用
  - 但 URL query 仍可能被浏览器历史、nginx access log、CDN log 记录
- **业务影响**：理论风险存在但被 30s 过期 + 一次性消费抵消，实际风险低。
- **修复建议**：评估后端是否支持 WebSocket 子协议（Sec-WebSocket-Protocol）传递票据，避免 URL query；或在 nginx 配置中过滤 access log 中的 ticket 参数。

---

## 维度 24.8 前端性能与 bundle 体积审计

### 检查方法
- Read：`/workspace/frontend/vite.config.ts`、`package.json`
- Grep：`debounce|throttle`、`loading="lazy"`

### 发现

#### ✅ 已落实的项
- `utils/index.ts:33-42` 提供统一的 `debounce` 函数实现，供全项目复用。
- 22+ 个 .vue 文件使用 `onBeforeUnmount` / `onUnmounted` 清理 ref/timer/eventListener（详见 24.5 维度）。
- `vite.config.ts:11-16` 配置 `unplugin-auto-import` + `unplugin-vue-components` + `ElementPlusResolver()`，Element Plus 组件按需自动导入（避免全量引入 element-plus）。
- `utils/lazy-loader.ts` 提供 `createLazyLoader` / `loadIfNot` / `resetLoad` 工具，支持 Tab 切换时按需加载数据，减少首屏并发请求。

#### ❌ 缺陷项

**缺陷 1：debounce/throttle 在搜索/滚动/resize 场景覆盖率低**
- **风险等级：P2**
- **证据**：Grep `debounce|throttle` 在 `src/` 下仅 `utils/index.ts` 1 个文件匹配（仅工具函数定义），未在任何 view 的搜索输入框、滚动监听、resize 监听处使用。
  - AdvancedFilter 等筛选组件无 debounce
  - MainLayout 等布局组件无 resize debounce
- **业务影响**：用户在搜索框快速输入时，每次按键都会触发 API 请求，造成请求风暴；window resize 时图表 resize 高频触发。
- **修复建议**：
  1. 在搜索输入组件 `@input` 处使用 `debounce(handleSearch, 300)`
  2. 在图表 resize 处使用 `throttle(handleResize, 200)`
  3. 在表格滚动加载处使用 `throttle(handleScroll, 100)`

**缺陷 2：bundle 体积未监控，无 visualizer 报告**
- **风险等级：P2**
- **证据**：同 24.2 缺陷 2，`package.json` 无 `rollup-plugin-visualizer`，CI 无 bundle 体积报告。
- **业务影响**：bundle 超限时无告警，无法定位体积来源。
- **修复建议**：同 24.2 缺陷 2。

**缺陷 3：Tree Shaking 未清零，`@typescript-eslint/no-explicit-any` warn 模式下 any 残留**
- **风险等级：P3**
- **证据**：`.eslintrc.cjs:34-38` 注释说明"main 分支累计 800+ 处 any，设为 error 会阻塞所有 PR。临时降级为 warn 以解锁 CI"。
- **业务影响**：未使用的导出可能因 any 类型不被 tree shaking 识别，bundle 体积膨胀。
- **修复建议**：按 `docs/tech-debt/no-explicit-any-rollout.md` 计划逐步收紧 any 规则为 error。

**缺陷 4：未启用 vite build 的 chunk size warning 自定义阈值**
- **风险等级：P3**
- **证据**：`vite.config.ts:33-37` 未配置 `build.chunkSizeWarningLimit`，使用默认 500KB 阈值。
- **业务影响**：默认阈值合理，但未与计划要求对齐。
- **修复建议**：可显式 `chunkSizeWarningLimit: 500` 明确策略。

---

## 维度 24.9 Vite 构建与 Tree Shaking 审计

### 检查方法
- Read：`/workspace/frontend/vite.config.ts`、`env.d.ts`、`.env.development`、`.env.production.example`

### 发现

#### ✅ 已落实的项
- `vite.config.ts:33-37` 配置 `build.sourcemap: false`，生产环境禁用 source map，符合计划"生产环境必须禁用 source map（或加密）"。
- `vite.config.ts:18-22` 配置 `resolve.alias: { '@': resolve(__dirname, 'src') }`，路径别名统一。
- `vite.config.ts:23-32` 配置 dev server proxy `/api` → `http://localhost:8082`，解决开发环境跨域。
- `env.d.ts:1` `/// <reference types="vite/client" />` 引入 vite 客户端类型，`import.meta.env.VITE_*` 有类型定义（部分）。
- `.env.production.example:1-44` 提供 11 项配置模板（API/timeout/mock/debug/CDN/第三方密钥），并明确"敏感 secret 必须在后端"。
- `.env.development:1-17` 配置开发环境变量，`VITE_USE_MOCK=true` 但实际未启用 mock（package.json 无 mock-server 依赖）。

#### ❌ 缺陷项

**缺陷 1：未配置 build.optimizeDeps 与 manualChunks**
- **风险等级：P1**
- **证据**：`vite.config.ts` 无 `optimizeDeps` 配置，无 `rollupOptions.output.manualChunks`。同 24.2 缺陷 1。
- **业务影响**：开发环境首次启动慢（element-plus + echarts 全量预构建）；生产环境 vendor chunk 单一过大。
- **修复建议**：同 24.2 缺陷 1，并增加 `optimizeDeps: { include: ['vue', 'vue-router', 'pinia', 'element-plus', 'echarts', 'axios'] }`。

**缺陷 2：未按业务域分割 chunk（finance/sales/inventory/production）**
- **风险等级：P2**
- **证据**：`vite.config.ts` 无任何 manualChunks 配置；router 中虽然路由懒加载，但相同业务域的多个 view（如 finance 下 17 个路由）会被分到独立 chunk，跨域共享的 utils 会被重复打包。
- **业务影响**：业务 chunk 体积未优化。
- **修复建议**：按业务域配置 manualChunks：
  ```ts
  manualChunks(id) {
    if (id.includes('node_modules')) return 'vendor'
    if (id.includes('views/finance') || id.includes('views/ap') || id.includes('views/ar')) return 'finance'
    if (id.includes('views/sales') || id.includes('views/quotations') || id.includes('views/sales-')) return 'sales'
    if (id.includes('views/inventory') || id.includes('views/warehouse')) return 'inventory'
    if (id.includes('views/production') || id.includes('views/bom') || id.includes('views/mrp')) return 'production'
  }
  ```

**缺陷 3：env.d.ts 未声明 VITE_* 自定义环境变量类型**
- **风险等级：P3**
- **证据**：`env.d.ts:1-7` 仅引入 `vite/client` 类型与 `.vue` 模块声明，无 `interface ImportMetaEnv { readonly VITE_API_BASE_URL: string; readonly VITE_API_TIMEOUT: number; ... }`。
- **业务影响**：`import.meta.env.VITE_API_BASE_URL` 类型为 `any | undefined`，运行时拼字符串风险存在。计划要求"import.meta.env.VITE_* 必须有类型定义"。
- **修复建议**：在 `env.d.ts` 添加：
  ```ts
  interface ImportMetaEnv {
    readonly VITE_API_BASE_URL: string
    readonly VITE_API_TIMEOUT: string
    readonly VITE_USE_MOCK: string
    readonly VITE_DEBUG: string
    readonly VITE_SHOW_ERROR_DETAILS: string
    readonly VITE_APP_TITLE: string
    readonly VITE_APP_VERSION: string
  }
  interface ImportMeta {
    readonly env: ImportMetaEnv
  }
  ```

---

## 维度 24.10 前端测试覆盖率与 mock fixtures 审计

### 检查方法
- Glob：`/workspace/frontend/tests/**/*.test.ts`、`/workspace/frontend/tests/**/*.spec.ts`
- Read：`/workspace/frontend/vitest.config.ts`、`tests/setup.ts`、`tests/unit/login.test.ts`、`tests/unit/user-store.test.ts`
- Read：`/workspace/frontend/playwright.config.ts`

### 发现

#### ✅ 已落实的项
- `vitest.config.ts:1-39` 配置完整：
  - `environment: 'jsdom'`（DOM 环境）
  - `coverage.provider: 'v8'`、`reporter: ['text', 'json', 'html']`
  - `include: ['tests/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}']`
  - `exclude: ['node_modules', 'dist', '.idea', '.git', '.cache', 'e2e/**']`（正确排除 e2e）
  - `setupFiles: ['./tests/setup.ts']`
- `tests/setup.ts:1-118` 完整的 mock 设置：
  - Mock Element Plus（保留真实导出 + ElTableV2/ElAutoResizer 测试桩）
  - Mock vue-router / pinia / axios
  - Mock `window.matchMedia` / `IntersectionObserver` / `ResizeObserver` 浏览器 API
- 单元测试覆盖：12 个 .test.ts 文件（v2-table 组件测试 / use-table-columns composable 测试 / password-strength-meter / login / user-store / storage / slow-query / inventory-store / v2-table / request / audit-log / utils）
- E2E 测试：`e2e/` 下有 `purchase/`、`sales/`、`smoke/`、`enhanced/` 4 大类共 20+ 个 .spec.ts 文件，覆盖采购/销售完整流程、冒烟测试、多角色协同、网络韧性、RPA 数据提取。
- `playwright.config.ts:1-60` 配置完整：多浏览器（chromium/firefox/webkit）、HTML+line reporter、60s 超时、CI 自动启动 dev server、trace on-first-retry、screenshot only-on-failure。
- `tests/fixtures/v2-table.ts` 存在 fixtures 文件，符合计划"测试 mock 数据必须 fixtures 化"。

#### ❌ 缺陷项

**缺陷 1：测试覆盖率未在 CI 中强制 ≥ 70%**
- **风险等级：P1**
- **证据**：
  - `vitest.config.ts:21-30` 配置了 coverage but 无 `thresholds` 字段（如 `thresholds: { statements: 70, branches: 70, functions: 70, lines: 70 }`）
  - `package.json:12` `"test:coverage": "vitest run --coverage"` 仅运行不校验阈值
  - 12 个 .test.ts 文件对应 75+ views + 17 components + 36 composables + 5 stores + 85+ api 文件，覆盖率估算 < 15%
- **业务影响**：测试覆盖率无法保障，回归风险高。计划要求"覆盖率 ≥ 70%（statements/branches/functions/lines）"。
- **修复建议**：
  1. 在 `vitest.config.ts` 添加 `coverage.thresholds: { statements: 70, branches: 70, functions: 70, lines: 70 }`
  2. CI 中 `vitest run --coverage` 失败时阻断合并
  3. 优先补充 5 个 store + 17 个公共组件 + 36 composables 的测试，再逐步覆盖 views

**缺陷 2：E2E 测试仅覆盖采购/销售，未覆盖财务/库存/生产/CRM 等核心模块**
- **风险等级：P2**
- **证据**：`e2e/` 下仅 `purchase/`（7 spec）、`sales/`（7 spec）、`smoke/`（5 spec）、`enhanced/`（3 spec）、3 个独立 spec（color-card/color-price/custom-order），共 25 个 spec。财务（finance/ap/ar/voucher）、库存（inventory/transfer/count）、生产（production/bom/mrp/scheduling）、CRM（crm/leads/opportunities）等核心模块无 E2E 测试。
- **业务影响**：核心财务/库存/生产流程回归无自动化保护。
- **修复建议**：按业务优先级补充 finance/inventory/production/CRM 的 E2E 测试，每模块至少 3-5 个核心流程 spec。

**缺陷 3：fixtures 化不彻底**
- **风险等级：P3**
- **证据**：
  - `tests/fixtures/v2-table.ts` 存在
  - `e2e/fixtures/` 下有 `auth.ts`、`multi-context.ts`、`network.ts`、`rpa.ts` 4 个 fixtures
  - 但 `tests/unit/login.test.ts:25-46` 在测试文件内联 mock 数据（如 `mockLogin`、`mockCheckLockStatus` 返回值），未抽到 fixtures
  - `tests/unit/user-store.test.ts:42-45` 内联 `mockResponse` 数据
- **业务影响**：mock 数据散落在测试文件，维护成本高。
- **修复建议**：将测试文件内的内联 mock 数据抽到 `tests/fixtures/` 下，如 `tests/fixtures/login.ts`、`tests/fixtures/user.ts`。

---

## 维度 24.11 前端 XSS 防护与 CSP 策略审计

### 检查方法
- Grep：`v-html` 在 src/ 下
- Read：`/workspace/frontend/src/views/report-templates/index.vue`、`views/print-templates/index.vue` 关键段
- Grep：`Content-Security-Policy|X-Frame-Options|frame-ancestors`
- Read：`/workspace/frontend/nginx.conf`

### 发现

#### ✅ 已落实的项
- `v-html` 使用点仅 2 处（`report-templates/index.vue:171`、`print-templates/index.vue:213`），且 100% 经过 DOMPurify 消毒：
  - `report-templates/index.vue:186` `import DOMPurify from 'dompurify'`
  - `report-templates/index.vue:337-345` `DOMPurify.sanitize(previewData.value, { USE_PROFILES: { html: true }, FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form'], FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover'] })`
  - 同时引入 `escapeHtml`（v14 批次 243 修复）对单元格值二次转义
- `package.json:21` `"dompurify": "^3.1.6"` 已安装 DOMPurify 依赖。
- `.eslintrc.cjs:27` `'vue/no-v-html': 'off'` 关闭了 v-html 警告（但实际项目中已通过 DOMPurify 兜底，关闭规则合理）。

#### ❌ 缺陷项

**缺陷 1：nginx.conf 未配置 Content-Security-Policy 头**
- **风险等级：P1**
- **证据**：`nginx.conf:1-50` 全文无 `add_header Content-Security-Policy` 配置；Grep `Content-Security-Policy|X-Frame-Options|frame-ancestors` 在 `frontend/` 下无匹配。
- **业务影响**：浏览器无 CSP 防护，若 XSS 漏洞被利用可加载外部脚本。计划要求"必须配置 Content-Security-Policy 头，禁止 unsafe-inline"。
- **修复建议**：在 `nginx.conf` 的 `location /` 块添加：
  ```nginx
  add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'self' wss: ws:; frame-ancestors 'none';" always;
  add_header X-Frame-Options "DENY" always;
  add_header X-Content-Type-Options "nosniff" always;
  add_header Referrer-Policy "strict-origin-when-cross-origin" always;
  ```

**缺陷 2：未配置 X-Frame-Options 防 Clickjacking**
- **风险等级：P2**
- **证据**：同缺陷 1，`nginx.conf` 无 `X-Frame-Options` 配置。
- **业务影响**：恶意站点可通过 iframe 嵌入 ERP 钓鱼用户操作。
- **修复建议**：同缺陷 1，添加 `add_header X-Frame-Options "DENY" always;`（或通过 CSP `frame-ancestors 'none'`）。

**缺陷 3：vue/no-v-html 规则关闭，依赖人工审查**
- **风险等级：P3**
- **证据**：`.eslintrc.cjs:27` `'vue/no-v-html': 'off'`
- **业务影响**：未来新增 v-html 使用点时无 lint 拦截，可能遗漏 DOMPurify 消毒。
- **修复建议**：改为 `'vue/no-v-html': 'warn'`，并在 overrides 中对 `report-templates/index.vue`、`print-templates/index.vue` 两个已审计文件单独 `off`，其他文件 warn 提示人工审查。

---

## 维度 24.12 敏感数据存储与 token 安全审计

### 检查方法
- Grep：`localStorage` 在 src/ 下
- Read：`/workspace/frontend/src/utils/storage.ts`、`api/request.ts`、`api/auth.ts`、`store/user.ts`、`views/system/tabs/CompanyTab.vue`、`i18n/index.ts`

### 发现

#### ✅ 已落实的项
- **Wave B-3 安全方案完整落地**：
  - `api/request.ts:4` 注释明确"移除 access_token / refresh_token 的 localStorage 引用"
  - `api/request.ts:69` `withCredentials: true` 开启凭据发送，httpOnly Cookie 由浏览器自动携带
  - `api/request.ts:82-84` 注释"不再手动注入 Authorization 头，凭据由 httpOnly Cookie 在 withCredentials=true 时由浏览器自动发送"
  - `store/user.ts:8` `token = ref<string | null>(null)` 仅内存引用，不入 localStorage
  - `store/user.ts:28-30` logout 时仅清空内存 state，Cookie 由后端清理
  - `api/auth.ts:25-29` login 不再写 localStorage，Cookie 由后端 Set-Cookie 自动写入
  - `api/auth.ts:46-50` refreshToken 入参保留但已无效，Cookie 自动携带
- `utils/storage.ts:1-54` 仅保留 `getCsrfToken` / `loadCsrfToken` / `clearCsrfToken` 三个工具（CSRF Token 是非 httpOnly Cookie，需 JS 读取注入头），旧的 setToken/getToken/removeToken/setRefreshToken/getRefreshToken 已废弃删除。
- `views/system/tabs/CompanyTab.vue:154-168` FE-P2-4 修复：过滤敏感字段（credit_code/legal_representative/bank_name/bank_account/tax_registration_number），仅缓存非敏感信息到 localStorage。
- `i18n/index.ts:21` STORAGE_KEY = 'bingxi.locale'，仅存储用户语言偏好（zh-CN/en-US），非敏感数据。
- `views/Login.vue:224-231` `safeRedirect` 函数实现开放重定向白名单校验：拒绝绝对 URL、协议相对 URL、反斜杠路径，仅允许以单个 `/` 开头的相对路径。完全符合计划"redirect query 参数必须白名单校验"。

#### ❌ 缺陷项

**缺陷 1：CSRF Token 通过 document.cookie 读取，存在 CSRF 防护绕过风险**
- **风险等级：P3**
- **证据**：`utils/storage.ts:16-29` `readCookie` 函数从 `document.cookie` 读取 `csrf_token`。
  - 这是 CSRF 防护的标准实现（Double Submit Cookie 模式），csrf_token 同时存在于 Cookie 和请求头
  - 但如果存在 XSS 漏洞，攻击者可读取 csrf_token Cookie 并构造请求
- **业务影响**：标准 CSRF 防护方案，风险可控；但依赖 XSS 防护到位。
- **修复建议**：当前实现是行业最佳实践，无需修改；重点是确保 XSS 防护（24.11 维度）到位。

**缺陷 2：CompanyTab.vue 缓存公司信息到 localStorage，未评估业务合理性**
- **风险等级：P3**
- **证据**：`CompanyTab.vue:168` `localStorage.setItem('company_info', JSON.stringify(nonSensitiveFields))`
  - 虽然已过滤敏感字段，但公司名称/电话/邮箱/地址等仍属于企业信息
  - 缓存目的不明确（后端有 /system/company 接口可每次获取）
- **业务影响**：XSS 攻击时可读取企业基础信息。
- **修复建议**：评估是否真的需要本地缓存，若仅用于"减少 API 调用"，可改为 Pinia store + 内存缓存（不持久化）。

**缺陷 3：前端密钥泄露审计未发现，但 .env.production.example 提示规则**
- **风险等级：P3**
- **证据**：
  - Grep `localStorage` 仅匹配 i18n/CompanyTab/user/request/auth，无 API Key/第三方密钥相关
  - `.env.production.example:40-44` 注释"仅前端可公开的 key 才能放在这里，敏感 secret 必须在后端"
  - 当前未实际配置任何第三方密钥（无 VITE_BAIDU_ANALYTICS_ID/VITE_SENTRY_DSN/VITE_GA_ID）
- **业务影响**：当前无密钥泄露，但未来接入第三方分析时需遵循此规则。
- **修复建议**：在项目规范中明确"前端 .env 仅允许公开 key，secret 走后端代理"，并在 CI 中扫描 .env 文件中 SECRET/PRIVATE/PASSWORD 关键字。

---

## 维度 24.13 前端可访问性 WCAG 2.1 AA 审计

### 检查方法
- Grep：`role=|aria-|tabindex` 在 src/ 下
- Read：`/workspace/frontend/src/components/Layout/MainLayout.vue`、`views/Login.vue`

### 发现

#### ✅ 已落实的项
- `MainLayout.vue:14-15` `<el-menu ... role="menubar" :aria-label="$t('layout.menuAriaLabel')">` 为侧边栏菜单添加 menubar 角色与 aria-label。
- `MainLayout.vue:19-186` 所有 `<el-menu-item>` 添加 `role="menuitem"`，所有 `<el-sub-menu>` 添加 `role="menuitem" aria-haspopup="true" :aria-expanded="openedMenus.includes('xxx')"`，并维护 `openedMenus` 状态（P0 4-3 修复）。
- `Login.vue:33-40` `<el-form ... role="form" :aria-label="$t('login.formLabel')">` 为登录表单添加 form 角色与 aria-label。
- `Login.vue:42-63` `el-form-item` 与 `el-input` 均添加 `:aria-label="$t('login.username')"` 等，确保屏幕阅读器朗读字段用途。
- `Login.vue:61` 密码输入框 `@keyup.enter="handleLogin"` 支持键盘 Enter 提交。

#### ❌ 缺陷项

**缺陷 1：键盘导航 Tab/Shift+Tab/Esc 焦点管理未全局规范化**
- **风险等级：P1**
- **证据**：
  - Grep `tabindex` 在 src/ 下无匹配（仅 MainLayout.vue 的 aria-expanded 相关）
  - 无全局焦点管理工具（如 `useFocusTrap`、`useFocusManagement`）
  - 模态框（el-dialog）依赖 Element Plus 默认焦点管理，但未配置 `:focus-on-close`/`append-to-body` 等无障碍选项
- **业务影响**：视障用户依赖键盘导航时，模态框关闭后焦点可能丢失，无法返回触发按钮。计划要求"必须支持 Tab/Shift+Tab/Esc/Enter 焦点管理，禁用 tabindex > 0"。
- **修复建议**：
  1. 引入 `@vueuse/core` 的 `useFocusTrap` 为所有 el-dialog 启用焦点陷阱
  2. 在路由守卫 `router.afterEach` 中重置焦点到 `document.body`
  3. 在 ESLint 中添加 `jsx-a11y/no-tabindex` 规则禁止 `tabindex > 0`

**缺陷 2：色彩对比度未自动检测**
- **风险等级：P2**
- **证据**：
  - `MainLayout.vue:11` `text-color="#bfcbd9"`（侧边栏文字颜色）+ `background-color="#304156"`（侧边栏背景色）对比度约 5.7:1，符合 WCAG AA
  - 但 `MainLayout.vue:12` `active-text-color="#409eff"`（菜单激活态文字）+ `background-color="#304156"` 对比度约 3.8:1，**低于 WCAG AA 4.5:1 要求**
  - 项目无自动对比度检测工具（如 `axe-core`、`pa11y`）
- **业务影响**：视障用户难以辨认当前激活的菜单项。
- **修复建议**：
  1. 将 `active-text-color` 调整为 `#67c23a`（绿色）或 `#ffffff`（白色），对比度 > 7:1
  2. 引入 `axe-core` 在 E2E 测试中自动检测对比度
  3. CI 中添加 `pa11y-ci` 扫描关键页面

**缺陷 3：图表无 ARIA 标签（同 24.6 缺陷 2）**
- **风险等级：P3**
- **证据**：同 24.6 缺陷 2。
- **业务影响**：视障用户无法理解图表内容。
- **修复建议**：同 24.6 缺陷 2。

**缺陷 4：路由切换后焦点未重置**
- **风险等级：P2**
- **证据**：`router/index.ts:876-928` `router.beforeEach` 中无焦点重置逻辑；Grep `document.activeElement` 在 src/ 下无匹配。
- **业务影响**：键盘用户切换路由后焦点停留在原位置（如侧边栏菜单），需要多次 Tab 才能进入主内容区。
- **修复建议**：在 `router.afterEach` 中 `document.body.focus()` 或将焦点移至 `<main>` 元素（需添加 `tabindex="-1"`）。

---

## 维度 24.14 错误边界与全局错误处理审计

### 检查方法
- Grep：`errorCaptured|onErrorCaptured`
- Read：`/workspace/frontend/src/main.ts`、`App.vue`

### 发现

#### ✅ 已落实的项
- `main.ts:15-17` 配置全局错误处理：
  ```ts
  app.config.errorHandler = (err, _instance, info) => {
    console.error('[Vue 错误]', err, info)
  }
  ```
- `main.ts:19-21` 监听 `unhandledrejection` 事件捕获未处理的 Promise rejection：
  ```ts
  window.addEventListener('unhandledrejection', (event) => {
    console.error('[未捕获 Promise]', event.reason)
  })
  ```
- `utils/logger.ts:1-49` 提供统一日志工具，生产环境自动禁用（`this.enabled = import.meta.env.DEV`），支持 debug/info/warn/error 4 级日志。

#### ❌ 缺陷项

**缺陷 1：未实现 Vue 3 ErrorBoundary 组件**
- **风险等级：P1**
- **证据**：Grep `errorCaptured|onErrorCaptured` 在 src/ 下无匹配；`components/` 下无 ErrorBoundary.vue 组件；`App.vue:1-5` 仅 `<router-view />`，无错误边界包裹。
- **业务影响**：组件渲染异常时整个应用白屏，用户无法继续操作。计划要求"必须用 Vue 3 errorCaptured 钩子实现 ErrorBoundary，错误时显示降级 UI"。
- **修复建议**：
  1. 新建 `components/ErrorBoundary.vue`，使用 `errorCaptured` 钩子捕获子组件错误，渲染降级 UI（如"页面加载失败，请刷新重试"）
  2. 在 `App.vue` 中用 `<ErrorBoundary><router-view /></ErrorBoundary>` 包裹
  3. 关键路由（如 Dashboard）独立包裹 ErrorBoundary，避免单页崩溃影响全局

**缺陷 2：未接入前端监控（Sentry/Bugsnag/自研）**
- **风险等级：P1**
- **证据**：
  - `package.json` dependencies 无 `@sentry/vue` / `@bugsnag/js` 等监控 SDK
  - `main.ts:15-17` 错误处理仅 `console.error`，未上报到监控系统
  - `.env.production.example:42-44` 注释"VITE_SENTRY_DSN="（占位但未启用）
- **业务影响**：生产环境错误无法实时告警与归因。计划要求"必须接入 Sentry/Bugsnag/自研监控，错误自动上报"。
- **修复建议**：
  1. 安装 `@sentry/vue`
  2. `main.ts` 初始化 Sentry：`Sentry.init({ Vue, dsn: import.meta.env.VITE_SENTRY_DSN, environment: import.meta.env.MODE })`
  3. 在 `app.config.errorHandler` 中调用 `Sentry.captureException(err)`
  4. 后端实现 `/api/v1/erp/tracking/error` 接口接收前端错误上报

**缺陷 3：错误去重未实现**
- **风险等级：P2**
- **证据**：`main.ts:15-17` errorHandler 每次错误都 console.error，无 5min 内去重逻辑。
- **业务影响**：相同错误循环触发时控制台爆炸，监控上报风暴。计划要求"相同错误 5min 内不重复上报"。
- **修复建议**：实现错误指纹（`err.message + info` 哈希），5min 内相同指纹仅上报一次。

**缺陷 4：logger.error 生产环境不输出**
- **风险等级：P3**
- **证据**：`utils/logger.ts:13` `this.enabled = import.meta.env.DEV`，生产环境 `shouldLog` 恒为 false，所有日志（包括 error）都不输出。
- **业务影响**：生产环境排查问题时无客户端日志，但可通过浏览器 DevTools 查看 console.error（main.ts 全局错误处理仍输出）。
- **修复建议**：将 `error` 级别日志改为生产环境也输出（移除 shouldLog 检查或单独配置 `this.level = 'error'` 时强制输出）。

---

## 维度 24.15 表单验证与异步校验审计

### 检查方法
- Grep：`validator:|validateField|validate\(`、`:loading="submitLoading|:loading="loading`、`beforeRouteLeave|beforeunload`
- Read：`/workspace/frontend/src/views/customer/tabs/CustomerFormTab.vue`

### 发现

#### ✅ 已落实的项
- 表单校验规则使用 Element Plus `FormRules` 配置：
  - `Login.vue:104-109` `rules: FormRules` 配置 required + trigger
  - `CustomerFormTab.vue:14` `:rules="formRules"`
  - `Setup.vue:212` 使用 `validator` 自定义校验函数
  - `AfterSalesPanel.vue:161` 使用 `validator` 自定义校验
  - `quotations/create.vue:238` 使用 `validator` 校验报价单明细数组
  - `fund/tabs/TransferTab.vue:183` 使用 `validator` 校验转账金额
  - 30+ 处使用 `formRef.value.validate()` 进行提交前校验
- 防重复提交广泛覆盖：20+ 处使用 `:loading="submitLoading"` 或 `:loading="loading"` 在提交按钮上，防止重复点击。
- `Login.vue:69` `:disabled="lockInfo.isLocked"` 账号锁定时禁用提交按钮。
- `Login.vue:188-197` `handleUsernameBlur` 失焦时异步检查账号锁定状态（异步校验范例）。

#### ❌ 缺陷项

**缺陷 1：唯一性校验（如客户编码）未异步校验**
- **风险等级：P2**
- **证据**：
  - `CustomerFormTab.vue:14-148` 表单有 customer_code/customer_name/contact_phone/contact_email 等字段，无异步唯一性校验
  - Grep `validator:` 仅匹配 `AfterSalesPanel.vue:161`、`quotations/create.vue:238`、`Setup.vue:212`、`fund/tabs/TransferTab.vue:183`，**无客户编码/物料编码/产品编码等业务唯一性校验**
- **业务影响**：用户输入重复编码提交后才被后端拒绝，体验差。计划要求"唯一性校验（如客户编码）必须异步校验，校验期间禁用提交"。
- **修复建议**：在 `customer_code` 等唯一字段的 `blur` 事件中调用后端 `GET /customers/check-code?code=xxx` 接口，校验期间 `submitLoading = true`。

**缺陷 2：脏数据检测（未保存提示）未实现**
- **风险等级：P1**
- **证据**：Grep `beforeRouteLeave|beforeunload` 在 src/ 下无任何匹配；所有表单组件未实现"表单有未保存修改时离开提示"。
- **业务影响**：用户填写大表单后误点其他菜单，数据丢失。计划要求"表单有未保存修改时离开必须提示（beforeRouteLeave/beforeunload）"。
- **修复建议**：
  1. 在 `CustomerFormTab.vue`、`BomForm.vue`、`PrdForm.vue` 等大表单组件中实现 `onBeforeRouteLeave`，检测表单 dirty 状态
  2. 使用 `window.addEventListener('beforeunload', handler)` 处理浏览器刷新/关闭
  3. 封装 `useDirtyCheck(formRef)` composable 复用

**缺陷 3：提交按钮 loading + debounce 未结合**
- **风险等级：P3**
- **证据**：20+ 处使用 `:loading="submitLoading"`，但无 debounce；如果用户在 loading 启动前快速双击，仍会触发两次提交（虽后端有幂等性保护，但前端应拦截）。
- **业务影响**：极端场景下重复提交。
- **修复建议**：在 `handleSubmit` 入口处 `if (submitLoading.value) return` 提前返回，或使用 `debounce(handleSubmit, 300, { leading: true, trailing: false })`。

---

## 维度 24.16 i18n 国际化深化与复数 RTL 审计

### 检查方法
- RunCommand：`wc -l zh-CN.ts en-US.ts`
- Read：`/workspace/frontend/src/i18n/index.ts`、`locales/zh-CN.ts`、`locales/en-US.ts`（前 50 行）
- Grep：`Intl\.DateTimeFormat|Intl\.NumberFormat|toLocaleDateString|toLocaleString`

### 发现

#### ✅ 已落实的项
- `zh-CN.ts` 与 `en-US.ts` 行数完全一致（467 行 = 467 行），key 数量同步。
- `i18n/index.ts:42-54` 配置完整：
  - `legacy: false` 使用 Composition API 模式
  - `globalInjection: true` 全局 `$t` 注入
  - `locale: detectPreferredLocale()` 从 localStorage 读取首选语言
  - `fallbackLocale: 'zh-CN'` 回退语言
  - `silentFallbackWarn: true` / `silentTranslationWarn: true` 缺失 key 静默回退
- `i18n/index.ts:24-39` `detectPreferredLocale` 从 localStorage 读取用户偏好，回退到浏览器语言协商。
- `i18n/index.ts:57-70` `setLocale` 支持运行时切换语言，持久化到 localStorage，同步更新 `<html lang>` 属性。
- `i18n/index.ts:15-18` `SUPPORTED_LOCALES` 列表导出，供语言切换器复用。
- `main.ts:38-39` 根据当前语言切换 Element Plus locale（`zhCn`/`en`）。
- `utils/index.ts:10-16` 使用 `Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' })` 格式化金额。
- `MainLayout.vue` 全文使用 `$t('layout.menu.xxx')` 等国际化 key，无硬编码中文（菜单项）。
- `Login.vue` 全文使用 `$t('login.xxx')`，国际化到位。

#### ❌ 缺陷项

**缺陷 1：i18n 资源文件 key 同步无自动检测**
- **风险等级：P2**
- **证据**：
  - 虽然当前 zh-CN.ts 与 en-US.ts 行数一致（467 行），但无 CI 校验
  - `i18n/index.ts:6-8` TODO 注释"批次 23 v5 P0-1 仅完成 Login.vue 示范接入，其余 .vue 文件的硬编码文本待后续迭代逐步替换为 $t() 调用。4506 行资源文件已就绪"——但实际 zh-CN.ts 仅 467 行，与 TODO 注释不一致（可能注释过期）
  - 无 `i18n-ally` / `vue-i18n-extract` 等工具检测缺失 key
- **业务影响**：新增 key 时可能遗漏某个语言，运行时回退到 fallbackLocale 但不告警。
- **修复建议**：
  1. CI 中添加 `vue-i18n-extract report` 步骤，检测 zh-CN 与 en-US 的 key 差异
  2. key 缺失时 CI 失败

**缺陷 2：硬编码中文仍广泛存在（仅 Login.vue + MainLayout.vue 接入 i18n）**
- **风险等级：P1**
- **证据**：
  - `i18n/index.ts:6-8` TODO 明确"批次 23 v5 P0-1 仅完成 Login.vue 示范接入"
  - Grep `:label="` 在 CustomerFormTab.vue 等表单组件中匹配硬编码中文（如 `label="客户编码"`、`label="客户名称"`），未使用 `$t('customer.code')` 等
  - `system/index.vue:34-66` Tab 标签 `label="用户管理"`、`label="角色管理"` 等全部硬编码
- **业务影响**：英文用户切换语言后，仅 Login + 主菜单 + Dashboard 部分文本翻译，其他页面仍是中文。计划要求"资源文件同步"和"运行时切换语言"。
- **修复建议**：按模块逐步替换硬编码中文为 `$t()` 调用，优先级：系统管理 → 客户/供应商 → 库存 → 销售/采购 → 财务。

**缺陷 3：日期/数字格式化未使用 i18n locale，硬编码 zh-CN**
- **风险等级：P2**
- **证据**：Grep `Intl\.DateTimeFormat|Intl\.NumberFormat|toLocaleDateString|toLocaleString` 匹配 69 处，**全部硬编码 'zh-CN'**：
  - `utils/index.ts:11` `new Intl.NumberFormat('zh-CN', { style: 'currency', currency: 'CNY' })`
  - `utils/index.ts:24` `date.toLocaleDateString('zh-CN')`
  - `AfterSalesPanel.vue:189` `new Date(d).toLocaleString('zh-CN')`
  - 等等
- **业务影响**：英文用户看到的日期格式仍是中文格式（如"2026/7/16"），未切换为英文格式（如"7/16/2026"）。计划要求"必须用 Intl.DateTimeFormat/NumberFormat，禁止硬编码格式"。
- **修复建议**：将 `'zh-CN'` 替换为 `getCurrentLocale()` 或 `i18n.global.locale.value`，使格式化随语言切换。

**缺陷 4：RTL（阿拉伯语等）支持未实现**
- **风险等级：P3**
- **证据**：
  - `i18n/index.ts:15-18` `SUPPORTED_LOCALES` 仅 zh-CN/en-US，无阿拉伯语
  - `index.html:2` `<html lang="zh-CN">` 固定方向 LTR
  - 无 `dir="rtl"` 切换逻辑
- **业务影响**：当前业务场景下无阿拉伯语用户，影响小。计划要求"阿拉伯语等 RTL 语言必须支持布局翻转"。
- **修复建议**：如未来有中东市场拓展需求，再评估 RTL 支持；当前可在 `setLocale` 中根据 locale 设置 `document.documentElement.dir = locale.startsWith('ar') ? 'rtl' : 'ltr'`。

---

## 维度 24.17 前端权限粒度按钮字段行级审计

### 检查方法
- Grep：`v-permission` 在 src/ 下
- Read：`/workspace/frontend/src/directives/permission.ts`、`main.ts`

### 发现

#### ✅ 已落实的项
- `main.ts:34-35` 全局注册 `v-permission` 与 `v-role` 指令（FE-P-1 修复）。
- `directives/permission.ts:1-43` `permission` 指令实现：
  - 支持 `v-permission="'user:create'"` 单权限码
  - 支持 `v-permission="['user:create', 'user:update']"` 多权限码（任一匹配即显示）
  - 复用 `hasRoutePermission` 函数，支持通配符 `*:*` 与 `resource:*`
  - 无权限时 `el.parentNode?.removeChild(el)` 移除元素
- `directives/permission.ts:51-71` `role` 指令实现：支持单角色与多角色数组。
- v-permission 在多个 view 中使用：`bom/index.vue`、`quotations/list.vue`、`purchase-contract/components/PcTbl.vue`、`financial-analysis/tabs/AnalysisListTab.vue`、`product/tabs/ProductListTab.vue`、`budget/tabs/BudgetListTab.vue` 等（Grep 匹配 20+ 处）。
- `router/index.ts:916-924` 路由守卫严格校验 `meta.permission`，与指令行为一致。

#### ❌ 缺陷项

**缺陷 1：v-permission 覆盖率未达 100%**
- **风险等级：P1**
- **证据**：Grep `v-permission` 在 src/ 下匹配 20 处，但项目有 75+ views + 17 components + 大量子组件，按钮总数估算 500+，覆盖率约 4%。
  - 抽样 `CustomerFormTab.vue` 全文无 v-permission
  - 抽样 `sales/components/OlvTbl.vue` 全文无 v-permission
  - 抽样 `inventory/tabs/InventoryStockTab.vue` 全文无 v-permission
- **业务影响**：未授权用户可看到无权操作的按钮（点击后被后端拒绝，但 UI 体验差）。计划要求"所有按钮必须用 v-permission，覆盖率 100%"。
- **修复建议**：逐个 view 排查所有 `<el-button>` 与操作入口，添加 v-permission；可编写 ESLint 自定义规则检测无 v-permission 的 button。

**缺陷 2：字段级权限（v-permission 控制字段显示/隐藏/只读）未实现**
- **风险等级：P2**
- **证据**：
  - `directives/permission.ts:39-41` 无权限时直接 `removeChild`，无法实现"只读"
  - 无 `v-permission:readonly` 修饰符
  - Grep `v-permission` 匹配处全部用于按钮，无表单字段级控制
  - `system/tabs/FieldPermissionTab.vue` 存在（说明后端支持字段权限），但前端无对应指令实现
- **业务影响**：字段级权限（如普通员工不可见 salary 字段）无法在前端落地。计划要求"表单字段必须支持 v-permission 控制显示/隐藏/只读"。
- **修复建议**：扩展 v-permission 指令支持修饰符：
  - `v-permission="'field:read'"` 默认移除
  - `v-permission:readonly="'field:read'"` 改为 disabled
  - `v-permission:disabled="'field:edit'"` 改为 readonly

**缺陷 3：行级权限依赖后端过滤，前端未评估**
- **风险等级：P3**
- **证据**：前端表格组件（V2Table、el-table）均直接渲染后端返回的 list 数据，无二次过滤逻辑，符合计划"列表数据必须由后端按数据范围过滤，前端不二次过滤"。
- **业务影响**：符合要求，无缺陷。
- **修复建议**：无需修改。

**缺陷 4：权限缓存刷新未实现**
- **风险等级：P2**
- **证据**：
  - `store/user.ts:34-48` `fetchUserInfo` 仅在路由守卫 `!userStore.userInfo` 时调用一次（`router/index.ts:900`），权限缓存到内存
  - 用户权限变更后（如管理员调整角色），前端不会自动刷新，需用户重新登录
  - 无 5min TTL 刷新机制，无 WebSocket 推送权限变更
- **业务影响**：权限变更后用户仍可操作 5min-数小时（直到退出登录）。计划要求"权限变更后前端必须刷新权限缓存（5min TTL 或 WebSocket 推送）"。
- **修复建议**：
  1. 方案 A：在路由守卫中记录 `lastPermissionFetchTime`，超过 5min 强制 `fetchUserInfo`
  2. 方案 B：通过 WebSocket（24.7 维度的 WebSocketClient）接收 `permission_changed` 事件，自动 `fetchUserInfo`

---

## 维度 24.18 路由元信息与动态路由审计

### 检查方法
- Read：`/workspace/frontend/src/router/index.ts` 全量

### 发现

#### ✅ 已落实的项
- `router/index.ts:15-25` 通过 `declare module 'vue-router'` 扩展 `RouteMeta` 接口，包含 `title?/requiresAuth?/icon?/permission?/hidden?/public?` 6 个字段，符合计划"meta 必须包含 title/icon/permission/hidden/public 5 字段"。
- 路由守卫完整实现：
  - `router/index.ts:876-881` 设置 `document.title` 基于 `meta.title`
  - `router/index.ts:882-885` `/setup` 路由免鉴权
  - `router/index.ts:887-894` `meta.public` 标识公开路由免初始化检查
  - `router/index.ts:896-925` `meta.requiresAuth` 触发登录态校验 + `meta.permission` 权限码校验
- `hasRoutePermission` 函数实现宽松匹配（通配符 `*:*` / `resource:*` / read-view 等价 / update-edit 等价），符合业务命名兼容需求。
- `MainLayout.vue:279-294` `canAccessMenu` 函数基于 `router.resolve(path)` + `leafRecord.meta.hidden` + `hasRoutePermission` 判定菜单可见性，与路由守卫行为一致。
- `MainLayout.vue:306-326` `visibleSubMenu` 父级子菜单可见性派生（任一子项可见则父级可见）。
- `MainLayout.vue:193-197` 基于 `route.meta.title` 自动生成面包屑。

#### ❌ 缺陷项

**缺陷 1：未配置 keep-alive，Tab 切换状态不保留**
- **风险等级：P1**
- **证据**：
  - Grep `keep-alive|KeepAlive` 在 src/ 下无匹配
  - `MainLayout.vue:216-218` `<el-main class="main-content"><router-view /></el-main>` 直接渲染 router-view，无 keep-alive 包裹
  - `App.vue:1-5` `<template><router-view /></template>` 同样无 keep-alive
- **业务影响**：用户在表格页输入筛选条件后切换到其他页面再返回，筛选条件丢失，需重新输入。计划要求"必须配置 keep-alive，Tab 切换状态保留"。
- **修复建议**：
  1. 在 `MainLayout.vue` 的 `<router-view />` 外包裹 `<keep-alive :include="cachedViews">`
  2. 通过 route meta 控制 `keepAlive: true`，如 `meta: { title: '库存管理', keepAlive: true }`
  3. 在 store 中维护 `cachedViews` 数组，根据路由变化动态增删

**缺陷 2：未实现基于权限的动态路由注册**
- **风险等级：P2**
- **证据**：
  - `router/index.ts:27-772` 所有 70+ 路由在初始化时静态注册，无 `router.addRoute()` 动态注册逻辑
  - 路由守卫通过 `meta.permission` 拦截，但路由本身始终存在（只是访问时 403）
  - `MainLayout.vue:279-294` 菜单可见性通过 `canAccessMenu` 拦截，但路由仍可手动输入 URL 访问（被守卫拦截到 /403）
- **业务影响**：路由表对未授权用户也可见（通过 F12 查看路由配置），虽被守卫拦截，但路由元信息泄露。计划要求"必须支持基于权限的动态路由注册（如 admin 才能看到系统管理）"。
- **修复建议**：
  1. 初始路由仅注册公开路由（login/setup/403/404）
  2. 登录成功后根据 `userStore.userInfo.permissions` 调用 `router.addRoute()` 动态注册有权限的路由
  3. 通过 `router.removeRoute()` 在登出时移除

**缺陷 3：meta 完整性已覆盖但部分路由缺 icon**
- **风险等级：P3**
- **证据**：
  - 大多数路由有 `icon: 'HomeFilled'/'Setting'/'Money'/'ShoppingCart'/'Box'/'User'/'Cpu'/'List'/'MagicStick'/'Goods'/'Histogram'` 等
  - `Login.vue:36` `meta: { title: '登录' }` 无 icon（合理，登录页不在菜单）
  - `Setup.vue:42` `meta: { title: '系统初始化' }` 无 icon（合理）
  - 抽查所有 MainLayout children 路由均有 icon，覆盖率 100%
- **业务影响**：符合要求，无缺陷。
- **修复建议**：无需修改。

---

## 维度 24.19 API 请求拦截器与超时重试审计

### 检查方法
- Read：`/workspace/frontend/src/api/request.ts` 全量

### 发现

#### ✅ 已落实的项
- `request.ts:60-77` Axios 实例配置：
  - `baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp'` 环境变量驱动
  - `timeout: 30000` 全局超时 30s，符合计划要求
  - `withCredentials: true` 开启凭据发送（httpOnly Cookie 关键开关）
  - `headers: { 'Content-Type': 'application/json', 'X-Requested-With': 'XMLHttpRequest' }` 默认头注入
- `request.ts:79-109` 请求拦截器：
  - 注入 `X-CSRF-Token` 头（从 document.cookie 读取 csrf_token）
  - 公开路径（login/refresh/logout/init/health/ready/live/tracking/page-view）跳过 CSRF
  - 安全方法（GET/HEAD/OPTIONS）跳过 CSRF
  - `isCsrfPublicPath` 使用 `startsWith` 前缀匹配（P3-3 修复，避免 includes 子串误匹配）
- `request.ts:111-127` 响应拦截器：
  - 业务码非 200/0 时 `ElMessage.error(safeMessage)` 统一错误提示
  - 业务码 401 跳转登录页
  - 返回 `res as unknown as AxiosResponse`（P2 1-11 修复类型）
- `request.ts:128-198` 错误拦截器：
  - HTTP 403 + CSRF 校验失败（`CSRF_TOKEN_MISSING`/`CSRF_TOKEN_INVALID`）清空 CSRF Token + 跳转登录
  - HTTP 401 自动刷新流程：
    - `isRefreshing` 标志防止并发刷新
    - 排队请求通过 `subscribeTokenRefresh` 持有 resolve/reject（FE-P1-1 修复）
    - 刷新成功 `onTokenRefreshed` 通知排队请求重放
    - 刷新失败 `onTokenRefreshFailed` 通知排队请求 reject（FE-P1-1 修复）
  - 重试逻辑：`shouldRetry(error)` 对 502/503/504 或网络错误重试，最多 3 次，指数退避 `Math.min(1000 * count + random*1000, 5000)`
- `request.ts:225-234` `SAFE_ERROR_MESSAGES` 提供 400/401/403/404/429/500/502/503 共 8 种业务码的安全错误消息（不泄露后端细节）。
- `request.ts:238-244` `shouldRetry` 函数实现：HTTP 5xx 重试，无 response 时默认重试（网络错误）。

#### ❌ 缺陷项

**缺陷 1：路由切换未用 AbortController 取消旧请求**
- **风险等级：P2**
- **证据**：
  - Grep `AbortController` 在 src/ 下仅匹配 `router/index.ts` 1 处（用于 checkInitStatus 超时控制）
  - `request.ts` 无 AbortController 集成，无路由切换时取消旧请求的逻辑
  - `router/index.ts:876-928` `router.beforeEach` 无取消进行中请求的逻辑
- **业务影响**：用户快速切换路由时，旧路由的请求仍在进行，浪费带宽；如果旧请求晚于新请求返回，可能覆盖新页面的状态。计划要求"路由切换必须用 AbortController 取消旧请求"。
- **修复建议**：
  1. 在 `request.ts` 的 get/post/put/delete/patch 方法中支持 `config.signal` 参数
  2. 在路由守卫 `router.beforeEach` 中维护全局 `AbortController` 实例，切换路由时 `abort()` 旧请求
  3. 或使用 vue-router 4 的 `onBeforeRouteLeave` 在组件内取消

**缺陷 2：超时重试策略覆盖所有请求，未区分幂等性**
- **风险等级：P3**
- **证据**：`request.ts:180-189` 重试逻辑对 `originalRequest._retry` 为 true 的请求重试，但 `_retry` 仅在 401 刷新场景设置。其他场景的重试通过 `shouldRetry(error)` 判断，**未区分 GET/POST/PUT/DELETE**：
  - POST/PUT/DELETE 等非幂等方法重试可能导致重复创建数据
  - 当前实现仅在 502/503/504 或网络错误时重试，这些场景下后端通常未处理请求，重试相对安全
- **业务影响**：极端场景下可能重复提交（后端应有幂等性保护兜底）。
- **修复建议**：在 `shouldRetry` 中增加方法判断：仅 GET/HEAD/PUT（幂等）重试，POST/DELETE 默认不重试（除非显式标记 `_retryable: true`）。

**缺陷 3：CSRF Token 缺失时未提示用户**
- **风险等级：P3**
- **证据**：`request.ts:97-102` 当 `csrfToken` 为 null 时静默跳过头注入，请求会发送到后端被 403 拦截。
- **业务影响**：用户首次访问时如果 CSRF Cookie 未下发，所有 POST 请求都会 403，体验差。
- **修复建议**：在 `request.ts` 启动时检查 csrf_token Cookie 是否存在，不存在时调用 `GET /auth/csrf-token`（如后端有此接口）主动获取；或在登录响应中确保下发。

---

## 维度 24.20 主题样式与暗黑模式审计

### 检查方法
- Read：`/workspace/frontend/src/App.vue`、`components/Layout/MainLayout.vue`
- Grep：`el-config-provider|dark|dark-mode|theme`、`:deep\(|::v-deep`、`@media|max-width|min-width`

### 发现

#### ✅ 已落实的项
- `App.vue:7-12` 全局样式使用 `body { margin: 0; padding: 0; font-family: ... }`，仅 reset 样式，未污染组件样式。
- `MainLayout.vue:334-373` 使用 `<style scoped>` 作用域样式，仅 `.main-layout/.aside/.logo/.menu/.header` 等布局类样式，符合计划"全局样式仅限 reset/变量"。
- 10+ 处使用 `:deep()` 修改 Element Plus 内部样式（如 `bom/index.vue:373` `:deep(.el-card__header)`），符合 Vue 3 推荐方式（替代 ::v-deep）。
- `tsconfig.json:14` `strict: true` 启用 TypeScript 严格模式。

#### ❌ 缺陷项

**缺陷 1：未使用 CSS 变量，主题色硬编码**
- **风险等级：P1**
- **证据**：
  - `MainLayout.vue:11-12` `background-color="#304156"` `text-color="#bfcbd9"` `active-text-color="#409eff"` 硬编码菜单颜色
  - `MainLayout.vue:339` `.aside { background-color: #304156; }` 硬编码
  - `MainLayout.vue:346` `.logo { background-color: #263445; }` 硬编码
  - `MainLayout.vue:361` `.header { background: #fff; box-shadow: 0 1px 4px rgba(0, 21, 41, 0.08); }` 硬编码
  - `MainLayout.vue:370` `.main-content { background: #f0f2f5; }` 硬编码
  - `App.vue:8-12` `body { font-family: -apple-system, ... }` 硬编码字体
  - `Dashboard.vue:65-79` `background-color: #f5f7fa` `color: #303133` 硬编码
- **业务影响**：无法实现主题切换，未来如需品牌定制需大改。计划要求"主题色/间距/字体必须用 CSS 变量，禁止硬编码颜色"。
- **修复建议**：
  1. 在 `App.vue` 或全局 CSS 中定义 CSS 变量：
     ```css
     :root {
       --color-bg-sidebar: #304156;
       --color-bg-header: #fff;
       --color-bg-main: #f0f2f5;
       --color-text-primary: #303133;
       --color-text-secondary: #606266;
       --color-primary: #409eff;
       --shadow-header: 0 1px 4px rgba(0, 21, 41, 0.08);
       --font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
     }
     ```
  2. 所有组件使用 `var(--color-xxx)` 引用

**缺陷 2：暗黑模式完全未实现**
- **风险等级：P1**
- **证据**：
  - Grep `el-config-provider|dark|dark-mode|theme` 在 src/ 下无匹配
  - `main.ts` 未引入 Element Plus dark theme CSS（`element-plus/theme-chalk/dark/css-vars.css`）
  - `package.json` 无 `dark-mode` 切换相关依赖
  - 无 `<html class="dark">` 切换逻辑
  - 无 `useDark` from `@vueuse/core` 等工具
- **业务影响**：夜间使用刺眼，与计划"必须支持 Element Plus dark theme 适配，持久化用户偏好"严重不符。
- **修复建议**：
  1. `main.ts` 引入 `import 'element-plus/theme-chalk/dark/css-vars.css'`
  2. 安装 `@vueuse/core`，使用 `useDark` / `useToggle` 实现暗黑切换
  3. 在 MainLayout 头部添加主题切换按钮
  4. 持久化用户偏好到 localStorage（非敏感数据）

**缺陷 3：主题切换（亮色/暗色/品牌主题）未实现**
- **风险等级：P2**
- **证据**：同缺陷 2，无任何主题切换机制。
- **业务影响**：单一主题，无法满足品牌定制需求。
- **修复建议**：在缺陷 2 基础上，支持多主题：
  - 亮色（默认）：`:root` 变量集
  - 暗色：`html.dark` 变量集
  - 品牌主题：`html.brand-xxx` 变量集
  - 切换不刷新页面（动态修改 `<html>` class）

**缺陷 4：scoped 样式覆盖不足，全局样式污染风险**
- **风险等级：P3**
- **证据**：
  - `App.vue:7-12` 全局 `<style>`（无 scoped）定义 body 样式，合理
  - 但 `MainLayout.vue:334-373` 使用 `<style scoped>` 仅作用于本组件，子组件如 `<el-menu>` 内部样式需 `:deep()` 穿透
  - 部分 view（如 `Dashboard.vue:62-99`）使用 `<style scoped>` 但定义了 `.dashboard-title` 等通用类名，可能与子组件冲突
- **业务影响**：样式冲突风险存在但被 scoped 隔离缓解。
- **修复建议**：在项目规范中明确"组件根类名必须加组件前缀"（如 `.dashboard-title` → `.db-title`），避免冲突。

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 24.1 响应式设计与移动端适配 | 0 | 2 | 1 | 1 | 5 | 9 |
| 24.2 路由懒加载与代码分割 | 0 | 1 | 2 | 1 | 3 | 7 |
| 24.3 Pinia 状态管理与持久化 | 0 | 0 | 2 | 2 | 5 | 9 |
| 24.4 组件设计与 Props/Emits 类型安全 | 0 | 0 | 1 | 2 | 7 | 10 |
| 24.5 composables 响应式与内存泄漏 | 0 | 0 | 1 | 2 | 5 | 8 |
| 24.6 ECharts 图表性能与无障碍 | 0 | 1 | 0 | 2 | 4 | 7 |
| 24.7 WebSocket 客户端连接重连心跳 | 0 | 0 | 1 | 2 | 7 | 10 |
| 24.8 前端性能与 bundle 体积 | 0 | 0 | 2 | 2 | 4 | 8 |
| 24.9 Vite 构建与 Tree Shaking | 0 | 1 | 1 | 1 | 6 | 9 |
| 24.10 前端测试覆盖率与 mock fixtures | 0 | 1 | 1 | 1 | 6 | 9 |
| 24.11 前端 XSS 防护与 CSP 策略 | 0 | 1 | 1 | 1 | 3 | 6 |
| 24.12 敏感数据存储与 token 安全 | 0 | 0 | 0 | 3 | 6 | 9 |
| 24.13 前端可访问性 WCAG 2.1 AA | 0 | 1 | 2 | 1 | 3 | 7 |
| 24.14 错误边界与全局错误处理 | 0 | 2 | 1 | 1 | 3 | 7 |
| 24.15 表单验证与异步校验 | 0 | 1 | 1 | 1 | 4 | 7 |
| 24.16 i18n 国际化深化与复数 RTL | 0 | 1 | 2 | 1 | 6 | 10 |
| 24.17 前端权限粒度按钮字段行级 | 0 | 1 | 2 | 1 | 4 | 8 |
| 24.18 路由元信息与动态路由 | 0 | 1 | 1 | 1 | 5 | 8 |
| 24.19 API 请求拦截器与超时重试 | 0 | 0 | 1 | 2 | 6 | 9 |
| 24.20 主题样式与暗黑模式 | 0 | 2 | 1 | 1 | 3 | 7 |
| **合计** | **0** | **16** | **23** | **28** | **92** | **159** |

**总体评估**：
- **P0 阻塞级缺陷：0**（无阻塞性问题）
- **P1 高优先级缺陷：16**（主要集中在 PWA/移动端、bundle 监控、CSP 安全头、ErrorBoundary、监控接入、keep-alive、暗黑模式、i18n 覆盖率、权限覆盖率等核心架构能力缺失）
- **P2 中优先级缺陷：23**（测试覆盖率、debounce、CSP、CSS 变量化、动态路由、AbortController、脏数据检测等）
- **P3 低优先级缺陷：28**（文档注释、ARIA 标签、错误去重、fixtures 化等优化项）
- **已落实项：92**（约 58% 的检查项已落实，基础架构合理）

---

## 修复优先级队列

### P1 高优先级（建议本迭代完成）

1. **24.1 缺陷 1**：无 PWA 支持（manifest.json + Service Worker 全缺）
2. **24.1 缺陷 2**：移动端布局未做侧边栏抽屉化与顶部导航折叠
3. **24.2 缺陷 1**：未配置 rollupOptions.manualChunks 代码分割策略
4. **24.6 缺陷 1**：未使用 echarts/core 按需引入，全量引入 echarts（约 1MB 浪费）
5. **24.9 缺陷 1**：未配置 build.optimizeDeps 与 manualChunks
6. **24.10 缺陷 1**：测试覆盖率未在 CI 中强制 ≥ 70%（实际 < 15%）
7. **24.11 缺陷 1**：nginx.conf 未配置 Content-Security-Policy 头
8. **24.13 缺陷 1**：键盘导航 Tab/Shift+Tab/Esc 焦点管理未全局规范化
9. **24.14 缺陷 1**：未实现 Vue 3 ErrorBoundary 组件
10. **24.14 缺陷 2**：未接入前端监控（Sentry/Bugsnag/自研）
11. **24.15 缺陷 2**：脏数据检测（未保存提示）未实现
12. **24.16 缺陷 2**：硬编码中文仍广泛存在（仅 Login.vue + MainLayout.vue 接入 i18n）
13. **24.17 缺陷 1**：v-permission 覆盖率未达 100%（约 4%）
14. **24.18 缺陷 1**：未配置 keep-alive，Tab 切换状态不保留
15. **24.20 缺陷 1**：未使用 CSS 变量，主题色硬编码
16. **24.20 缺陷 2**：暗黑模式完全未实现

### P2 中优先级（建议下迭代完成）

1. **24.1 缺陷 3**：图片懒加载未启用
2. **24.2 缺陷 2**：未配置 rollup-plugin-visualizer 进行 chunk 体积监控
3. **24.2 缺陷 3**：无 prefetch 策略，关键路由未预加载
4. **24.3 缺陷 2**：未使用 pinia-plugin-persistedstate 持久化用户偏好
5. **24.3 缺陷 4**：Store 单元测试覆盖率不全（5 个 store 仅 2 个有测试）
6. **24.4 缺陷 1**：`vue/require-explicit-emits` 规则被关闭
7. **24.5 缺陷 1**：useTableApi 内部 watch 触发的 fetchData 无 onUnmounted 取消订阅
8. **24.7 缺陷 1**：心跳超时 60s 断开未实现
9. **24.8 缺陷 1**：debounce/throttle 在搜索/滚动/resize 场景覆盖率低
10. **24.8 缺陷 2**：bundle 体积未监控，无 visualizer 报告
11. **24.9 缺陷 2**：未按业务域分割 chunk（finance/sales/inventory/production）
12. **24.10 缺陷 2**：E2E 测试仅覆盖采购/销售，未覆盖财务/库存/生产/CRM 等核心模块
13. **24.11 缺陷 2**：未配置 X-Frame-Options 防 Clickjacking
14. **24.13 缺陷 2**：色彩对比度未自动检测（active-text-color 对比度 3.8:1 低于 AA 4.5:1）
15. **24.13 缺陷 4**：路由切换后焦点未重置
16. **24.14 缺陷 3**：错误去重未实现
17. **24.15 缺陷 1**：唯一性校验（如客户编码）未异步校验
18. **24.16 缺陷 1**：i18n 资源文件 key 同步无自动检测
19. **24.16 缺陷 3**：日期/数字格式化未使用 i18n locale，硬编码 zh-CN
20. **24.17 缺陷 2**：字段级权限（v-permission 控制字段显示/隐藏/只读）未实现
21. **24.17 缺陷 4**：权限缓存刷新未实现
22. **24.18 缺陷 2**：未实现基于权限的动态路由注册
23. **24.19 缺陷 1**：路由切换未用 AbortController 取消旧请求
24. **24.20 缺陷 3**：主题切换（亮色/暗色/品牌主题）未实现

### P3 低优先级（建议长期优化）

1. **24.1 缺陷 4**：触屏按钮 ≥ 44px 未在全局规范中强制
2. **24.2 缺陷 4**：首屏性能指标（FCP/LCP/TTI）未监控
3. **24.3 缺陷 1**：Store 数量未覆盖所有业务域，缺 system store
4. **24.3 缺陷 3**：Store 跨模块通信未规范化，无事件总线解耦机制
5. **24.4 缺陷 2**：组件文档注释覆盖不全，多数公共组件无 props/emits/slots 文档
6. **24.4 缺陷 3**：`ColorCardGrid.vue` emits 未使用泛型 + kebab-case 风格
7. **24.5 缺陷 2**：composables 缺乏统一 try/catch 包裹
8. **24.5 缺陷 3**：composables 文档注释覆盖率低
9. **24.6 缺陷 2**：图表无 ARIA 标签，屏幕阅读器不友好
10. **24.6 缺陷 3**：dataZoom 仅在 2 个组件使用，其他图表未评估大数据场景
11. **24.7 缺陷 2**：超过最大重连次数后未降级为轮询
12. **24.7 缺陷 3**：票据鉴权未评估 URL query 传递的泄露风险
13. **24.8 缺陷 3**：Tree Shaking 未清零，`@typescript-eslint/no-explicit-any` warn 模式下 any 残留
14. **24.8 缺陷 4**：未启用 vite build 的 chunk size warning 自定义阈值
15. **24.9 缺陷 3**：env.d.ts 未声明 VITE_* 自定义环境变量类型
16. **24.10 缺陷 3**：fixtures 化不彻底
17. **24.11 缺陷 3**：vue/no-v-html 规则关闭，依赖人工审查
18. **24.12 缺陷 1**：CSRF Token 通过 document.cookie 读取，存在 CSRF 防护绕过风险
19. **24.12 缺陷 2**：CompanyTab.vue 缓存公司信息到 localStorage，未评估业务合理性
20. **24.12 缺陷 3**：前端密钥泄露审计未发现，但 .env.production.example 提示规则
21. **24.13 缺陷 3**：图表无 ARIA 标签（同 24.6 缺陷 2）
22. **24.14 缺陷 4**：logger.error 生产环境不输出
23. **24.15 缺陷 3**：提交按钮 loading + debounce 未结合
24. **24.16 缺陷 4**：RTL（阿拉伯语等）支持未实现
25. **24.17 缺陷 3**：行级权限依赖后端过滤，前端未评估（已落实，无缺陷）
26. **24.18 缺陷 3**：meta 完整性已覆盖但部分路由缺 icon（已落实，无缺陷）
27. **24.19 缺陷 2**：超时重试策略覆盖所有请求，未区分幂等性
28. **24.19 缺陷 3**：CSRF Token 缺失时未提示用户
29. **24.20 缺陷 4**：scoped 样式覆盖不足，全局样式污染风险

---

## 审计结论

**类二十四前端架构与体验审计**已完成 20 维度全部检查，共发现 **0 个 P0 阻塞级缺陷、16 个 P1 高优先级缺陷、23 个 P2 中优先级缺陷、28 个 P3 低优先级缺陷**，已落实 92 项检查项。

**核心问题**：
1. **移动端与 PWA**：完全缺失 PWA 支持与移动端适配，无法满足移动办公场景
2. **构建优化**：未配置 manualChunks/visualizer/optimizeDeps，bundle 体积未监控，echarts 全量引入
3. **测试覆盖率**：单元测试覆盖率 < 15%，远低于 70% 目标，且 CI 无阈值强制
4. **安全防护**：nginx.conf 缺失 CSP/X-Frame-Options 等安全头
5. **错误处理**：未实现 ErrorBoundary 组件与前端监控接入
6. **主题与暗黑**：未使用 CSS 变量，暗黑模式完全未实现
7. **i18n 覆盖率**：仅 Login.vue + MainLayout.vue 接入 i18n，其他 view 硬编码中文广泛存在
8. **权限粒度**：v-permission 覆盖率约 4%，字段级权限未实现
9. **可访问性**：键盘焦点管理、色彩对比度、ARIA 标签等 WCAG AA 合规性不足
10. **keep-alive**：未配置 keep-alive，Tab 切换状态丢失

**亮点**：
1. 路由懒加载 100% 覆盖，70+ 路由全部使用 `() => import()`
2. Wave B-3 httpOnly Cookie 鉴权方案完整落地，token 不入 localStorage
3. v-html 仅 2 处使用且 100% 经过 DOMPurify 消毒
4. WebSocket 客户端实现指数退避重连 + 30s 心跳 + 一次性票据鉴权
5. CSRF 防护使用 Double Submit Cookie 模式，公开路径白名单 + 前缀匹配
6. 5 个 Pinia store 全部使用 Composition API 风格，Object.freeze 保护 permissions
7. 公共组件 100% 使用 `defineProps<T>()` + `defineEmits<T>()` 泛型类型安全
8. ECharts 4 组件复用 BaseChart，dispose/resize 清理到位
9. Vue Test Utils + Playwright 双层测试框架就绪，E2E 覆盖采购/销售完整流程
10. 路由守卫与菜单可见性判定行为一致（hasRoutePermission 复用）

**建议**：按 P1 → P2 → P3 优先级分迭代修复，建议本迭代优先解决 PWA/移动端、bundle 优化、CSP 安全头、ErrorBoundary、keep-alive、CSS 变量化、暗黑模式、i18n 覆盖率 8 大核心能力缺失。
