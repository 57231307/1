import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_contract::Model，键为实体 snake_case。
// status 词表 draft/active/cancelled/rejected（权威 models/status/bpm_crm_contract.rs::contract，
// 采销两合同域共用；approve 写 active、reject 写 rejected，两动作各落一列理由）。
// 审批理由两列（approval_reason/rejected_reason，m0079 加列）本接口刻意不声明：
// 当前列表/详情均无展示位，补键即恒空假列（与销售合同域同一口径）。
// contract 表无 created_by 姓名列、无 currency/delivery_terms/明细列；下方保留键为待后端补齐的缺口。
export interface PurchaseContract {
  id: number;
  contract_no: string;
  contract_name: string;
  contract_type: string | null;
  supplier_id: number;
  supplier_name: string | null;
  /** 后端 rust_decimal::Decimal 默认 serde 序列化为字符串（如 "12345.67"）；读展示须 Number() 化 */
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
  status: string;
  created_by: number;
  created_at: string;
  updated_at: string;
  /** 需要后端 JOIN：purchase_contracts.created_by -> users 姓名（Model 无该列） */
  created_by_name?: string | null;
  /** 需要后端字段：purchase_contracts 无 currency 列（模型确认，见 backend/src/models/purchase_contract.rs） */
  currency?: string | null;
  /** 需要后端字段：purchase_contracts 无 delivery_terms 列（仅有 delivery_location） */
  delivery_terms?: string | null;
  /** get_contract 仅返回单条 Model、不含明细；后端无合同明细表，列已保留 */
  items?: ContractItem[];
}

// 合同明细：后端无对应表/端点，保留供建单表单结构；字段命名对齐建单入参，非响应契约。
export interface ContractItem {
  id: number;
  contract_id: number;
  product_id: number;
  product_name?: string | null;
  product_code?: string | null;
  quantity: number;
  unit: string;
  price: number;
  amount: number;
  remark?: string | null;
}

// 后端 purchase_contract_handler::ContractQuery（list_contracts 的 Query<T>，全字段 Option、snake_case）
export interface PurchaseContractQuery {
  keyword?: string;
  status?: string;
  supplier_id?: number;
  page?: number;
  page_size?: number;
}

/**
 * 创建采购合同请求，逐字段对齐后端 CreateContractRequestDto。
 * - delivery_date 后端可空（真实列 delivery_date 可空）；前端表单口径仍必填交货日期（产品要求），类型可选不冲突。
 * - contract_type/signed_date/effective_date/expiry_date/payment_method/delivery_location
 *   均为真实 DB 列，DTO 接收并落库，须随建单入参提交。
 * - 后端字段名是 remark（单数）而非 remarks，m0016 补齐的可空列。
 */
export interface CreatePurchaseContractPayload {
  contract_no: string;
  contract_name: string;
  supplier_id: number;
  total_amount: number;
  contract_type?: string;
  payment_terms?: string;
  delivery_date?: string;
  signed_date?: string;
  effective_date?: string;
  expiry_date?: string;
  payment_method?: string;
  delivery_location?: string;
  /** 后端字段名为 remark（单数），非 remarks */
  remark?: string;
}

/**
 * 更新采购合同请求，逐字段对齐后端 UpdateContractDto。
 * 字段语义 = 显式三态（RFC 7386 JSON Merge Patch）：
 * 键缺席=保持原值、显式 null=清空该列为 NULL、有值=覆盖。
 * - contract_name/supplier_id 为 NOT NULL 列：不开 null 清空，空则应省略键（保持原值），
 *   发送显式 null 会被后端判业务错误（400 + 外显文案）。
 * - 其余均为 DB 可空列，类型如实声明 `T | null`：清空须送 null，
 *   禁止塌成 `|| undefined` 省略键（省略=保持原值）。
 * - 不含 contract_no：单据号系统生成、编辑链路忽略，发送只会构成"后端不读"的死键。
 */
