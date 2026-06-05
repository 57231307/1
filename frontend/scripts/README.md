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

## 后续处理建议

1. 这些脚本原本是开发期的临时工具，建议转换为 Playwright/Cypress 等 E2E 测试框架
2. 转换后归入 `frontend/tests/e2e/`，并与 CI 集成
3. 临时调试脚本超过 6 个月未更新即可删除
