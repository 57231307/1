// 定制订单全流程跟踪 API 客户端
// 16 端点封装
// 创建时间: 2026-06-17
// 端点路径相对于 baseURL（/api/v1/erp），不要重复添加前缀，否则会产生双重前缀

import { request } from './request'
import type { ApiResponse } from '@/types/api'

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

// P2-9a 修复（批次 82 v1 复审）：定制订单 API 强类型化，替代 11 处 any
// 字段与后端 DTO 对齐：custom_order_create_dto.rs / custom_order_update_dto.rs /
// quality_issue_dto.rs / custom_order_aftersales_service.rs

/** 创建定制订单请求（对齐后端 CreateCustomOrderDto） */
export interface CustomOrderCreateDto {
  customer_id: number
  product_id: number
  color_id?: number
  spec: string
  quantity: number
  unit: string
  custom_requirements?: unknown
  yarn_spec?: string
  dye_method?: string
  finishing_method?: string
  expected_delivery_date?: string
  sales_order_id?: number
  total_amount?: number
  currency?: string
  notes?: string
}

/** 更新定制订单请求（对齐后端 UpdateCustomOrderDto） */
export interface CustomOrderUpdateDto {
  spec?: string
  quantity?: number
  unit?: string
  custom_requirements?: unknown
  yarn_spec?: string
  dye_method?: string
  finishing_method?: string
  expected_delivery_date?: string
  total_amount?: number
  notes?: string
}

/** 推进订单状态请求（对齐后端 AdvanceRequest）
 * v11 批次 160 P2-6 修复：后端 handler 实际使用 AdvanceRequest（不含 target_status），
 * service.advance 自动判断下一状态；AdvanceStatusDto 死代码已从后端删除 */
export interface CustomOrderAdvanceDto {
  operator_id: number
  notes?: string
}

/** 添加工艺节点请求（对齐后端 CreateProcessNodeDto） */
export interface ProcessNodeCreateDto {
  node_type: string
  node_name: string
  sequence: number
  planned_start_date?: string
  planned_end_date?: string
}

/** 更新工艺节点请求（对齐后端 UpdateProcessNodeDto） */
export interface ProcessNodeUpdateDto {
  status?: string
  operator_id?: number
  actual_start_date?: string
  actual_end_date?: string
  notes?: string
}

/** 推进工艺节点请求（对齐后端 AdvanceNodeDto） */
export interface ProcessNodeAdvanceDto {
  action: string
  operator_id: number
  notes?: string
  attachments?: string[]
}

/** 添加节点日志请求（对齐后端 AddProcessLogDto） */
export interface NodeLogCreateDto {
  action: string
  operator_id: number
  before_status?: string
  after_status?: string
  log_content?: string
  attachments?: string[]
}

/** 上报质量异常请求（对齐后端 ReportQualityIssueDto）
 * 注意：custom_order_id 通过 URL 路径参数传递，请求体中可选 */
export interface QualityIssueCreateDto {
  custom_order_id?: number
  process_node_id?: number
  issue_type: string
  severity: string
  description: string
  color_delta_e?: number
  color_fastness_grade?: number
}

/** 质量异常列表查询参数 */
export interface QualityIssueQueryParams {
  page?: number
  page_size?: number
  status?: string
  severity?: string
}

/** 创建售后工单请求（对齐后端 CreateAfterSalesDto）
 * 注意：custom_order_id 通过 URL 路径参数传递，请求体中可选 */
export interface AfterSalesCreateDto {
  custom_order_id?: number
  customer_id?: number
  issue_type: string
  description: string
  refund_amount?: number
}

/** 更新售后工单请求（对齐后端 UpdateAfterSalesDto） */
export interface AfterSalesUpdateDto {
  status?: string
  resolution?: string
  refund_amount?: number
}

/** 售后列表查询参数 */
export interface AfterSalesQueryParams {
  page?: number
  page_size?: number
  status?: string
  type?: string
}

// v3 复审 P1-7：定制订单响应类型定义，对齐后端 response DTO

/** 定制订单列表项（对齐后端 CustomOrderListItemResponse） */
export interface CustomOrderListItem {
  id: number
  order_no: string
  customer_id: number
  product_id: number
  color_id?: number
  spec: string
  quantity: number
  unit: string
  status: string
  expected_delivery_date?: string
  actual_delivery_date?: string
  total_amount?: number
  currency: string
  sales_order_id?: number
  created_at: string
  notes?: string
}

