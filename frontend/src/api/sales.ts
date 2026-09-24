import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface SalesOrder {
  id: number;
  order_no: string;
  customer_id: number;
  customer_name: string;
  order_date: string;
  required_date?: string;
  status: string;
  total_amount: number;
  tax_amount?: number;
  discount_amount?: number;
  /** 后端 SalesOrderDetail 未返回此键（services/so/mod.rs:43 仅 created_by），需后端补进详情 DTO */
  contact_person: string | null;
  /** 后端 SalesOrderDetail 未返回此键（services/so/mod.rs:43 仅 created_by），需后端补进详情 DTO */
  contact_phone: string | null;
  delivery_address?: string;
  /** 后端 sales_orders.shipping_address（收货地址快照） */
  shipping_address?: string;
  /** 后端 sales_orders.notes（备注） */
  notes?: string;
  /** 后端 SalesOrderDetail 未返回创建人名称（services/so/mod.rs:43 仅 created_by id），需后端 JOIN users 补 creator_name */
  creator_name: string | null;
  created_at?: string;
  updated_at?: string;
  items: SalesOrderItem[];
}

export interface SalesOrderItem {
  id: number;
  product_id: number;
  product_name: string;
  product_code: string;
  /** 色号：后端 sales_order_items.color_no（String）。空串=白坯布，非空=染色布 */
  color_no?: string;
  /** 后端 SalesOrderItemDetail.dye_lot_requirement（services/so/mod.rs:109） */
  dye_lot_requirement: string | null;
  quantity: number;
  /** 后端 SalesOrderItemDetail 无 unit 键（services/so/mod.rs:80 至 :116 全字段核对），需后端补 unit */
  unit: string | null;
  unit_price: number;
  tax_rate?: number;
  tax_amount?: number;
  discount_rate?: number;
  discount_amount?: number;
  subtotal: number;
  /** 后端 SalesOrderItemDetail.shipped_quantity（services/so/mod.rs:94），非 delivered_quantity */
  shipped_quantity: number;
  /** 后端 sales_order_items.quantity_tolerance_pct（可空，NULL=用品类/全局默认） */
  quantity_tolerance_pct: number | null;
}

export interface SalesOrderQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  customer_id?: number;
  status?: string;
  order_date_from?: string;
  order_date_to?: string;
}

export interface SalesDelivery {
  id: number;
  delivery_no: string;
  order_id: number;
  order_no: string;
  customer_id: number;
  customer_name: string;
  delivery_date: string;
  warehouse_id?: number;
  /** 与后端 models/status/sales.rs 的 sales_delivery 常量同源（该表无 draft/delivered） */
  status: 'pending' | 'shipped' | 'cancelled';
  items: SalesDeliveryItem[];
  remark?: string;
  created_at?: string;
}

export interface SalesDeliveryItem {
  id?: number;
  delivery_id?: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  quantity: number;
  unit?: string;
  remark?: string;
}

export interface SalesDeliveryQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  order_id?: number;
  status?: string;
  delivery_date_from?: string;
  delivery_date_to?: string;
}

export interface SalesStatisticsParams {
  date_from?: string;
  date_to?: string;
  group_by?: 'day' | 'week' | 'month';
  customer_id?: number;
}

export interface SalesStatisticsData {
  total_amount: number;
  total_orders: number;
  total_customers: number;
  trends: { date: string; amount: number; orders: number }[];
}

// D14 Batch 5b：原 salesApi.getOrderList 转为风格 B 函数
export const getSalesOrderList = (params?: SalesOrderQueryParams) =>
  request.get<ApiResponse<{ items: SalesOrder[]; total: number }>>('/sales/orders', {
    params,
  });

// D14 Batch 5b：原 salesApi.getOrderById 转为风格 B 函数
export const getSalesOrderById = (id: number) =>
  request.get<ApiResponse<SalesOrder>>(`/sales/orders/${id}`);

// D14 Batch 5b：原 salesApi.createOrder 转为风格 B 函数
export const createSalesOrder = (data: Partial<SalesOrder>) =>
  request.post<ApiResponse<SalesOrder>>('/sales/orders', data);

// D14 Batch 5b：原 salesApi.updateOrder 转为风格 B 函数
export const updateSalesOrder = (id: number, data: Partial<SalesOrder>) =>
  request.put<ApiResponse<SalesOrder>>(`/sales/orders/${id}`, data);

// D14 Batch 5b：原 salesApi.deleteOrder 转为风格 B 函数
export const deleteSalesOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/sales/orders/${id}`);

// D14 Batch 5b：原 salesApi.submitOrder 转为风格 B 函数
export const submitSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/submit`);

// D14 Batch 5b：原 salesApi.approveOrder 转为风格 B 函数
export const approveSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/approve`);

// D14 Batch 5b：原 salesApi.rejectOrder 转为风格 B 函数
export const rejectSalesOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/reject`, { reason });

// D14 Batch 5b：原 salesApi.cancelOrder 转为风格 B 函数
export const cancelSalesOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/sales/orders/${id}/cancel`);

// D14 Batch 5b：原 salesApi.createDelivery 转为风格 B 函数
export const createSalesDelivery = (orderId: number, data: Partial<SalesDelivery>) =>
  request.post<ApiResponse<SalesDelivery>>(`/sales/orders/${orderId}/deliveries`, data);

// D14 Batch 5b：原 salesApi.getDeliveries 转为风格 B 函数
export const getSalesDeliveryList = (orderId: number) =>
  request.get<ApiResponse<{ list: SalesDelivery[]; total: number }>>(
    `/sales/orders/${orderId}/deliveries`
  );

/**
 * 销售发货出库（真实扣减库存）——与后端 ShipOrderRequest/ShipOrderItemRequest 同构
 * （backend/src/services/so/delivery.rs）。
 * 出库按"款号(product_id)+色号+缸号+批次"四维匹配扣减：三个维度必填，
 * 指定缸号数量不足时后端才走显式跨缸回退，并把实际扣减缸号记入出库明细/流水。
 */
export interface SalesShipItem {
  product_id: number;
  quantity: number;
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
  piece_no?: string;
}

export interface SalesShipPayload {
  order_id: number;
  warehouse_code: string;
  items: SalesShipItem[];
  remarks?: string;
}

export const shipSalesOrder = (orderId: number, data: SalesShipPayload) =>
  request.post<ApiResponse<null>>(`/sales/orders/${orderId}/ship`, data);

// D14 Batch 5b：原 salesApi.getOrderStatistics 转为风格 B 函数
export const getSalesOrderStatistics = (params: SalesStatisticsParams) =>
  request.get<ApiResponse<SalesStatisticsData>>('/sales/orders/statistics', { params });

/**
 * 生成销售订单号（P1-1 补齐 generate-no 端点）
 * 后端: GET /api/v1/erp/sales/orders/generate-no
 * 返回: { prefix: "SO", order_no: "SO20260617001" }
 */
// D14 Batch 5b：原 salesApi.generateOrderNo 转为风格 B 函数
export const generateSalesOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/sales/orders/generate-no');
