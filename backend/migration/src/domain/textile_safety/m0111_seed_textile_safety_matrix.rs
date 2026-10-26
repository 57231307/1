//! 纺织品安全合规域种子迁移（GB 18401 与 GB 31701 项目与限值矩阵、出口市场要求登记）
//!
//! ## 六要素
//! - **功能**：播种检测项目主数据 21 项、限值矩阵行（四维：项目乘安全类别乘产品形态乘
//!   条件标志）、出口市场要求登记 6 行。数值一律来自权威裁定文本，判定器只查表不硬编码。
//! - **调用方**：SeaORM migrator 链，经 `domain/textile_safety` 域注册，本支在域内末位
//!   执行，晚于 m0109 建表与 m0110 词表 CHECK，种子行天然受已生效 CHECK 校验。
//! - **入参**：无（纯种子迁移）。
//! - **传给谁**：`services/textile_safety_limit_judge.rs` 按行判定；送检服务按 basis_standard
//!   与矩阵推项目集；报价与订单侧经出口要求只读端点消费登记面。
//! - **存什么**：项目编码与测量类型、每项目在各维组合下的限值行（比较器与上下界）、
//!   出口要求的类型与细则文字。
//! - **存哪里**：PostgreSQL `textile_test_item`、`textile_limit_rule`、
//!   `textile_export_market_requirement` 三表。
//!
//! ## 幂等与可维护性口径
//! - 全部 INSERT 走 `ON CONFLICT ... DO NOTHING`（唯一键：项目 item_code、限值行四维
//!   加比较器、要求 requirement_code），可任意重放；已有行一律不覆盖，人工在判定表上
//!   的维护（放宽值修订、新增行）不被重放冲掉。要改标准值请走限值行维护端点，不回来改本种子。
//! - down 回收本域播种的项目与限值行与要求行：先删播种项目的全部限值行（含人工在播种
//!   项目上新增的行，down 属整体域回滚，回滚前先备份判定表），再删项目与要求行；送检单
//!   若已引用播种项目，清单行与核查行外键 RESTRICT 会显式拦住 down，不留静默半态。
//!
//! ## 限值来源逐条（裁定文本，勿凭记忆改动）
//! - GB 18401-2010：甲醛 A 不大于 20、B 75、C 300 mg/kg；pH 最终产品 A 4.0~7.5、
//!   B 4.0~8.5、C 4.0~9.0，后续湿处理的中间产品 pH 放宽到 4.0~10.5（放宽行只建在
//!   intermediate 形态加 wet_processing 条件上，最终产品沿用该档时矩阵无行可查，判定器
//!   显式报错）；耐水与耐酸汗渍与耐碱汗渍色牢度 A 不低于 3-4 级、B 与 C 不低于 3 级；
//!   耐干摩擦 A 不低于 4 级、B 与 C 不低于 3 级；耐湿摩擦 A 不低于 3-4 级、B 与 C 不低于 3 级；
//!   耐唾液 A 不低于 4 级、B 与 C 不考核（na 行）；可分解致癌芳香胺染料禁用且不大于
//!   20 mg/kg（forbidden 与 lte 两行并存，判定取合取）；异味必须为无。需洗涤褪色的非
//!   最终产品与本色漂白与扎染蜡染不考核色牢度（三枚豁免标签乘六个色牢度项目的 na 行）。
//! - GB 31701-2015：铅不大于 90 mg/kg、镉不大于 100 mg/kg、邻苯二甲酸酯六种各不大于
//!   0.1%、外层面料燃烧性能须达 1 级（1 级为最佳档，lte 1 表达）；包装与产品不得残留
//!   金属针等锐利物（must_none）；7 岁以下头部颈部不得有任何绳带（must_none，档位按
//!   条款口径把握）；腰部绳带伸出不大于 360 mm；附件抗拉强力、无锐利尖端边缘、不得用
//!   过小易吞咽附件、贴身婴幼儿服装耐久标签置于不接触皮肤位置四条款无裁定数值，建
//!   manual 条目与人工结论位，不伪装自动判定。
//! - 等级型限值以半级编码数值存储（3-4 级记 3.5，编码表在词表 textile_fastness_grade）。
//! - 出口要求行是登记位：OEKO-TEX 分级与 REACH 与 CPSIA 的法规数值以官方现行文本为准，
//!   本种子只在 detail 文字中记载通行口径，不生成限值矩阵行，不把法规数值混进判定表。

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// 播种的 21 枚项目编码，down 按此清单回收项目与其派生限值行。
const SEEDED_ITEM_CODES: &[&str] = &[
    "HCHO",
    "PH",
    "AZO",
    "ODOR",
    "FAST_WATER",
    "FAST_ACID_SWEAT",
    "FAST_ALKALI_SWEAT",
    "FAST_DRY_RUB",
    "FAST_WET_RUB",
    "FAST_SALIVA",
    "LEAD",
    "CADMIUM",
    "PHTHALATE",
    "FLAM_EXTERIOR",
    "METAL_SHARP",
    "ACCESSOR_TENSILE",
    "SHARP_EDGE",
    "SMALL_PARTS",
    "CORD_HEAD",
    "CORD_WAIST",
    "LABEL_POSITION",
];

