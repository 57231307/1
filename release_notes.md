## 当前版本

- 发布包版本：v1.0.0（见根目录 [VERSION](file:///workspace/VERSION)）

## 一键安装 / 更新

### 全新安装

```bash
curl -fsSL --http1.1 --retry 3 https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s install
```

### 在线更新

```bash
curl -fsSL --http1.1 --retry 3 https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s update
```

### 安装后管理命令（bingxi）

安装脚本会在系统中写入 `bingxi` 命令行工具，可用于启动/停止/更新：

```bash
bingxi status
bingxi restart
bingxi update
```

## 数据库初始化 / 迁移

- 发布包中包含数据库初始化脚本目录：`database/migration/`（仓库对应路径为 [backend/database/migration](file:///workspace/backend/database/migration/)）。
- 初始化流程会按文件名排序执行目录内的 `.sql` 脚本；当前为整合脚本 [001_consolidated_schema.sql](file:///workspace/backend/database/migration/001_consolidated_schema.sql)。

## 版本号维护规则

为避免“运行版本、界面显示版本、API 文档版本”不一致，发布时建议至少检查并同步以下位置：

- 发布包版本文件： [VERSION](file:///workspace/VERSION)
- 后端构建版本： [backend/Cargo.toml](file:///workspace/backend/Cargo.toml)
- 前端构建版本： [frontend/Cargo.toml](file:///workspace/frontend/Cargo.toml)
- 界面/API 显示版本（如存在硬编码）：[init.rs](file:///workspace/frontend/src/pages/init.rs)、[login.rs](file:///workspace/frontend/src/pages/login.rs)、[openapi.rs](file:///workspace/backend/src/openapi.rs)

## 发布记录（请在每次发布时补充）

### vX.Y.Z

- 变更：待补充
- 升级注意事项：待补充
