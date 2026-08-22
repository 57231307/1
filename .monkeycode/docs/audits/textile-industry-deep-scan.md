# 纺织行业深度字段扫描审计报告

- 审计代理：审计代理（只读模式）
- 审计范围：`backend/src/`
- 审计日期：2026-08-22
- 审计约束：不修改代码文件、不创建 PR、不推送；仅写入本报告文件
- 扫描方法：按指定 grep 关键词对 `backend/src/` 进行正则匹配，逐项记录字段清单与完整性评估

---

## 5.4 面料规格参数

**扫描命令**

```bash
grep -rn "weight\|width\|fabric_type\|yarn_spec\|composition\|克重\|幅宽\|成分\|纱支" backend/src/models/product.rs
```

**命中字段清单（backend/src/models/product.rs）**

| 行号 | 字段 | 类型 | 业务含义 |
|------|------|------|----------|
| 44-45 | `fabric_composition` | `Option<String>` | 面料成分（如 65% 棉 35% 涤） |
| 46 | 纱支（注释） | — | 如 40S |
| 50-52 | `width` | `Option<Decimal>` | 幅宽（cm） |
| 53-55 | `gram_weight` | `Option<Decimal>` | 克重（g/m²） |

**未命中关键词**

- `fabric_type`（面料类型/织造类别，如机织/针织/梭织）
- `yarn_spec`（纱支作为独立结构化字段，当前仅出现在注释中）

**完整性评估**

面料的四个核心规格参数（成分、纱支、幅宽、克重）已在 `product.rs` 落地三个结构化字段（`fabric_composition`、`width`、`gram_weight`），覆盖了纺织贸易中最关键的交易规格。

存在两个缺口：
1. **纱支未结构化**：注释提到"纱支：如 40S"，但没有对应的 `yarn_spec` 或 `yarn_count` 字段，纱支信息可能被并入 `fabric_composition` 文本或缺失。纱支是坯布规格和报价的核心参数，建议独立成字段。
2. **面料类型缺失**：未发现 `fabric_type`/`weave_type` 字段（机织/针织/非织造等），影响品类归类和工艺路线选择。

**结论：部分完整。** 核心 4 项规格中覆盖 3 项结构化字段，纱支与面料类型存在结构化缺失。

---

## 5.5 色牢度标准

**扫描命令**

```bash
grep -rn "color_fastness\|色牢度\|light_fastness\|wash_fastness\|rubbing" backend/src/
```

**命中字段清单**

| 文件 | 行号 | 字段/常量 | 类型 | 业务含义 |
|------|------|-----------|------|----------|
| models/color_card.rs | 30 | `color_fastness_grade` | `Option<String>` | 色牢度等级（A/B/C/D） |
| models/lab_dip_request.rs | 45-46 | `color_fastness_req` | `Option<String>` | 色牢度要求 JSON：{soaping, rubbing, daylight, chlorine, dry_cleaning} |
| models/quality_issue_dto.rs | 32-33 | `color_fastness_grade` | `Option<i32>` | ISO 105 色牢度等级（1-5） |
| services/lab_dip_ops/types.rs | 29, 53 | `color_fastness_req` | `Option<String>` | 打样色牢度要求 |
| services/custom_order_quality_service.rs | 81-85 | — | — | ISO 105 等级校验（1-5 范围） |
| services/fabric_inspection_service.rs | 877 | `COLOR_FASTNESS` 常量 | `&str` | 检测项目标识 |
| models/fabric_physical_test_record.rs | 33 | 检测项枚举 | — | color_fastness 作为检测项目之一 |

**标准引用**

- `models/lab_dip_request.rs:45`：色牢度要求以 JSON 结构化存储，覆盖 5 项子指标：soaping（皂洗）、rubbing（摩擦）、daylight（日晒）、chlorine（氯漂）、dry_cleaning（干洗）。
- `services/custom_order_quality_service.rs:4`：明确引用 **GB/T 26377-2022 + ISO 105** 行业规则。
- `services/custom_order_quality_service.rs:81-85`：对 ISO 105 等级做 1-5 范围校验。

**未命中关键词**

- `light_fastness`、`wash_fastness` 未作为独立字段名出现，但已包含在 `color_fastness_req` 的 JSON 子键 `daylight`、`soaping` 中。

**完整性评估**

