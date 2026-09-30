// 定制订单全流程跟踪 API 客户端
// 16 端点封装
// 创建时间: 2026-06-17
// 端点路径相对于 baseURL（/api/v1/erp），不要重复添加前缀，否则会产生双重前缀

import { request } from './request';
import type { ApiResponse } from '@/types/api';

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
};

export const CUSTOM_ORDER_STATUS_COLORS: { [key: string]: string } = {
  draft: 'info',
  yarn_purchasing: 'primary',
  dyeing: 'warning',
  finishing: 'warning',
  delivery: 'success',
  after_sales: 'danger',
  completed: 'success',
  cancelled: 'info',
};

export const NODE_STATUS: { [key: string]: string } = {
  pending: '待开始',
  in_progress: '进行中',
  completed: '已完成',
  blocked: '阻塞',
};

export const NODE_STATUS_COLORS: { [key: string]: string } = {
  pending: 'info',
  in_progress: 'primary',
  completed: 'success',
  blocked: 'danger',
};

export const ISSUE_SEVERITY: { [key: string]: string } = {
  low: '低',
  medium: '中',
  high: '高',
  critical: '严重',
};

export const ISSUE_SEVERITY_COLORS: { [key: string]: string } = {
  low: 'info',
  medium: 'warning',
  high: 'danger',
  critical: '#f56c6c',
};

export const AFTER_SALES_TYPE: Record<string, string> = {
  complaint: '客诉',
  repair: '维修',
  exchange: '换货',
  return_goods: '退货',
  refund: '退款',
};

// 任务 #148 缺陷 B：词表对齐后端写入方（custom_order_aftersales_service.rs
// is_valid_transition + accept/evaluate 方法实际写入的状态全集），补齐 accepted/evaluated
export const AFTER_SALES_STATUS: Record<string, string> = {
  opened: '已开',
  accepted: '已受理',
  processing: '处理中',
  resolved: '已解决',
  evaluated: '已评价',
  closed: '已关闭',
  rejected: '已拒绝',
};

// P2-9a 修复（批次 82 v1 复审）：定制订单 API 强类型化，替代 11 处 any
// 字段与后端 DTO 对齐：custom_order_create_dto.rs / custom_order_update_dto.rs /
// quality_issue_dto.rs / custom_order_aftersales_service.rs

/** 创建定制订单请求（对齐后端 CreateCustomOrderDto） */
export interface CustomOrderCreateDto {
  customer_id: number;
  product_id: number;
  color_id?: number;
  spec: string;
  quantity: number;
  unit: string;
  custom_requirements?: unknown;
  yarn_spec?: string;
  dye_method?: string;
  finishing_method?: string;
  expected_delivery_date?: string;
  sales_order_id?: number;
  total_amount?: number;
  currency?: string;
  notes?: string;
}

/** 更新定制订单请求（对齐后端 UpdateCustomOrderDto） */
export interface CustomOrderUpdateDto {
  spec?: string;
  quantity?: number;
  unit?: string;
  custom_requirements?: unknown;
  yarn_spec?: string;
  dye_method?: string;
  finishing_method?: string;
  expected_delivery_date?: string;
  total_amount?: number;
  notes?: string;
}

/** 推进订单状态请求（对齐后端 AdvanceRequest）
 * v11 批次 160 P2-6 修复：后端 handler 实际使用 AdvanceRequest（不含 target_status），
 * service.advance 自动判断下一状态；AdvanceStatusDto 死代码已从后端删除 */
export interface CustomOrderAdvanceDto {
  operator_id: number;
  notes?: string;
}

/** 添加工艺节点请求（对齐后端 CreateProcessNodeDto） */
export interface ProcessNodeCreateDto {
  node_type: string;
  node_name: string;
  sequence: number;
  planned_start_date?: string;
  planned_end_date?: string;
}

/** 更新工艺节点请求（对齐后端 UpdateProcessNodeDto） */
export interface ProcessNodeUpdateDto {
  status?: string;
  operator_id?: number;
  actual_start_date?: string;
  actual_end_date?: string;
  notes?: string;
}

/** 推进工艺节点请求（对齐后端 AdvanceNodeDto） */
export interface ProcessNodeAdvanceDto {
  action: string;
  operator_id: number;
  notes?: string;
  attachments?: string[];
}

/** 添加节点日志请求（对齐后端 AddProcessLogDto） */
export interface NodeLogCreateDto {
  action: string;
  operator_id: number;
  before_status?: string;
  after_status?: string;
  log_content?: string;
  attachments?: string[];
}

