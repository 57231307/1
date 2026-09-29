/**
 * 委外收回单质检结论（outsourcing_receipt.quality_status）单一真相源。
 *
 * 取值必须与后端 `models/status/wage_energy_chemical_business.rs::outsourcing_receipt_quality_status`
 * 一致：pending / qualified / concession / unqualified。
 * 建单与改单入口都会按该取值域校验，别域同义写法（染化料来料检验的 passed/failed、
 * 库存质量状态的中文「合格」）会被直接拒绝；confirm 侧把 qualified 与 concession
 * 都视为接收（让步接收的降级体现在 grade 上），unqualified 才触发不合格品处理。
 *
 * 本表另有独立的单据状态列（draft/confirmed/cancelled），不要与本取值域混用。
 */
export const OUTSOURCING_QUALITY_STATUS = {
  pending: 'pending',
  qualified: 'qualified',
  concession: 'concession',
  unqualified: 'unqualified',
} as const;

export type OutsourcingQualityStatusValue =
  (typeof OUTSOURCING_QUALITY_STATUS)[keyof typeof OUTSOURCING_QUALITY_STATUS];

export const OUTSOURCING_QUALITY_STATUS_VALUES: OutsourcingQualityStatusValue[] = [
  OUTSOURCING_QUALITY_STATUS.pending,
  OUTSOURCING_QUALITY_STATUS.qualified,
  OUTSOURCING_QUALITY_STATUS.concession,
  OUTSOURCING_QUALITY_STATUS.unqualified,
];

export const OUTSOURCING_QUALITY_STATUS_LABELS: Record<string, string> = {
  [OUTSOURCING_QUALITY_STATUS.pending]: '待检',
  [OUTSOURCING_QUALITY_STATUS.qualified]: '合格',
  [OUTSOURCING_QUALITY_STATUS.concession]: '让步接收',
  [OUTSOURCING_QUALITY_STATUS.unqualified]: '不合格',
};

/** 建单可选结论：待检由「未给结论」自动落库，不作为界面选项 */
export const OUTSOURCING_QUALITY_FORM_VALUES: OutsourcingQualityStatusValue[] = [
  OUTSOURCING_QUALITY_STATUS.qualified,
  OUTSOURCING_QUALITY_STATUS.concession,
  OUTSOURCING_QUALITY_STATUS.unqualified,
];

export type OutsourcingQualityTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

export const OUTSOURCING_QUALITY_STATUS_TAG_TYPE: Record<string, OutsourcingQualityTagType> = {
  [OUTSOURCING_QUALITY_STATUS.pending]: 'info',
  [OUTSOURCING_QUALITY_STATUS.qualified]: 'success',
  [OUTSOURCING_QUALITY_STATUS.concession]: 'warning',
  [OUTSOURCING_QUALITY_STATUS.unqualified]: 'danger',
};
