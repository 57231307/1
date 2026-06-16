# 冰溪 ERP 主备隔离调用设计

> **设计时间**: 2026-06-16
> **设计原则**: 仅主调用不可用时才自动切换至备用，主调用正常运行时禁用备用
> **适用范围**: 8 大核心功能

---

## 0. 现状扫描（2026-06-16）

| 核心功能 | 当前实现 | 主备隔离 | 优先级 |
|----------|----------|----------|--------|
| 数据库连接 | 单点（DATABASE_URL） | ❌ 无 | P0 |
| 缓存服务 | 内存（init_service 注释：生产应改用 Redis） | ❌ 无 | P0 |
| 消息队列 | 无 | ❌ 无 | P1 |
| 文件存储 | 无 | ❌ 无 | P1 |
| 短信网关 | 通知类型 SMS（单点） | ❌ 无 | P1 |
| 邮件服务 | EmailService 已支持多 provider（sendgrid/aliyun/tencent/smtp） | ⚠️ 部分 | P1 |
| 搜索引擎 | 无（依赖 PostgreSQL LIKE） | ❌ 无 | P2 |
| OCR 识别 | 无 | ❌ 无 | P2 |

---

## 1. 设计原则

### 1.1 核心约束

1. **仅主调用不可用时才自动切换**
   - 主调用超时（默认 3 秒）
   - 主调用失败（5xx / 业务异常）
   - 主调用降级（熔断器打开）
2. **主调用正常运行时禁用备用**
   - 避免资源浪费
   - 避免双写不一致
3. **故障转移后支持回切**
   - 主调用恢复后自动回切（半开状态探测）
4. **告警和监控**
   - 主备切换时记录事件
   - 切换频率超阈值告警

### 1.2 状态机

```
[主调用] --失败/超时--> [熔断] --探测成功--> [主调用]
   ↑                       ↓
   └────回切成功──────── [备用调用]
```

### 1.3 关键参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| 主调用超时 | 3s | 单次调用最长等待 |
| 熔断阈值 | 5 次失败 | 触发熔断的连续失败数 |
| 熔断时长 | 30s | 熔断器打开后多久进入半开 |
| 半开探测 | 1 次 | 半开状态探测请求数 |
| 备用超时 | 5s | 备用调用最长等待 |
| 告警阈值 | 5 次/小时 | 切换频率告警 |

---

## 2. 8 大核心功能的主备隔离设计

### 2.1 数据库连接（P0）

**主**：PostgreSQL 16（远程 39.99.34.194:5432）
**备**：PostgreSQL 16（同实例只读副本 / 远程灾备）

**故障转移条件**：
- 主库连接超时
- 主库 5xx 错误
- 熔断器打开

**回切条件**：
- 半开探测查询成功

**实现位置**：
- `backend/src/database/mod.rs`（新增 FailoverDatabase）
- `backend/src/utils/failover_db.rs`（新增）

### 2.2 缓存服务（P0）

**主**：Redis 7（远程）
**备**：进程内 LRU 缓存（`moka` crate）

**故障转移条件**：
- Redis 连接超时
- Redis 返回错误

**回切条件**：
- Redis 恢复后，新写入优先写 Redis，旧的进程内 LRU 异步清除

**实现位置**：
- `backend/src/cache/mod.rs`（重写）
- `backend/src/cache/failover.rs`（新增）

### 2.3 消息队列（P1）

**主**：Redis Stream（轻量）
**备**：进程内 Channel（mpsc）

**故障转移条件**：
- Redis 不可用

**回切条件**：
- Redis 恢复后将进程内 Channel 积压消息批量迁移到 Redis Stream

**实现位置**：
- `backend/src/mq/mod.rs`（新增）
- `backend/src/mq/failover.rs`（新增）

### 2.4 文件存储（P1）

**主**：阿里云 OSS
**备**：本地文件系统（`/opt/bingxi-erp/storage`）

**故障转移条件**：
- OSS 上传/下载超时
- OSS 5xx 错误

**回切条件**：
- 半开探测成功
- 定期将本地文件异步上传到 OSS（防止本地磁盘满）

**实现位置**：
- `backend/src/storage/mod.rs`（新增）
- `backend/src/storage/failover.rs`（新增）

### 2.5 短信网关（P1）

**主**：阿里云短信
**备**：腾讯云短信

**故障转移条件**：
- 主网关 4xx/5xx
- 主网关超时

**回切条件**：
- 主网关恢复（探测成功 3 次）

**实现位置**：
- `backend/src/services/sms_service.rs`（重写）
- `backend/src/services/sms/failover.rs`（新增）

### 2.6 邮件服务（P1）

**主**：SendGrid
**备**：阿里云邮件 / SMTP（按用户配置）

**当前实现**：`EmailService::from_env()` 已支持多 provider
**改造点**：在 `from_env()` 之上增加 FailoverEmailService

