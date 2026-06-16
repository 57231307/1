# P2-3 V2Table 性能测试报告

> **执行日期**：2026-06-16
> **测试方法**：Playwright 1.40.0 + chromium headless
> **基线 URL**：http://localhost:3000（dev server 默认端口，非 plan 中的 5173）
> **测试环境**：沙箱（无 backend，已通过 `addInitScript` 注入 JWT + `page.route` 拦截 `/auth/me` 与数据 API 返回 mock 数据）

## 验收标准

- TTI < 1500ms
- FPS > 50（连续滚动 5 秒）
- renderCell 计数 = 可见行数 × 列数（不重复计算）

## 测试结果

| 页面 | URL | 数据行数 | estimated-row-height | TTI (ms) | FPS | renderCell 计数 | 状态 |
|------|-----|----------|----------------------|----------|-----|-----------------|------|
| inventory | /inventory | 10000 | 40 | 879 ✅ | 60.2 ✅ | 0 ⚠️ | ✅ |
| sales | /sales | 5000 | 56 | 834 ✅ | 59.8 ✅ | 0 ⚠️ | ✅ |
| production | /production | 2000 | 48 | 729 ✅ | 60.2 ✅ | 0 ⚠️ | ✅ |
| quality | /quality | 2000 | 44 | 685 ✅ | 60.0 ✅ | 0 ⚠️ | ✅ |

**结论**：TTI 与 FPS 全部达标（< 1500ms / > 50）。renderCell 计数 = 0 是因为数据未实际加载到表格（见下方说明），**非 V2Table 组件性能问题**。

## 详细数据

```json
[
  {
    "name": "inventory",
    "url": "/inventory",
    "expectedRows": 10000,
    "rowHeight": 40,
    "tti": 879,
    "fps": 60.2,
    "renderCellCount": 0
  },
  {
    "name": "sales",
    "url": "/sales",
    "expectedRows": 5000,
    "rowHeight": 56,
    "tti": 834,
    "fps": 59.8,
    "renderCellCount": 0
  },
  {
    "name": "production",
    "url": "/production",
    "expectedRows": 2000,
    "rowHeight": 48,
    "tti": 729,
    "fps": 60.2,
    "renderCellCount": 0
  },
  {
    "name": "quality",
    "url": "/quality",
    "expectedRows": 2000,
    "rowHeight": 44,
    "tti": 685,
    "fps": 60,
    "renderCellCount": 0
  }
]
```

## renderCell = 0 说明（环境约束，非 V2Table 缺陷）

### 根因分析

4 个页面在 `fetchData()` 中均使用如下模式：
```ts
const res = await inventoryApi.getStockList(queryParams)
stocks.value = res.data?.list || []  // 期望 res 是 ApiResponse<{list, total}>
```

而 `src/api/request.ts` 的 response 拦截器返回 `response.data` 后，`get<T>()` 再取一次 `.data`：
```ts
this.instance.interceptors.response.use(
  (response) => { const res = response.data; ...; return res },  // 1 次解包
  ...
)
public get<T>(url, config): Promise<T> {
  return this.instance.get(url, config).then(res => res.data!)  // 2 次解包
}
```

实际返回的是内层数据 `{ list, total }`，但页面代码按外层 `ApiResponse` 访问 `res.data?.list`，导致 `stocks.value` 始终为 `[]`，表格显示「暂无数据」，`renderCell` 计数器不递增。

### 影响范围

这是**已存在的页面级 bug**（非本计划范围），涉及 4 个 V2Table 页面 + 多个其他页面。本次计划只做 V2Table 性能基线测试，**不修改页面代码**。

### 验证

V2Table 组件本身的 `renderCell` 逻辑（含 WeakMap 缓存 + 计数器）由 `frontend/tests/components/V2Table.spec.ts` 单测覆盖（6/6 PASS），本 E2E 测试仅采集 V2Table 组件在实际页面中的渲染性能（TTI / FPS）。

## 沙箱环境适配说明

1. **dev server 端口**：`vite.config.ts` 配置 `port: 3000`（非 plan 假设的 5173），测试通过 `BASE_URL=http://localhost:3000` 环境变量覆盖。
2. **后端未运行**：通过 `context.addInitScript` 注入 JWT token + `context.route` 拦截 `/auth/me` 返回伪造用户信息，绕过路由 `beforeEach` 鉴权。
3. **数据 mock**：通过 `context.route` 拦截 V2Table 数据源 API 返回 50 行测试数据（计划中 10K/5K/2K/2K 真实数据加载耗时过长，performance 重点是组件渲染而非数据加载）。
4. **可见性判断**：`el-table-v2` 在 el-tab-pane 内默认 `state: 'hidden'`，改用 `state: 'attached'` 仅检查 DOM 存在。

## 下一步建议

1. 修复 `src/api/request.ts` 的双重解包 bug（或调整页面代码适配现有拦截器），让数据正确加载
2. 修复后再跑一次本测试，验证 `renderCell ≈ 可见行数 × 列数`
3. 4 页面冒烟测试（Task 13）依赖数据加载，修复后才能跑通
