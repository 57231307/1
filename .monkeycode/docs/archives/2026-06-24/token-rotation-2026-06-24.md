# GitHub Token 轮换操作指南（2026-06-24）

## ⚠️ 背景

发现 GitHub Token（`ghu_` 前缀）明文存储在本地 `.git/config`：
```
origin  https://x-access-token:<REDACTED>@github.com/57231307/1
```

> **本指南不含真实 Token**：如需查看具体 Token 字符串，请直接查看本地 `.git/config`。

## Token 权限评估（2026-06-24 13:39 UTC）

| 项 | 值 |
|---|---|
| 认证用户 | 57231307 (id: 59329781) |
| 仓库权限 | **admin**（全部仓库） |
| 影响仓库 | 57231307/1, 57231307/2 |
| Token 类型 | Personal Access Token (PAT) |
| 风险等级 | 🔴 **高危**：admin 权限意味着可推送任意代码、修改 release、删除 PR |

## 🚨 必须立即执行的步骤（人工操作）

### 1. 撤销当前 Token

访问 https://github.com/settings/tokens ，找到泄露的 Token（前缀 `ghu_`），点击 **Delete**。

> **注意**：沙箱环境无浏览器，需要用户在本地浏览器完成此操作。
> **安全提示**：文档中不应包含真实 Token 字符串（已被 GitHub Secret Scanning 阻止 push）。

### 2. 生成新 Token（仅在需要时）

访问 https://github.com/settings/tokens/new ：

| 配置项 | 推荐值 |
|---|---|
| Note | `bingxi-erp-local-dev-2026-06-24` |
| Expiration | 30 天（短期 token，强制轮换） |
| Scopes | 最小化（只勾选必要权限） |

- ✅ `repo` (完整仓库访问)
- ✅ `workflow` (CI/CD 触发)
- ❌ 其他（不勾选）

### 3. 安全存储新 Token

**推荐方式**：环境变量 + 启动脚本

```bash
# ~/.bashrc 或 ~/.zshrc
export GITHUB_TOKEN="ghu_NEW_TOKEN_HERE"
```

git remote URL 改为读取环境变量：
```bash
git remote set-url origin "https://x-access-token:${GITHUB_TOKEN}@github.com/57231307/1.git"
```

## 🔄 替代认证方式（更安全）

### 方式 A：SSH Key 认证（推荐）

1. 生成 SSH key：
   ```bash
   ssh-keygen -t ed25519 -C "your_email@example.com" -f ~/.ssh/github_bingxi
   ```

2. 添加到 GitHub：https://github.com/settings/keys

3. 修改 remote URL：
   ```bash
   git remote set-url origin git@github.com:57231307/1.git
   ```

4. 验证：
   ```bash
   ssh -T git@github.com
   ```

### 方式 B：GitHub CLI 认证

```bash
# 安装
brew install gh  # macOS
# 或
sudo apt install gh  # Ubuntu

# 登录
gh auth login

# 验证
gh auth status
```

## 📝 撤销后本地清理步骤

1. 撤销 token 后，本地 git push 会失败（401 Unauthorized）
2. 立即执行：
   ```bash
   # 备份 .git/config
   cp .git/config .git/config.bak

   # 移除 .git/config 中的 token
   git remote set-url origin https://github.com/57231307/1.git

   # 切换到新认证方式（SSH 或 新 token）
   git remote set-url origin git@github.com:57231307/1.git
   # 或
   git remote set-url origin https://x-access-token:$GITHUB_TOKEN@github.com/57231307/1.git

   # 验证
   git fetch origin
   ```

## ✅ 完成确认清单

- [ ] 旧 token 已在 GitHub 撤销
- [ ] 旧 token 不再出现在 .git/config
- [ ] 新认证方式（SSH 或新 token）已配置
- [ ] `git fetch origin` 成功
- [ ] CI 流水线仍能正常工作（CI 端使用自己独立的 secret）
- [ ] .monkeycode/MEMORY.md 记录此次轮换
