import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_return::Model（services/purchase_return_service.rs:506/543）。
// 键为实体 snake_case；return_status 词表 draft/submitted/approved/rejected
// （models/status/purchase_inventory.rs:98）。表内无订单号/供应商名/创建人名，需后端 JOIN（见注释）。
export interface PurchaseReturn {
  id?: number;
  return_no: string;
  receipt_id: number | null;
  order_id: number | null;
  supplier_id: number;
  return_date: string;
  warehouse_id: number | null;
  department_id: number | null;
  reason_type: string | null;
  reason_detail: string | null;
  return_status: string | null;
  total_quantity: number | null;
  total_quantity_alt: number | null;
  total_amount: number | null;
  /** 后端 purchase_return.notes（备注） */
  notes: string | null;
  created_by: number | null;
  created_at: string;
  updated_by: number | null;
  updated_at: string;
  approved_by: number | null;
  approved_at: string | null;
  rejected_reason: string | null;
  /** 需要后端 JOIN：purchase_return.order_id -> purchase_orders.order_no（Model 无该列） */
  purchase_order_no?: string | null;
  /** 需要后端 JOIN：purchase_return.supplier_id -> suppliers.supplier_name（Model 无该列） */
  supplier_name?: string | null;
  /** 需要后端 JOIN：purchase_return.created_by -> users 姓名（Model 无该列） */
  created_by_name?: string | null;
  /** get_purchase_return 仅返回单条 Model、不含明细；明细需另调 /returns/:id/items，列已保留 */
  items?: PurchaseReturnItem[];
}

// 明细出参 = 后端 PurchaseReturnItemDto（list_items，services/purchase_return_service.rs:631）。
// material_code/material_name 由 DTO 的 JOIN 提供（Option ⇒ string | null）。
export interface PurchaseReturnItem {
  id: number;
  return_id: number;
  line_no: number;
  material_id: number;
  material_code: string | null;
  material_name: string | null;
  quantity_returned: number;
  unit_price: number;
  tax_rate: number;
  discount_percent: number;
  subtotal: number;
  tax_amount: number;
  discount_amount: number;
  total_amount: number;
  notes: string | null;
}

// 列表查询参数 = 后端 ReturnQueryParams（handlers/purchase_return_handler.rs:206）。
// 真实字段：page/page_size/status/supplier_id。keyword/startDate/endDate/按订单号筛选后端不提供（见交付报告）。
export interface PurchaseReturnQueryParams {
  page?: number;
  page_size?: number;
  supplier_id?: number;
  status?: string;
  /** 需要后端支持：ReturnQueryParams 无 keyword 字段（退货单号搜索） */
  keyword?: string;
  /** 需要后端支持：ReturnQueryParams 无日期范围字段 */
  start_date?: string;
  /** 需要后端支持：ReturnQueryParams 无日期范围字段 */
  end_date?: string;
}

// D14 Batch 5b：原 purchaseReturnApi.list 转为风格 B 函数
export const getPurchaseReturnList = (params?: PurchaseReturnQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReturn[]; total: number }>>('/purchase/returns', {
    params,
  });

// D14 Batch 5b：原 purchaseReturnApi.create 转为风格 B 函数
export const createPurchaseReturn = (data: Partial<PurchaseReturn>) =>
  request.post<ApiResponse<PurchaseReturn>>('/purchase/returns', data);

// D14 Batch 5b：原 purchaseReturnApi.getById 转为风格 B 函数
export const getPurchaseReturnById = (id: number) =>
  request.get<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`);

// D14 Batch 5b：原 purchaseReturnApi.update 转为风格 B 函数
export const updatePurchaseReturn = (id: number, data: Partial<PurchaseReturn>) =>
  request.put<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`, data);

// D14 Batch 5b：原 purchaseReturnApi.delete 转为风格 B 函数
export const deletePurchaseReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}`);

// D14 Batch 5b：原 purchaseReturnApi.submit 转为风格 B 函数
export const submitPurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/submit`);

// D14 Batch 5b：原 purchaseReturnApi.approve 转为风格 B 函数
export const approvePurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/approve`);

// D14 Batch 5b：原 purchaseReturnApi.reject 转为风格 B 函数
export const rejectPurchaseReturn = (id: number, reason?: string) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/reject`, { reason });

// D14 Batch 5b：原 purchaseReturnApi.listItems 转为风格 B 函数
export const getPurchaseReturnItemList = (id: number) =>
  request.get<ApiResponse<PurchaseReturnItem[]>>(`/purchase/returns/${id}/items`);

// D14 Batch 5b：原 purchaseReturnApi.createItem 转为风格 B 函数
export const createPurchaseReturnItem = (id: number, data: Partial<PurchaseReturnItem>) =>
  request.post<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items`, data);

// D14 Batch 5b：原 purchaseReturnApi.updateItem 转为风格 B 函数
export const updatePurchaseReturnItem = (
  id: number,
  itemId: number,
  data: Partial<PurchaseReturnItem>
) => request.put<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items/${itemId}`, data);

// D14 Batch 5b：原 purchaseReturnApi.deleteItem 转为风格 B 函数
export const deletePurchaseReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}/items/${itemId}`);
