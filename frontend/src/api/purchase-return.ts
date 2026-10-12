import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_return::Model，键为实体 snake_case；
// return_status 词表权威 = backend/src/models/status/purchase_inventory.rs（purchase_return 模块）。
// 表内无订单号/供应商名/创建人姓名列，下方这些键需后端 JOIN 产出（见字段注）。
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

// 明细出参 = 后端 PurchaseReturnItemDto（list_items）；
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
   * 面料追溯维度：审批扣减按 产品+色号+缸号+批次 四维精确定位库存行。
   * 后端出参为非空 String（白坯/单库存行为空串），故如实回读。
   */
  color_no: string;
  dye_lot_no: string;
  batch_no: string;
}

// 列表查询参数 = 后端 ReturnQueryParams（page/page_size/status/supplier_id/keyword/start_date/end_date）；
// keyword 按 return_no 模糊匹配，日期区间作用于 return_date 闭区间。
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
 * 创建采购退货单请求，逐字段对齐后端 CreatePurchaseReturnRequest。
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
 * 更新采购退货单请求，逐字段对齐后端 UpdatePurchaseReturnRequest。
 * 仅 reason_type/reason_detail/notes 三字段可更新，且三者均为 DB 可空列。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空为 NULL、有值=覆盖
 * ——清空须显式送 null，禁止塌成 `|| undefined`（省略=保持原值）。
 */
export interface UpdatePurchaseReturnPayload {
  reason_type?: string | null;
  reason_detail?: string | null;
  notes?: string | null;
}

/** 创建退货明细请求，逐字段对齐后端 CreateReturnItemRequest。 */
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
 * 更新退货明细请求，逐字段对齐后端 UpdateReturnItemRequest。
 * 三态语义（RFC 7386 JSON Merge Patch）：键缺席=保持原值、显式 null=清空、有值=覆盖。
 * - line_no/material_id/quantity_returned/unit_price/tax_rate/discount_percent 及追溯列
 *   color_no/dye_lot_no/batch_no（NOT NULL DEFAULT ''）均为 NOT NULL：禁止送 null
 *   （后端 400「XX不能清空：该字段为必填项」）；"追溯维度改回白坯/空值"提交空串而非 null；
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

export const getPurchaseReturnList = (params?: PurchaseReturnQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseReturn[]; total: number }>>('/purchase/returns', {
    params,
  });

export const createPurchaseReturn = (data: CreatePurchaseReturnPayload) =>
  request.post<ApiResponse<PurchaseReturn>>('/purchase/returns', data);

export const getPurchaseReturnById = (id: number) =>
  request.get<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`);

export const updatePurchaseReturn = (id: number, data: UpdatePurchaseReturnPayload) =>
  request.put<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}`, data);

export const deletePurchaseReturn = (id: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}`);

export const submitPurchaseReturn = (id: number) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/submit`);

// 审批「通过」：POST /purchase/returns/{id}/approve。approval_reason 选填：
// 留空即省略该键，purchase_return.approval_reason 列写 NULL（不伪造成必填、不落空串）。
// 采集见 useActionPrompts.promptApprovalReason(false)。
export const approvePurchaseReturn = (id: number, approvalReason?: string) =>
  request.post<ApiResponse<PurchaseReturn>>(
    `/purchase/returns/${id}/approve`,
    approvalReason ? { approval_reason: approvalReason } : {}
  );

// 审批「拒绝」：reason 后端必填且 trim 非空 ⇒ 必带体、必发非空 reason（不得标 `?` 掩盖必填）。
// 采集见 useActionPrompts.promptRejectReason()。
export const rejectPurchaseReturn = (id: number, reason: string) =>
  request.post<ApiResponse<PurchaseReturn>>(`/purchase/returns/${id}/reject`, { reason });

export const getPurchaseReturnItemList = (id: number) =>
  request.get<ApiResponse<PurchaseReturnItem[]>>(`/purchase/returns/${id}/items`);

export const createPurchaseReturnItem = (id: number, data: CreatePurchaseReturnItemPayload) =>
  request.post<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items`, data);

export const updatePurchaseReturnItem = (
  id: number,
  itemId: number,
  data: UpdatePurchaseReturnItemPayload
) => request.put<ApiResponse<PurchaseReturnItem>>(`/purchase/returns/${id}/items/${itemId}`, data);

export const deletePurchaseReturnItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/returns/${id}/items/${itemId}`);