色牢度标准建模完整度较高：
1. **多业务场景覆盖**：色卡（color_card）、打样（lab_dip_request）、质检异常（quality_issue_dto）、物理测试（fabric_physical_test_record）四条业务线均落地色牢度字段。
2. **双重等级体系**：既有 A/B/C/D 字母等级（色卡），又有 ISO 105 的 1-5 数字等级（质检），符合纺织行业双轨评级惯例。
3. **结构化子指标**：`color_fastness_req` 以 JSON 承载 5 项子指标，覆盖纺织行业核心色牢度维度。
4. **标准引用与校验**：显式引用 GB/T 26377 与 ISO 105，并在服务层做范围校验。

存在一个口径不一致点：`color_card.color_fastness_grade` 用 `Option<String>`（A/B/C/D），`quality_issue_dto.color_fastness_grade` 用 `Option<i32>`（1-5），两种评级体系并存但未统一抽象，可能导致跨模块比较时需手工映射。

**结论：完整且规范。** 是 6 项中建模最充分的一项，标准引用、范围校验、多业务线覆盖齐备；建议统一字母/数字等级的抽象层。

---

## 5.6 工艺路线与 BOM

**扫描命令**

```bash
grep -rn "process_route\|bom\b" backend/src/models/ | head -20
```

**命中字段清单**

| 文件 | 行号 | 字段/关系 | 类型 | 业务含义 |
|------|------|-----------|------|----------|
| models/process_route.rs | 13 | `process_route` 表 | Entity | 工序路线模板 |
| models/bom_item.rs | 51-58 | `belongs_to` / `Related` | 关系 | BOM 明细归属 BOM 主表 |
| models/production_flow_card.rs | 32 | `process_route_id` | `Option<i32>` | 流程卡关联工艺路线 |
| models/production_flow_card.rs | 111-160 | `belongs_to` / `Related` | 关系 | 流程卡 → 工艺路线 |
| models/dto/flow_card_dto.rs | 48, 67, 101 | `process_route_id` | `Option<i32>` | 流程卡 DTO 承载工艺路线 |
| models/dto/wage_dto.rs | 18, 51 | `process_route_id` | `i32` / `Option<i32>` | 工资核算关联工艺路线 |
| models/energy_consumption_record.rs | 54, 90-105 | `process_route_id` + 关系 | `Option<i32>` | 能耗按工艺路线归集 |
| models/energy_allocation_rule.rs | 32, 60 | `process_route_id` + 关系 | `Option<i32>` | 能耗分摊规则绑定工艺路线 |

**完整性评估**

工艺路线与 BOM 建模具备完整的主从结构：
1. **主表落地**：`process_route`（工序路线模板）与 `bom`（主表，由 `bom_item.rs` 的 `belongs_to` 反推）均存在独立实体。
2. **主从关系**：`bom_item` 通过 `belongs_to = "super::bom::Entity"` 建立明细到主表的归属关系，符合 BOM 标准结构。
3. **跨业务穿透**：`process_route_id` 在流程卡（生产执行）、工资核算（计件成本）、能耗记录与分摊规则（能源成本）四个模块均有外键引用，工艺路线成为成本与执行的核心枢纽，建模深度优秀。

**结论：完整。** 主表 + 明细 + 跨模块外键引用齐全，工艺路线作为成本归集枢纽的设计符合纺织生产管理惯例。

---

## 5.7 批次追溯

**扫描命令**

```bash
grep -rn "dye_lot_no\|batch_no\|匹号\|piece_no\|米数" backend/src/models/ | head -20
```

**命中字段清单**

| 文件 | 行号 | 字段 | 类型 | 业务含义 |
|------|------|------|------|----------|
| models/color_card_issue.rs | 29 | `dye_lot_no` | `Option<String>` | 色卡领用关联染批号 |
| models/ar_invoice.rs | 31, 33 | `batch_no` / `dye_lot_no` | `Option<String>` | 应收发票承载批次 |
| models/inventory_stock.rs | 35, 39 | `batch_no` / `dye_lot_no` | `String` / `Option<String>` | 库存按批次管理 |
| models/dto/dye_batch_dto.rs | 16, 34, 93, 95, 108, 134, 136 | `batch_no` / `original_batch_no` / `rework_batch_no` / `target_batch_no` / `source_batch_nos` | 多类型 | 染批全生命周期（原始/返工/合并/拆分） |
| models/dto/scheduling_dto.rs | 117 | `batch_no` | `Option<String>` | 排程关联批次 |
| models/dto/flow_card_dto.rs | 47, 66, 85 | `dye_lot_no` | `Option<String>` | 流程卡关联染批 |
| models/inventory_count_item.rs | 62, 64 | `dye_lot_no` / `batch_no` | — | 盘点按批次粒度 |
| models/business_trace.rs | 19 | `batch_no` | `String` | 业务追溯主键 |
| models/production_flow_card.rs | 29 | 缸号（注释指向 dye_batch.batch_no） | — | 冗余便于扫码查询 |

