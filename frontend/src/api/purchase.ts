import { request } from './request';
import type { ApiResponse } from '@/types/api';
// 创建入库单请求类型与 purchase-receipt.ts 同源复用，不复写。
import type { CreatePurchaseReceiptRequest } from './purchase-receipt';

// 列表/详情出参 = 后端 PurchaseOrderDto：键按实体 snake_case 序列化（order_status 出参键为 status）；
// supplier_name/warehouse_name/department_name 为 LEFT JOIN 列（Option ⇒ string | null）。
export interface PurchaseOrder {
  id: number;
  order_no: string;
  supplier_id: number;
  supplier_name: string | null;
  order_date: string;
  expected_delivery_date: string | null;
  actual_delivery_date: string | null;
  warehouse_id: number;
  warehouse_name: string | null;
  department_id: number;
  department_name: string | null;
  purchaser_id: number;
  currency: string;
  exchange_rate: number;
  total_amount: number;
  total_amount_foreign: number;
  total_quantity: number;
  total_quantity_alt: number;
  status: string;
  payment_terms: string | null;
  shipping_terms: string | null;
  /** 后端 PurchaseOrderDto.notes（备注） */
  notes: string | null;
  /**
   * 审批理由两键（后端 PurchaseOrderDto 补键中，与报价单域同范式）：DB 真实列
   * purchase_orders.approval_reason / purchase_orders.rejected_reason 均 Option<String>
   * （models/purchase_order.rs:108,111），出参键恒存在、无值即 null。
   * 前端如实声明 string | null（不给「缺键」留 undefined 空间），回显只读原文，
   * 禁止 `?? ''` 之类兜底把「字段缺失」与「无值」混成一谈。
   */
  approval_reason: string | null;
  rejected_reason: string | null;
  /** 后端 purchase_orders.attachment_urls 真实列（TEXT[]）：详情/列表原样回显，未上传附件时为 null */
  attachment_urls: string[] | null;
  created_by: number;
  created_at: string;
  updated_at: string;
  /** 需要后端 JOIN：PurchaseOrderDto 仅有 created_by 数值，无创建人姓名列 */
  creator_name?: string | null;
  /** 需要后端聚合：PurchaseOrderDto 无 received_amount 汇总字段 */
  received_amount?: number | null;
  /** 需要后端字段：PurchaseOrderDto 无 payment_status，付款状态来源与词表未定 */
  payment_status?: string | null;
  items: PurchaseOrderItem[];
}

// 详情明细 = 后端 get_order 序列化的 purchase_order_item::Model，键为实体 snake_case；
// 实体只有 product_id，产品名/编码需后端 JOIN products 补全。
export interface PurchaseOrderItem {
  id: number;
  order_id: number;
  line_no: number;
  product_id: number;
  /** 需要后端 JOIN：purchase_order_item.product_id -> products.product_name（Model 无名列） */
  product_name?: string | null;
  /** 需要后端 JOIN：purchase_order_item.product_id -> products.product_code（Model 无编码列） */
  product_code?: string | null;
  quantity: number;
  unit_price: number;
  subtotal: number;
  tax_amount: number;
  total_amount: number;
  received_quantity: number;
  /** 换算后辅助单位数量（面料米/公斤双计量）：后端 Decimal 非 Option ⇒ 出参为字符串；消费处须 Number() 归一，禁止对字符串直接 .toFixed */
  quantity_alt: string;
  /**
   * 行级交货允差 %：后端可空（NULL=用品类/全局默认）；出参 Decimal 为字符串、入参侧提交数值，
   * 故取并集 number | string | null；展示须 Number() 归一。
   */
  quantity_tolerance_pct: number | string | null;
  /** 折扣率（百分比）：后端 Decimal 非 Option ⇒ 出参字符串，消费处 Number() 归一 */
  discount_percent: string;
  /** 折扣金额（本位币）：同上，Decimal ⇒ 字符串 */
  discount_amount: string;
  /**
   * 色号：DB 列 purchase_order_item.color_code（可空）。
   * 创建入参侧键名是 color_no，服务层落库写入 color_code——读写键名不同属既有口径。
   */
  color_code: string | null;
  /** 后端 purchase_order_item.notes（备注） */
  notes?: string | null;
}

export interface PurchaseOrderQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_id?: number;
  status?: string;
  order_date_from?: string;
  order_date_to?: string;
}

export interface PurchaseReceipt {
  id: number;
  receipt_no: string;
  order_id: number;
  order_no: string;
  supplier_id: number;
  supplier_name: string;
  receipt_date: string;
  warehouse_id?: number;
  status: 'draft' | 'pending' | 'completed';
  items: PurchaseReceiptItem[];
  remark?: string;
  created_at?: string;
}

export interface PurchaseReceiptItem {
  id?: number;
  receipt_id?: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  expected_quantity?: number;
  received_quantity: number;
  unit?: string;
  remark?: string;
}

/**
 * GET /purchase/receipts 查询参数 —— 逐键对齐后端 ReceiptQueryParams（snake_case，全可选）。
 * 后端另有 keyword/warehouse_id/receipt_date_from/receipt_date_to 四个筛选键，
 * 当前本仓无调用方使用 ⇒ 未纳入本契约。
 */
export interface PurchaseReceiptQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  supplier_id?: number;
  order_id?: number;
}

