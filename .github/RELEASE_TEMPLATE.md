# Release 变更说明模板

> **用途**：发布新版本时，CI 自动依据此模板生成 GitHub Release 的变更说明。
> 所有 commit message 应遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范，
> CI 会按 commit 类型（feat/fix/refactor/docs/style/test/chore/perf/remove/breaking）自动归类。

## 模板格式

```markdown
# Bingxi ERP {VERSION}

**发布时间**: {DATE}
**上一版本**: {PREV_TAG}
**Commit 范围**: {PREV_TAG}..{TAG_NAME}（共 {COMMIT_COUNT} 个提交）

---

## 📋 版本概述

{SUMMARY}

---

## 🆕 新增（feat）

{FEATURES}

## ✏️ 修改（refactor / perf / style）

{MODIFIED}

## 🗑️ 删除（remove / chore-delete）

{REMOVED}

## 🐛 修复（fix）

{FIXES}

## 🔄 变更（breaking / 其他变更）

{CHANGES}

---

## 📊 统计

| 类型 | 数量 |
|------|------|
| 新增（feat） | {FEAT_COUNT} |
| 修复（fix） | {FIX_COUNT} |
| 修改（refactor/perf/style） | {MODIFIED_COUNT} |
| 删除（remove） | {REMOVE_COUNT} |
| 变更（breaking） | {BREAKING_COUNT} |
| 文档（docs） | {DOCS_COUNT} |
| 测试（test） | {TEST_COUNT} |
| 构建/工具（chore） | {CHORE_COUNT} |
| **合计** | {TOTAL_COUNT} |

---

## 📝 完整 Commit 列表

{FULL_LOG}

---

## 🚀 快速部署

```bash
# 解压发布包
tar -xzf release-{VERSION}.tar.gz
cd bingxi-erp

# 部署后端
cp backend/server /opt/bingxi-erp/backend/
systemctl restart bingxi-backend

# 部署前端
cp -r frontend/dist/* /var/www/html/
```

## 🛠️ 技术栈

- **后端**: Rust + Axum + SeaORM
- **前端**: Vue 3 + TypeScript + Element Plus
- **数据库**: PostgreSQL 14+

---

*此发布说明由 CI 自动生成，依据 `.github/RELEASE_TEMPLATE.md` 模板格式化。*
```

## Commit 类型映射规则

CI 脚本按以下规则解析 commit message 前缀并归类到对应章节：

| Conventional Commit 前缀 | 归类章节 | 说明 |
|--------------------------|----------|------|
| `feat:` / `feat(scope):` | 🆕 新增 | 新功能、新特性 |
| `fix:` / `fix(scope):` | 🐛 修复 | Bug 修复 |
| `refactor:` / `perf:` / `style:` | ✏️ 修改 | 重构、性能优化、代码风格 |
| `remove:` / `chore(remove):` / `chore(delete):` | 🗑️ 删除 | 移除功能、删除文件 |
| `BREAKING CHANGE:` / `!:` | 🔄 变更 | 不兼容的破坏性变更 |
| `docs:` | （统计但不列入主体） | 文档更新 |
| `test:` | （统计但不列入主体） | 测试相关 |
| `chore:` / `build:` / `ci:` | （统计但不列入主体） | 构建、CI、工具链 |
| 其他 | 🔄 变更 | 未识别的提交 |

## 填写规范

1. **Commit message 必须遵循 Conventional Commits 规范**，否则无法被正确归类。
2. **Breaking Change** 必须在 commit message 中标注 `BREAKING CHANGE:` 或使用 `!:` 语法。
3. **无对应类型的章节** 显示「_无_」，不省略章节标题。
4. **版本概述** 由 CI 提取首个 feat/fix commit 的描述生成，或显示「_详见下方变更分类_」。