**未命中关键词**

- `piece_no`（匹号）、`米数`（匹长/码数）未在 models/ 中命中。

**完整性评估**

染批维度追溯建模深度优秀：
1. **双号体系**：`batch_no`（缸号/染批号）与 `dye_lot_no`（色批号）并存，符合纺织染整"一缸多色批"的实际业务。
2. **全生命周期**：`dye_batch_dto` 覆盖 `original_batch_no`（原始批）、`rework_batch_no`（返工批）、`target_batch_no`/`source_batch_nos`（合并/拆分），批次流转链路完整。
3. **跨模块穿透**：批次号在库存、盘点、发票、排程、流程卡、色卡领用、业务追溯 7 个模块均有引用，批次作为主追溯键。
4. **冗余查询优化**：`production_flow_card` 注释明确为扫码查询冗余 `batch_no`，体现工程化考量。

存在两个缺口：
1. **匹号（piece_no）缺失**：染批之下通常还需按"匹"管理（一缸出多匹布），未发现 `piece_no` 字段，匹级追溯无法实现。
2. **米数/匹长缺失**：未发现匹长/码数字段，影响按米数发货与结算的精细化管理。

**结论：染批级完整，匹级缺失。** 批次追溯在染批粒度上覆盖全生命周期与多模块；但匹号与匹长未建模，无法支撑"匹"级追溯与按米结算。

---

## 5.8 色差评级

**扫描命令**

```bash
grep -rn "color_diff\|色差\|delta_e\|评级\|color_fastness_grade" backend/src/ | head -20
```

> 备注：原指令关键词 `grade` 在 `cli/util/upgrade.rs` 等非业务文件中产生大量噪声命中（如 "upgrade"、"downgrade"），已改用 `color_fastness_grade` 精确替代，并补充 `delta_e`、`color_diff`、`色差`、`评级` 聚焦业务语义。

**命中字段清单**

| 文件 | 行号 | 字段/函数 | 类型 | 业务含义 |
|------|------|-----------|------|----------|
| utils/color_space_converter.rs | 7 | CIELab ΔE 色差计算（注释） | — | CIE76 公式 |
| utils/color_space_converter.rs | 157-159 | `delta_e_is_acceptable` | `fn(f64)->bool` | ΔE ≤ 3.0 判可接受（GB/T 26377） |
| utils/color_space_converter.rs | 162-163 | `delta_e_76` | `fn(Lab,Lab)->f64` | CIE76 色差公式 |
| services/bulk_color_approval_service.rs | 176 | `delta_e_value` | `Option<Decimal>` | 大货色差 ΔE 值 |
| services/bulk_color_approval_service.rs | 180 | 色差判定结果（注释） | — | V15 P0-F17 审计报告 11.3 |
| services/bulk_color_approval_service.rs | 191-199 | `evaluate_delta_e` | `fn` | 色差判定标准（阈值分级） |
| services/bulk_color_approval_service.rs | 625 | 色差判定规则（注释） | — | ΔE≤1.2 同色通过 / ΔE≤2.5 让步接收 / ΔE>2.5 不合格 |
| models/fabric_defect_record.rs | 23 | `color_diff` | 疵点编码 | 色差作为疵点类型 |
| services/ai/quality_pred.rs | 177-182 | `color_diff` / `color_fastness` 归因映射 | — | 质检归因分类 |

**完整性评估**

色差评级建模具备完整的"计算-判定-应用"三层结构：
1. **底层计算**：`color_space_converter` 提供 CIE76 ΔE 公式与可接受性判定，引用 GB/T 26377 行业标准（ΔE ≤ 3.0）。
2. **业务判定**：`bulk_color_approval_service.evaluate_delta_e` 实现三级阈值判定：通过（≤1.2）/ 让步接收（≤2.5）/ 不合格（>2.5），符合纺织大货对色惯例，并支持高光敏感度场景。
3. **数据落地**：`delta_e_value` 以 `Option<Decimal>` 持久化到大货色差审批记录。
4. **疵点关联**：`fabric_defect_record` 将 `color_diff` 作为标准疵点编码，质检归因模型 `quality_pred` 将 `color_diff` 映射为"颜色差异"类别，形成从检测到归因的闭环。
5. **审计留痕**：注释多次引用 "V15 P0-F17 审计报告 11.3"，说明该模块已经过既往审计迭代。

**结论：完整且规范。** ΔE 计算、三级阈值判定、数据持久化、疵点关联、归因映射五层齐备，标准引用明确。

---

