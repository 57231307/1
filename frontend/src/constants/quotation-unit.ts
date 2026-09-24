/**
 * 计量单位（产品主数据交易单位 product.unit / 报价行 sales_quotation_items.unit）取值清单。
 *
 * 该列是 VARCHAR 自由文本、无字典表也无 CHECK，库里存的是中文单位名。此前编辑弹窗的
 * el-option 把译文当 value，英文界面写入的 "Meter"/"Roll"/"Piece" 与中文界面写入的
 * 「米/卷/件」裂成两套，连默认值本身也是译文。因此 value 必须是稳定的中文单位名，
 * label 才走 i18n —— 中文单位 token 禁止英文化（落库真相是「米/公斤/卷/件…」）。
 *
 * 后端 Q1（validate_item_units_against_products）后，报价行 unit 必须逐字符等于所引用
 * 产品的交易单位，故报价行单位不再自由可选，只由产品带出并锁定；本词表主要用于产品主
 * 数据配置交易单位与换算视图展示。按纺织实务稳定 token 收录 米/码/公斤/匹/卷/条/件/吨，
 * 其中 码/匹/卷 仅换算视图（真相仍为米/公斤双列），落库比对以产品 unit 为准。
 */
export const QUOTATION_UNIT = {
  meter: '米',
  yard: '码',
  kilogram: '公斤',
  bolt: '匹',
  roll: '卷',
  strip: '条',
  piece: '件',
  ton: '吨',
} as const;

export type QuotationUnitValue = (typeof QUOTATION_UNIT)[keyof typeof QUOTATION_UNIT];

export const QUOTATION_UNIT_VALUES: QuotationUnitValue[] = [
  QUOTATION_UNIT.meter,
  QUOTATION_UNIT.yard,
  QUOTATION_UNIT.kilogram,
  QUOTATION_UNIT.bolt,
  QUOTATION_UNIT.roll,
  QUOTATION_UNIT.strip,
  QUOTATION_UNIT.piece,
  QUOTATION_UNIT.ton,
];

/** 单位 → i18n 文案键 */
export const QUOTATION_UNIT_LABEL_KEY: Record<QuotationUnitValue, string> = {
  [QUOTATION_UNIT.meter]: 'quotations.itemEditor.unitMeter',
  [QUOTATION_UNIT.yard]: 'quotations.itemEditor.unitYard',
  [QUOTATION_UNIT.kilogram]: 'quotations.itemEditor.unitKg',
  [QUOTATION_UNIT.bolt]: 'quotations.itemEditor.unitBolt',
  [QUOTATION_UNIT.roll]: 'quotations.itemEditor.unitRoll',
  [QUOTATION_UNIT.strip]: 'quotations.itemEditor.unitStrip',
  [QUOTATION_UNIT.piece]: 'quotations.itemEditor.unitPiece',
  [QUOTATION_UNIT.ton]: 'quotations.itemEditor.unitTon',
};

/** 词表外的单位（例如产品主数据自带的其它单位）原样展示，不猜测换算 */
export const isQuotationUnit = (value: string): value is QuotationUnitValue =>
  QUOTATION_UNIT_VALUES.includes(value as QuotationUnitValue);