/// 播种的 6 枚出口要求编码，down 按此清单回收。
const SEEDED_REQUIREMENT_CODES: &[&str] = &[
    "REQ_OEKO_100_EU",
    "REQ_REACH_SVHC_EU",
    "REQ_CPSIA_LEAD_US",
    "REQ_CPSIA_PHTHALATE_US",
    "REQ_CPSIA_CPC_US",
    "REQ_JP_FORMALDEHYDE",
];

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let sql = r#"
-- 一、检测项目主数据（GB 18401 基本 10 项加 GB 31701 加考 11 项）
INSERT INTO textile_test_item (item_code, item_name, item_kind, basis_standard, check_kind, method_standard, unit, description) VALUES
    ('HCHO', '甲醛含量', 'numeric', 'GB18401', 'measurable', 'GB/T 2912.1', 'mg/kg', 'GB 18401-2010 表1 甲醛限量，按安全类别三分'),
    ('PH', 'pH值', 'numeric', 'GB18401', 'measurable', 'GB/T 7573', NULL, '水萃取 pH 值，最终产品按类别区间，后续湿处理的中间产品可放宽到 4.0~10.5'),
    ('AZO', '可分解致癌芳香胺染料', 'numeric', 'GB18401', 'measurable', 'GB/T 19942', 'mg/kg', '禁用且不大于 20 mg/kg，判定为禁用行与上限行的合取'),
    ('ODOR', '异味', 'sensory', 'GB18401', 'measurable', NULL, NULL, '不得检出霉味、高沸程石油味、鱼腥味、芳香烃气味等任何异味'),
    ('FAST_WATER', '耐水色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 5713', '级', '耐水色牢度等级，A 类不低于 3-4 级、B 与 C 类不低于 3 级'),
    ('FAST_ACID_SWEAT', '耐酸汗渍色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 3921', '级', '耐酸汗渍色牢度等级，A 类不低于 3-4 级、B 与 C 类不低于 3 级'),
    ('FAST_ALKALI_SWEAT', '耐碱汗渍色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 3921', '级', '耐碱汗渍色牢度等级，A 类不低于 3-4 级、B 与 C 类不低于 3 级'),
    ('FAST_DRY_RUB', '耐干摩擦色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 3920', '级', '耐干摩擦色牢度等级，A 类不低于 4 级、B 与 C 类不低于 3 级'),
    ('FAST_WET_RUB', '耐湿摩擦色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 3920', '级', '耐湿摩擦色牢度等级，A 类不低于 3-4 级、B 与 C 类不低于 3 级'),
    ('FAST_SALIVA', '耐唾液色牢度', 'grade', 'GB18401', 'measurable', 'GB/T 18886', '级', '耐唾液色牢度等级，仅 A 类考核且不低于 4 级'),
    ('LEAD', '铅含量', 'numeric', 'GB31701', 'measurable', NULL, 'mg/kg', 'GB 31701-2015 儿童纺织品铅限量，不大于 90 mg/kg；方法标准号按检测机构报告承载'),
    ('CADMIUM', '镉含量', 'numeric', 'GB31701', 'measurable', NULL, 'mg/kg', 'GB 31701-2015 儿童纺织品镉限量，不大于 100 mg/kg；方法标准号按检测机构报告承载'),
    ('PHTHALATE', '邻苯二甲酸酯', 'numeric', 'GB31701', 'measurable', NULL, '%', '六种邻苯二甲酸酯各不大于 0.1%'),
    ('FLAM_EXTERIOR', '外层面料燃烧性能', 'numeric', 'GB31701', 'measurable', NULL, '级', '须达 1 级（1 级为最佳档，数值越小要求越严，限值为上限 1）'),
    ('METAL_SHARP', '金属针等锐利物残留', 'boolean', 'GB31701', 'measurable', NULL, NULL, '包装与产品中不得残留金属针等锐利物，检出即不合格'),
    ('ACCESSOR_TENSILE', '附件抗拉强力', 'boolean', 'GB31701', 'manual', NULL, NULL, '附件应能承受相应抗拉强力不脱落不破损；裁定未给数值，登记条目与人工结论位'),
    ('SHARP_EDGE', '锐利尖端与锐利边缘', 'boolean', 'GB31701', 'manual', NULL, NULL, '产品上不得存在可触及的锐利尖端和锐利边缘；人工核查结论位'),
    ('SMALL_PARTS', '过小易吞咽附件', 'boolean', 'GB31701', 'manual', NULL, NULL, '不得使用了过小易吞咽的附件；人工核查结论位'),
    ('CORD_HEAD', '头部与颈部绳带', 'boolean', 'GB31701', 'measurable', NULL, NULL, '7 岁以下儿童产品头部颈部不得有任何绳带，检出即不合格'),
    ('CORD_WAIST', '腰部绳带伸出长度', 'numeric', 'GB31701', 'measurable', NULL, 'mm', '腰部绳带伸出长度不大于 360 mm'),
    ('LABEL_POSITION', '耐久标签缝制位置', 'boolean', 'GB31701', 'manual', NULL, NULL, '贴身婴幼儿服装耐久标签应置于不接触皮肤位置；人工核查结论位')