## 5.9 缩水率/纬斜

**扫描命令**

```bash
grep -rn "shrinkage\|缩水\|skew\|纬斜" backend/src/
```

**命中字段清单**

| 文件 | 行号 | 字段/常量 | 类型 | 业务含义 |
|------|------|-----------|------|----------|
| models/fabric_physical_test_record.rs | 7-8 | 纬斜（skewness）/ 缩水率（shrinkage）模块注释 | — | 物理测试核心项 |
| models/fabric_physical_test_record.rs | 33 | 检测项枚举 | — | skewness/shrinkage/pilling/handfeel/tensile/tear/weight_gsm/color_fastness/width/density |
| services/fabric_inspection_service.rs | 736 | `skew_lane` | 疵点编码值 | 纬斜作为疵点类型 |
| services/fabric_inspection_service.rs | 870-871 | `SKEWNESS` / `SHRINKAGE` 常量 | `&str` | 检测项常量 |
| models/fabric_defect_record.rs | 7 | 疵点类型注释 | — | 纬斜列入疵点类型清单 |
| models/fabric_defect_record.rs | 23 | `skew_lane` | 疵点标准编码 | 纬斜（标准编码） |

**噪声命中（已剔除）**

- `services/totp_service.rs:19,63,102` 的 `.with_skew(1)` 属于 TOTP 时间窗口容差，与纺织纬斜无关，已剔除。

**完整性评估**

缩水率与纬斜建模具备"检测项 + 疵点"双视角：
1. **物理测试视角**：`fabric_physical_test_record` 将 `skewness`（纬斜）与 `shrinkage`（缩水率）列为独立检测项目，模块注释明确为前两项核心指标。
2. **疵点视角**：`fabric_defect_record` 与 `fabric_inspection_service` 将 `skew_lane`（纬斜）作为标准疵点编码，检测常量 `SKEWNESS`/`SHRINKAGE` 统一管理。
3. **常量集中**：`fabric_inspection_service` 集中定义检测项常量，便于跨服务引用一致性。

存在一个缺口：**缩水率未作为疵点编码**。纬斜同时出现在"物理测试检测项"和"疵点编码"双视角，但缩水率仅在物理测试视角出现，未发现 `shrinkage` 作为疵点编码。业务上缩水率超标通常也作为疵点/质量异常记录，建议补齐疵点视角。

**结论：基本完整。** 两项指标在物理测试与疵点双视角均有覆盖（纬斜双视角齐全，缩水率仅在物理测试视角），缩水率的疵点视角存在缺口。

---

## 6 项完整性评估总结

| 项号 | 审计维度 | 完整性等级 | 核心结论 |
|------|----------|------------|----------|
| 5.4 | 面料规格参数 | 部分完整 | 成分/幅宽/克重已结构化；纱支仅注释无字段；面料类型缺失 |
| 5.5 | 色牢度标准 | 完整且规范 | 4 业务线覆盖、双等级体系、JSON 子指标、GB/T 26377+ISO 105 标准引用与校验齐备；字母/数字等级口径不统一 |
| 5.6 | 工艺路线与 BOM | 完整 | 主表+明细+流程卡/工资/能耗多模块外键，工艺路线作为成本枢纽 |
| 5.7 | 批次追溯 | 染批级完整，匹级缺失 | 染批全生命周期+7 模块穿透；匹号、匹长/米数未建模 |
| 5.8 | 色差评级 | 完整且规范 | CIE76 计算+三级阈值判定+持久化+疵点关联+归因映射五层齐备 |
| 5.9 | 缩水率/纬斜 | 基本完整 | 双视角覆盖（纬斜齐全）；缩水率缺疵点视角 |

**整体评估**

6 项中有 3 项（5.5 色牢度、5.6 工艺路线与 BOM、5.8 色差评级）达到完整或完整且规范，建模深度与行业标准引用表现优秀，体现纺织 ERP 的专业底色。

3 项存在明确缺口：
- **5.4 面料规格**：纱支与面料类型结构化缺失，影响坯布规格与品类管理。
- **5.7 批次追溯**：染批级建模优秀，但匹号/匹长缺失，无法支撑匹级追溯与按米结算。
- **5.9 缩水率/纬斜**：缩水率未补齐疵点视角编码。

优先级建议：5.7 匹级追溯缺口影响按米结算与匹级追溯，业务影响最广，建议优先补齐 `piece_no` 与匹长字段；5.4 纱支字段化次之；5.9 缩水率疵点编码再次之。

---

*本报告仅基于 grep 关键词扫描结果生成，未修改任何代码文件，未创建 PR，未执行推送。*
