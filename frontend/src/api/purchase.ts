import { request } from './request';
import type { ApiResponse } from '@/types/api';
// 创建入库单契约与孪生实现同源（purchase-receipt.ts 已逐字段对齐后端 DTO），此处只引类型不复写。
import type { CreatePurchaseReceiptRequest } from './purchase-receipt';

// 列表/详情出参 = 后端 PurchaseOrderDto（services/po/order.rs:19，handler purchase_order_handler.rs:26）。
// DTO 键以 snake_case 原样序列化，order_status 经 #[serde(rename="status")] 改名；
// supplier_name/warehouse_name/department_name 为 LEFT-JOIN 列，DTO 里是 Option ⇒ string | null。
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
   * 后端 PurchaseOrderDto.attachment_urls（purchase_orders.attachment_urls TEXT[]
   * 真实列，键名与模型列同源 snake_case）：创建/更新落库后详情/列表原样回显，
   * 未上传附件时为 null。
   */
  attachment_urls: string[] | null;
  created_by: number;
  created_at: string;
  updated_at: string;
  /**
   * 需要后端 JOIN：PurchaseOrderDto 仅有 created_by 数值，无创建人姓名
   * （backend/src/services/po/order.rs:19）。列已保留，待后端补 created_by -> users 名称。
   */
  creator_name?: string | null;
  /**
   * 需要后端聚合：PurchaseOrderDto 无 received_amount 字段
   * （backend/src/services/po/order.rs:19）。列已保留，待后端补该汇总。
   */
  received_amount?: number | null;
  /**
   * 需要后端字段：PurchaseOrderDto 无 payment_status（backend/src/services/po/order.rs:19）。
   * 列已保留，待后端确定付款状态来源与词表。
   */
  payment_status?: string | null;
  items: PurchaseOrderItem[];
}

// 详情明细 = 后端 get_order 直接序列化的 purchase_order_item::Model
// （handlers/purchase_order_handler.rs:116），键为实体 snake_case；
// 实体只有 product_id，无产品名/编码，需后端 JOIN products 补全（列已保留）。
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
  /**
   * 换算后辅助单位数量（面料米/公斤双计量）。后端 PurchaseOrderItemDto.quantity_alt
   * 为 rust_decimal::Decimal（非 Option，见 services/po/order.rs:70），经 JSON 序列化为**字符串**
   * （如 "120.500000"）。消费处须 Number() 归一后再参与运算/.toFixed，禁止直接对字符串 .toFixed。
   */
  quantity_alt: string;
  /**
   * 后端 purchase_order_item.quantity_tolerance_pct（可空，NULL=用品类/全局默认）。
   * 后端 rust_decimal 经 JSON 序列化为字符串（如 "5.00"），NULL→null；
   * 本接口同时用于创建入参 items（useCreate.submitCreate 提交数值），故取并集 number | string | null。
   * 消费点：详情展示 PurchaseViewDialog 需 Number() 归一。
   */
  quantity_tolerance_pct: number | string | null;
  /**
   * 折扣率（百分比）：后端 PurchaseOrderItemDto.discount_percent 为 rust_decimal::Decimal
   * 非 Option（services/po/order.rs），JSON 序列化为字符串；消费处须 Number() 归一。
   */
  discount_percent: string;
  /** 折扣金额（本位币）：同上，DTO 非 Option Decimal → 字符串。 */
  discount_amount: string;
  /**
   * 色号：DB 列 purchase_order_item.color_code（可空）。
   * 注意创建入参侧键名是 color_no（CreateOrderItemRequest，services/po/mod.rs:125），
   * 服务层落库时写入 color_code（order_ops/crud.rs:317），读写键名不同属既有口径。
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
 * GET /purchase/receipts 查询参数 —— 逐键对齐后端 handlers/purchase_receipt_handler.rs:376-382
 * ReceiptQueryParams（list_receipts :26-40 仅透传这五键），无 rename_all → snake_case。
 * keyword / warehouse_id / receipt_date_from / receipt_date_to 后端根本不接收
 * （serde 反序列化后无消费点，发出即整体丢弃，筛选恒无效）——已从前端契约摘除，
 * 不再伪装可筛；该四项属后端列表功能缺口，修复派单见交付报告，禁止用前端本地过滤假装有效。
 */
export interface PurchaseReceiptQueryParams {
  page?: number;
  page_size?: number;
  status?: string;
  supplier_id?: number;
  order_id?: number;
}

// D14 Batch 5b：原 purchaseApi.getOrderList 转为风格 B 函数
export const getPurchaseOrderList = (params?: PurchaseOrderQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseOrder[]; total: number }>>('/purchase/orders', {
    params,
  });