ON CONFLICT (item_code) DO NOTHING;

-- 二、GB 18401 限值矩阵
-- 甲醛（形态通配，中间产品沿用同档；类别精确三行）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, limit_high, clause_ref, remark)
SELECT id, 'GB18401_A', 'any', 'none', 'lte', 20, 'GB 18401-2010', '婴幼儿用品甲醛不大于 20 mg/kg' FROM textile_test_item WHERE item_code = 'HCHO'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'lte', 75, 'GB 18401-2010', '直接接触皮肤产品甲醛不大于 75 mg/kg' FROM textile_test_item WHERE item_code = 'HCHO'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'lte', 300, 'GB 18401-2010', '非直接接触皮肤产品甲醛不大于 300 mg/kg' FROM textile_test_item WHERE item_code = 'HCHO'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- pH（最终产品按类别区间；中间产品放宽行只挂在 intermediate 乘 wet_processing 上）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, limit_low, limit_high, clause_ref, remark)
SELECT id, 'GB18401_A', 'final', 'none', 'between', 4.0, 7.5, 'GB 18401-2010', 'A 类最终产品 pH 4.0~7.5' FROM textile_test_item WHERE item_code = 'PH'
UNION ALL
SELECT id, 'GB18401_B', 'final', 'none', 'between', 4.0, 8.5, 'GB 18401-2010', 'B 类最终产品 pH 4.0~8.5' FROM textile_test_item WHERE item_code = 'PH'
UNION ALL
SELECT id, 'GB18401_C', 'final', 'none', 'between', 4.0, 9.0, 'GB 18401-2010', 'C 类最终产品 pH 4.0~9.0' FROM textile_test_item WHERE item_code = 'PH'
UNION ALL
SELECT id, 'any', 'intermediate', 'wet_processing', 'between', 4.0, 10.5, 'GB 18401-2010', '后续湿处理的中间产品 pH 可放宽到 4.0~10.5，最终产品不得沿用本档' FROM textile_test_item WHERE item_code = 'PH'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 可分解致癌芳香胺染料（禁用行与上限行并存，判定取合取）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, limit_high, clause_ref, remark)
SELECT id, 'any', 'any', 'none', 'forbidden', NULL, 'GB 18401-2010', '禁用可分解致癌芳香胺染料，报告检出即不合格' FROM textile_test_item WHERE item_code = 'AZO'
UNION ALL
SELECT id, 'any', 'any', 'none', 'lte', 20, 'GB 18401-2010', '且不大于 20 mg/kg' FROM textile_test_item WHERE item_code = 'AZO'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 异味（必须为无）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, clause_ref, remark)
SELECT id, 'any', 'any', 'none', 'must_none', 'GB 18401-2010', '异味不得检出' FROM textile_test_item WHERE item_code = 'ODOR'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 色牢度等级限值（半级编码，3-4 级下限记 3.5）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, limit_low, clause_ref, remark)
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 3.5, 'GB 18401-2010', '耐水 A 类不低于 3-4 级' FROM textile_test_item WHERE item_code = 'FAST_WATER'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐水 B 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_WATER'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐水 C 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_WATER'
UNION ALL
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 3.5, 'GB 18401-2010', '耐酸汗渍 A 类不低于 3-4 级' FROM textile_test_item WHERE item_code = 'FAST_ACID_SWEAT'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐酸汗渍 B 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_ACID_SWEAT'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐酸汗渍 C 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_ACID_SWEAT'
UNION ALL
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 3.5, 'GB 18401-2010', '耐碱汗渍 A 类不低于 3-4 级' FROM textile_test_item WHERE item_code = 'FAST_ALKALI_SWEAT'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐碱汗渍 B 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_ALKALI_SWEAT'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐碱汗渍 C 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_ALKALI_SWEAT'
UNION ALL
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 4.0, 'GB 18401-2010', '耐干摩擦 A 类不低于 4 级' FROM textile_test_item WHERE item_code = 'FAST_DRY_RUB'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐干摩擦 B 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_DRY_RUB'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐干摩擦 C 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_DRY_RUB'
UNION ALL
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 3.5, 'GB 18401-2010', '耐湿摩擦 A 类不低于 3-4 级' FROM textile_test_item WHERE item_code = 'FAST_WET_RUB'
UNION ALL
SELECT id, 'GB18401_B', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐湿摩擦 B 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_WET_RUB'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'gte', 3.0, 'GB 18401-2010', '耐湿摩擦 C 类不低于 3 级' FROM textile_test_item WHERE item_code = 'FAST_WET_RUB'
UNION ALL
SELECT id, 'GB18401_A', 'any', 'none', 'gte', 4.0, 'GB 18401-2010', '耐唾液 A 类不低于 4 级' FROM textile_test_item WHERE item_code = 'FAST_SALIVA'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 耐唾液 B 与 C 类不考核（法定第三态用 na 行表达；耐湿摩擦全类别考核，A 类 3-4 级档见上）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, clause_ref, remark)
SELECT id, 'GB18401_B', 'any', 'none', 'na', 'GB 18401-2010', 'B 类不考核耐唾液色牢度' FROM textile_test_item WHERE item_code = 'FAST_SALIVA'
UNION ALL
SELECT id, 'GB18401_C', 'any', 'none', 'na', 'GB 18401-2010', 'C 类不考核耐唾液色牢度' FROM textile_test_item WHERE item_code = 'FAST_SALIVA'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 色牢度豁免（需洗涤褪色的非最终产品、本色及漂白、扎染与蜡染传统手工着色）
-- 三枚豁免标签乘六个色牢度项目的 na 行，判定器按条件行压过无条件行的特异度出 na
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, clause_ref, remark)
SELECT i.id, 'any', 'any', t.tag, 'na', 'GB 18401-2010', '命中色牢度豁免标签，本项目在本单不考核'
FROM textile_test_item i,
     (VALUES ('laundering_fade_non_final'), ('plain_or_bleached'), ('traditional_hand_dyed')) AS t(tag)
