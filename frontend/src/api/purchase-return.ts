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
  /** get_purchase_return 仅返回单条 Model、不含明细；明细需另调 /returns/:id/items */
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
  /**
   * 面料追溯维度（审批按 产品+色号+缸号+批次 四维精确定位库存行扣减）。
   * 后端 PurchaseReturnItemDto（services/purchase_return_service.rs:994）已随 list_items 的
   * 全列 SELECT + into_model 输出这三列（非空 String，白坯/单库存行为空串），故此处真实回读。
   */
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
}

// 列表查询参数 = 后端 ReturnQueryParams（handlers/purchase_return_handler.rs:206）。
// 真实字段：page/page_size/status/supplier_id/keyword/start_date/end_date，
// 过滤实现见 services/purchase_return_service.rs:577-591（keyword LIKE return_no，日期为 return_date 闭区间）。
export interface PurchaseReturnQueryParams {
  page?: number;
  page_size?: number;
  supplier_id?: number;
  status?: string;
  /** 后端按 return_no 模糊匹配 */
  keyword?: string;
  /** 退货日期下界（含），后端 parse_date_bound 容受 ISO 与 YYYY-MM-DD */
  start_date?: string;
  /** 退货日期上界（含） */
  end_date?: string;
}

/**
 * 创建采购退货单请求（严格对齐 backend CreatePurchaseReturnRequest，
 * services/purchase_return_service.rs:570）。
 * 不含 items/return_no/return_status/total_*: 表头 total_* 由服务端汇总明细时写入。
 */
export interface CreatePurchaseReturnPayload {
  receipt_id?: number;
  order_id?: number;
  supplier_id: number;
  return_date: string;
  warehouse_id?: number;
  department_id?: number;
  reason_type: string;
  reason_detail?: string;
  notes?: string;
}

/**
 * 更新采购退货单请求（严格对齐 backend UpdatePurchaseReturnRequest，
 * services/purchase_return_service.rs）。仅 reason_type/reason_detail/notes 三字段可更新，
 * 且三者均为 DB 可空列（purchase_return，m0009 DDL）。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖
 * ——清空须显式送 null，禁止塌成 `|| undefined`（省略=保持原值）。
 */
export interface UpdatePurchaseReturnPayload {
  reason_type?: string | null;
  reason_detail?: string | null;
  notes?: string | null;
}

/**
 * 创建退货明细请求（严格对齐 backend CreateReturnItemRequest，
 * services/purchase_return_service.rs:608）。
 */
export interface CreatePurchaseReturnItemPayload {
  line_no: number;
  material_id: number;
  quantity_ordered?: number;
  quantity_returned: number;
  unit_price: number;
  tax_rate?: number;
  discount_percent?: number;
  notes?: string;
  /** 面料追溯维度；空串=白坯/单库存行（对齐后端 CreateReturnItemRequest Option<String>） */
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
}

/**
 * 更新退货明细请求（严格对齐 backend UpdateReturnItemRequest，
 * services/purchase_return_service.rs）。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空、有值=覆盖。
 * - line_no/material_id/quantity_returned/unit_price/tax_rate/discount_percent 及追溯列
 *   color_no/dye_lot_no/batch_no（NOT NULL DEFAULT ''，见 models/purchase_return_item.rs 头注）
 *   均为 NOT NULL：禁止送 null（后端 400「XX不能清空：该字段为必填项」）；
 *   "追溯维度改回白坯/空值"提交空串而非 null；
 * - notes 为 DB 可空列：清空须显式送 null。
 */
export interface UpdatePurchaseReturnItemPayload {
  line_no?: number;
  material_id?: number;
  quantity_returned?: number;
  unit_price?: number;
  tax_rate?: number;
  discount_percent?: number;
  notes?: string | null;
  color_no?: string;
  dye_lot_no?: string;
  batch_no?: string;
}

// D14 Batch 5b：原 purchaseReturnApi.list 转为风格 B 函数
export const getPurchaseReturnList = (params?: PurchaseReturnQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReturn[]; total: number }>>('/purchase/returns', {
    params,
  });

// 创建：请求体严格为 CreatePurchaseReturnPayload（对齐后端 CreatePurchaseReturnRequest）。
export const createPurchaseReturn = (data: CreatePurchaseReturnPayload) =>
  request.post<ApiResponse<PurchaseReturn>>('/purchase/returns', data);

// D14 Batch 5b：原 purchaseReturnApi.getById 转为风格 B 函数
export const getPurchaseReturnById = (id: number) =>
  request.get<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`);

// 更新：请求体严格为 UpdatePurchaseReturnPayload（对齐后端 UpdatePurchaseReturnRequest）。
export const updatePurchaseReturn = (id: number, data: UpdatePurchaseReturnPayload) =>
  request.put<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`, data);

// D14 Batch 5b：原 purchaseReturnApi.delete 转为风格 B 函数
export const deletePurchaseReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}`);

// D14 Batch 5b：原 purchaseReturnApi.submit 转为风格 B 函数
export const submitPurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/submit`);

// 审批「通过」：POST /purchase/returns/{id}/approve，体 purchase_return_handler::ApproveReturnRequest
// （approval_reason 为 Option<String> ⇒ 选填：留空即省略该键，后端归一为 None，purchase_return.approval_reason
// 专列保持 NULL，不伪造成必填、不落空串）。采集见 useActionPrompts.promptApprovalReason(false)。
export const approvePurchaseReturn = (id: number, approvalReason?: string) =>
  request.post<ApiResponse<PurchaseReturn>>(
    `/purchase/returns/${id}/approve`,
    approvalReason ? { approval_reason: approvalReason } : {}
  );

// 审批「拒绝」：reason 后端为强类型 Json<RejectReturnRequest>（String 非 Option）且服务端 trim 非空必填
// ⇒ 必带体、必发非空 reason（不得标 `?` 掩盖必填）。采集见 useActionPrompts.promptRejectReason()。
export const rejectPurchaseReturn = (id: number, reason: string) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/reject`, { reason });

// D14 Batch 5b：原 purchaseReturnApi.listItems 转为风格 B 函数
export const getPurchaseReturnItemList = (id: number) =>
  request.get<ApiResponse<PurchaseReturnItem[]>>(`/purchase/returns/${id}/items`);

// 创建明细：请求体严格为 CreatePurchaseReturnItemPayload（对齐后端 CreateReturnItemRequest）。
export const createPurchaseReturnItem = (id: number, data: CreatePurchaseReturnItemPayload) =>
  request.post<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items`, data);

// 更新明细：请求体严格为 UpdatePurchaseReturnItemPayload（对齐后端 UpdateReturnItemRequest）。
export const updatePurchaseReturnItem = (
  id: number,
  itemId: number,
  data: UpdatePurchaseReturnItemPayload
) => request.put<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items/${itemId}`, data);

// D14 Batch 5b：原 purchaseReturnApi.deleteItem 转为风格 B 函数
export const deletePurchaseReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}/items/${itemId}`);