/** 上报质量异常请求（对齐后端 ReportQualityIssueDto）
 * - custom_order_id **不属于请求体**（任务 #148 同构契约修复）：异常归属由 URL
 *   path 参数权威提供（handler `service.report_issue(id, dto)` 注入），后端 DTO
 *   已删除该字段，body 即使携带也会被 serde 忽略（防伪造覆盖）。此前声明为
 *   虚标可选键，掩盖了后端曾必填 422 的真实契约缺口。 */
export interface QualityIssueCreateDto {
  process_node_id?: number;
  issue_type: string;
  severity: string;
  description: string;
  color_delta_e?: number;
  color_fastness_grade?: number;
}

/** 质量异常列表查询参数 */
export interface QualityIssueQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  severity?: string;
}

/** 售后工单信息（对齐后端 AfterSalesInfo）
 * 注意：refund_amount 后端为 rust_decimal::Decimal，JSON 出参序列化为**字符串**
 *（如 "1200.50"），消费处不得按 number 直接做算术 / .toFixed，
 *  展示统一走 utils/formatCurrency 的 Number() 归一（任务 #148 缺陷 D）。 */
export interface AfterSales {
  id: number;
  issue_type: string;
  /** 后端实体 after_sales.customer_id 为 NOT NULL i32，出参恒有键，不得标可选 */
  customer_id: number;
  description: string;
  status: string;
  opened_at: string;
  closed_at?: string;
  resolution?: string;
  refund_amount?: string;
  quality_issue_id?: number;
  /** 原因分类（后端权威词表 quality/logistics/customer_preference/other，可空） */
  reason_category?: string;
  /** 原因明细（自由文本，可空） */
  reason_detail?: string;
}

/** 创建售后工单请求（任务 #148 契约修复，逐字段对齐后端 CreateAfterSalesDto）
 * - custom_order_id **不属于请求体**：工单归属由 URL path 参数权威提供，
 *   body 即使携带也会被后端忽略（防伪造覆盖）。
 * - issue_type / customer_id / description 对应后端 NOT NULL 必填列，不得标可选；
 * - refund_amount 后端为 Option<Decimal>，serde 同时接受 number 与 string，
 *   refund 类型时后端业务校验必填；
 * - quality_issue_id / reason_category / reason_detail 为后端可选字段，如实声明。 */
export interface AfterSalesCreateDto {
  customer_id: number;
  issue_type: string;
  description: string;
  refund_amount?: string | number;
  quality_issue_id?: number;
  reason_category?: string;
  reason_detail?: string;
}

/** 更新售后工单请求（对齐后端 UpdateAfterSalesDto，全部可选；
 * refund_amount 为 Option<Decimal>，入参接受 string | number） */
export interface AfterSalesUpdateDto {
  status?: string;
  resolution?: string;
  refund_amount?: string | number;
}

/** 售后列表查询参数 */
export interface AfterSalesQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  type?: string;
}

// v3 复审 P1-7：定制订单响应类型定义，对齐后端 response DTO

/** 定制订单列表项（对齐后端 CustomOrderListItemResponse） */
export interface CustomOrderListItem {
  id: number;
  order_no: string;
  customer_id: number;
  product_id: number;
  color_id?: number;
  spec: string;
  quantity: number;
  unit: string;
  status: string;
  expected_delivery_date?: string;
  actual_delivery_date?: string;
  total_amount?: number;
  currency: string;
  sales_order_id?: number;
  created_at: string;
  notes?: string;
}

/** 定制订单工艺节点（详情接口返回结构） */
export interface CustomOrderProcessNode {
  id: number;
  node_type: string;
  node_name: string;
  sequence: number;
  status: string;
  planned_start_date?: string;
  planned_end_date?: string;
  actual_start_date?: string;
  actual_end_date?: string;
  notes?: string;
}

/** 质量异常信息（对齐后端 QualityIssueResponse，详情接口返回 quality_issues 字段元素类型） */
export interface QualityIssue {
  id: number;
  custom_order_id?: number;
  process_node_id?: number;
  issue_type: string;
  severity: string;
  description?: string;
  color_delta_e?: number;
  color_fastness_grade?: number;
  status: string;
  resolution?: string;
  discovered_at?: string;
  created_at?: string;
  updated_at?: string;
}

/** 定制订单详情（对齐后端 CustomOrderDetailResponse，含 notes + process_nodes + quality_issues + after_sales） */
export interface CustomOrderDetail extends CustomOrderListItem {
  yarn_spec?: string;
  dye_method?: string;
  finishing_method?: string;
  /** 定制要求（JSONB，含 note 等自由键） */
  custom_requirements?: unknown;
  updated_at: string;
  process_nodes: CustomOrderProcessNode[];
  // v11 批次 181 P2-1 修复：详情接口返回的关联字段，之前未声明导致前端用 unknown[] 绕过
  quality_issues?: QualityIssue[];
  after_sales?: AfterSales[];
}

