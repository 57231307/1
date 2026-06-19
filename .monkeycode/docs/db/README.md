# 冰溪 ERP 数据库文档

本目录包含冰溪 ERP 项目的完整数据库结构文档。

## 📚 文档清单

| 文档 | 说明 | 行数 |
|------|------|------|
| [SCHEMA.md](./SCHEMA.md) | 完整数据库 Schema 文档（v2026.617.0001） | 876 行 |

## 📋 SCHEMA.md 目录

1. **数据库总览** — 规模统计 / 字符集 / 演进原则
2. **ER 图（ASCII）** — 核心实体关系图
3. **表清单（按业务模块分类）** — 13 大业务域 / 213 个表
4. **关键表字段说明** — 7 个核心表详细字段
5. **关键索引清单** — P4-1 性能优化 7 个索引
6. **约束说明** — 主键 / 外键 / UNIQUE / CHECK / NOT NULL
7. **多租户隔离** — tenant_id 字段分布 / 提取规范
8. **性能优化记录** — P4-1 优化效果
9. **数据库 Migration 演进** — 33 个 migration 文件
10. **备份与恢复策略** — RTO 4h / RPO 1h
11. **监控与维护** — 关键指标 / 维护任务
12. **参考资料** — PostgreSQL / SeaORM 文档

## 🔗 相关目录

- **migration 源文件**：`/workspace/database/migration/`（33 个 SQL 文件）
- **SeaORM 模型**：`/workspace/backend/src/models/`（自动生成）
- **数据库配置**：`/workspace/.env.example`
