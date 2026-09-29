import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/* ============================================================================
 * MRP 真实契约（backend/src/handlers/mrp_handler.rs + models/product.rs）
 * index.vue 计算/转单链路使用本节类型，字段与后端 DTO 逐一同名同形。
 * ==========================================================================*/

/** 后端 mrp_handler.rs:42 MrpCalculateItemPayload（items 有 length(min=1) 校验） */
export interface MrpCalculateItem {
  product_id: number;
  required_quantity: number;
  required_date: string; // NaiveDate，序列化为 'YYYY-MM-DD'
}

/** 后端 mrp_handler.rs:30 MrpCalculatePayload */
export interface MrpCalculatePayload {
  items: MrpCalculateItem[];
  source_type?: string;
  source_id?: number;
  consider_safety_stock?: boolean;
  consider_in_transit?: boolean;
}

/**
 * 计算页输入：页面以「多产品 + 同一需求数量/日期」录入，
 * 在 API 层映射为后端要求的 items[]（字段名与后端 items 同族：required_date）。
 */
export interface MrpCalculateInput {
  product_ids: number[];
  demand_quantity: number;
  required_date: string;
  consider_safety_stock?: boolean;
  consider_in_transit?: boolean;
}

/** 后端 mrp_handler.rs:51 MrpResultResponse（可转单的结果行，含主键 id） */
// mrp_results.status 的闭合词表，与后端 `crate::models::status::mrp` 常量同源
// （models/status/production.rs 的 `pub mod mrp`；原 `MrpResultStatus` ActiveEnum 与之
// 各缺一个成员且无人引用，已删除，避免同一列存在两套词表）。
export type MrpStatus = 'PLANNED' | 'CONFIRMED' | 'RELEASED' | 'CANCELLED';

export interface MrpResultResponse {
  id: number;
  calculation_no: string;
  product_id: number;
  required_quantity: number;
  required_date: string | null;
  source_type: string;
  source_id: number | null;
  planned_order_quantity: number | null;
  planned_order_date: string | null;
  status: MrpStatus;
  remarks: string | null;
  created_at: string;
  updated_at: string;
}

/** 后端 mrp_handler.rs:81 MaterialRequirementResponse（需求行；不含产品主数据） */
export interface MrpRequirementRow {
  product_id: number;
  required_quantity: number;
  required_date: string;
  on_hand_quantity: number;
  in_transit_quantity: number;
  safety_stock: number;
  available_quantity: number;
  shortage_quantity: number;
  source_type: string;
  source_id: number | null;
  bom_level: number;
}

/** 后端 mrp_handler.rs:70 MrpCalculationSummaryResponse */
export interface MrpCalculationSummary {
  calculation_no: string;
  total_items: number;
  items_with_shortage: number;
  results: MrpResultResponse[];
  requirements: MrpRequirementRow[];
}

/** /mrp/products 返回 product::Model（models/product.rs）；仅声明页面用到的真实字段 */
export interface MrpProductOption {
  id: number;
  code: string;
  name: string;
  unit: string;
  specification?: string | null;
  product_type: string;
}

/** 后端 mrp_handler.rs:126 ConvertOrderPayload；order_type 仅接受大写枚举值 */
export type MrpOrderType = 'PURCHASE' | 'PRODUCTION';

export interface ConvertOrderPayload {
  result_ids: number[];
  order_type: MrpOrderType;
}

/** 后端 mrp_handler.rs:98 MrpResultQuery（/mrp/results 分页查询） */
export interface MrpResultQuery {
  calculation_no?: string;
  product_id?: number;
  status?: MrpStatus;
  page?: number;
  page_size?: number;
}

/** 触发 MRP 计算：把页面的多产品同数量/日期输入映射为 items[] */
export function calculateMrp(
  input: MrpCalculateInput
): Promise<ApiResponse<MrpCalculationSummary>> {
  const payload: MrpCalculatePayload = {
    items: input.product_ids.map(productId => ({
      product_id: productId,
      required_quantity: input.demand_quantity,
      required_date: input.required_date,
    })),
    consider_safety_stock: input.consider_safety_stock,
    consider_in_transit: input.consider_in_transit,
  };
  return request.post('/production/mrp/calculate', payload);
}

/**
 * 真实分页结果列表（routes/production.rs:646 GET /mrp/results）。
 * 响应为 PaginatedResponse，消费方读 res.data.items。
 */
export function getMrpResults(
  params: MrpResultQuery = {}
): Promise<ApiResponse<PaginatedResponse<MrpResultResponse>>> {
  return request.get('/production/mrp/results', { params });
}

/** 将选定的 MRP 结果行转为采购/生产订单（routes/production.rs:648 POST /mrp/convert-orders） */
export function convertToOrder(
  data: ConvertOrderPayload
): Promise<ApiResponse<MrpResultResponse[]>> {
  return request.post('/production/mrp/convert-orders', data);
}

/** 可用于 MRP 计算的产品主数据（routes/production.rs:649 GET /mrp/products） */
export function getProductsForMrp(params?: {
  keyword?: string;
}): Promise<ApiResponse<MrpProductOption[]>> {
  return request.get('/production/mrp/products', { params });
}

/* ============================================================================
 * MRP 历史页动作端点（backend routes/production.rs mrp_history()）。
 * 列表改由上方 GET /mrp/results 提供；以下均为已接真实 handler 的操作：
 * 取消 / 导出 / 行内库存分解。calculation_id 一律传 mrp_result 行主键 id。
 * ==========================================================================*/

/**
 * 后端 mrp_handler::get_material_detail 的 JSON（mrp_engine_ops/query.rs）。
 * 返回单条结果行对应产品的实时库存分解；无 supply_details（后端恒为空，故不建模）。
 */
export interface MrpMaterialDetail {
  calculation_id: number;
  calculation_no: string;
  material_id: number;
  required_quantity: number;
  required_date: string | null;
  on_hand_quantity: number;
  in_transit_quantity: number;
  safety_stock: number;
  available_quantity: number;
  shortage_quantity: number | null;
  planned_order_date: string | null;
  source_type: string;
  source_id: number | null;
  status: MrpStatus;
  remarks: string | null;
}

/** 取消 MRP 计算：后端仅把状态置为 CANCELLED；id 为 mrp_result 行主键 */
export function cancelMrpCalculation(id: number): Promise<ApiResponse<MrpResultResponse>> {
  return request.put(`/production/mrp-history/${id}/cancel`);
}

/** 导出单条 mrp_result 行为 xlsx；id 为 mrp_result 行主键 */
export function exportMrpResult(id: number): Promise<Blob> {
  return request.get<Blob>(`/production/mrp-history/${id}/export`, {
    responseType: 'blob',
  });
}

/** 结果行的实时库存分解明细；calculationId 为行主键，materialId 为该行 product_id */
export function getMaterialRequirementDetail(
  calculationId: number,
  materialId: number
): Promise<ApiResponse<MrpMaterialDetail>> {
  return request.get(`/production/mrp-history/${calculationId}/materials/${materialId}`);
}
