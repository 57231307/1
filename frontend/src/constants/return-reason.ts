/**
 * 退货原因分类候选（销售退货与采购退货共用）。
 *
 * 两侧后端 reason_type 均为自由文本列（sales_return / purchase_return 都不校验枚举），
 * 因此取值本身就是业务数据：若采购侧另立一套英文 slug，同一含义会在库里分裂成两种写法，
 * 打印与按原因聚合的报表都会失真，故统一从这一份候选取。
 * 取值沿用纺织面料退货的实际质量判定口径（缸差 / 色差 / 克重 / 幅宽 为常规缺陷分类）。
 *
 * labelKey 写完整 i18n 路径：check-i18n 会把 labelKey 字面量当引用校验存在性，
 * 写半截路径（旧 salesReturns.editDialog.<key> 拼接式）则落在"动态拼接不判定"里。
 */
export interface ReturnReasonOption {
  value: string;
  labelKey: string;
}

export const RETURN_REASON_OPTIONS: ReturnReasonOption[] = [
  { value: '色差', labelKey: 'common.returnReason.colorDifference' },
  { value: '缸差', labelKey: 'common.returnReason.dyeLotDifference' },
  { value: '克重不符', labelKey: 'common.returnReason.gramWeightMismatch' },
  { value: '幅宽不符', labelKey: 'common.returnReason.widthMismatch' },
  { value: '品质瑕疵', labelKey: 'common.returnReason.qualityDefect' },
  { value: '数量不符', labelKey: 'common.returnReason.quantityMismatch' },
  { value: '发错货', labelKey: 'common.returnReason.wrongShipment' },
  { value: '客户取消订单', labelKey: 'common.returnReason.customerCancel' },
];
