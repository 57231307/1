# CI/CD 辅助脚本

本目录收录 CI/CD 相关辅助脚本，用于本地复现/修复 CI 检查。

## 脚本清单

| 脚本 | 用途 | 用法 |
|------|------|------|
| `fix-rustfmt.sh` | 自动修复所有 Rust 格式问题 | `bash scripts/ci/fix-rustfmt.sh` |
| `fix-prettier.sh` | 自动修复所有前端格式问题 | `bash scripts/ci/fix-prettier.sh` |
| `setup-clippy-baseline.sh` | 建立/更新 clippy 警告基线 | `bash scripts/ci/setup-clippy-baseline.sh` |
| `clippy-check.sh` | 本地复现 CI 的 clippy 严格检查 | `bash scripts/ci/clippy-check.sh` |

## 典型工作流

### 1. 新代码提交前自检

```bash
# 修复格式
bash scripts/ci/fix-rustfmt.sh
bash scripts/ci/fix-prettier.sh

# 复现 CI 的 clippy 检查
bash scripts/ci/clippy-check.sh
```

### 2. 建立 clippy 基线（首次）

```bash
bash scripts/ci/setup-clippy-baseline.sh
git add backend/.clippy-baseline.txt
git commit -m "chore(ci): 建立 clippy 警告基线"
```

### 3. 修复历史 clippy 警告（可选）

```bash
# 1. 跑 clippy-check.sh 查看现状
bash scripts/ci/clippy-check.sh

# 2. 修复警告
# ... 编辑代码 ...

# 3. 验证修复
bash scripts/ci/clippy-check.sh

# 4. 刷新基线（已修复的警告从基线移除）
bash scripts/ci/setup-clippy-baseline.sh
git add backend/.clippy-baseline.txt
git commit -m "chore(clippy): 修复 N 个警告 + 刷新基线"
```

## 设计原则

- **本地优先**：所有 CI 检查都可以在本地复现，避免 push 后才发现问题
- **渐进式严格化**：历史代码与新代码分别对待，避免 PR 阻塞
- **可观测性**：所有脚本输出 reports/ 目录，便于诊断
- **可重复**：脚本幂等，多次运行结果一致
