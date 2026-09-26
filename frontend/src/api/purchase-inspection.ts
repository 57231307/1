import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_inspection::Model（services/purchase_inspection_service.rs:202/229）。
// 键为实体 snake_case；inspection_status 词表仅 pending/completed（models/status/purchase_inventory.rs:114）。
// receipt_no/supplier_name/inspector_name 实体不提供，需后端 JOIN，列已保留（见注释）。
export interface PurchaseInspection {
  id?: number;
  inspection_no: string;
  receipt_id: number | null;
  order_id: number | null;
  supplier_id: number;
  inspection_date: string;
  inspector_id: number | null;
  inspection_type: string | null;
  sample_size: number | null;
  defect_count: number | null;
  pass_quantity: number | null;
  reject_quantity: number | null;
  inspection_status: string | null;
  inspection_result: string | null;
  quality_score: number | null;
  defect_description: string | null;
  attachment_urls: string | null;
  /** 后端 purchase_inspection.notes（备注） */
  notes: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  completed_by: number | null;
  /** 需要后端 JOIN：purchase_inspection.receipt_id -> purchase_receipt.receipt_no（Model 无该列） */
  receipt_no?: string | null;
  /** 需要后端 JOIN：purchase_inspection.supplier_id -> suppliers.supplier_name（Model 无该列） */
  supplier_name?: string | null;
  /** 需要后端 JOIN：purchase_inspection.inspector_id -> users 姓名（Model 无该列） */
  inspector_name?: string | null;
  /** get_inspection 仅返回单条 Model、不含明细；明细需另调 /inspections/:id/items，列已保留 */
  items?: PurchaseInspectionItem[];
}

/**
 * 明细出参：后端 purchase_inspection_item::Model 的键为 snake_case。
 * product_name/product_code 由 list_inspection_items 的 JOIN 产出（schema gap：后端无此列）。
 */
export interface PurchaseInspectionItem {
  id?: number;
  inspection_id?: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  expected_quantity?: number;
  inspected_quantity: number;
  passed_quantity: number;
  failed_quantity: number;
  defect_reason?: string;
  remark?: string;
}

export interface PurchaseInspectionQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_id?: number;
  status?: string;
  result?: string;
  inspection_date_from?: string;
  inspection_date_to?: string;
}

/**
 * 创建质检单请求（严格对齐 backend CreatePurchaseInspectionRequest，
 * services/purchase_inspection_service.rs:355）。
 * 注意：后端 supplier_id 是 Option<i32>，但 service 代码在 None 时报错，实际为必填。
 * 不含 items：明细通过 POST /inspections/{id}/items 单独提交。
 * remark 字段：后端键名是 notes（非 remark），历史前端用 remark 字段会被 Axum 丢弃。
 */
export interface CreatePurchaseInspectionPayload {
  receipt_id?: number;
  order_id?: number;
  /** 后端声明为 Option<i32>，但 create_inspection 在 None 时直接 validation 拒绝，故按必填建模 */
  supplier_id: number;
  inspection_date?: string;
  inspector_id?: number;
  inspection_type?: string;
  sample_size?: number;
  /** 后端键名是 notes（非 remark） */
  notes?: string;
}

/**
 * 更新质检单请求（严格对齐 backend UpdatePurchaseInspectionRequest，
 * services/purchase_inspection_service.rs:383）。
 * 仅 sample_size/defect_description/notes 三字段可更新。
 */
export interface UpdatePurchaseInspectionPayload {
  sample_size?: number;
  defect_description?: string;
  notes?: string;
}

/**
 * 创建质检明细请求（严格对齐 backend CreateInspectionItemDto，
 * handlers/purchase_inspection_handler.rs:134）。
 */
export interface CreateInspectionItemPayload {
  product_id: number;
  item_name: string;
  qualified_quantity: number;
  unqualified_quantity: number;
  remark?: string;
}

/**
 * 更新质检明细请求（严格对齐 backend UpdateInspectionItemDto，
 * handlers/purchase_inspection_handler.rs:153）。
 */
export interface UpdateInspectionItemPayload {
  qualified_quantity?: number;
  unqualified_quantity?: number;
  remark?: string;
}

// 检验单列表
export const getPurchaseInspectionList = (params?: PurchaseInspectionQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseInspection[]; total: number }>>(
    '/purchase/inspections',
    { params }
  );

// 创建检验单：请求体严格为 CreatePurchaseInspectionPayload（对齐后端 CreatePurchaseInspectionRequest）。
export const createPurchaseInspection = (data: CreatePurchaseInspectionPayload) =>
  request.post<ApiResponse<PurchaseInspection>>('/purchase/inspections', data);

// 获取检验单详情
export const getPurchaseInspectionById = (id: number) =>
  request.get<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`);

// 更新检验单：请求体严格为 UpdatePurchaseInspectionPayload（对齐后端 UpdatePurchaseInspectionRequest）。
export const updatePurchaseInspection = (id: number, data: UpdatePurchaseInspectionPayload) =>
  request.put<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`, data);

// 后端 purchase_inspection_handler::complete_inspection 的 Json<CompleteInspectionRequest>
// 必填 pass_quantity / reject_quantity / inspection_result（snake_case，无 rename_all）。
export interface CompleteInspectionPayload {
  pass_quantity: number;
  reject_quantity: number;
  inspection_result: string;
}

export const completePurchaseInspection = (id: number, data: CompleteInspectionPayload) =>
  request.post<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}/complete`, data);

// 获取检验明细；后端 list_inspection_items 返回 {items:[], total, inspection_id} 嵌套对象
export const getPurchaseInspectionItemList = (id: number) =>
  request.get<ApiResponse<{ items: PurchaseInspectionItem[]; total: number }>>(
    `/purchase/inspections/${id}/items`
  );

// 创建检验明细：请求体严格为 CreateInspectionItemPayload（对齐后端 CreateInspectionItemDto）。
export const createPurchaseInspectionItem = (id: number, data: CreateInspectionItemPayload) =>
  request.post<ApiResponse<PurchaseInspectionItem>>(`/purchase/inspections/${id}/items`, data);

// 更新检验明细：请求体严格为 UpdateInspectionItemPayload（对齐后端 UpdateInspectionItemDto）。
export const updatePurchaseInspectionItem = (
  id: number,
  itemId: number,
  data: UpdateInspectionItemPayload
) =>
  request.put<ApiResponse<PurchaseInspectionItem>>(
    `/purchase/inspections/${id}/items/${itemId}`,
    data
  );

// 删除检验明细
export const deletePurchaseInspectionItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/inspections/${id}/items/${itemId}`);
