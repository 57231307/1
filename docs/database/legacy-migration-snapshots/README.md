# 旧版 SQL 迁移脚本快照

## 来源

本目录归档了项目历史中存在的两套非权威 SQL 迁移脚本，**仅作历史参考**：

| 原始位置 | 文件数 | 状态 |
|---|---|---|
| `backend/database/migration/` | 26 | 已删除（无任何代码引用） |
| `backend/src/database/migration/` | 9 | 已删除（无任何代码引用） |

## 删除原因

经过全仓库 `grep` 搜索（`grep -rE "database/migration"`），这两个目录在以下位置均无引用：

- `backend/src/**/*.rs` - 业务代码
- `Cargo.toml` - 依赖配置
- `deploy/*.sh`、`快速部署/install.sh` - 部署脚本
- `.github/workflows/*.yml` - CI/CD

## 运行时实际使用的 SQL 迁移目录

| 用途 | 路径 | 引用位置 |
|---|---|---|
| 启动初始化（init_service 启动兜底） | `database/migration/` | `backend/src/services/init_service.rs:185-191` + `deploy/deploy.sh:245-248` + `.github/workflows/ci-cd.yml:335-337` |
| SeaORM 标准迁移 | `backend/migrations/` | `Cargo.toml`（sea-orm-cli 工具链） |
| 内置 TOTP 字段与索引（硬编码） | `main.rs:239-247` | `backend/src/main.rs` |

## 后续建议

- `cli/migrate.rs:5, 26` 注释提到 "src/database/migration/" 是**错误的**（已修正，指向正确的两个目录）
- 如需将 `backend/migrations/` 的迁移自动执行，请补齐 `struct Migrator` + `impl MigratorTrait` 注册
- `database/migration/` 的执行顺序按文件名字典序，重复运行安全（全部使用 `IF NOT EXISTS`）
