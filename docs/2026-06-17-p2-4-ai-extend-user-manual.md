# P2-4 AI 分析深化用户手册

> 创建时间: 2026-06-17
> 关联版本: 2026.522.2
> 目标用户: 工艺员 / 质量管理员 / 工厂主管

## 一、功能概述

P2-4 在原有 4 个 AI 子能力（销售预测、库存优化、异常检测、智能推荐）基础上，新增 **2 个能力**：

| 子能力 | 目标 | 算法 |
| --- | --- | --- |
| 工艺优化 | 染色参数智能推荐 | k-NN 加权平均 + 兜底表 |
| 质量预测 | 风险评分 + 趋势分析 | 历史趋势 + 保守兜底 |

合计新增 **16 个 API 端点 + 4 个前端页面 + 2 个组件 + 2 张持久化表**。

## 二、入口

1. 登录后侧边栏展开「智能分析」→「AI 分析深化」
2. 直链：`/ai-extend`
3. 看板展示 4 个 KPI（工艺优化历史 / 应用率 / 质量预测历史 / 待确认预警）

## 三、工艺优化使用流程

### 3.1 触发新推荐

1. 进入「AI 工艺优化」页面
2. 点击「+ 触发新推荐」
3. 填写：
   - 色号（必填，如 `BL-301`）
   - 色名（选填）
   - 布类（必填，如 `棉` / `涤纶`）
   - 染料类型（选填，下拉选择）
   - k-NN k 值（默认 5，范围 1-20）
4. 点击「生成推荐」
5. 系统返回 2 字段：
   - `id`：记录 ID（用于后续应用反馈）
   - `response.recommended_params`：推荐参数
   - `response.confidence`：置信度
   - `response.source`：`knn` 或 `fallback`
   - `response.candidates`：相似案例（仅 knn 路径）

### 3.2 算法说明

| 场景 | 路径 | 表现 |
| --- | --- | --- |
| 历史相似案例 ≥ 3 条 | k-NN 加权 | 置信度 ≈ 0.85-0.95 |
| 历史相似案例 < 3 条 | 典型参数表 | 置信度 = 0.6 |
| 无任何历史 | 典型参数表 | 置信度 = 0.6 |

### 3.3 应用反馈

1. 在「工艺优化详情」页面查看推荐参数
2. 如已下生产，填写：
   - 反馈评分（1-5 星）
   - 反馈备注（最多 200 字）
3. 点击「标记为已应用」
4. 系统会更新记录 `is_applied=true` 与 `feedback_score`

## 四、质量预测使用流程

### 4.1 触发新预测

1. 进入「AI 质量预测」页面
2. 点击「+ 触发新预测」
3. 填写：
   - 产品 ID（必填，关联具体产品）
   - 检验类型（默认 `all`，可选 `incoming` / `inprocess` / `final` / `outgoing`）
   - 时间窗（默认 90 天，范围 7-365）
4. 点击「生成预测」

### 4.2 解读结果

| 字段 | 含义 |
| --- | --- |
| `risk_score` | 0-100 风险评分 |
| `risk_level` | low / medium / high |
| `trend` | up / flat / down / nodata |
| `trend_rate` | 趋势变化率（百分点） |
| `avg_qualification_rate` | 平均合格率 |
| `top_issues` | 主要问题归因 top 3 |
| `recommendations` | 建议措施 1-3 条 |
| `period_breakdown` | 按月分段统计 |

### 4.3 风险等级响应

- **高风险（评分 ≥ 60）**：建议 24 小时内确认 + 工艺员复盘
- **中风险（30-59）**：建议 3 天内确认
- **低风险（< 30）**：常规确认即可

### 4.4 确认与归档

1. 详情抽屉中查看完整趋势图 + 归因 + 建议
2. 列表中点击「确认」→ 标记为 `is_acknowledged=true`
3. 高风险记录可「删除」（仅创建者或管理员）

## 五、API 速查

### 工艺优化
- `POST /api/v1/erp/ai/process-optimizations` 触发推荐
- `GET  /api/v1/erp/ai/process-optimizations` 列表
- `GET  /api/v1/erp/ai/process-optimizations/{id}` 详情
- `POST /api/v1/erp/ai/process-optimizations/{id}/apply` 应用反馈
- `DELETE /api/v1/erp/ai/process-optimizations/{id}` 删除
- `GET  /api/v1/erp/ai/process-optimizations/by-color?color_no=...&fabric_type=...` 按色号+布类历史
- `POST /api/v1/erp/ai/process-optimizations/batch` 批量（最多 20 条）

### 质量预测
- `POST /api/v1/erp/ai/quality-predictions` 触发预测
- `GET  /api/v1/erp/ai/quality-predictions` 列表
- `GET  /api/v1/erp/ai/quality-predictions/{id}` 详情
- `POST /api/v1/erp/ai/quality-predictions/{id}/acknowledge` 确认
- `DELETE /api/v1/erp/ai/quality-predictions/{id}` 删除
- `GET  /api/v1/erp/ai/quality-predictions/by-product?product_id=...` 按产品历史
- `POST /api/v1/erp/ai/quality-predictions/batch` 批量（最多 20 条）

### 看板 / 健康
- `GET /api/v1/erp/ai/summary` AI 概览
- `GET /api/v1/erp/ai/health` 健康检查

## 六、最佳实践

1. **积累历史**：系统启动初期（如 < 3 条相似案例）会走兜底；建议至少录入 10 条历史染色配方后再依赖推荐
2. **人工兜底**：AI 推荐仅作参考，实际投产前由工艺员复核
3. **反馈闭环**：每次应用后必须填写反馈评分，否则无法形成「应用→反馈→学习」闭环
4. **多租户隔离**：所有数据按 `tenant_id` 严格隔离，跨租户访问会被拒绝
5. **定期看板**：建议每周一查看 AI 概览，对比应用率与高风险预警数

## 七、常见问题

**Q1：为什么 k-NN 路径下置信度反而是 0.85 起步？**
A：k-NN 加权时已用相似度归一化，相似度 > 0.7 的案例占比越高置信度越高。最低保底 0.85 以保证推荐稳定性。

**Q2：删除工艺优化记录会丢失反馈吗？**
A：是。删除前请确认该批次已闭环归档。

**Q3：批量请求超过 20 条会怎样？**
A：返回 `400 Bad Request` 与「批量请求数不得超过 20」错误。建议分批提交。

**Q4：质量预测趋势为 `nodata` 表示什么？**
A：当前时间窗内无任何检验记录。建议先录入历史数据。

**Q5：如何知道服务是否正常？**
A：调用 `GET /api/v1/erp/ai/health` 应返回 `{ "status": "ok", "version": "P2-4" }`。