**故障转移条件**：
- 主邮件服务失败
- 配额耗尽

**回切条件**：
- 探测成功

**实现位置**：
- `backend/src/services/email_service.rs`（扩展）
- `backend/src/services/email/failover.rs`（新增）

### 2.7 搜索引擎（P2）

**主**：PostgreSQL FTS（现有 LIKE 升级）
**备**：进程内倒排索引（`tantivy` crate）

**故障转移条件**：
- PostgreSQL FTS 超时

**回切条件**：
- 自动

**实现位置**：
- `backend/src/search/mod.rs`（新增）

### 2.8 OCR 识别（P2）

**主**：百度 OCR
**备**：腾讯云 OCR

**故障转移条件**：
- 主 OCR 失败

**回切条件**：
- 主 OCR 恢复

**实现位置**：
- `backend/src/services/ocr_service.rs`（新增）

---

## 3. 统一抽象接口

```rust
// backend/src/utils/failover/mod.rs
#[async_trait]
pub trait FailoverCall<T, E> {
    async fn primary_call(&self) -> Result<T, E>;
    async fn backup_call(&self) -> Result<T, E>;
    
    /// 带主备隔离的调用
    async fn call(&self) -> Result<T, FailoverError<E>> {
        if !self.is_primary_circuit_open() {
            match timeout(self.primary_timeout, self.primary_call()).await {
                Ok(Ok(v)) => {
                    self.record_primary_success();
                    return Ok(v);
                }
                Ok(Err(e)) => self.record_primary_failure(),
                Err(_) => self.record_primary_timeout(),
            }
        }
        
        // 主不可用，调用备用
        warn!("Primary call failed, switching to backup");
        self.record_failover();
        match timeout(self.backup_timeout, self.backup_call()).await {
            Ok(Ok(v)) => {
                self.record_backup_success();
                Ok(v)
            }
            Ok(Err(e)) => Err(FailoverError::BothFailed(e)),
            Err(_) => Err(FailoverError::BothTimeout),
        }
    }
}
```

---

## 4. 配置示例

```toml
# config/failover.toml
[database]
primary_url = "postgresql://user:pass@39.99.34.194:5432/bingxi"
backup_url = "postgresql://user:pass@backup.example.com:5432/bingxi"
primary_timeout_ms = 3000
backup_timeout_ms = 5000
circuit_breaker_threshold = 5
circuit_breaker_duration_s = 30

[cache]
primary_url = "redis://redis.example.com:6379"
backup_max_entries = 10000  # 进程内 LRU 大小
primary_timeout_ms = 1000
backup_timeout_ms = 0  # 内存操作无超时

[sms]
primary_provider = "aliyun"
backup_provider = "tencent"
primary_timeout_ms = 3000
backup_timeout_ms = 5000

[email]
primary_provider = "sendgrid"
backup_provider = "aliyun"
primary_timeout_ms = 5000
backup_timeout_ms = 8000
```

---

## 5. 监控和告警

### 5.1 指标

- `failover_primary_total{func="..."}` — 主调用次数
- `failover_primary_failed_total{func="..."}` — 主调用失败次数
- `failover_backup_total{func="..."}` — 备用调用次数
- `failover_switch_total{func="..."}` — 切换次数
- `failover_circuit_state{func="..."}` — 熔断器状态（0/1/2）

### 5.2 告警规则

| 规则 | 阈值 | 级别 |
|------|------|------|
| 主备切换频率 | > 5 次/小时 | P2 |
| 备用调用失败率 | > 10% | P1 |
| 熔断器持续打开 | > 5 分钟 | P1 |
| 主备同时不可用 | 任意时长 | P0 |

### 5.3 日志

```rust
info!(
    func = "send_sms",
    primary = "aliyun",
    backup = "tencent",
    switch = true,
    latency_ms = 3200,
    "failover triggered"
);
```

---

## 6. 实施计划

### 阶段 1（P0，2 周）

- 数据库主备隔离
- 缓存主备隔离

### 阶段 2（P1，2 周）

- 消息队列
- 文件存储
- 短信网关
- 邮件服务

### 阶段 3（P2，1 周）

- 搜索引擎
- OCR 识别

### 阶段 4（验证，1 周）

- 故障注入测试（chaos testing）
- 性能基准
- 监控大盘

**总工作量**：6 周

---

## 7. 验收标准

- [ ] 8 大功能均实现主备隔离
- [ ] 主调用失败 → 备用调用 < 100ms 切换
- [ ] 主调用恢复 → 自动回切 < 30s
- [ ] 故障注入测试 100% 通过
- [ ] 监控指标完整
- [ ] 告警规则覆盖所有异常
- [ ] 配置化（不改代码可切换主备）
- [ ] 文档完整（每功能有 README）
