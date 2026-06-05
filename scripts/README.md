# 项目脚本目录

本目录收录项目级运维/测试/部署辅助脚本。

## 部署相关

| 脚本 | 用途 | 依赖 |
|---|---|---|
| `check-deploy.sh` | 部署前配置检查（环境变量/文件/网络） | bash 4+ |
| `validate-env.sh` | 验证 .env / .env.example 配置完整性 | bash 4+ |
| `api-crud-test.sh` | API CRUD 端到端冒烟测试 | curl, jq |

## 部署脚本

完整部署脚本位于 `../deploy/` 目录：
- `deploy/deploy.sh` - 主部署脚本
- `deploy/deploy-latest.sh` - 远程增量更新脚本
- `deploy/install.sh` - 一键安装脚本

## 使用方法

```bash
# 部署前检查
bash scripts/check-deploy.sh

# 验证环境变量
bash scripts/validate-env.sh

# API 冒烟测试（需要先设置 BINGXI_ADMIN_PASSWORD）
export BINGXI_ADMIN_PASSWORD=xxx
bash scripts/api-crud-test.sh
```

## 维护说明

- 新增脚本统一在此目录创建
- 命名规范：kebab-case（例：`backup-db.sh`）
- 顶部需有 `#!/bin/bash` 与 set -e
- 需被 root 执行的脚本用前缀 `_root_` 命名（仅约定）