// v3 复审 P2-5：时间线相关类型，供 tracking.vue 替代 any

/** 节点日志（对齐后端 ProcessLogResponse，tracking.vue 时间线日志项） */
export interface NodeLog {
  id: number;
  action: string;
  operator_id: number;
  before_status?: string;
  after_status?: string;
  log_content?: string;
  log_time: string;
  attachments?: string[];
}

/** 时间线工艺节点（扩展 CustomOrderProcessNode，含节点日志） */
export interface TimelineProcessNode extends CustomOrderProcessNode {
  logs: NodeLog[];
}

/** 订单时间线响应（getTimeline 返回结构） */
export interface OrderTimeline {
  order_no: string;
  current_status: string;
  nodes: TimelineProcessNode[];
}

// 列表查询
// 后端 custom_order_handler::list_custom_orders 返回 ApiResponse<PagedResponse<CustomOrderListItem>>，
// PagedResponse { items, total, page, page_size }，真实列表键为 items（非裸数组）。
export function getCustomOrderList(params: {
  page?: number;
  page_size?: number;
  status?: string;
  customer_id?: number;
  keyword?: string;
}): Promise<
  ApiResponse<{ items: CustomOrderListItem[]; total: number; page: number; page_size: number }>
> {
  return request.get('/custom-orders', { params });
}

// 创建草稿
export function createCustomOrder(
  data: CustomOrderCreateDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.post('/custom-orders', data);
}

// 详情
export function getCustomOrder(id: number): Promise<ApiResponse<CustomOrderDetail>> {
  return request.get(`/custom-orders/${id}`);
}

// 更新
export function updateCustomOrder(
  id: number,
  data: CustomOrderUpdateDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.put(`/custom-orders/${id}`, data);
}

// 取消
export function cancelCustomOrder(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.delete(`/custom-orders/${id}`, {
    data: { reason },
  });
}

// 推进状态
export function advanceCustomOrder(
  id: number,
  data: CustomOrderAdvanceDto
): Promise<ApiResponse<CustomOrderListItem>> {
  return request.post(`/custom-orders/${id}/advance`, data);
}

// 添加工艺节点
export function createProcessNode(orderId: number, data: ProcessNodeCreateDto) {
  return request.post(`/custom-orders/${orderId}/nodes`, data);
}

// 更新工艺节点
export function updateProcessNode(orderId: number, nodeId: number, data: ProcessNodeUpdateDto) {
  return request.put(`/custom-orders/${orderId}/nodes/${nodeId}`, data);
}

// 推进工艺节点
export function advanceProcessNode(orderId: number, nodeId: number, data: ProcessNodeAdvanceDto) {
  return request.post(`/custom-orders/${orderId}/nodes/${nodeId}/advance`, data);
}

// 添加节点日志
export function createNodeLog(orderId: number, nodeId: number, data: NodeLogCreateDto) {
  return request.post(`/custom-orders/${orderId}/nodes/${nodeId}/logs`, data);
}

// 获取时间线
export function getTimeline(orderId: number): Promise<ApiResponse<OrderTimeline>> {
  return request.get(`/custom-orders/${orderId}/timeline`);
}

// 上报质量异常
export function reportQualityIssue(orderId: number, data: QualityIssueCreateDto) {
  return request.post(`/custom-orders/${orderId}/issues`, data);
}

// 列出异常
export function getQualityIssueList(orderId: number, params?: QualityIssueQueryParams) {
  return request.get(`/custom-orders/${orderId}/issues`, { params });
}

// 解决异常
export function resolveQualityIssue(
  issueId: number,
  data: { resolution: string; operator_id: number }
) {
  return request.put(`/custom-orders/issues/${issueId}/resolve`, data);
}

// 创建售后
export function createAfterSales(orderId: number, data: AfterSalesCreateDto) {
  return request.post(`/custom-orders/${orderId}/after-sales`, data);
}

// 列出售后
export function getAfterSalesList(orderId: number, params?: AfterSalesQueryParams) {
  return request.get(`/custom-orders/${orderId}/after-sales`, { params });
}

// 更新售后
export function updateAfterSales(afterSalesId: number, data: AfterSalesUpdateDto) {
  return request.put(`/custom-orders/after-sales/${afterSalesId}`, data);
}
