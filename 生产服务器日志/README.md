# 生产服务器日志

导出时间：2026-06-13

## 目录结构

```
生产服务器日志/
├── 应用日志/          # 后端应用日志
│   ├── bingxi_backend.log.2026-06-11
│   ├── bingxi_backend.log.2026-06-12
│   └── bingxi_backend.log.2026-06-13
├── 错误日志/          # 错误日志
│   ├── error.log.2026-06-11
│   ├── error.log.2026-06-12
│   └── error.log.2026-06-13
├── 系统日志/          # systemd 服务日志
│   └── systemd_日志.txt
├── 审计日志/          # 审计日志
│   ├── business_audit.log.*      # 业务审计
│   ├── database_audit.log.*      # 数据库审计
│   ├── financial_audit.log.*     # 财务审计
│   └── permission_audit.log.*    # 权限审计
├── 性能日志/          # 性能监控日志
│   ├── performance_audit.log.*   # 性能审计
│   └── system_health.log.*       # 系统健康
├── 安全日志/          # 安全审计日志
│   └── security_audit.log.*
└── README.md          # 本文件
```

## 服务器信息

- 服务器 IP：111.230.99.236
- 数据库 IP：39.99.34.194
- 服务端口：8082
- 服务名称：bingxi-backend
