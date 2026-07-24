// ap-invoice.ts - 应付发票 API 兼容层
// 实际定义在 ap.ts 中
export {
  getAPInvoiceList,
  getAPInvoice,
  createAPInvoice,
  updateAPInvoice,
  deleteAPInvoice,
  approveAPInvoice,
  cancelAPInvoice,
  type APInvoice,
} from './ap'

/**
 * 应付发票状态文本映射
 */
export function getAPInvoiceStatusText(status: string): string {
  const map: Record<string, string> = {
    pending: '待审核',
    approved: '已审核',
    verified: '已核销',
    cancelled: '已取消',
  }
  return map[status] || status
}