export interface UpdatePurchaseContractPayload {
  /** NOT NULL 列：空则省略键，禁止显式 null */
  contract_name?: string;
  /** NOT NULL 列：空则省略键，禁止显式 null */
  supplier_id?: number;
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
}

export function getPurchaseContractList(
  params?: PurchaseContractQuery
): Promise<ApiResponse<PurchaseContract[]>> {
  return request.get('/purchase/purchase-contracts', { params });
}

export function getPurchaseContract(id: number): Promise<ApiResponse<PurchaseContract>> {
  return request.get(`/purchase/purchase-contracts/${id}`);
}

export function createPurchaseContract(
  data: CreatePurchaseContractPayload
): Promise<ApiResponse<PurchaseContract>> {
  return request.post('/purchase/purchase-contracts', data);
}

export function updatePurchaseContract(
  id: number,
  data: UpdatePurchaseContractPayload
): Promise<ApiResponse<PurchaseContract>> {
  return request.put(`/purchase/purchase-contracts/${id}`, data);
}

export function deletePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/purchase/purchase-contracts/${id}`);
}

// 审批「通过」请求体，对齐后端 ApproveContractRequest。
// approval_reason 必填语义在后端 handler 收口（缺失/空串/纯空白一律 400），
// 值落 purchase_contracts.approval_reason 列 ⇒ 前端必带且必采（采集见 useActionPrompts）。
export interface ApprovePurchaseContractRequest {
  approval_reason: string;
}

// 审批「拒绝」请求体，对齐后端 RejectContractRequest：reason 为 String 非 Option ⇒ 必带体；
// trim 后空串 400，非空值落 purchase_contracts.rejected_reason 列。
export interface RejectPurchaseContractRequest {
  reason: string;
}

// approve/reject 两个端点成功出参均为 ApiResponse<String>（后端回含记录 ID 的拼接文案）：
// 该 data 不外显，文案一律走前端 i18n，ID 不进用户可见文案。
// 状态门：仅 draft 可通过/拒绝。

/** 审批通过采购合同（draft → active，端点 POST /purchase/purchase-contracts/{id}/approve） */
export function approvePurchaseContract(
  id: number,
  data: ApprovePurchaseContractRequest
): Promise<ApiResponse<string>> {
  return request.post(`/purchase/purchase-contracts/${id}/approve`, data);
}

/** 拒绝采购合同（draft → rejected 终态，端点 POST /purchase/purchase-contracts/{id}/reject，与 approve 各自独立） */
export function rejectPurchaseContract(
  id: number,
  data: RejectPurchaseContractRequest
): Promise<ApiResponse<string>> {
  return request.post(`/purchase/purchase-contracts/${id}/reject`, data);
}

// 执行采购合同请求体，对齐后端 ExecuteContractRequestDto。
// execution_type / execution_amount / execution_date 必填；
// related_bill_type/related_bill_id 为可选关联单据，本表单不采集（不发送即后端 None），避免留死字段。
export interface ExecutePurchaseContractRequest {
  execution_type: string;
  execution_amount: number;
  execution_date: string;
  remark?: string;
}

export function executePurchaseContract(
  id: number,
  data: ExecutePurchaseContractRequest
): Promise<ApiResponse<void>> {
  return request.put(`/purchase/purchase-contracts/${id}/execute`, data);
}

// 后端 purchase_contract_handler::CancelContractRequest 必填 reason（取消原因）
export function cancelPurchaseContract(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.put(`/purchase/purchase-contracts/${id}/cancel`, { reason });
}

// 导出采购合同：GET /purchase/purchase-contracts/export 返回 xlsx blob，前端 createObjectURL 触发下载。
// 查询参数按同资源 list_contracts 的 ContractQuery（PurchaseContractQuery）定型，避免泛型键被静默丢弃。
export function exportPurchaseContracts(params?: PurchaseContractQuery): Promise<Blob> {
  return request.get('/purchase/purchase-contracts/export', {
    params,
    responseType: 'blob',
  });
}
