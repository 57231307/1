/**
 * 不合格品处理（unqualified_products）的取值单一真相源。
 *
 * 三张词表（处理方式 / 处理状态 / 等级）的唯一事实来源是后端写入方常量
 * （models/status/quality_dyeing.rs 的 quality_handling 与
 * services/quality_inspection_service.rs 的 HANDLING_* / QUALITY_GRADE_*），大小写与下划线
 * 必须逐字符相同：列表筛选入参 status 直接对该列做等值比较，比较点写错一个字母即恒零命中。
 *
 * handling_method 只有三个码且后端会按质检等级校验合法组合（A 级拒绝处理、B 级必须降级销售、
 * C 级必须返工或报废），界面不得自造第四个取值；handling_status 由建单写 pending，
 * 之后只会被报废两级审批写为 approved / rejected，不存在 done/processed 这类布尔语义。
 *
 * 词表外存量值（该列 VARCHAR 无 CHECK）界面原样直出不猜含义，与质检记录结论列同一处置法；
 * 动作门控只认真实写入值本身，未知值一律不放行（fail-closed），避免把脏值当待处理重复建单。
 */
export const QUALITY_HANDLING_METHOD = {
  downgradeSale: 'downgrade_sale',
  rework: 'rework',
  scrap: 'scrap',
} as const;

export type QualityHandlingMethodValue =
  (typeof QUALITY_HANDLING_METHOD)[keyof typeof QUALITY_HANDLING_METHOD];

/** 处理方式全部取值，界面选项与文案翻译的唯一来源 */
export const QUALITY_HANDLING_METHOD_VALUES: QualityHandlingMethodValue[] = [
  QUALITY_HANDLING_METHOD.downgradeSale,
  QUALITY_HANDLING_METHOD.rework,
  QUALITY_HANDLING_METHOD.scrap,
];

/** 处理方式 → i18n 文案键（沿用缺陷管理页既有键，避免同一含义两套文案） */
export const QUALITY_HANDLING_METHOD_LABEL_KEY: Record<QualityHandlingMethodValue, string> = {
  [QUALITY_HANDLING_METHOD.downgradeSale]: 'quality.defectTab.handlingDowngradeSale',
  [QUALITY_HANDLING_METHOD.rework]: 'quality.defectTab.handlingRework',
  [QUALITY_HANDLING_METHOD.scrap]: 'quality.defectTab.handlingScrap',
};

/** 判断已存库的处理方式是否在本词表内（越界值界面原样展示，不猜测含义） */
export const isQualityHandlingMethod = (value: string): value is QualityHandlingMethodValue =>
  QUALITY_HANDLING_METHOD_VALUES.includes(value as QualityHandlingMethodValue);

/** 处理状态（handling_status，小写）：pending 建单写入，approved/rejected 由报废审批写入 */
export const QUALITY_HANDLING_STATUS = {
  pending: 'pending',
  approved: 'approved',
  rejected: 'rejected',
} as const;

export type QualityHandlingStatusValue =
  (typeof QUALITY_HANDLING_STATUS)[keyof typeof QUALITY_HANDLING_STATUS];

export const QUALITY_HANDLING_STATUS_VALUES: QualityHandlingStatusValue[] = [
  QUALITY_HANDLING_STATUS.pending,
  QUALITY_HANDLING_STATUS.approved,
  QUALITY_HANDLING_STATUS.rejected,
];

/** 处理状态 → i18n 文案键（沿用同页"是否处理"列与质量标准驳回既有键） */
export const QUALITY_HANDLING_STATUS_LABEL_KEY: Record<QualityHandlingStatusValue, string> = {
  [QUALITY_HANDLING_STATUS.pending]: 'quality.defectTab.processedNo',
  [QUALITY_HANDLING_STATUS.approved]: 'quality.defectTab.processedYes',
  [QUALITY_HANDLING_STATUS.rejected]: 'quality.standardStatus.rejected',
};

/** 处理状态 → el-tag 配色 */
export const QUALITY_HANDLING_STATUS_TAG_TYPE: Record<
  QualityHandlingStatusValue,
  'warning' | 'success' | 'danger'
> = {
  [QUALITY_HANDLING_STATUS.pending]: 'warning',
  [QUALITY_HANDLING_STATUS.approved]: 'success',
  [QUALITY_HANDLING_STATUS.rejected]: 'danger',
};

/** 判断已存库的处理状态是否在本词表内（越界值界面原样展示，不猜测含义） */
export const isQualityHandlingStatus = (value: string): value is QualityHandlingStatusValue =>
  QUALITY_HANDLING_STATUS_VALUES.includes(value as QualityHandlingStatusValue);

/**
 * 不合格品等级（grade）：后端写入方 quality_inspection_service.rs 的 QUALITY_GRADE_*，
 * A 级合格 / B 级让步接收（降级销售）/ C 级不合格（返工或报废），是质量严重度的真实载体。
 * 与库存等级（一等品/二等品/等外品中文稳定值，constants/stock-grade.ts）分属两套取值域，不可互抄。
 */
export const QUALITY_UNQUALIFIED_GRADE = {
  a: 'A',
  b: 'B',
  c: 'C',
} as const;

export type QualityUnqualifiedGradeValue =
  (typeof QUALITY_UNQUALIFIED_GRADE)[keyof typeof QUALITY_UNQUALIFIED_GRADE];

export const QUALITY_UNQUALIFIED_GRADE_VALUES: QualityUnqualifiedGradeValue[] = [
  QUALITY_UNQUALIFIED_GRADE.a,
  QUALITY_UNQUALIFIED_GRADE.b,
  QUALITY_UNQUALIFIED_GRADE.c,
];

/**
 * 等级文案键：通用质检严重度里的插值键（实参为等级码本身，中文渲染「A 级」、英文渲染「Grade A」），
 * 沿用既有键而非新造，避免同一含义两套文案。
 */
export const QUALITY_UNQUALIFIED_GRADE_LABEL_KEY = 'common.qualityCheck.severity.grade';

/** 等级 → el-tag 配色（A 合格信息态、B 让步警告态、C 不合格危险态） */
export const QUALITY_UNQUALIFIED_GRADE_TAG_TYPE: Record<
  QualityUnqualifiedGradeValue,
  'info' | 'warning' | 'danger'
> = {
  [QUALITY_UNQUALIFIED_GRADE.a]: 'info',
  [QUALITY_UNQUALIFIED_GRADE.b]: 'warning',
  [QUALITY_UNQUALIFIED_GRADE.c]: 'danger',
};

/** 判断已存库的等级是否在本词表内（越界值界面原样展示，不猜测含义） */
export const isQualityUnqualifiedGrade = (value: string): value is QualityUnqualifiedGradeValue =>
  QUALITY_UNQUALIFIED_GRADE_VALUES.includes(value as QualityUnqualifiedGradeValue);
