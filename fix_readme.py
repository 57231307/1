import re

with open('README.md', 'r') as f:
    content = f.read()

# Find the start of "## 部署指南" and the start of "## 开发指南"
start_idx = content.find("## 部署指南")
end_idx = content.find("## 开发指南")

if start_idx != -1 and end_idx != -1:
    new_deployment = """## 部署指南

项目现已支持纯物理机环境的 **一键自动化部署**，无需 Docker 容器。该脚本将自动下载最新代码、配置运行环境、设置 Nginx 反向代理，并注册 Systemd 服务实现开机自启和崩溃保活。

### 1. 一键快速部署

在您的 Linux 服务器（推荐 Ubuntu/Debian/CentOS）上，直接运行以下命令即可完成全自动安装：

```bash
curl -fsSL https://raw.githubusercontent.com/57231307/1/main/快速部署/install.sh | sudo bash -s install
```

### 2. 一键管理工具 (bingxi)

安装成功后，系统会自动为您在终端注入一个叫做 `bingxi` 的全局命令。后续日常运维非常极简：

```bash
sudo bingxi start    # 启动系统 (后端及Nginx网关)
sudo bingxi stop     # 停止系统
sudo bingxi restart  # 重启系统
sudo bingxi status   # 查看系统运行与保活状态
sudo bingxi update   # 一键平滑升级（自动拉取最新 Release 包并平滑重启）
```

### 3. 环境要求与手动配置

**硬件要求**：
- CPU：至少 2 核
- 内存：至少 4GB
- 磁盘：至少 20GB
- 操作系统：Linux (推荐 Ubuntu 20.04+)

**手动配置 (可选)**：
如果需要修改数据库连接或其它高级配置，请编辑配置文件：
```bash
nano /etc/bingxi/.env
```
修改完成后，重启服务使其生效：
```bash
sudo bingxi restart
```

---

"""
    content = content[:start_idx] + new_deployment + content[end_idx:]
    with open('README.md', 'w') as f:
        f.write(content)
