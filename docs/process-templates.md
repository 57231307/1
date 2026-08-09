# 10 道精细工序模板

## 概述

本文档定义了面料行业标准的 10 道精细工序模板，用于生产排程和工艺路线配置。

## 工序列表

| 序号 | 工序代码 | 工序名称 | 英文名称 | 所属分类 | 标准工时（天） |
|------|----------|----------|----------|----------|----------------|
| 1 | PREP-DISTRIBUTE | 配布 | Fabric Distribution | 前处理 | 0.5 |
| 2 | PREP-SCOURING | 精练 | Scouring | 前处理 | 1.0 |
| 3 | PREP-BLEACHING | 漂白 | Bleaching | 前处理 | 1.0 |
| 4 | DYE-MAIN | 染色 | Dyeing | 染色 | 2.0 |
| 5 | DYE-MATCH | 对色 | Color Matching | 染色 | 0.5 |
| 6 | FINISH-FOLDING | 理布 | Fabric Folding | 后整理 | 0.5 |
| 7 | FINISH-DRYING | 烘干 | Drying | 后整理 | 1.0 |
| 8 | FINISH-SETTING | 定型 | Setting | 后整理 | 1.0 |
| 9 | QC-FINAL-COLOR | 成品对色 | Final Color Check | 质检 | 0.5 |
| 10 | QC-INSPECTION | 成检 | Final Inspection | 质检 | 0.5 |

## 工艺路线模板

### 标准染色工艺路线

```
配布 → 精练 → 漂白 → 染色 → 对色 → 理布 → 烘干 → 定型 → 成品对色 → 成检
```

### 简化工艺路线（浅色）

```
配布 → 精练 → 染色 → 对色 → 烘干 → 定型 → 成检
```

### 特殊工艺路线（需要丝光）

```
配布 → 精练 → 漂白 → 丝光 → 染色 → 对色 → 理布 → 烘干 → 定型 → 成品对色 → 成检
```

## 配置说明

1. 在 `process_route` 表中创建工艺路线
2. 在 `process_step_record` 表中配置各工序步骤
3. 关联工艺路线到产品或订单
4. 根据实际需求调整工时和工序顺序
