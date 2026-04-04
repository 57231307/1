# 秉羲管理系统 - CI/CD 指南

## 📋 概述

本指南详细介绍秉羲管理系统的CI/CD（持续集成/持续部署）流水线配置和使用方法。

**主要功能**：
- ✅ 自动运行测试
- ✅ 自动构建前端（Yew WebAssembly）
- ✅ 自动构建后端（Rust Axum）
- ✅ 自动打包为ZIP文件
- ✅ 自动发布到 GitHub Release

---

## 🏗️ 流水线架构

### 工作流文件

| 文件 | 用途 |
|------|------|
| [.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) | 完整的CI/CD流水线（测试、构建、打包、发布） |
| [.github/workflows/ci.yml](file:///workspace/.github/workflows/ci.yml) | 简化版CI流水线（仅测试和构建） |

### 工作流触发条件

```yaml
on:
  push:
    branches:
      - main        # 推送到main分支触发
      - master      # 推送到master分支触发
    tags:
      - 'v*'        # 推送v开头的标签触发
  pull_request:
    branches:
      - main
      - master      # PR时触发测试
  release:
    types: [created] # Release创建时触发
```

### Job 依赖关系

```
test (运行测试)
    ↓
┌───┴───┐
↓       ↓
build-backend  build-frontend (并行构建)
↓       ↓
└───┬───┘
    ↓
package-release (创建ZIP包)
    ↓
github-release (发布到GitHub)
    ↓
notify (构建通知)
```

---

## 🔧 配置说明

### 环境变量

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `RUST_VERSION` | 1.94.1 | Rust工具链版本 |
| `CARGO_TERM_COLOR` | always | Cargo输出颜色 |

### 触发方式

#### 1. 自动触发（推送到main/master）

```bash
# 推送到main分支，自动触发完整流水线
git add .
git commit -m "feat: 新功能"
git push origin main
```

#### 2. 标签触发（版本发布）

```bash
# 创建并推送标签，触发版本发布
git tag -a v1.0.0 -m "版本1.0.0"
git push origin v1.0.0
```

#### 3. Pull Request触发

创建PR到main/master分支时，会自动运行测试和构建，确保代码质量。

---

## 📦 Job 详细说明

### 1. Test Job - 运行测试

**目的**：确保代码质量，运行所有测试

**步骤**：
1. 检出代码
2. 安装Rust工具链（包含rustfmt, clippy）
3. 缓存Cargo依赖
4. 安装系统依赖（pkg-config, libssl-dev, postgresql-client, protobuf-compiler）
5. 运行后端测试：`cargo test --all`

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L22-L54)

### 2. Build Backend Job - 构建后端

**依赖**：需要Test Job成功

**目的**：编译Rust后端为Release版本可执行文件

**步骤**：
1. 检出代码
2. 安装Rust工具链
3. 缓存Cargo依赖
4. 安装系统依赖
5. 构建后端：`cargo build --release`
6. 验证可执行文件存在
7. 上传编译产物：`backend/target/release/server`

**输出产物**：
- 名称：`backend-binary`
- 保留时间：30天

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L56-L105)

### 3. Build Frontend Job - 构建前端

**依赖**：需要Test Job成功

**目的**：编译Yew前端为WebAssembly静态文件

**步骤**：
1. 检出代码
2. 安装Rust工具链（包含wasm32-unknown-unknown目标）
3. 安装Trunk构建工具：`cargo install --locked trunk`
4. 缓存Cargo依赖
5. 构建前端：`trunk build --release`
6. 验证dist目录存在
7. 上传编译产物：`frontend/dist/`

**输出产物**：
- 名称：`frontend-dist`
- 保留时间：30天

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L107-L156)

### 4. Package Release Job - 创建发布包

**依赖**：需要Build Backend和Build Frontend都成功

**触发条件**：仅在push到main/master或推送v*标签时运行

**目的**：将前端、后端、部署脚本、文档等打包为一个ZIP文件

**步骤**：
1. 检出代码
2. 下载后端编译产物
3. 下载前端编译产物
4. 准备目录结构：
   ```
   release/bingxi-erp/
   ├── backend/
   │   ├── server           (可执行文件)
   │   ├── .env.example     (环境变量示例)
   │   ├── config.toml      (配置文件)
   │   └── config.yaml      (配置文件)
   ├── frontend/
   │   └── dist/            (WebAssembly静态文件)
   ├── deploy/              (部署脚本)
   ├── database/            (数据库迁移)
   ├── docs/                (项目文档)
   └── README.md
   ```
5. 生成版本号：
   - 标签触发：使用标签名（如v1.0.0）
   - 分支触发：使用日期+短commit（如20260404-abc123）
6. 创建ZIP包：`zip -r bingxi-erp-{version}.zip bingxi-erp/`
7. 上传ZIP包

**输出产物**：
- 名称：`bingxi-erp-release-zip`
- 保留时间：90天

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L158-L240)

### 5. GitHub Release Job - 发布到GitHub Release

**依赖**：需要Package Release Job成功

**触发条件**：仅在push到main/master或推送v*标签时运行

**权限要求**：`contents: write`

**目的**：自动创建GitHub Release并上传ZIP文件

**步骤**：
1. 检出代码
2. 下载ZIP发布包
3. 生成版本号
4. 生成发布说明（Markdown格式）
5. 使用softprops/action-gh-release创建Release：
   - 标签名：v{version}
   - 发布名：秉羲管理系统 v{version}
   - 自动生成发布说明
   - 上传ZIP文件
   - 设为最新版本

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L242-L345)

### 6. Notify Job - 构建通知

