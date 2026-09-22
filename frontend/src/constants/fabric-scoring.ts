/**
 * 验布评分制式（fabric_inspection_record.scoring_system）单一真相源。
 *
 * 取值域来自后端 models/status/quality_dyeing.rs::fabric_scoring（小写码 four_point/ten_point），
 * 建单时不传由后端默认四分制，传越界值会被 400 拒绝——因此界面必须提交码值而不是文案。
 */
export const FABRIC_SCORING = {
  fourPoint: 'four_point',
  tenPoint: 'ten_point',
} as const;

export type FabricScoringValue = (typeof FABRIC_SCORING)[keyof typeof FABRIC_SCORING];

/** 制式 → 界面说明文案（本页其余标签同为中文字面量，整页 i18n 收敛已单独挂账） */
export const FABRIC_SCORING_LABEL: Record<FabricScoringValue, string> = {
  [FABRIC_SCORING.fourPoint]: '四分制（AATCC/ASTM D5430）',
  [FABRIC_SCORING.tenPoint]: '十分制（梭织专用）',
};

/** 可选项：只列后端真实取值 */
export const FABRIC_SCORING_OPTIONS: FabricScoringValue[] = [
  FABRIC_SCORING.fourPoint,
  FABRIC_SCORING.tenPoint,
];
