import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Supplier {
  id: number;
  supplier_code: string;
  supplier_name: string;
  supplier_short_name?: string;
  supplier_type?: string;
  credit_code?: string;
  registered_address?: string;
  business_address?: string;
  legal_representative?: string;
  registered_capital?: number;
  establishment_date?: string;
  business_term?: string;
  business_scope?: string;
  taxpayer_type?: string;
  bank_name?: string;
  bank_account?: string;
  contact_phone?: string;
  fax?: string;
  website?: string;
  email?: string;
  main_business?: string;
  main_market?: string;
  employee_count?: number;
  annual_revenue?: number;
  grade?: string;
  grade_score?: number;
  last_evaluation_date?: string;
  status: string;
  is_enabled?: boolean;
  assist_batch?: boolean;
  assist_supplier?: boolean;
  remarks?: string;
  created_at?: string;
  updated_at?: string;
}

// D14 Batch 5b：原 supplierApi.list 与本函数 URL 同为 /purchase/suppliers，判定为重复，移除对象方法，保留本函数
export function getSupplierList(
  params?: SupplierQueryParams
): Promise<ApiResponse<{ items: Supplier[]; total: number }>> {
  return request.get('/purchase/suppliers', { params });
}

// 对应后端 supplier_service::SupplierQueryParams（services/supplier_service.rs:991），
// 无 rename_all → snake_case，全 Option → 可选；category 键后端不存在（原为假筛选，已移除）。
// download_token 为敏感导出审批令牌（V15 P0-S15 fail-closed），导出时按需传入。
export interface SupplierQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_type?: string;
  grade?: string;
  status?: string;
  sort_by?: string;
  sort_order?: string;
  is_enabled?: boolean;
  category_id?: number;
  /** batch-13 P3: 加工商筛选 */
  is_processor?: boolean;
  /** batch-13 P3: 加工商类型筛选 */
  processor_type?: string;
  download_token?: string;
}

export interface SupplierEvaluationData {
  score: number;
  rating: string;
  indicators?: Array<{
    indicator_id: number;
    score: number;
    remark?: string;
  }>;
  remark?: string;
}

export interface SupplierEvaluationResult {
  id: number;
  supplier_id: number;
  score: number;
  rating: string;
  evaluation_date: string;
  evaluator_id?: number;
  evaluator_name?: string;
  remark?: string;
  created_at: string;
}

// D14 Batch 5b：原 supplierApi.getById 转为风格 B 函数
export const getSupplierById = (id: number) =>
  request.get<ApiResponse<Supplier>>(`/purchase/suppliers/${id}`);

// D14 Batch 5b：原 supplierApi.create 转为风格 B 函数
export const createSupplier = (data: Partial<Supplier>) =>
  request.post<ApiResponse<Supplier>>('/purchase/suppliers', data);

// D14 Batch 5b：原 supplierApi.update 转为风格 B 函数
export const updateSupplier = (id: number, data: Partial<Supplier>) =>
  request.put<ApiResponse<Supplier>>(`/purchase/suppliers/${id}`, data);