**依赖**：所有前面的Job

**触发条件**：always（无论成功失败都运行）

**目的**：输出构建状态总结

**输出内容**：
- 测试状态
- 后端构建状态
- 前端构建状态
- 发布包创建状态
- GitHub Release状态

**文件位置**：[ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml#L347-L366)

---

## 📥 发布包内容

### ZIP包结构

```
bingxi-erp-{version}.zip
└── bingxi-erp/
    ├── backend/
    │   ├── server              # 后端可执行文件
    │   ├── .env.example        # 环境变量模板
    │   ├── config.toml         # 配置文件
    │   └── config.yaml         # 配置文件
    ├── frontend/
    │   ├── index.html          # HTML入口
    │   ├── *.wasm              # WebAssembly文件
    │   ├── *.js                # JavaScript文件
    │   └── *.css               # 样式文件
    ├── deploy/
    │   ├── deploy.sh           # 主部署脚本
    │   ├── deploy-backend.sh   # 后端部署脚本
    │   ├── deploy-frontend.sh  # 前端部署脚本
    │   ├── nginx.conf          # Nginx配置
    │   └── bingxi-backend.service # Systemd服务配置
    ├── database/               # 数据库迁移文件（如存在）
    ├── docs/
    │   ├── CODE_WIKI.md        # 项目Wiki
    │   ├── PROJECT_DOCUMENTATION.md # 项目文档
    │   └── DOCUMENTATION_INDEX.md # 文档索引
    └── README.md               # 项目说明
```

### 发布说明内容

自动生成的Release说明包含：
- 📦 发布内容列表
- 🚀 快速开始指南
- 📋 系统要求
- 🔧 技术栈
- 📚 文档说明
- 🐛 问题反馈

---

## 🚀 使用指南

### 方式一：推送到main分支（自动发布）

```bash
# 1. 提交代码
git add .
git commit -m "feat: 添加新功能"

# 2. 推送到main分支
git push origin main

# 3. 等待CI/CD完成
# 访问 Actions 页面查看进度
# 完成后自动创建Release
```

### 方式二：推送标签（版本发布）

```bash
# 1. 创建标签
git tag -a v1.0.0 -m "版本1.0.0 - 首次正式发布"

# 2. 推送标签
git push origin v1.0.0

# 3. 等待CI/CD完成
# 自动创建Release并上传ZIP包
```

### 方式三：手动创建Release

也可以手动创建Release，触发CI/CD：

1. 访问GitHub仓库的Releases页面
2. 点击 "Draft a new release"
3. 填写版本信息
4. 发布后自动触发CI/CD

---

## 🔍 查看CI/CD进度

### GitHub Actions页面

1. 访问仓库主页
2. 点击 "Actions" 标签
3. 选择对应的工作流
4. 查看每个Job的详细日志

### 快速链接

```
https://github.com/{owner}/{repo}/actions
```

---

## ⚙️ 自定义配置

### 修改Rust版本

编辑 [ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml)：

```yaml
env:
  RUST_VERSION: 1.94.1  # 修改为需要的版本
```

### 修改缓存配置

```yaml
- name: 缓存 Cargo 依赖
  uses: actions/cache@v5
  with:
    path: |
      ~/.cargo/registry
      ~/.cargo/git
      backend/target
    key: ${{ runner.os }}-cargo-backend-${{ hashFiles('**/Cargo.lock') }}
```

### 添加额外的系统依赖

在install步骤添加：

```yaml
- name: 安装系统依赖
  run: |
    sudo apt-get update
    sudo apt-get install -y pkg-config libssl-dev postgresql-client protobuf-compiler
    # 添加你的依赖
    sudo apt-get install -y your-package
```

---

## 📊 缓存策略

### 缓存内容

| 缓存 | 路径 | 目的 |
|------|------|------|
| Cargo Registry | `~/.cargo/registry` | 加速依赖下载 |
| Cargo Git | `~/.cargo/git` | 加速Git依赖 |
| Backend Target | `backend/target` | 加速后端重新编译 |
| Frontend Target | `frontend/target` | 加速前端重新编译 |

### 缓存键

```
${{ runner.os }}-cargo-{type}-${{ hashFiles('**/Cargo.lock') }}
```

当Cargo.lock变化时，缓存会自动失效重建。

---

## 🐛 故障排查

### 问题1：Rust版本不兼容

**症状**：编译错误，提示Rust版本过低

**解决**：
1. 更新ci-cd.yml中的RUST_VERSION
2. 或者使用stable代替固定版本

### 问题2：缓存导致的问题

**症状**：奇怪的编译错误，但本地可以编译

**解决**：
1. 在Actions页面手动清除缓存
2. 或者修改缓存键使缓存失效

### 问题3：GitHub Token权限不足

**症状**：Release创建失败，权限错误

**解决**：
1. 确保Job有`permissions: contents: write`
2. 检查仓库Settings > Actions > General中的权限设置

### 问题4：构建超时

**症状**：Job运行超过时间限制被取消

**解决**：
1. 优化构建时间（使用更快的机器类型）
2. 启用缓存减少重复工作
3. 拆分大Job为多个小Job

---

## 📚 相关文档

- [GitHub Actions文档](https://docs.github.com/en/actions)
- [softprops/action-gh-release](https://github.com/softprops/action-gh-release)
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain)
- [Trunk文档](https://trunkrs.dev/)

---

## 📞 支持

如有问题，请：
1. 查看Actions日志获取详细错误信息
2. 参考本文档的故障排查部分
3. 提交Issue到项目仓库

---

*本文档最后更新：2026-04-04*