/** 定制订单工艺节点（详情接口返回结构） */
export interface CustomOrderProcessNode {
  id: number
  node_type: string
  node_name: string
  sequence: number
  status: string
  planned_start_date?: string
  planned_end_date?: string
  actual_start_date?: string
  actual_end_date?: string
  notes?: string
}

/** 定制订单详情（对齐后端 CustomOrderDetailResponse，含 notes + process_nodes） */
export interface CustomOrderDetail extends CustomOrderListItem {
  yarn_spec?: string
  dye_method?: string
  finishing_method?: string
  updated_at: string
  process_nodes: CustomOrderProcessNode[]
}

// v3 复审 P2-5：时间线相关类型，供 tracking.vue 替代 any

/** 节点日志（对齐后端 ProcessLogResponse，tracking.vue 时间线日志项） */
export interface NodeLog {
  id: number
  action: string
  operator_id: number
  before_status?: string
  after_status?: string
  log_content?: string
  log_time: string
  attachments?: string[]
}

/** 时间线工艺节点（扩展 CustomOrderProcessNode，含节点日志） */
export interface TimelineProcessNode extends CustomOrderProcessNode {
  logs: NodeLog[]
}

/** 订单时间线响应（getTimeline 返回结构） */
export interface OrderTimeline {
  order_no: string
  current_status: string
  nodes: TimelineProcessNode[]
}

// 列表查询
export function listCustomOrders(params: {
  page?: number
  page_size?: number
  status?: string
  customer_id?: number
  keyword?: string
}): Promise<ApiResponse<CustomOrderListItem[]>> {
  return request.get('/custom-orders', { params })
}

// 创建草稿
export function createCustomOrder(
  data: CustomOrderCreateDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.post('/custom-orders', data)
}

// 详情
export function getCustomOrder(id: number): Promise<ApiResponse<CustomOrderDetail>> {
  return request.get(`/custom-orders/${id}`)
}

// 更新
export function updateCustomOrder(
  id: number,
  data: CustomOrderUpdateDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.put(`/custom-orders/${id}`, data)
}

// 取消
export function cancelCustomOrder(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.delete(`/custom-orders/${id}`, {
    data: { reason },
  })
}

// 推进状态
export function advanceCustomOrder(
  id: number,
  data: CustomOrderAdvanceDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.post(`/custom-orders/${id}/advance`, data)
}

// 添加工艺节点
export function addProcessNode(orderId: number, data: ProcessNodeCreateDto) {
  return request.post(`/custom-orders/${orderId}/nodes`, data)
}

// 更新工艺节点
export function updateProcessNode(orderId: number, nodeId: number, data: ProcessNodeUpdateDto) {
  return request.put(`/custom-orders/${orderId}/nodes/${nodeId}`, data)
}

// 推进工艺节点
export function advanceProcessNode(orderId: number, nodeId: number, data: ProcessNodeAdvanceDto) {
  return request.post(`/custom-orders/${orderId}/nodes/${nodeId}/advance`, data)
}

// 添加节点日志
export function addNodeLog(orderId: number, nodeId: number, data: NodeLogCreateDto) {
  return request.post(`/custom-orders/${orderId}/nodes/${nodeId}/logs`, data)
}

// 获取时间线
export function getTimeline(orderId: number): Promise<ApiResponse<OrderTimeline>> {
  return request.get(`/custom-orders/${orderId}/timeline`)
}

// 上报质量异常
export function reportQualityIssue(orderId: number, data: QualityIssueCreateDto) {
  return request.post(`/custom-orders/${orderId}/issues`, data)
}

// 列出异常
export function listQualityIssues(orderId: number, params?: QualityIssueQueryParams) {
  return request.get(`/custom-orders/${orderId}/issues`, { params })
}

// 解决异常
export function resolveQualityIssue(issueId: number, data: { resolution: string; operator_id: number }) {
  return request.put(`/custom-orders/issues/${issueId}/resolve`, data)
}

// 创建售后
export function createAfterSales(orderId: number, data: AfterSalesCreateDto) {
  return request.post(`/custom-orders/${orderId}/after-sales`, data)
}

// 列出售后
export function listAfterSales(orderId: number, params?: AfterSalesQueryParams) {
  return request.get(`/custom-orders/${orderId}/after-sales`, { params })
}

// 更新售后
export function updateAfterSales(afterSalesId: number, data: AfterSalesUpdateDto) {
  return request.put(`/custom-orders/after-sales/${afterSalesId}`, data)
}
