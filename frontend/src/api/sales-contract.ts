import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 列表/详情出参 = 后端 sales_contract::Model（handler 直接序列化实体）+ created_by_name 富化键。
 * - total_amount/stamp_tax_amount 为后端 Decimal ⇒ JSON 字符串（或 null），展示/回填处 Number() 归一。
 * - status 词表 draft/active/cancelled/rejected（权威 models/status/bpm_crm_contract.rs::contract，
 *   采销两合同域共用；approve 写 active、reject 写 rejected，两动作各落一列理由）。
 * - 审批理由两列（approval_reason/rejected_reason，m0079 加列）本接口刻意不声明：
 *   当前列表/详情均无展示位，补键即恒空假列（与价目域同一口径）。
 * - 明细行不在本对象上（后端 list/get 不联查 items），编辑回显需调 getSalesContractItems。
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
  /** 真实列 remark（m0016 迁移补列，可空）；后端出参键为单数 remark，非 remarks */
  remark: string | null;
  status: 'draft' | 'active' | 'cancelled' | 'rejected';
  created_by: number;
  /** list_contracts handler 富化键（users.real_name 批量查询），get_contract 详情不带 */
  created_by_name?: string | null;
  created_at: string;
  updated_at: string;
  /** 电子签章列（可空） */
  signed_at: string | null;
  signed_by_user_id: number | null;
  signature_hash: string | null;
  signature_image_url: string | null;
  signature_certificate: string | null;
  /** 合同条款列（可空） */
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
 * 创建合同明细行入参，逐字段对齐后端 CreateContractItemDto。
 * 数值以 JSON number 提交（serde/rust_decimal 入参同时接受 number 与 string）。
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
 * 创建销售合同入参，逐字段对齐后端 CreateSalesContractRequestDto。
 * - delivery_date 后端可空（真实列可空）⇒ 前端可缺省。
 * - signed_date/effective_date/expiry_date/payment_method/delivery_location 均为真实 DB 列，
 *   DTO 接收并落库，须随建单入参提交。
 * - remark：DTO 接收并由后端真实落 sales_contracts.remark 列（m0016 补列，可空）。
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
 * 更新销售合同入参（对齐后端 UpdateSalesContractDto）。
 * 字段语义 = 显式三态（RFC 7386 JSON Merge Patch）：
 * 键缺席=保持原值、显式 null=清空该列为 NULL、有值=覆盖；
 * items 传数组=明细整表替换，不传=不动明细（数组字段不开放 null 清空，清空须送 []）。
 * - contract_name/customer_id 为 NOT NULL 列：不开 null 清空，空则应省略键（保持原值），
 *   发送显式 null 会被后端判业务错误（400 + 外显文案）。
 * - 其余均为 DB 可空列，类型如实声明 `T | null`：清空须送 null，
 *   禁止塌成 `|| undefined` 省略键（省略=保持原值）。
 * - 不含 contract_no：后端 UpdateSalesContractDto 无该字段（编号系统生成、编辑链路忽略），
 *   前端发送它只会构成"后端不读"的死键（check-api-request 判负）。
 */
export interface UpdateSalesContractPayload {
  /** NOT NULL 列：空则省略键，禁止显式 null */
  contract_name?: string;
  /** NOT NULL 列：空则省略键，禁止显式 null */
  customer_id?: number;
  /** 以下均为 DB 可空列：有值=覆盖、null=清空、键缺席=保持原值 */
  total_amount?: number | null;
  contract_type?: string | null;
  payment_terms?: string | null;
  delivery_date?: string | null;
  signed_date?: string | null;
  effective_date?: string | null;
  expiry_date?: string | null;
  payment_method?: string | null;
  delivery_location?: string | null;
  remark?: string | null;
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

// 审批「通过」请求体，对齐后端 ApproveSalesContractRequest。
// approval_reason 必填语义在后端 handler 收口（缺失/空串/纯空白一律 400），
// 值落 sales_contracts.approval_reason 列 ⇒ 前端必带且必采（采集见 useActionPrompts）。
export interface ApproveSalesContractRequest {
  approval_reason: string;
}

// 审批「拒绝」请求体，对齐后端 RejectSalesContractRequest：reason 为 String 非 Option ⇒ 必带体；
// trim 后空串 400，非空值落 sales_contracts.rejected_reason 列。
export interface RejectSalesContractRequest {
  reason: string;
}

// approve/reject 两个端点成功出参均为 ApiResponse<String>（后端回含记录 ID 的拼接文案）：
// 该 data 不外显，文案一律走前端 i18n，ID 不进用户可见文案；调用方只判成功与否。
// 状态门：仅 draft 可通过/拒绝。

/** 审批通过销售合同（draft → active，端点 POST /sales/sales-contracts/{id}/approve） */
export function approveSalesContract(
  id: number,
  data: ApproveSalesContractRequest
): Promise<ApiResponse<string>> {
  return request.post(`/sales/sales-contracts/${id}/approve`, data);
}

/** 拒绝销售合同（draft → rejected 终态，端点 POST /sales/sales-contracts/{id}/reject，与 approve 各自独立） */
export function rejectSalesContract(
  id: number,
  data: RejectSalesContractRequest
): Promise<ApiResponse<string>> {
  return request.post(`/sales/sales-contracts/${id}/reject`, data);
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
