// 定制订单全流程跟踪 API 客户端
// 16 端点封装
// 创建时间: 2026-06-17

import { request } from './request'

// 状态枚举（使用显式索引签名以支持外部字符串索引）
export const CUSTOM_ORDER_STATUS: { [key: string]: string } = {
  draft: '草稿',
  yarn_purchasing: '纱线采购中',
  dyeing: '染整中',
  finishing: '后整理中',
  delivery: '交付中',
  after_sales: '售后中',
  completed: '已完成',
  cancelled: '已取消',
}

export const CUSTOM_ORDER_STATUS_COLORS: { [key: string]: string } = {
  draft: 'info',
  yarn_purchasing: 'primary',
  dyeing: 'warning',
  finishing: 'warning',
  delivery: 'success',
  after_sales: 'danger',
  completed: 'success',
  cancelled: 'info',
}

export const NODE_STATUS: { [key: string]: string } = {
  pending: '待开始',
  in_progress: '进行中',
  completed: '已完成',
  blocked: '阻塞',
}

export const NODE_STATUS_COLORS: { [key: string]: string } = {
  pending: 'info',
  in_progress: 'primary',
  completed: 'success',
  blocked: 'danger',
}

export const ISSUE_SEVERITY: { [key: string]: string } = {
  low: '低',
  medium: '中',
  high: '高',
  critical: '严重',
}

export const ISSUE_SEVERITY_COLORS: { [key: string]: string } = {
  low: 'info',
  medium: 'warning',
  high: 'danger',
  critical: '#f56c6c',
}

export const AFTER_SALES_TYPE: Record<string, string> = {
  complaint: '客诉',
  repair: '维修',
  exchange: '换货',
  refund: '退款',
}

export const AFTER_SALES_STATUS: Record<string, string> = {
  opened: '已开',
  processing: '处理中',
  resolved: '已解决',
  closed: '已关闭',
  rejected: '已拒绝',
}

// 列表查询
export function listCustomOrders(params: {
  page?: number
  page_size?: number
  status?: string
  customer_id?: number
  keyword?: string
}) {
  return request.get('/api/v1/erp/custom-orders', { params })
}

// 创建草稿
export function createCustomOrder(data: any) {
  return request.post('/api/v1/erp/custom-orders', data)
}

// 详情
export function getCustomOrder(id: number) {
  return request.get(`/api/v1/erp/custom-orders/${id}`)
}

// 更新
export function updateCustomOrder(id: number, data: any) {
  return request.put(`/api/v1/erp/custom-orders/${id}`, data)
}

// 取消
export function cancelCustomOrder(id: number, reason: string) {
  return request.delete(`/api/v1/erp/custom-orders/${id}`, {
    data: { reason },
  })
}

// 推进状态
export function advanceCustomOrder(id: number, data: { operator_id: number; notes?: string }) {
  return request.post(`/api/v1/erp/custom-orders/${id}/advance`, data)
}

// 添加工艺节点
export function addProcessNode(orderId: number, data: any) {
  return request.post(`/api/v1/erp/custom-orders/${orderId}/nodes`, data)
}

// 更新工艺节点
export function updateProcessNode(orderId: number, nodeId: number, data: any) {
  return request.put(`/api/v1/erp/custom-orders/${orderId}/nodes/${nodeId}`, data)
}

// 推进工艺节点
export function advanceProcessNode(orderId: number, nodeId: number, data: any) {
  return request.post(`/api/v1/erp/custom-orders/${orderId}/nodes/${nodeId}/advance`, data)
}

// 添加节点日志
export function addNodeLog(orderId: number, nodeId: number, data: any) {
  return request.post(`/api/v1/erp/custom-orders/${orderId}/nodes/${nodeId}/logs`, data)
}

// 获取时间线
export function getTimeline(orderId: number) {
  return request.get(`/api/v1/erp/custom-orders/${orderId}/timeline`)
}

// 上报质量异常
export function reportQualityIssue(orderId: number, data: any) {
  return request.post(`/api/v1/erp/custom-orders/${orderId}/issues`, data)
}

// 列出异常
export function listQualityIssues(orderId: number, params?: any) {
  return request.get(`/api/v1/erp/custom-orders/${orderId}/issues`, { params })
}

// 解决异常
export function resolveQualityIssue(issueId: number, data: { resolution: string; operator_id: number }) {
  return request.put(`/api/v1/erp/custom-orders/issues/${issueId}/resolve`, data)
}

// 创建售后
export function createAfterSales(orderId: number, data: any) {
  return request.post(`/api/v1/erp/custom-orders/${orderId}/after-sales`, data)
}

// 列出售后
export function listAfterSales(orderId: number, params?: any) {
  return request.get(`/api/v1/erp/custom-orders/${orderId}/after-sales`, { params })
}

// 更新售后
export function updateAfterSales(afterSalesId: number, data: any) {
  return request.put(`/api/v1/erp/custom-orders/after-sales/${afterSalesId}`, data)
}
