import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 列表/详情出参 = 后端 sales_contract::Model（handlers/sales_contract_handler.rs
 * list_contracts/get_contract 直接序列化实体）+ created_by_name 富化键。
 *
 * P0 契约修复（本轮）：
 * - 删除历史自创键 contract_date/start_date/end_date/currency/delivery_terms/remarks/items：
 *   后端从不返回这些键（models/sales_contract.rs 全字段核对），读取恒 undefined，
 *   属「前端类型 ≠ 后端出参」的第二套假契约。
 * - rust_decimal::Decimal 序列化为**字符串**（如 "12345.67"），total_amount/stamp_tax_amount
 *   声明为 string | null，展示/回填处 Number() 归一。
 * - status 词表出自后端 models/status/bpm_crm_contract.rs contract 模块：draft/active/cancelled。
 * - 明细行不在本对象上（后端 get_list/get_by_id 不联查 items），编辑回显需调
 *   getSalesContractItems（GET /sales/sales-contracts/:id/items，后端既有端点）。
 */
export interface SalesContract {
  id: number;
  contract_no: string;
  contract_name: string;
  contract_type: string | null;
  customer_id: number;
  customer_name: string | null;
  total_amount: string | null;
  signed_date: string | null;
  effective_date: string | null;
  expiry_date: string | null;
  payment_terms: string | null;
  payment_method: string | null;
  delivery_date: string | null;
  delivery_location: string | null;
  status: 'draft' | 'active' | 'cancelled';
  created_by: number;
  /** list_contracts handler 富化键（users.real_name 批量查询），get_contract 详情不带 */
  created_by_name?: string | null;
  created_at: string;
  updated_at: string;
  /** V15 电子签章列（可空） */
  signed_at: string | null;
  signed_by_user_id: number | null;
  signature_hash: string | null;
  signature_image_url: string | null;
  signature_certificate: string | null;
  /** V15 P2 合同条款列（可空） */
  quality_terms: string | null;
  breach_liability: string | null;
  dispute_resolution: string | null;
  performance_period: string | null;
  /** 印花税金额（后端按合同类型×金额自动计算），Decimal→string */
  stamp_tax_amount: string | null;
}

/**
 * 合同明细行出参 = 后端 sales_contract_item::Model（GET /sales/sales-contracts/:id/items）。
 * 键名与实体一致：unit_price/quantity/amount 为 Decimal→string；remarks（复数）为实体列名；
 * quantity_tolerance_pct 可空（NULL=走品类/全局默认）。
 */
export interface SalesContractItem {
  id: number;
  contract_id: number;
  product_id: number | null;
  product_name: string;
  product_spec: string | null;
  unit: string;
  quantity: string;
  quantity_tolerance_pct: string | null;
  unit_price: string;
  amount: string;
  delivery_date: string | null;
  remarks: string | null;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

/** 后端 sales_contract_handler::SalesContractQuery（list_contracts 的 Query<T>，全字段 Option、snake_case） */
export interface SalesContractQuery {
  keyword?: string;
  status?: string;
  customer_id?: number;
  page?: number;
  page_size?: number;
}

export function getSalesContractList(
  params?: SalesContractQuery
): Promise<ApiResponse<SalesContract[]>> {
  return request.get('/sales/sales-contracts', { params });
}

export function getSalesContract(id: number): Promise<ApiResponse<SalesContract>> {
  return request.get(`/sales/sales-contracts/${id}`);
}

/** 明细行回源（编辑对话框打开前调用；后端既有端点 get_contract_items） */
export function getSalesContractItems(id: number): Promise<ApiResponse<SalesContractItem[]>> {
  return request.get(`/sales/sales-contracts/${id}/items`);
}

/**
 * 创建合同明细行入参（对齐后端 CreateContractItemDto，handlers/sales_contract_handler.rs:65-78）。
 * 入参数值类型：serde/rust_decimal 同时接受 JSON number 与 string，输入方向保持 number。
 * 必填性以 DTO 为准：product_name/unit/quantity/unit_price 后端非 Option ⇒ 不得标可选；
 * product_id/product_spec/delivery_date/remarks 后端 Option ⇒ 可 null（编辑保存为明细整表
 * 重插，回显的真实值必须原样回传，null 即真实空值，禁止塞默认值）。
 */
export interface CreateContractItemInput {
  product_id?: number | null;
  product_name: string;
  product_spec?: string | null;
  unit: string;
  quantity: number;
  quantity_tolerance_pct?: number | null;
  unit_price: number;
  delivery_date?: string | null;
  remarks?: string | null;
}

/**
 * 创建销售合同入参（对齐后端 CreateSalesContractRequestDto）。
 * P0 契约修复（本轮）：
 * - delivery_date 后端已改 Option（真实列可空）⇒ 前端可缺省，不再触发 400。
 * - 补齐 signed_date/effective_date/expiry_date/payment_method/delivery_location
 *   （真实 DB 列，此前「DTO 不收、service 不写、表单却有输入框」⇒ 创建即丢数据）。
 * - remark：DTO 仍接收但 sales_contracts 无 remark 列（迁移需求已上报后端负责人），
 *   落库前该字段不生效。
 */
export interface CreateSalesContractPayload {
  contract_no: string;
  contract_name: string;
  customer_id: number;
  total_amount: number;
  contract_type?: string;
  payment_terms?: string;
  delivery_date?: string;
  signed_date?: string;
  effective_date?: string;
  expiry_date?: string;
  payment_method?: string;
  delivery_location?: string;
  remark?: string;
  items?: CreateContractItemInput[];
}

/**
 * 更新销售合同入参（对齐后端 UpdateSalesContractDto，PATCH 语义：
 * Some=覆盖、None/缺省=保持原值；items 传数组=明细整表替换，不传=不动明细）。
 * P0 契约修复（本轮）：原后端仅 contract_name/payment_terms，其余表头编辑被静默丢弃。
 */
export interface UpdateSalesContractPayload {
  contract_no?: string;
  contract_name?: string;
  customer_id?: number;
  total_amount?: number;
  contract_type?: string;
  payment_terms?: string;
  delivery_date?: string;
  signed_date?: string;
  effective_date?: string;
  expiry_date?: string;
  payment_method?: string;
  delivery_location?: string;
  remark?: string;
  items?: CreateContractItemInput[];
}

export function createSalesContract(
  data: CreateSalesContractPayload
): Promise<ApiResponse<SalesContract>> {
  return request.post('/sales/sales-contracts', data);
}

export function updateSalesContract(
  id: number,
  data: UpdateSalesContractPayload
): Promise<ApiResponse<SalesContract>> {
  return request.put(`/sales/sales-contracts/${id}`, data);
}

export function deleteSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-contracts/${id}`);
}

export function approveSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-contracts/${id}/approve`);
}

// 执行销售合同请求体：对齐后端 sales_contract_handler::ExecuteSalesContractRequestDto。
// execution_type / execution_amount 必填（后端仅接受 delivery=出库 / payment=收款）。
// related_bill_type/related_bill_id 为可选关联单据，本表单不采集，避免留死字段。
export interface ExecuteSalesContractRequest {
  execution_type: string;
  execution_amount: number;
  remark?: string;
}

export function executeSalesContract(
  id: number,
  data: ExecuteSalesContractRequest
): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/execute`, data);
}

// 后端 sales_contract_handler::CancelSalesContractRequest 必填 reason（取消原因）
export function cancelSalesContract(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/cancel`, { reason });
}
