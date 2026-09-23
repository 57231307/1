import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_contract::Model（services/purchase_contract_service.rs get_list/get_by_id）。
// 键为实体 snake_case；status 词表 draft/active/cancelled（models/status/bpm_crm_contract.rs:34 contract）。
// supplier_name/contract_name/total_amount/signed_date/effective_date/expiry_date/payment_terms 均为真实列。
// 注：contract 表无 created_by 姓名列、无 currency/delivery_terms/明细，历史前端按这些自创键读取 ⇒ 恒空。
export interface PurchaseContract {
  id: number;
  contract_no: string;
  contract_name: string;
  contract_type: string | null;
  supplier_id: number;
  supplier_name: string | null;
  total_amount: number | null;
  signed_date: string | null;
  effective_date: string | null;
  expiry_date: string | null;
  payment_terms: string | null;
  payment_method: string | null;
  delivery_date: string | null;
  delivery_location: string | null;
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
 * 创建采购合同请求（严格对齐 backend CreateContractRequestDto，
 * handlers/purchase_contract_handler.rs:31）。
 * 注意：后端字段是 remark（单数），前端历史用 remarks（复数）⇒ 键名不对齐导致 remark 恒为 None。
 * contract_type/signed_date/effective_date/expiry_date/payment_method/delivery_location
 * 为真实 DB 列但不在 CreateContractRequestDto 内（schema gap）。
 */
export interface CreatePurchaseContractPayload {
  contract_no: string;
  contract_name: string;
  supplier_id: number;
  total_amount: number;
  payment_terms?: string;
  /** 后端为 chrono::NaiveDate（必填），格式 YYYY-MM-DD */
  delivery_date: string;
  /** 后端字段名为 remark（单数），非 remarks */
  remark?: string;
}

/**
 * 更新采购合同请求（严格对齐 backend UpdateContractDto，
 * handlers/purchase_contract_handler.rs:45）。
 * 仅 contract_name/payment_terms 可更新；其余字段为 schema gap。
 */
export interface UpdatePurchaseContractPayload {
  contract_name?: string;
  payment_terms?: string;
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

export function approvePurchaseContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/purchase/purchase-contracts/${id}/approve`);
}

// 执行采购合同请求体：对齐后端 purchase_contract_handler::ExecuteContractRequestDto。
// execution_type / execution_amount / execution_date 必填；related_bill_type/related_bill_id 为可选关联单据，
// 本表单暂不采集（后端为 Option，不发送即 None），避免留死字段。
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

// 批次 94 P2-12 修复：补全采购合同导出接口（原缺失，导致 usePcProc 导出占位假成功）
// 返回 blob，前端用 URL.createObjectURL 触发下载。
// 注：后端目前未注册 /purchase/purchase-contracts/export 路由，查询参数按同资源 list_contracts 的
// ContractQuery（PurchaseContractQuery）定型，避免使用泛型 QueryParams 让任意键被静默丢弃。
export function exportPurchaseContracts(params?: PurchaseContractQuery): Promise<Blob> {
  return request.get('/purchase/purchase-contracts/export', {
    params,
    responseType: 'blob',
  });
}