export const getPurchaseOrderList = (params?: PurchaseOrderQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseOrder[]; total: number }>>('/purchase/orders', {
    params,
  });

export const getPurchaseOrderById = (id: number) =>
  request.get<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`);

// 创建采购订单请求体：逐字段对齐后端 CreatePurchaseOrderRequest 与明细 CreateOrderItemRequest。
// 契约以实体/DTO snake_case 为准：明细用 material_id（=产品 ID）/quantity_ordered，
// 表头用 expected_delivery_date/notes；后端校验强制 warehouse_id、department_id 非空且真实存在。
export interface CreatePurchaseOrderItemPayload {
  line_no?: number;
  material_id?: number;
  quantity_ordered?: number;
  quantity_alt_ordered?: number;
  unit_price?: number;
  tax_rate?: number;
  discount_percent?: number;
  quantity_tolerance_pct?: number;
  color_no?: string;
  notes?: string;
}

export interface CreatePurchaseOrderPayload {
  supplier_id: number;
  order_date: string;
  expected_delivery_date?: string;
  warehouse_id: number;
  department_id: number;
  currency?: string;
  exchange_rate?: number;
  payment_terms?: string;
  shipping_terms?: string;
  notes?: string;
  /** 后端 CreatePurchaseOrderRequest.attachment_urls（Option<Vec<String>>） */
  attachment_urls?: string[];
  /** 转采购来源销售订单 ID（后端据此触发 SKU 对照翻译） */
  source_sales_order_id?: number;
  /** 资质例外放行原因（后端 Option<String>，校验保证传入即非空白） */
  qualification_waiver_reason?: string;
  items: CreatePurchaseOrderItemPayload[];
}

// 更新明细请求体：逐字段对齐后端 UpdateOrderItemRequest（写侧字段与 CreateOrderItemRequest 同源）。
// 「缺省即不改」语义：省略的键保持库中原值；色号写侧键名 color_no、回读键为 DB 列 color_code。
// supplier_* 保密快照列**不在**本请求字段中，前端禁止直写（转采购行由后端按 color_no 反查权威映射刷新）。
// 数值键提交 number（后端 rust_decimal 双形态接受）；回读出参 Decimal 为字符串，展示处须 Number() 归一。
export interface UpdatePurchaseOrderItemPayload {
  material_id?: number;
  unit_price?: number;
  quantity_ordered?: number;
  quantity_alt_ordered?: number;
  tax_rate?: number;
  discount_percent?: number;
  /** 行级交货允差 %：后端复用 validate_quantity_tolerance_pct 校验 0~100，越界以 BUSINESS_ERROR 外显拒绝 */
  quantity_tolerance_pct?: number;
  color_no?: string;
  notes?: string;
}

export const createPurchaseOrder = (data: CreatePurchaseOrderPayload) =>
  request.post<ApiResponse<PurchaseOrder>>('/purchase/orders', data);

export const updatePurchaseOrder = (id: number, data: Partial<PurchaseOrder>) =>
  request.put<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`, data);

// 更新单行采购明细：PUT /purchase/orders/{orderId}/items/{itemId}；
// 仅 DRAFT 订单且创建人本人可改；出参 Decimal 键为字符串，消费处 Number() 归一。
export const updatePurchaseOrderItem = (
  orderId: number,
  itemId: number,
  data: UpdatePurchaseOrderItemPayload
) =>
  request.put<ApiResponse<PurchaseOrderItem>>(`/purchase/orders/${orderId}/items/${itemId}`, data);

export const deletePurchaseOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/orders/${id}`);

export const submitPurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/submit`);

// 审批「通过」：POST /purchase/orders/{id}/approve。approval_reason 选填：
// 留空即省略该键，purchase_orders.approval_reason 列写 NULL（不伪造成必填、不落空串）。
// 采集见 useActionPrompts.promptApprovalReason(false)。
export const approvePurchaseOrder = (id: number, approvalReason?: string) =>
  request.post<ApiResponse<null>>(
    `/purchase/orders/${id}/approve`,
    approvalReason ? { approval_reason: approvalReason } : {}
  );

export const rejectPurchaseOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/reject`, { reason });

export const getPurchaseReceiptList = (params?: PurchaseReceiptQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReceipt[]; total: number }>>('/purchase/receipts', {
    params,
  });

// 创建入库单：请求体逐字段对齐后端 CreatePurchaseReceiptRequest（类型与孪生实现 purchase-receipt.ts 同源）。
// 单据号由服务端生成、禁手输；响应侧键（id/receipt_no/order_no/supplier_name/status 等）不属请求契约。
export const createPurchaseReceipt = (data: CreatePurchaseReceiptRequest) =>
  request.post<ApiResponse<PurchaseReceipt>>('/purchase/receipts', data);

export const receivePurchaseItems = (receiptId: number, data: Partial<PurchaseReceiptItem>[]) =>
  request.post<ApiResponse<PurchaseReceipt>>(`/purchase/receipts/${receiptId}/receive`, data);

/**
 * 生成采购订单号（创建表单预填用）
 * 后端: GET /api/v1/erp/purchase/orders/generate-no
 * 返回: { prefix: "PO", order_no: "PO20260617001" }
 */
export const generatePurchaseOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/purchase/orders/generate-no');
