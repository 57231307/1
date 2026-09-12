# 销售预测与订单/库存联动设计

## 概述

本文档描述销售预测与订单管理、库存补货的联动机制设计。

## 业务需求

1. 销售预测应自动触发库存补货建议
2. 订单变化应反馈到预测模型
3. 库存预警应考虑预测数据

## 联动流程

### 预测 → 补货

```
销售预测 → 计算未来需求 → 对比现有库存 → 生成补货建议 → 审批 → 采购订单
```

### 订单 → 预测

```
销售订单 → 更新实际销量 → 修正预测模型 → 调整预测值
```

### 库存 → 预警

```
库存监控 → 结合预测数据 → 计算安全库存 → 触发预警
```

## 实现建议

1. **预测服务**：使用 AI 模型生成销售预测
2. **联动服务**：监听订单和库存变化，触发联动逻辑
3. **补货服务**：根据预测和库存生成补货建议
4. **审批流程**：补货建议需经审批后生成采购订单

## 数据流

```rust
// 预测 → 补货
fn generate_replenishment_suggestion(
    forecast: &SalesForecast,
    current_stock: &InventoryStock,
    safety_stock: Decimal,
) -> ReplenishmentSuggestion {
    let future_demand = forecast.predicted_quantity;
    let shortage = future_demand - current_stock.available_quantity - safety_stock;
    if shortage > Decimal::ZERO {
        ReplenishmentSuggestion {
            product_id: forecast.product_id,
            suggested_quantity: shortage,
            reason: "基于销售预测".to_string(),
        }
    } else {
        // 不需要补货
    }
}
```