// D14 Batch 5b：原 supplierApi.delete 转为风格 B 函数
export const deleteSupplier = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/suppliers/${id}`);

// D14 Batch 5b：原 supplierApi.evaluate 转为风格 B 函数
export const evaluateSupplier = (id: number, data: SupplierEvaluationData) =>
  request.post<ApiResponse<SupplierEvaluationResult>>(`/purchase/suppliers/${id}/evaluate`, data);

// D14 Batch 5b：原 supplierApi.getEvaluationHistory 转为风格 B 函数
export const getSupplierEvaluationHistory = (id: number) =>
  request.get<ApiResponse<SupplierEvaluationResult[]>>(`/purchase/suppliers/${id}/evaluations`);

// D14 Batch 5b：原 supplierApi.export 转为风格 B 函数
// V15 P0-S12 + P0-S15 新增（Batch 474）：带水印的 xlsx 导出
// 后端 GET /purchase/suppliers/export 返回 application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
// 水印已由后端注入（操作员/IP/时间戳），前端只需下载 Blob
export const exportSuppliers = (params?: SupplierQueryParams) =>
  request.get<Blob>('/purchase/suppliers/export', { params, responseType: 'blob' });

// ============== 供应商余额/供货历史/联系人/资质（Batch 补齐 API 封装）==============

/** 供应商账户余额（对应后端 supplier_service.rs::SupplierBalance） */
export interface SupplierBalance {
  supplier_id: number;
  supplier_name: string;
  total_amount: number;
  paid_amount: number;
  balance: number;
  order_count: number;
}

/** 供货历史项（对应后端 supplier_service.rs::PurchaseHistoryItem） */
export interface PurchaseHistoryItem {
  order_id: number;
  order_no: string;
  order_date: string;
  total_amount: number;
  status: string;
  item_count: number;
}

/** 供应商联系人（对应后端 models/supplier_contact.rs::Model） */
export interface SupplierContact {
  id: number;
  supplier_id: number;
  contact_name: string;
  department?: string;
  position?: string;
  mobile_phone: string;
  tel_phone?: string;
  email?: string;
  wechat?: string;
  qq?: string;
  is_primary: boolean;
  remarks?: string;
  created_at?: string;
  updated_at?: string;
}

/** 创建联系人请求（对应后端 supplier_service.rs::CreateContactRequest） */
export interface SupplierContactInput {
  contact_name: string;
  department?: string;
  position?: string;
  mobile_phone: string;
  tel_phone?: string;
  email?: string;
  wechat?: string;
  qq?: string;
  is_primary: boolean;
  remarks?: string;
}

/** 更新联系人请求（对应后端 supplier_service.rs::UpdateContactRequest） */
export type SupplierContactUpdate = Partial<SupplierContactInput>;

/** 供应商资质（对应后端 models/supplier_qualification.rs::Model） */
export interface SupplierQualification {
  id: number;
  supplier_id: number;
  qualification_name: string;
  qualification_type: string;
  qualification_no: string;
  issuing_authority: string;
  issue_date: string;
  valid_until: string;
  attachment_path?: string;
  need_annual_check: boolean;
  annual_check_record?: string;
  is_expired?: boolean;
  created_at?: string;
  updated_at?: string;
}

/** 创建/更新资质请求（对应后端 supplier_service.rs::CreateQualificationRequest） */
export interface SupplierQualificationInput {
  qualification_name: string;
  qualification_type: string;
  qualification_no: string;
  issuing_authority: string;
  issue_date: string;
  valid_until: string;
  attachment_path?: string;
  need_annual_check: boolean;
  annual_check_record?: string;
}

/**
 * 查询供应商账户余额（batch-13 P2：订单总额 - 已付款）
 * 后端路由：GET /api/v1/erp/purchase/suppliers/{id}/balance（routes/purchase.rs suppliers）
 */
export const getSupplierBalance = (id: number) =>
  request.get<ApiResponse<SupplierBalance>>(`/purchase/suppliers/${id}/balance`);

/**
 * 查询供应商供货历史（batch-13 P3；limit 缺省时后端取默认条数）
 * 后端路由：GET /api/v1/erp/purchase/suppliers/{id}/purchase-history（routes/purchase.rs suppliers）
 */
export const getSupplierPurchaseHistory = (id: number, params?: { limit?: number }) =>
  request.get<ApiResponse<{ history: PurchaseHistoryItem[]; total: number }>>(
    `/purchase/suppliers/${id}/purchase-history`,
    { params }
  );

/**
 * 获取供应商联系人列表
 * 后端路由：GET /api/v1/erp/purchase/suppliers/{id}/contacts（routes/purchase.rs suppliers）
 */
export const getSupplierContactList = (supplierId: number) =>
  request.get<ApiResponse<SupplierContact[]>>(`/purchase/suppliers/${supplierId}/contacts`);

/**
 * 创建供应商联系人
 * 后端路由：POST /api/v1/erp/purchase/suppliers/{id}/contacts（routes/purchase.rs suppliers）
 */
export const createSupplierContact = (supplierId: number, data: SupplierContactInput) =>
  request.post<ApiResponse<SupplierContact>>(`/purchase/suppliers/${supplierId}/contacts`, data);

/**
 * 更新供应商联系人
 * 后端路由：PUT /api/v1/erp/purchase/suppliers/{id}/contacts/{contact_id}（routes/purchase.rs suppliers）
 */
export const updateSupplierContact = (
  supplierId: number,
  contactId: number,
  data: SupplierContactUpdate
) =>
  request.put<ApiResponse<SupplierContact>>(
    `/purchase/suppliers/${supplierId}/contacts/${contactId}`,
    data
  );

/**
 * 删除供应商联系人
 * 后端路由：DELETE /api/v1/erp/purchase/suppliers/{id}/contacts/{contact_id}（routes/purchase.rs suppliers）
 */
export const deleteSupplierContact = (supplierId: number, contactId: number) =>
  request.delete<ApiResponse<null>>(`/purchase/suppliers/${supplierId}/contacts/${contactId}`);

/**
 * 获取供应商资质列表
 * 后端路由：GET /api/v1/erp/purchase/suppliers/{id}/qualifications（routes/purchase.rs suppliers）
 */
export const getSupplierQualificationList = (supplierId: number) =>
  request.get<ApiResponse<SupplierQualification[]>>(
    `/purchase/suppliers/${supplierId}/qualifications`
  );

/**
 * 创建供应商资质
 * 后端路由：POST /api/v1/erp/purchase/suppliers/{id}/qualifications（routes/purchase.rs suppliers）
 */
export const createSupplierQualification = (supplierId: number, data: SupplierQualificationInput) =>
  request.post<ApiResponse<SupplierQualification>>(
    `/purchase/suppliers/${supplierId}/qualifications`,
    data
  );

/**
 * 更新供应商资质
 * 后端路由：PUT /api/v1/erp/purchase/suppliers/{id}/qualifications/{qualification_id}（routes/purchase.rs suppliers）
 */
export const updateSupplierQualification = (
  supplierId: number,
  qualificationId: number,
  data: SupplierQualificationInput
) =>
  request.put<ApiResponse<SupplierQualification>>(
    `/purchase/suppliers/${supplierId}/qualifications/${qualificationId}`,
    data
  );

/**
 * 删除供应商资质（后端返回 { deleted_id }）
 * 后端路由：DELETE /api/v1/erp/purchase/suppliers/{id}/qualifications/{qualification_id}（routes/purchase.rs suppliers）
 */
export const deleteSupplierQualification = (supplierId: number, qualificationId: number) =>
  request.delete<ApiResponse<{ deleted_id: number }>>(
    `/purchase/suppliers/${supplierId}/qualifications/${qualificationId}`
  );
