/**
 * 物流公司取值（logistics_waybills.logistics_company）单一真相源。
 *
 * 该列是 VARCHAR 自由文本、没有字典表，库里存的就是中文公司名（中文界面与后端一致地写入），
 * 后端筛选按 `LogisticsCompany.eq(值)` 精确匹配。此前 el-option 的 value 取译文，
 * 英文界面建的单把 "SF Express" 写进同一列，切回中文界面按「顺丰速运」筛选即查不到，
 * 反之英文界面也筛不出中文存量行——同一承运商在库里裂成两套值。
 *
 * 因此 value 必须是稳定的中文公司名本身，label 才走 i18n。
 * 建正式的物流公司字典（编码 + 名称 + 联系人）属数据模型决策，另见台账登记。
 */
export const LOGISTICS_COMPANY = {
  sf: '顺丰速运',
  zto: '中通快递',
  yto: '圆通速递',
  yunda: '韵达快递',
  jd: '京东物流',
} as const;

export type LogisticsCompanyValue = (typeof LOGISTICS_COMPANY)[keyof typeof LOGISTICS_COMPANY];

export const LOGISTICS_COMPANY_VALUES: LogisticsCompanyValue[] = [
  LOGISTICS_COMPANY.sf,
  LOGISTICS_COMPANY.zto,
  LOGISTICS_COMPANY.yto,
  LOGISTICS_COMPANY.yunda,
  LOGISTICS_COMPANY.jd,
];

/** 公司 → i18n 文案键（沿用物流模块既有键） */
export const LOGISTICS_COMPANY_LABEL_KEY: Record<LogisticsCompanyValue, string> = {
  [LOGISTICS_COMPANY.sf]: 'logistics.common.company.sf',
  [LOGISTICS_COMPANY.zto]: 'logistics.common.company.zto',
  [LOGISTICS_COMPANY.yto]: 'logistics.common.company.yto',
  [LOGISTICS_COMPANY.yunda]: 'logistics.common.company.yunda',
  [LOGISTICS_COMPANY.jd]: 'logistics.common.company.jd',
};

/** 词表外的存量值（人工录入的其他承运商名）界面原样展示，不猜测其归属 */
export const isLogisticsCompany = (value: string): value is LogisticsCompanyValue =>
  LOGISTICS_COMPANY_VALUES.includes(value as LogisticsCompanyValue);
