/**
 * 报价单行计量单位（sales_quotation_items.unit）取值清单。
 *
 * 该列是 VARCHAR(20) 自由文本、无字典表也无 CHECK（产品主数据的 unit 同样是自由文本），
 * 库里存的是中文单位名。此前编辑弹窗的 el-option 把译文当 value，
 * 英文界面新建的报价行存进 "Meter"/"Roll"/"Piece"，与中文界面写入的「米/卷/件」裂成两套，
 * 连默认值本身也是译文（`unit: t('quotations.itemEditor.unitMeter')`）。
 *
 * 因此 value 必须是稳定的中文单位名，label 才走 i18n。
 * 注意这只是编辑器提供的快捷项，不是全仓计量单位字典：纺织实务还要用 码/公斤/条/吨 等，
 * 且报价行单位本应跟随所选产品的单位；扩成字典或改为随产品带出属功能决策，见台账登记。
 */
export const QUOTATION_UNIT = {
  meter: '米',
  roll: '卷',
  kilogram: '公斤',
  piece: '件',
} as const;

export type QuotationUnitValue = (typeof QUOTATION_UNIT)[keyof typeof QUOTATION_UNIT];

export const QUOTATION_UNIT_VALUES: QuotationUnitValue[] = [
  QUOTATION_UNIT.meter,
  QUOTATION_UNIT.roll,
  QUOTATION_UNIT.kilogram,
  QUOTATION_UNIT.piece,
];

/** 单位 → i18n 文案键 */
export const QUOTATION_UNIT_LABEL_KEY: Record<QuotationUnitValue, string> = {
  [QUOTATION_UNIT.meter]: 'quotations.itemEditor.unitMeter',
  [QUOTATION_UNIT.roll]: 'quotations.itemEditor.unitRoll',
  [QUOTATION_UNIT.kilogram]: 'quotations.itemEditor.unitKg',
  [QUOTATION_UNIT.piece]: 'quotations.itemEditor.unitPiece',
};

/** 词表外的单位（例如产品主数据自带的其它单位）原样展示，不猜测换算 */
export const isQuotationUnit = (value: string): value is QuotationUnitValue =>
  QUOTATION_UNIT_VALUES.includes(value as QuotationUnitValue);
