/**
 * 质量检验记录（quality_inspection_records）的取值单一真相源。
 *
 * 检验类型沿用 `constants/quality-inspection-type.ts` 的四个稳定码（质检记录弹窗即按此写入）；
 * 该列还有一个只能由系统产生的来源标识 outsourcing_receipt（委外收回单确认回仓时自动生成记录，
 * 见 services/outsourcing_ops/receipt.rs），它不出现在下拉选项里，但列表必须能翻成文案。
 *
 * 检验结论 inspection_result 库里落的是中文稳定值：唯一的自动写入方写的就是「合格/不合格」
 * （receipt.rs 的 is_accepted 分支），该列在迁移里是 VARCHAR 无 CHECK。界面此前用
 * pass/fail/pending 三个码提交，而提交本身因契约错位从未成功，所以库里没有这三套写法，
 * 直接以中文稳定值为词表即可，不需要存量归一。
 */
export const QUALITY_RECORD_RESULT = {
  pending: '待检',
  qualified: '合格',
  unqualified: '不合格',
} as const;

export type QualityRecordResultValue =
  (typeof QUALITY_RECORD_RESULT)[keyof typeof QUALITY_RECORD_RESULT];

export const QUALITY_RECORD_RESULT_VALUES: QualityRecordResultValue[] = [
  QUALITY_RECORD_RESULT.pending,
  QUALITY_RECORD_RESULT.qualified,
  QUALITY_RECORD_RESULT.unqualified,
];

/** 结论 → i18n 文案键 */
export const QUALITY_RECORD_RESULT_LABEL_KEY: Record<QualityRecordResultValue, string> = {
  [QUALITY_RECORD_RESULT.pending]: 'quality.recordTab.resultPending',
  [QUALITY_RECORD_RESULT.qualified]: 'quality.recordTab.resultQualified',
  [QUALITY_RECORD_RESULT.unqualified]: 'quality.recordTab.resultUnqualified',
};

/** 结论 → el-tag 配色 */
export const QUALITY_RECORD_RESULT_TAG_TYPE: Record<
  QualityRecordResultValue,
  'warning' | 'success' | 'danger'
> = {
  [QUALITY_RECORD_RESULT.pending]: 'warning',
  [QUALITY_RECORD_RESULT.qualified]: 'success',
  [QUALITY_RECORD_RESULT.unqualified]: 'danger',
};

/** 判断已存库的结论值是否在本词表内（越界值界面原样展示，不猜测含义） */
export const isQualityRecordResult = (value: string): value is QualityRecordResultValue =>
  QUALITY_RECORD_RESULT_VALUES.includes(value as QualityRecordResultValue);

/** 自动写入方产生的来源标识 → 文案键（不作为界面选项） */
export const QUALITY_INSPECTION_SOURCE_LABEL_KEY: Record<string, string> = {
  outsourcing_receipt: 'quality.recordTab.typeOutsourcing',
};
