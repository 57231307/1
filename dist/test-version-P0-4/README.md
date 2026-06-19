# 冰溪 ERP P0-4 色卡仓储管理测试版本

## 概述

本测试版本包含 P0-4 色卡仓储管理模块的完整功能：
- 3 张表（color_cards / color_card_items / color_card_borrow_records）
- 16 个 API 端点
- 4 个前端页面 + 3 个组件
- 5 个集成测试
- E2E 测试

## 快速启动

### 方式一：Docker Compose（推荐）

```bash
cd dist/test-version-P0-4
cp config/color-card.toml.example config/color-card.toml
./start.sh
```

启动后访问：
- 前端：http://localhost:3000
- 后端：http://localhost:8080
- 色卡模块路径：登录后访问「色卡仓储管理 → 色卡列表」

### 方式二：手动启动

1. 启动 PostgreSQL + Redis
2. 编译并启动后端：
   ```bash
   cd backend
   cargo build --release
   DATABASE_URL=... ./target/release/bingxi-erp
   ```
3. 构建并启动前端：
   ```bash
   cd frontend
   npm install && npm run build
   # 用 nginx 或其他静态服务器托管 dist/
   ```

## 测试场景

详见 `test-scenarios.md`（19 个测试场景）。

## 停止服务

```bash
cd dist/test-version-P0-4
./stop.sh
```

## 关键端点

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/erp/color-cards` | GET/POST | 色卡列表/创建 |
| `/api/v1/erp/color-cards/:id` | GET/PUT/DELETE | 详情/更新/归档 |
| `/api/v1/erp/color-cards/:id/items` | GET/POST | 色号列表/添加 |
| `/api/v1/erp/color-cards/:id/items/batch` | POST | 批量导入 |
| `/api/v1/erp/color-cards/borrow` | POST | 借出 |
| `/api/v1/erp/color-cards/return/:record_id` | POST | 归还 |
| `/api/v1/erp/color-cards/lost/:record_id` | POST | 登记遗失 |
| `/api/v1/erp/color-cards/scan/:code` | GET | 扫码查询 |
| `/api/v1/erp/color-cards/export/:id` | GET | 导出 CSV |

## 配置说明

详见 `config/color-card.toml.example`。

## 故障排查

| 现象 | 排查 |
|------|------|
| 数据库连接失败 | 检查 `DATABASE_URL` 环境变量 |
| 色号导入失败 | 检查色号编码是否在同一色卡内重复 |
| 借出失败 | 检查色卡状态是否为 active |
| 扫码查询无结果 | 检查色号编码是否正确 |
