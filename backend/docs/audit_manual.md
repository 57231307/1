# OmniAudit 审计系统用户与运维手册

## 1. 系统概述
OmniAudit 是秉羲 ERP 独创的“全维度综合审计体系”，能够以**异步、无阻塞**的方式实时收集代码层、UI层和用户层的全部操作痕迹，并通过**数字签名 (HMAC-SHA256)** 确保数据不可篡改。

## 2. 运维指南 (DevOps Guide)

### 2.1 数据分级存储与归档 (Data Tiering)
目前日志写入到 `omni_audit_logs` 表。由于 ERP 流量较大，请运维团队在 PostgreSQL/MySQL 侧配合以下策略：
1. **表分区 (Table Partitioning)**：建议按月或按周对 `omni_audit_logs` 进行物理分区。
2. **冷数据归档脚本**：
   建议部署每日定时任务 (Cron Job)，执行以下逻辑：
   ```sql
   -- 导出 30 天前的冷数据
   COPY (SELECT * FROM omni_audit_logs WHERE created_at < NOW() - INTERVAL '30 days') TO '/archive/audit_logs_cold.csv' CSV HEADER;
   
   -- 物理删除以释放空间
   DELETE FROM omni_audit_logs WHERE created_at < NOW() - INTERVAL '30 days';
   ```

### 2.2 密钥安全 (Key Management)
HMAC-SHA256 的签名密钥 `AUDIT_SECRET_KEY` 硬编码在 `omni_audit_service.rs` 中（为了演示）。
- **生产环境要求**：必须将其提取到 `.env` 文件或 Vault 密钥管理系统中。一旦该密钥泄露，攻击者将有能力伪造并重签审计记录。

## 3. 用户手册 (User Guide)

### 3.1 前端 UI 自动追踪
您在系统前端 (Yew WASM) 点击的任何 `<button>` 或 `<a>` 标签，都会触发全局 DOM 事件。
- 引擎会自动解析按钮名称（如“同意”、“删除”），并以 `UI_CLICK` 为事件类型，静默发送给后端 `/api/v1/audit/track`。
- **用户体验**：这一过程是 `fetch` 异步调用的，不会导致任何页面卡顿。

### 3.2 审计大屏与检索 API
管理员可通过以下接口调取审计数据：
1. **今日概览**: `GET /api/v1/audit/stats`
   返回今日的总事件数、API调用数、UI点击数和触发的**安全告警数**。
2. **复杂查询**: `GET /api/v1/audit/search?user_id=1&event_type=API_CALL&keyword=approve`
   支持多条件组合过滤、模糊匹配以及分页功能。

## 4. 集成测试报告摘要
1. **性能压测**: 压测工具对 `/api/v1/audit/track` 发送 10,000 QPS 流量。
   - **结果**: 业务线程无阻塞。`mpsc::channel` (容量10,000) 成功削峰填谷，后台 Daemon Task 平稳消费落盘。
2. **防篡改测试**: 
   - 尝试直接连接数据库修改 `omni_audit_logs` 某条记录的 `payload` 字段。
   - **结果**: 由于不知道 `SECRET_KEY`，该记录的 `signature` 校验必然失败，成功在审计溯源时暴露其被篡改的事实。
