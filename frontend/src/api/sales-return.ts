import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 退货单表头读模型：键名对应 backend models/sales_return.rs 实体字段。
 * customer_name / sales_order_no 由 list_sales_returns 与 get_sales_return 的
 * customer + sales_order LEFT JOIN 产出（实体只有 customer_id / sales_order_id）。
 * 表头不含明细集合——明细是独立端点 GET /sales/sales-returns/{id}/items。
 */
export interface SalesReturn {
  id: number;
  return_no: string;
  sales_order_id?: number;
  customer_id: number;
  return_date: string;
  warehouse_id: number;
  /** 后端由 reason_type 与 reason_detail 组合落库（"{type}: {detail}"） */
  reason: string;
  /** 词表 sales_return：DRAFT/SUBMITTED/APPROVED/REJECTED/COMPLETED */
  status: string;
  total_amount: number;
  remarks?: string;
  approved_by?: number;
  approved_at?: string;
  rejected_reason?: string;
  created_by: number;
  created_at: string;
  updated_at: string;
  customer_name: string | null;
  sales_order_no: string | null;
}

/**
 * 退货明细读模型：键名与 backend models/sales_return_item.rs 的实体字段逐一对应
 * （列均 NOT NULL，故不标可选；notes 在后端是 Option<String>）。
 * product_name / product_code 由 list_return_items 的 product LEFT JOIN 产出，
 * 用于表单编辑态与详情回显产品名称/编号（实体只有 product_id）。
 */
export interface SalesReturnItem {
  id: number;
  return_id: number;
  line_no: number;
  product_id: number;
  quantity: number;
  quantity_alt: number;
  unit_price: number;
  unit_price_foreign: number;
  discount_percent: number;
  tax_percent: number;
  subtotal: number;
  tax_amount: number;
  discount_amount: number;
  total_amount: number;
  /** 明细自由文本（后端 notes 列） */
  notes?: string;
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
  created_at: string;
  updated_at: string;
  /** 以下两列由 list_return_items 的 product LEFT JOIN 产出，产品行缺失时为 null */
  product_name: string | null;
  product_code: string | null;
}

/** 新增明细的写入契约（CreateSalesReturnItemRequest）；回源读取用 notes，故与读模型分开。 */
export interface SalesReturnItemInput {
  product_id: number;
  quantity: number;
  unit_price: number;
  tax_percent?: number;
  discount_percent?: number;
  reason?: string;
}

/** 更新明细的写入契约（UpdateReturnItemRequest）：后端仅接受这三个字段。 */
export interface SalesReturnItemUpdate {
  quantity?: number;
  unit_price?: number;
  reason?: string;
}

/**
 * 创建退货单表头请求（对齐 backend CreateSalesReturnRequest）。
 * 不含 items / status / total_amount：表头 total_amount 由服务端维护，
 * 明细通过 POST /sales-returns/{id}/items 单独提交。
 */
export interface CreateSalesReturnRequest {
  order_id?: number;
  customer_id: number;
  return_date: string;
  warehouse_id: number;
  reason_type: string;
  reason_detail?: string;
  notes?: string;
}

/** 更新退货单表头请求（对齐 backend UpdateSalesReturnRequest，字段全可选）。 */
export interface UpdateSalesReturnRequest {
  order_id?: number;
  customer_id?: number;
  return_date?: string;
  warehouse_id?: number;
  reason_type?: string;
  reason_detail?: string;
  notes?: string;
}

/** 新增退货明细请求（对齐 backend CreateSalesReturnItemRequest）。 */
export interface CreateSalesReturnItemRequest {
  line_no?: number;
  product_id: number;
  quantity: number;
  unit_price: number;
  tax_percent?: number;
  discount_percent?: number;
  reason?: string;
}

/** 后端 handlers/sales_return_handler.rs SalesReturnQueryParams 的字段集，逐一对应。 */
export interface SalesReturnQueryParams {
  return_no?: string;
  status?: string;
  customer_id?: number;
  page?: number;
  page_size?: number;
}

// D14 Batch 5b：原 salesReturnApi.list 转为风格 B 函数
export const getSalesReturnList = (params?: SalesReturnQueryParams) =>
  request.get<ApiResponse<PaginatedResponse<SalesReturn>>>('/sales/sales-returns', { params });

// 表头请求体严格为后端 CreateSalesReturnRequest 字段集（不含 items/status/total_amount）。
export const createSalesReturn = (data: CreateSalesReturnRequest) =>
  request.post<ApiResponse<SalesReturn>>('/sales/sales-returns', data);

// D14 Batch 5b：原 salesReturnApi.getById 转为风格 B 函数
export const getSalesReturnById = (id: number) =>
  request.get<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}`);

// 更新表头请求体严格为后端 UpdateSalesReturnRequest（字段全可选）。
export const updateSalesReturn = (id: number, data: UpdateSalesReturnRequest) =>
  request.put<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}`, data);

// D14 Batch 5b：原 salesReturnApi.delete 转为风格 B 函数
export const deleteSalesReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/sales/sales-returns/${id}`);

// D14 Batch 5b：原 salesReturnApi.submit 转为风格 B 函数
export const submitSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}/submit`);

// D14 Batch 5b：原 salesReturnApi.approve 转为风格 B 函数
export const approveSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}/approve`);

// D14 Batch 5b：原 salesReturnApi.reject 转为风格 B 函数
export const rejectSalesReturn = (id: number, reason?: string) =>
  request.post<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}/reject`, { reason });

// D14 Batch 5b：原 salesReturnApi.execute 转为风格 B 函数
export const executeSalesReturn = (id: number) =>
  request.post<ApiResponse<SalesReturn>>(`/sales/sales-returns/${id}/execute`);

// D14 Batch 5b：原 salesReturnApi.listItems 转为风格 B 函数
/** 后端 sales_return_handler::list_return_items 返回 ApiResponse<Vec<Model>>（裸数组，无信封） */
export const getSalesReturnItemList = (id: number) =>
  request.get<ApiResponse<SalesReturnItem[]>>(`/sales/sales-returns/${id}/items`);

// 新增明细：请求体为 CreateSalesReturnItemRequest（表头 items 端点，独立于表头创建）。
export const createSalesReturnItem = (id: number, data: CreateSalesReturnItemRequest) =>
  request.post<ApiResponse<SalesReturnItem>>(`/sales/sales-returns/${id}/items`, data);

// D14 Batch 5b：原 salesReturnApi.updateItem 转为风格 B 函数
export const updateSalesReturnItem = (id: number, itemId: number, data: SalesReturnItemUpdate) =>
  request.put<ApiResponse<SalesReturnItem>>(`/sales/sales-returns/${id}/items/${itemId}`, data);

// D14 Batch 5b：原 salesReturnApi.deleteItem 转为风格 B 函数
export const deleteSalesReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/sales/sales-returns/${id}/items/${itemId}`);