// D14 Batch 5b：原 purchaseApi.getOrderById 转为风格 B 函数
export const getPurchaseOrderById = (id: number) =>
  request.get<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`);

// 创建采购订单请求体：严格对齐后端 CreatePurchaseOrderRequest
// （backend/src/services/po/mod.rs:31）与明细 CreateOrderItemRequest（:96）。
// 契约以实体/DTO snake_case 为准：明细用 material_id（=产品 ID）/quantity_ordered，
// 表头用 expected_delivery_date/notes；服务层 validate_order_request
// （services/po/order_ops/crud.rs:137/:149）强制 warehouse_id、department_id 非空且真实存在。
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
  /** 转采购来源销售订单 ID（后端据此触发 SKU 对照翻译，services/po/mod.rs:73） */
  source_sales_order_id?: number;
  /** 资质例外放行原因（后端 Option<String>，validator 保证传入即非空白，mod.rs:75-85） */
  qualification_waiver_reason?: string;
  items: CreatePurchaseOrderItemPayload[];
}

// 更新明细请求体：逐字段对齐后端 UpdateOrderItemRequest
// （backend/src/services/po/mod.rs:154-179，写侧字段已与 CreateOrderItemRequest 补齐同源）。
// 「缺省即不改」语义：省略的键保持库中原值（Option::None）；
// 色号写侧键名 color_no、回读键为 DB 列 color_code（与创建口径一致）；
// supplier_* 保密快照列**不在**本请求字段中——普通行直写 color_code 不反查
// （与创建路径普通单同口径），仅转采购行由后端按 color_no 走与创建路径同一套
// 映射反查权威逻辑刷新快照，前端禁止直写、伪造该两键只会被 serde 忽略。
// 数值键提交 number（后端 rust_decimal 双形态接受）；回读出参 Decimal 为字符串，
// 展示处须 Number() 归一后再格式化。
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

// D14 Batch 5b：原 purchaseApi.createOrder 转为风格 B 函数
export const createPurchaseOrder = (data: CreatePurchaseOrderPayload) =>
  request.post<ApiResponse<PurchaseOrder>>('/purchase/orders', data);

// D14 Batch 5b：原 purchaseApi.updateOrder 转为风格 B 函数
export const updatePurchaseOrder = (id: number, data: Partial<PurchaseOrder>) =>
  request.put<ApiResponse<PurchaseOrder>>(`/purchase/orders/${id}`, data);

// 更新单行采购明细：PUT /purchase/orders/{orderId}/items/{itemId}
// （routes/purchase.rs:76-80 → handlers/purchase_order_handler.rs::update_order_item，
// 仅 DRAFT 订单且创建人本人可改；出参为 purchase_order_item::Model 序列化，
// Decimal 键为字符串，消费处 Number() 归一）
export const updatePurchaseOrderItem = (
  orderId: number,
  itemId: number,
  data: UpdatePurchaseOrderItemPayload
) =>
  request.put<ApiResponse<PurchaseOrderItem>>(`/purchase/orders/${orderId}/items/${itemId}`, data);

// D14 Batch 5b：原 purchaseApi.deleteOrder 转为风格 B 函数
export const deletePurchaseOrder = (id: number) =>
  request.delete<ApiResponse<null>>(`/purchase/orders/${id}`);

// D14 Batch 5b：原 purchaseApi.submitOrder 转为风格 B 函数
export const submitPurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/submit`);

// D14 Batch 5b：原 purchaseApi.approveOrder 转为风格 B 函数
export const approvePurchaseOrder = (id: number) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/approve`);

// D14 Batch 5b：原 purchaseApi.rejectOrder 转为风格 B 函数
export const rejectPurchaseOrder = (id: number, reason: string) =>
  request.post<ApiResponse<null>>(`/purchase/orders/${id}/reject`, { reason });

// D14 Batch 5b：原 purchaseApi.getReceipts 转为风格 B 函数
export const getPurchaseReceiptList = (params?: PurchaseReceiptQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReceipt[]; total: number }>>('/purchase/receipts', {
    params,
  });

// D14 Batch 5b：原 purchaseApi.createReceipt 转为风格 B 函数
// 后端 body 是 Json<CreatePurchaseReceiptRequest>（purchase_receipt_handler.rs:119-123 →
// services/purchase_receipt_dto.rs:51-79），仅读取 order_id/supplier_id/receipt_date/warehouse_id/
// department_id/inspector_id/notes/attachment_urls/items（明细键集见 CreateReceiptItemRequest :127-193）。
// 此前以 Partial<PurchaseReceipt>（响应模型）作入参，把 id/receipt_no/order_no/supplier_name/
// status/created_at 等响应侧或生成列冒充为可写键提交——单据号是服务端生成、禁手输
// （同族契约先例：backend/src/handlers/production_order_handler.rs 的 generate-no 流程），
// 多发键被 serde 静默丢弃即契约漂移，且掩盖了明细键名不符（product_id vs material_id 等）的真缺陷。
// 现直接引用与孪生实现（purchase-receipt.ts createPurchaseReceipt）同一份逐 DTO 对齐的请求类型。
export const createPurchaseReceipt = (data: CreatePurchaseReceiptRequest) =>
  request.post<ApiResponse<PurchaseReceipt>>('/purchase/receipts', data);

// D14 Batch 5b：原 purchaseApi.receiveItems 转为风格 B 函数
export const receivePurchaseItems = (receiptId: number, data: Partial<PurchaseReceiptItem>[]) =>
  request.post<ApiResponse<PurchaseReceipt>>(`/purchase/receipts/${receiptId}/receive`, data);

/**
 * 生成采购订单号（P1-1 补齐 generate-no 端点）
 * 后端: GET /api/v1/erp/purchase/orders/generate-no
 * 返回: { prefix: "PO", order_no: "PO20260617001" }
 */
// D14 Batch 5b：原 purchaseApi.generateOrderNo 转为风格 B 函数
export const generatePurchaseOrderNo = () =>
  request.get<ApiResponse<{ prefix: string; order_no: string }>>('/purchase/orders/generate-no');
