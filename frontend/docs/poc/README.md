# 虚拟列表 POC (B5 P2-1) 文档索引

| 文件 | 用途 |
|------|------|
| [el-table-v2-poc-report.md](./el-table-v2-poc-report.md) | **POC 报告主文档**(必读) |

## 配套文件

- `frontend/src/views/inventory/tabs/VirtualStockTabPOC.vue` - el-table-v2 POC 组件
- `frontend/src/views/inventory/tabs/StockTab.vue` - 原 el-table 对照组
- `frontend/src/views/inventory/tabs/testData.ts` - 测试数据生成(同 src 内)
- `frontend/src/views/inventory/index-poc.vue` - POC 演示入口
- `frontend/scripts/gen-test-data.ts` - 测试数据生成(同 scripts)
- `frontend/scripts/poc-perf-test.cjs` - Playwright 性能采集
- `frontend/tests/unit/poc-virtual-table.test.ts` - 单元测试(17 用例)

## 快速体验

```bash
# 1. 启动前端 dev server
cd frontend
npm run dev

# 2. 浏览器访问
#    http://localhost:3000/inventory-poc

# 3. 性能测试(需本地有 GUI 浏览器)
npm run build
npx vite preview --port 5182 &
node scripts/poc-perf-test.cjs
```
