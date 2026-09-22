/**
 * 质检记录类型（quality_inspection_records.inspection_type）单一真相源。
 *
 * 库里落的是稳定英文码：质检记录弹窗按这四个值写入
 * （views/quality/index.vue 的 value="incoming"/"process"/"finished"/"outgoing"），
 * 委外回仓自动生成的记录写的是另一来源标识 outsourcing_receipt；
 * AI 预测接口拿这个值直接对 inspection_type 做等值筛选
 * （handlers/advanced/quality_pred.rs 的 Column::InspectionType.eq）。
 *
 * 注意与 `ai_quality_predictions.inspection_type` 区分：那张表的 CHECK 约束
 * 允许的是 all/incoming/inprocess/final/outgoing（另一套码），两者不可互相赋值。
 */
export const QUALITY_INSPECTION_TYPE = {
  incoming: 'incoming',
  process: 'process',
  finished: 'finished',
  outgoing: 'outgoing',
} as const;

export type QualityInspectionTypeValue =
  (typeof QUALITY_INSPECTION_TYPE)[keyof typeof QUALITY_INSPECTION_TYPE];

/** 界面可选项（不含 all，「全部」由空值表达） */
export const QUALITY_INSPECTION_TYPE_VALUES: QualityInspectionTypeValue[] = [
  QUALITY_INSPECTION_TYPE.incoming,
  QUALITY_INSPECTION_TYPE.process,
  QUALITY_INSPECTION_TYPE.finished,
  QUALITY_INSPECTION_TYPE.outgoing,
];

/** 类型 → i18n 文案键（沿用高级筛选面板既有键） */
export const QUALITY_INSPECTION_TYPE_LABEL_KEY: Record<QualityInspectionTypeValue, string> = {
  [QUALITY_INSPECTION_TYPE.incoming]: 'advancedModule.quality.typeIncoming',
  [QUALITY_INSPECTION_TYPE.process]: 'advancedModule.quality.typeInprocess',
  [QUALITY_INSPECTION_TYPE.finished]: 'advancedModule.quality.typeFinal',
  [QUALITY_INSPECTION_TYPE.outgoing]: 'advancedModule.quality.typeOutgoing',
};
