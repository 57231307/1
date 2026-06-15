// ap-payment.ts - 应付付款 API 兼容层
// 实际定义在 ap.ts 中
export {
  listAPPayments,
  getAPPayment,
  createAPPayment,
  updateAPPayment,
  confirmAPPayment,
  type APPayment,
} from './ap'

/**
 * 付款方式文本映射
 */
export function getAPPaymentMethodText(method: string): string {
  const map: Record<string, string> = {
    bank_transfer: '银行转账',
    cash: '现金',
    check: '支票',
    bill: '承兑汇票',
  }
  return map[method] || method
}
