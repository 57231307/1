# frontend/scripts/

本目录收录前端开发/调试用的临时脚本，由 `frontend/` 根目录迁移整理而来。

## 脚本清单

| 文件 | 用途 |
|---|---|
| `check_console_errors.js` | 浏览器控制台错误检测（开发期辅助） |
| `check_remaining_errors.js` | 残留错误检查（开发期辅助） |
| `comprehensive_test.cjs` | 综合性功能测试（开发期辅助） |
| `full_test.js` | 完整功能测试（开发期辅助） |

## 使用方法

```bash
# 进入前端目录
cd frontend

# 启动开发服务器
npm run dev

# 在另一个终端运行检测脚本（需要页面在 dev 模式运行）
node scripts/check_console_errors.js
```

## 契约门禁（CI ci-static-checks 阻断链，非临时脚本）

| 文件 | 用途 |
|---|---|
| `check-contract.mjs` | 前后端契约对齐（基础） |
| `check-api-paths.mjs` | 前端调用路径 ↔ 后端真实路由（nest 展开后） |
| `check-api-envelope.mjs` | 响应信封形状 ↔ handler 出参 |
| `check-api-request.mjs` | 请求体/查询参数键集 ↔ 后端 `Json<T>`/`Query<T>` 字段集；解析不出入「盲区」，叶子类型入「未覆盖清单」，均逐条计数、禁止静默跳过 |
| `check-api-request-fixture-selftest.mjs` | 上面解析器的 fixture 自测（四类历史盲区 (a)箭头签名 (b)Partial (c)交叉类型 (d)叶子/索引签名/config 三态 的防回归哨兵） |
| `api-request-baseline.json` | `check-api-request` 存量失配基线（**过渡机制**：命中只列示不判负，新失配仍红，只准缩短；清零后删除本文件恢复全量阻断） |
| `check-api-keys.mjs` / `api-keys-baseline.json` | 前端类型键编造棘轮 |
| `route-snapshot.mjs` | 端点全量快照漂移 |

运行方式与 CI 一致：`cd frontend && node scripts/<name>.mjs`（退出码非 0 即门禁失败）。

## 后续处理建议

1. 这些脚本原本是开发期的临时工具，建议转换为 Playwright/Cypress 等 E2E 测试框架
2. 转换后归入 `frontend/tests/e2e/`，并与 CI 集成
3. 临时调试脚本超过 6 个月未更新即可删除
