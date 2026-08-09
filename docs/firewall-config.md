# 防火墙配置指南

## 概述

本文档描述 ERP 系统部署时的防火墙配置要求。

## 端口要求

### 必需端口

| 端口 | 协议 | 用途 | 方向 |
|------|------|------|------|
| 443 | TCP | HTTPS API 访问 | 入站 |
| 80 | TCP | HTTP 重定向到 HTTPS | 入站 |
| 5432 | TCP | PostgreSQL 数据库 | 内部 |
| 6379 | TCP | Redis 缓存 | 内部 |
| 9090 | TCP | Prometheus 监控 | 内部 |
| 9093 | TCP | Alertmanager 告警 | 内部 |

### 可选端口

| 端口 | 协议 | 用途 | 方向 |
|------|------|------|------|
| 8080 | TCP | 开发环境 API | 入站 |
| 3000 | TCP | Grafana 仪表盘 | 内部 |

## iptables 示例规则

```bash
# 允许 HTTPS
iptables -A INPUT -p tcp --dport 443 -j ACCEPT

# 允许 HTTP（重定向用）
iptables -A INPUT -p tcp --dport 80 -j ACCEPT

# 允许 SSH（仅限管理网段）
iptables -A INPUT -p tcp --dport 22 -s 10.0.0.0/8 -j ACCEPT

# 拒绝其他入站
iptables -P INPUT DROP
```

## 安全建议

1. **最小权限原则**：仅开放必需端口
2. **网络隔离**：数据库和缓存仅允许内部访问
3. **IP 白名单**：管理端口限制来源 IP
4. **日志记录**：记录所有被拒绝的连接
5. **定期审计**：每季度审查防火墙规则
