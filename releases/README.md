# Bingxi ERP 发布包目录

此目录用于存放编译好的前端和后端集成压缩包。

## 目录说明

- 所有压缩包文件名应包含版本号和日期，例如：`bingxi-erp-v1.0.0-20260328.zip`
- 压缩包应包含完整的后端二进制文件、前端静态文件和部署脚本
- 每次正式发布时都应在此目录保留相应的压缩包

## 压缩包内容规范

每个压缩包应包含以下内容：

```
bingxi-erp-vx.y.z-yyyymmdd/
├── backend/
│   ├── server              # 后端二进制文件
│   └── .env.example        # 环境变量示例
├── frontend/
│   └── dist/               # 前端静态文件
├── database/
│   └── migration/          # 数据库迁移脚本
├── deploy/
│   ├── nginx.conf          # Nginx配置
│   ├── deploy.sh           # 部署脚本
│   └── bingxi-backend.service  # systemd服务文件
└── README.md               # 部署说明
```
