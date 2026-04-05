import re

with open('.github/workflows/ci-cd.yml', 'r') as f:
    content = f.read()

# Replace the release_notes.md generation block with a safe one
old_block_pattern = r"cat > release_notes\.md << EOF.*?EOF"
new_block = """cat > release_notes.md << EOF
          # 秉羲管理系统 - ${{ needs.package-release.outputs.version }}

          ## 📦 发布内容

          本次发布包含完整的秉羲管理系统，包括：

          - ✅ 后端服务（Rust Axum）
          - ✅ 前端应用（Yew WebAssembly）
          - ✅ 部署脚本和配置文件
          - ✅ 完整的项目文档

          ## ✨ 详细改动变化

          $CHANGELOG

          ## 🚀 快速开始

          ### 1. 一键安装
          \\`\\`\\`bash
          curl -fsSL https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s install
          \\`\\`\\`

          ### 2. 常用命令
          安装成功后，你可以使用以下命令管理系统：
          \\`\\`\\`bash
          sudo bingxi start    # 启动系统
          sudo bingxi stop     # 停止系统
          sudo bingxi status   # 查看状态
          sudo bingxi update   # 在线更新到最新版本
          \\`\\`\\`

          ### 3. 手动配置环境 (可选)
          如果需要修改数据库连接或其它高级配置，请编辑 \\`/etc/bingxi/.env\\` 文件并重启服务：
          \\`\\`\\`bash
          nano /etc/bingxi/.env
          sudo bingxi restart
          \\`\\`\\`

          ## 📋 系统要求

          - **操作系统**: Linux (推荐 Ubuntu 20.04+)
          - **CPU**: 至少 2 核
          - **内存**: 至少 4GB
          - **磁盘**: 至少 20GB
          - **数据库**: PostgreSQL 14+

          ## 🔧 技术栈

          - **后端**: Rust 2021 + Axum 0.7 + SeaORM 1.0
          - **前端**: Yew 0.21 (Rust WebAssembly)
          - **数据库**: PostgreSQL 14+
          - **部署**: Systemd + Nginx

          ## 📚 文档

          完整文档请查看 docs/ 目录或访问项目仓库。

          ## 🐛 问题反馈

          如有问题，请提交 Issue 或联系开发团队。

          ---
          *发布时间: $(date +'%Y-%m-%d %H:%M:%S')*
          EOF"""

# Actually, to make it even safer, let's write to a file first and then cat it without EOF variable interpolation.
# But we need $CHANGELOG.
# How about we use Python to write a shell script that constructs the markdown?
# Better: 
safer_block = """echo "# 秉羲管理系统 - ${{ needs.package-release.outputs.version }}" > release_notes.md
          echo "" >> release_notes.md
          echo "## 📦 发布内容" >> release_notes.md
          echo "" >> release_notes.md
          echo "本次发布包含完整的秉羲管理系统，包括：" >> release_notes.md
          echo "" >> release_notes.md
          echo "- ✅ 后端服务（Rust Axum）" >> release_notes.md
          echo "- ✅ 前端应用（Yew WebAssembly）" >> release_notes.md
          echo "- ✅ 部署脚本和配置文件" >> release_notes.md
          echo "- ✅ 完整的项目文档" >> release_notes.md
          echo "" >> release_notes.md
          echo "## ✨ 详细改动变化" >> release_notes.md
          echo "" >> release_notes.md
          echo "$CHANGELOG" >> release_notes.md
          echo "" >> release_notes.md
          echo "## 🚀 快速开始" >> release_notes.md
          echo "" >> release_notes.md
          echo "### 1. 一键部署" >> release_notes.md
          echo "\`\`\`bash" >> release_notes.md
          echo "curl -fsSL https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s install" >> release_notes.md
          echo "\`\`\`" >> release_notes.md
          echo "" >> release_notes.md
          echo "### 2. 常用命令" >> release_notes.md
          echo "安装成功后，你可以使用以下命令管理系统：" >> release_notes.md
          echo "\`\`\`bash" >> release_notes.md
          echo "sudo bingxi start    # 启动系统" >> release_notes.md
          echo "sudo bingxi stop     # 停止系统" >> release_notes.md
          echo "sudo bingxi status   # 查看状态" >> release_notes.md
          echo "sudo bingxi update   # 在线更新到最新版本" >> release_notes.md
          echo "\`\`\`" >> release_notes.md
          echo "" >> release_notes.md
          echo "### 3. 手动配置环境 (可选)" >> release_notes.md
          echo "如果需要修改数据库连接或其它高级配置，请编辑 \`/etc/bingxi/.env\` 文件并重启服务：" >> release_notes.md
          echo "\`\`\`bash" >> release_notes.md
          echo "nano /etc/bingxi/.env" >> release_notes.md
          echo "sudo bingxi restart" >> release_notes.md
          echo "\`\`\`" >> release_notes.md
          echo "" >> release_notes.md
          echo "## 📋 系统要求" >> release_notes.md
          echo "" >> release_notes.md
          echo "- **操作系统**: Linux (推荐 Ubuntu 20.04+)" >> release_notes.md
          echo "- **CPU**: 至少 2 核" >> release_notes.md
          echo "- **内存**: 至少 4GB" >> release_notes.md
          echo "- **磁盘**: 至少 20GB" >> release_notes.md
          echo "- **数据库**: PostgreSQL 14+" >> release_notes.md
          echo "" >> release_notes.md
          echo "## 🔧 技术栈" >> release_notes.md
          echo "" >> release_notes.md
          echo "- **后端**: Rust 2021 + Axum 0.7 + SeaORM 1.0" >> release_notes.md
          echo "- **前端**: Yew 0.21 (Rust WebAssembly)" >> release_notes.md
          echo "- **数据库**: PostgreSQL 14+" >> release_notes.md
          echo "- **部署**: Systemd + Nginx" >> release_notes.md
          echo "" >> release_notes.md
          echo "## 📚 文档" >> release_notes.md
          echo "" >> release_notes.md
          echo "完整文档请查看 docs/ 目录或访问项目仓库。" >> release_notes.md
          echo "" >> release_notes.md
          echo "## 🐛 问题反馈" >> release_notes.md
          echo "" >> release_notes.md
          echo "如有问题，请提交 Issue 或联系开发团队。" >> release_notes.md
          echo "" >> release_notes.md
          echo "---" >> release_notes.md
          echo "*发布时间: $(date +'%Y-%m-%d %H:%M:%S')*" >> release_notes.md"""

content = re.sub(old_block_pattern, safer_block, content, flags=re.DOTALL)

with open('.github/workflows/ci-cd.yml', 'w') as f:
    f.write(content)