WHERE i.item_code IN ('FAST_WATER', 'FAST_ACID_SWEAT', 'FAST_ALKALI_SWEAT', 'FAST_DRY_RUB', 'FAST_WET_RUB', 'FAST_SALIVA')
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 三、GB 31701 加考限值（童装项目按人群由送检服务推入清单，类别维取通配）
INSERT INTO textile_limit_rule (item_id, safety_category, product_form, condition_tag, comparator, limit_high, clause_ref, remark)
SELECT id, 'any', 'any', 'none', 'lte', 90, 'GB 31701-2015', '铅不大于 90 mg/kg' FROM textile_test_item WHERE item_code = 'LEAD'
UNION ALL
SELECT id, 'any', 'any', 'none', 'lte', 100, 'GB 31701-2015', '镉不大于 100 mg/kg' FROM textile_test_item WHERE item_code = 'CADMIUM'
UNION ALL
SELECT id, 'any', 'any', 'none', 'lte', 0.1, 'GB 31701-2015', '六种邻苯二甲酸酯各不大于 0.1%' FROM textile_test_item WHERE item_code = 'PHTHALATE'
UNION ALL
SELECT id, 'any', 'any', 'none', 'lte', 1, 'GB 31701-2015', '外层面料燃烧性能须达 1 级，1 级为最佳档数值越小越严' FROM textile_test_item WHERE item_code = 'FLAM_EXTERIOR'
UNION ALL
SELECT id, 'any', 'any', 'none', 'must_none', NULL, 'GB 31701-2015', '包装与产品不得残留金属针等锐利物' FROM textile_test_item WHERE item_code = 'METAL_SHARP'
UNION ALL
SELECT id, 'any', 'any', 'none', 'must_none', NULL, 'GB 31701-2015', '7 岁以下头部颈部不得有任何绳带，检出即不合格' FROM textile_test_item WHERE item_code = 'CORD_HEAD'
UNION ALL
SELECT id, 'any', 'any', 'none', 'lte', 360, 'GB 31701-2015', '腰部绳带伸出长度不大于 360 mm' FROM textile_test_item WHERE item_code = 'CORD_WAIST'
ON CONFLICT (item_id, safety_category, product_form, condition_tag, comparator) DO NOTHING;

