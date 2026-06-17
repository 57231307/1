# 冰溪 ERP Helm Chart

> P4-6 K8s 部署
> 适用版本：bingxi-erp 2026.522.2+

## 一、Chart 结构

```text
deploy/helm/erp/
├── Chart.yaml              # Chart 元数据
├── values.yaml             # 默认配置
├── README.md               # 本文档
└── templates/
    ├── _helpers.tpl         # 模板辅助函数
    ├── deployment.yaml      # Deployment
    ├── service.yaml         # Service
    ├── ingress.yaml         # Ingress
    ├── configmap.yaml       # ConfigMap（非敏感配置）
    ├── secret.yaml          # Secret（敏感配置）
    └── hpa.yaml             # HorizontalPodAutoscaler
```

## 二、前置条件

- Kubernetes 1.24+
- Helm 3.8+
- 已安装 nginx-ingress-controller
- 已安装 cert-manager（用于自动签发 TLS 证书）
- Prometheus Operator（可选，用于 ServiceMonitor）

## 三、配置敏感信息

**重要**：生产环境必须通过外部 Secret 管理敏感信息，**禁止**把真实密码提交到 Git。

```bash
# 1. 创建外部 Secret（推荐用 sealed-secrets 或 external-secrets）
kubectl create secret generic bingxi-erp-secret \
  --from-literal=DATABASE_URL='postgresql://erp:STRONG_PWD@postgres:5432/bingxi_erp' \
  --from-literal=REDIS_URL='redis://:STRONG_PWD@redis:6379' \
  --from-literal=JWT_SECRET='BASE64-256BIT-KEY' \
  --from-literal=ENCRYPTION_KEY='BASE64-256BIT-KEY' \
  --namespace=erp

# 2. 安装 Chart 时通过 values 引用（可选覆盖）
helm install erp ./deploy/helm/erp \
  --set secret.DATABASE_URL='postgresql://...' \
  --set secret.JWT_SECRET='...' \
  --namespace erp
```

## 四、部署命令

### 4.1 安装

```bash
# 默认配置
helm install erp ./deploy/helm/erp --namespace erp --create-namespace

# 自定义 values
helm install erp ./deploy/helm/erp \
  -f custom-values.yaml \
  --namespace erp --create-namespace
```

### 4.2 升级

```bash
# 升级到新版本
helm upgrade erp ./deploy/helm/erp \
  --set image.tag=2026.522.3 \
  --namespace erp

# 回滚到上一版本
helm history erp --namespace erp
helm rollback erp 1 --namespace erp
```

### 4.3 卸载

```bash
helm uninstall erp --namespace erp
```

## 五、配置项（values.yaml）

| 类别 | Key | 默认值 | 说明 |
|------|-----|--------|------|
| 镜像 | image.repository | 阿里云镜像仓库 | |
| 镜像 | image.tag | 2026.522.2 | 应用版本 |
| 副本 | replicaCount | 2 | 仅在 HPA 关闭时生效 |
| 资源 | resources.requests.cpu | 500m | |
| 资源 | resources.requests.memory | 512Mi | |
| 资源 | resources.limits.cpu | 2000m | |
| 资源 | resources.limits.memory | 2Gi | |
| 端口 | containerPort | 8080 | |
| Service | service.type | ClusterIP | |
| Ingress | ingress.enabled | true | |
| Ingress | ingress.className | nginx | |
| Ingress | ingress.hosts[0].host | erp.bingxi.example.com | |
| 缓存 | config.CACHE_CAPACITY | 10000 | |
| 缓存 | config.CACHE_TTL_SECS | 60 | |
| HPA | autoscaling.enabled | true | |
| HPA | autoscaling.minReplicas | 2 | |
| HPA | autoscaling.maxReplicas | 10 | |
| HPA | autoscaling.targetCPUUtilizationPercentage | 70 | |
| 安全 | podSecurityContext.runAsNonRoot | true | |
| 安全 | securityContext.readOnlyRootFilesystem | true | |
| 安全 | securityContext.allowPrivilegeEscalation | false | |

## 六、健康检查

| 探针 | 路径 | 用途 |
|------|------|------|
| livenessProbe | `/api/health` | 进程存活 |
| readinessProbe | `/api/health/ready` | 服务就绪（DB/Redis 已连接） |

## 七、HPA 自动扩缩

当满足以下任一条件时自动扩容：
- CPU > 70%
- 内存 > 80%

最大副本 10，最小副本 2。冷启动时间约 30s。

## 八、Pod 安全

- `runAsNonRoot: true` - 禁止以 root 运行
- `readOnlyRootFilesystem: true` - 只读根文件系统
- `allowPrivilegeEscalation: false` - 禁止提权
- `capabilities.drop: [ALL]` - 丢弃所有 Linux capabilities

## 九、验证

```bash
# 1. 查看 Pod 状态
kubectl get pods -n erp -l app.kubernetes.io/name=bingxi-erp

# 2. 查看 Service
kubectl get svc -n erp bingxi-erp

# 3. 查看 Ingress
kubectl get ingress -n erp

# 4. 端口转发测试
kubectl port-forward -n erp svc/bingxi-erp 8080:80

# 5. 健康检查
curl http://localhost:8080/api/health
```

## 十、故障排查

| 症状 | 排查方向 |
|------|---------|
| Pod 一直 Pending | 节点资源不足 / PVC 等待 |
| Pod CrashLoopBackOff | 镜像拉取失败 / 健康检查不通过 / Secret 缺失 |
| Ingress 502 | 后端 Service 不可达 / readinessProbe 失败 |
| HPA 不扩容 | metrics-server 未装 / CPU 指标缺失 |

## 十一、生产环境建议

1. **镜像**：使用私有镜像仓库 + 镜像签名验证
2. **Secret**：使用 sealed-secrets 或 external-secrets-operator
3. **备份**：Velero 定期备份 etcd 与 PVC
4. **监控**：Prometheus + Grafana + Alertmanager
5. **日志**：Loki + Promtail
6. **CI/CD**：ArgoCD GitOps 自动同步
7. **多副本**：生产环境至少 2 副本 + PodDisruptionBudget
