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

// D14 Batch 5b：原 purchaseInspectionApi.list 转为风格 B 函数
// 检验单列表
export const getPurchaseInspectionList = (params?: PurchaseInspectionQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseInspection[]; total: number }>>(
    '/purchase/inspections',
    { params }
  );

// D14 Batch 5b：原 purchaseInspectionApi.create 转为风格 B 函数
// 创建检验单
export const createPurchaseInspection = (data: Partial<PurchaseInspection>) =>
  request.post<ApiResponse<PurchaseInspection>>('/purchase/inspections', data);

// D14 Batch 5b：原 purchaseInspectionApi.getById 转为风格 B 函数
// 获取检验单详情
export const getPurchaseInspectionById = (id: number) =>
  request.get<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`);

// D14 Batch 5b：原 purchaseInspectionApi.update 转为风格 B 函数
// 更新检验单
export const updatePurchaseInspection = (id: number, data: Partial<PurchaseInspection>) =>
  request.put<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`, data);

// D14 Batch 5b：原 purchaseInspectionApi.complete 转为风格 B 函数
// 后端 purchase_inspection_handler::complete_inspection 的 Json<CompleteInspectionRequest>
// 必填 pass_quantity / reject_quantity / inspection_result（snake_case，无 rename_all）。
export interface CompleteInspectionPayload {
  pass_quantity: number;
  reject_quantity: number;
  inspection_result: string;
}

export const completePurchaseInspection = (id: number, data: CompleteInspectionPayload) =>
  request.post<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}/complete`, data);

// D14 Batch 5b：原 purchaseInspectionApi.getItems 转为风格 B 函数
// 获取检验明细
export const getPurchaseInspectionItemList = (id: number) =>
  request.get<ApiResponse<{ items: PurchaseInspectionItem[] }>>(
    `/purchase/inspections/${id}/items`
  );

// D14 Batch 5b：原 purchaseInspectionApi.createItem 转为风格 B 函数
// 创建检验明细
export const createPurchaseInspectionItem = (id: number, data: Partial<PurchaseInspectionItem>) =>
  request.post<ApiResponse<PurchaseInspectionItem>>(`/purchase/inspections/${id}/items`, data);

// D14 Batch 5b：原 purchaseInspectionApi.updateItem 转为风格 B 函数
// 更新检验明细
export const updatePurchaseInspectionItem = (
  id: number,
  itemId: number,
  data: Partial<PurchaseInspectionItem>
) =>
  request.put<ApiResponse<PurchaseInspectionItem>>(
    `/purchase/inspections/${id}/items/${itemId}`,
    data
  );

// D14 Batch 5b：原 purchaseInspectionApi.deleteItem 转为风格 B 函数
// 删除检验明细
export const deletePurchaseInspectionItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/inspections/${id}/items/${itemId}`);