-- 四、出口市场要求登记（登记位，法规数值以官方现行文本为准，不生成限值矩阵行）
INSERT INTO textile_export_market_requirement (requirement_code, market, requirement_type, oeko_tex_class, standard_ref, detail, mandatory) VALUES
    ('REQ_OEKO_100_EU', 'eu', 'oeko_tex', NULL, 'OEKO-TEX STANDARD 100', '自愿认证分级要求：I 级婴幼儿最严、II 级直接接触皮肤、III 级非直接接触、IV 级装饰材料；证书分级位记在第三方报告登记列，是否加验以客户要求为准', false),
    ('REQ_REACH_SVHC_EU', 'eu', 'reach_svhc', NULL, 'REACH 法规', '物品中高度关注物质（SVHC）管控与信息告知义务，通行浓度阈值口径为质量分数 0.1%，义务清单以欧盟现行授权法规文本为准', true),
    ('REQ_CPSIA_LEAD_US', 'us', 'cpsia_lead', NULL, 'CPSIA 第 101 条', '儿童产品铅含量管控，具体限量与基材涂层分档以 CPSIA 及其最终规则为准，本行为登记位', true),
    ('REQ_CPSIA_PHTHALATE_US', 'us', 'cpsia_phthalate', NULL, 'CPSIA 第 108 条', '儿童玩具与儿童护理产品邻苯二甲酸酯管控，受限种类与限量以 CPSC 现行最终规则为准，本行为登记位', true),
    ('REQ_CPSIA_CPC_US', 'us', 'cpsia_cert', NULL, 'CPSIA 第 102 条', '儿童产品证书（CPC）须基于 CPSC 认可实验室的第三方检测签发，本行为登记位', true),
    ('REQ_JP_FORMALDEHYDE', 'jp', 'jp_formaldehyde', NULL, '日本卫生法自主管控标准（家庭用品品质表示标签相关）', '甲醛限量要求较 GB 18401 严，检测项目与判定口径以与日本客户的质量协议为准', true)
ON CONFLICT (requirement_code) DO NOTHING;
"#;
        manager.get_connection().execute_unprepared(sql).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 按播种编码回收：先删播种项目的全部限值行（含人工在播种项目上新增的行，down 属
        // 整体域回滚，回滚前备份判定表），再删项目与出口要求行。送检单或结果行仍引用
        // 播种项目时外键 RESTRICT 显式报错，不静默级联销账。
        let mut item_codes = String::new();
        for (idx, code) in SEEDED_ITEM_CODES.iter().enumerate() {
            if idx > 0 {
                item_codes.push(',');
            }
            item_codes.push_str(&format!("'{}'", code));
        }
        let mut req_codes = String::new();
        for (idx, code) in SEEDED_REQUIREMENT_CODES.iter().enumerate() {
            if idx > 0 {
                req_codes.push(',');
            }
            req_codes.push_str(&format!("'{}'", code));
        }
        let sql = format!(
            r#"
DELETE FROM textile_limit_rule WHERE item_id IN (SELECT id FROM textile_test_item WHERE item_code IN ({items}));
DELETE FROM textile_export_market_requirement WHERE requirement_code IN ({reqs});
DELETE FROM textile_test_item WHERE item_code IN ({items});
"#,
            items = item_codes,
            reqs = req_codes,
        );
        manager.get_connection().execute_unprepared(&sql).await?;
        Ok(())
    }
}
