/**
 * 产品主档规格枚举词表（安全类别与色牢度等级）前端唯一消费源。
 *
 * 与后端唯一词表同源，禁止各处再抄一份中文或字母副本：
 * - 安全类别 backend/src/models/status/general.rs::product_safety_category::ALL_CLASSES = [A, B, C]
 *   （GB 18401-2010 国家纺织产品基本安全技术规范三类，A 类婴幼儿另须满足 GB 31701-2015）
 * - 色牢度等级 backend/src/models/status/general.rs::color_fastness::ALL_GRADES
 *   （九档，含半级 1-2 与 2-3 与 3-4 与 4-5，半级是标准内合法档不是脏值）
 * value 即落库真相（安全类别存 A/B/C 单字母，色牢度存档位文本），label 走 i18n。
 * 取值域收口：安全类别与色牢度的 DB CHECK 由 product_vocab_check 域施加，与此处清单逐项相等；
 * 前端表单拦录入、后端服务层拒非法，两处判据同源于上面的后端常量。
 */
export const PRODUCT_SAFETY_CATEGORY = {
  /** A 类：婴幼儿纺织产品（另须满足 GB 31701-2015） */
  classA: 'A',
  /** B 类：直接接触皮肤的产品 */
  classB: 'B',
  /** C 类：非直接接触皮肤的产品 */
  classC: 'C',
} as const;

export type ProductSafetyCategoryValue =
  (typeof PRODUCT_SAFETY_CATEGORY)[keyof typeof PRODUCT_SAFETY_CATEGORY];

/** 全量安全类别词表，与后端 ALL_CLASSES 同序 */
export const PRODUCT_SAFETY_CATEGORY_VALUES: ProductSafetyCategoryValue[] = [
  PRODUCT_SAFETY_CATEGORY.classA,
  PRODUCT_SAFETY_CATEGORY.classB,
  PRODUCT_SAFETY_CATEGORY.classC,
];

/** 安全类别 → i18n 文案键（label 走 i18n，value 保持字母落库真相） */
export const PRODUCT_SAFETY_CATEGORY_LABEL_KEY: Record<ProductSafetyCategoryValue, string> = {
  [PRODUCT_SAFETY_CATEGORY.classA]: 'product.constants.safetyClassA',
  [PRODUCT_SAFETY_CATEGORY.classB]: 'product.constants.safetyClassB',
  [PRODUCT_SAFETY_CATEGORY.classC]: 'product.constants.safetyClassC',
};

/**
 * 色牢度等级九档（GB 18401 品色牢度分档，含半级）。落 product_colors.color_fastness_grade。
 * 与后端 color_fastness::ALL_GRADES 逐项同序；半级形态（1-2 等）是合法档位。
 * 空值语义为未考核或未录入，表单允许留空。
 */
export const COLOR_FASTNESS_GRADE_VALUES = ['1', '1-2', '2', '2-3', '3', '3-4', '4', '4-5', '5'];

/**
 * 组织结构行业词表（products.structure 为自由文本列，本词表只作录入下拉建议，不收 DB CHECK）。
 * 分类口径按纺织行业组织两大系：梭织（机织物）与针织，针织再分纬编与经编；
 * 纬编与经编织物名称带「纬编」「经编」前缀落库，检索与统计可按前缀区分两类针织形态，
 * 这就是「针织分纬编与经编」在录入面的表达。词表外结构（如提花、特殊组织）允许自填，
 * 与后端 50 字符长度校验同源；不做枚举拒绝的原因：组织结构名词量大且随工艺扩展，
 * 收成封闭 CHECK 会在应用层与库表间漂移（裁定依据见车道回报）。
 */
export const PRODUCT_STRUCTURE_GROUPS: { label: string; options: string[] }[] = [
  {
    label: 'product.constants.structureGroupWoven',
    options: ['平纹', '斜纹', '缎纹', '府绸', '卡其', '华达呢', '牛津纺', '泡泡纱', '提花'],
  },
  {
    label: 'product.constants.structureGroupWeftKnit',
    options: ['纬编汗布', '纬编珠地', '纬编罗纹', '纬编双面布', '纬编毛圈布', '纬编卫衣布'],
  },
  {
    label: 'product.constants.structureGroupWarpKnit',
    options: ['经编特里科', '经编拉舍尔', '经编网眼', '经编起绒布'],
  },
];

/** 全量组织词表扁平值，供静态锁与检索前缀判据消费 */
export const PRODUCT_STRUCTURE_TERMS: string[] = PRODUCT_STRUCTURE_GROUPS.flatMap(
  group => group.options
);

/** 纬编与经编前缀判据（检索与列表过滤用，落库值以这两词开头即针织子类可辨） */
export const KNIT_SUBTYPE_PREFIXES = { weft: '纬编', warp: '经编' } as const;
