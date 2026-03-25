# 删除非 Rust 项目文件清单

## 需要删除的文件夹

### 1. Python 脚本和自动化测试
- scripts/ (整个文件夹)
- maintenance_scripts/ (整个文件夹)
- test_artifacts/ (整个文件夹)

### 2. Go 语言项目
- src/ (整个文件夹，包含 backend 和 frontend-go)
- config/ (整个文件夹)
- database/ (整个文件夹)

### 3. 测试文件和报告
- test/ (整个文件夹)
- security_tests/ (整个文件夹)
- releases/ (整个文件夹)

### 4. 根目录文件（非 Rust 相关）
- *.py (所有 Python 脚本)
- *.go (所有 Go 文件)
- *.sh (所有 Shell 脚本，除了 Rust 项目的)
- *.bat (所有 Windows 批处理，除了 Rust 项目的)
- *.ps1 (所有 PowerShell 脚本，除了 Rust 项目的)
- *.md (所有 Markdown 文档，除了 README.md)
- *.json (所有 JSON 配置文件，除了 Rust 项目的)
- *.txt (所有文本文件)
- *.rules (所有规则文件)
- *.png (所有图片文件)

## 保留的文件和文件夹

### 1. Rust 项目
- bingxi-rust/ (整个文件夹)

### 2. 必要的配置和文档
- README.md (项目主文档)
- .gitignore (Git 忽略配置)
- .lingmaignore (Lingma 忽略配置)
