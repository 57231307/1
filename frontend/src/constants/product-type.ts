/**
 * 产品类型（产品主数据 product.product_type）取值清单。
 *
 * 该列存的是中文类型名（落库真相即「坯布/成品布/辅料」），与后端唯一词表同源：
 * - backend/src/models/status/general.rs::product_type::ALL_TYPES = [坯布, 成品布, 辅料]
 * - backend/src/models/status/general.rs::product_type::FABRIC_TYPES = [坯布, 成品布]
 * 后端「按品类分级必填」以 FABRIC_TYPES 判定是否面料类（面料类才要求幅宽/克重/面料成分必录），
 * 故前端标星与拦提交必须用同一份面料集合判定，不能各抄一套导致「前端放行、后端拒」或反之。
 * 中文类型 token 禁止英文化。
 */
export const PRODUCT_TYPE = {
  greige: '坯布',
  finished: '成品布',
  auxiliary: '辅料',
} as const;

export type ProductTypeValue = (typeof PRODUCT_TYPE)[keyof typeof PRODUCT_TYPE];

/** 全量类型词表，与后端 ALL_TYPES 同序 */
export const PRODUCT_TYPE_VALUES: ProductTypeValue[] = [
  PRODUCT_TYPE.greige,
  PRODUCT_TYPE.finished,
  PRODUCT_TYPE.auxiliary,
];

/**
 * 面料集合，与后端 general.rs::product_type::FABRIC_TYPES 同源（坯布 + 成品布）。
 * 前端表单据此对幅宽/克重/面料成分动态加必填；静态锁测试断言此集合与后端一致。
 */
export const FABRIC_PRODUCT_TYPE_VALUES: ProductTypeValue[] = [
  PRODUCT_TYPE.greige,
  PRODUCT_TYPE.finished,
];

/** 类型 → i18n 文案键（label 走 i18n，value 保持中文落库真相） */
export const PRODUCT_TYPE_LABEL_KEY: Record<ProductTypeValue, string> = {
  [PRODUCT_TYPE.greige]: 'product.constants.typeGreige',
  [PRODUCT_TYPE.finished]: 'product.constants.typeFinished',
  [PRODUCT_TYPE.auxiliary]: 'product.constants.typeAuxiliary',
};

/** 是否面料类（坯布或成品布），与后端 FABRIC_TYPES.contains 判定同源 */
export const isFabricProductType = (value: string | undefined | null): boolean =>
  value != null && FABRIC_PRODUCT_TYPE_VALUES.includes(value as ProductTypeValue);
