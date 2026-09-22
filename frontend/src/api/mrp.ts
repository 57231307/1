import { request } from './request';
import type { ApiResponse, PageResult, PaginatedResponse } from '@/types/api';

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
 * 在 API 层映射为后端要求的 items[]。
 */
export interface MrpCalculateInput {
  product_ids: number[];
  demand_quantity: number;
  demand_date: string;
  consider_safety_stock?: boolean;
  consider_in_transit?: boolean;
}

/** 后端 mrp_handler.rs:51 MrpResultResponse（可转单的结果行，含主键 id） */
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
  status: string;
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
  status?: string;
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
      required_date: input.demand_date,
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
 * 以下为 MRP 历史页（views/mrp/history.vue）遗留类型/函数。
 * 后端 /mrp-history 与 /mrp-history/{id} 仍指向 missing_handlers 占位桩，
 * 不返回真实数据；本节形状系历史页依赖，当前无法据后端契约修正。
 * ==========================================================================*/

export interface MrpProduct {
  id: number;
  product_code: string;
  product_name: string;
  specification?: string;
  unit?: string;
}

export interface MrpMaterialRequirement {
  id: number;
  material_code: string;
  material_name: string;
  specification?: string;
  unit: string;
  required_quantity: number;
  available_stock: number;
  in_transit_quantity: number;
  safety_stock: number;
  net_requirement: number;
  suggested_order_quantity: number;
  suggested_date: string;
  warehouse_name?: string;
}

export interface MrpSupplyDetail {
  source_type: 'stock' | 'in_transit' | 'planned_order';
  source_id?: number;
  source_no?: string;
  available_quantity: number;
  suggested_quantity: number;
  expected_date?: string;
}

export interface MrpCalculationResult {
  calculation_id: number;
  calculation_no: string;
  status: 'pending' | 'calculating' | 'completed' | 'failed';
  products: MrpProduct[];
  demand_quantity: number;
  demand_date: string;
  materials: MrpMaterialRequirement[];
  created_at: string;
  completed_at?: string;
}

export interface MrpHistoryRecord {
  id: number;
  calculation_no: string;
  products: MrpProduct[];
  demand_quantity: number;
  demand_date: string;
  status: 'pending' | 'calculating' | 'completed' | 'failed';
  created_at: string;
  completed_at?: string;
}

/** 指向桩 handler GET /mrp-history（missing_handlers），不返回真实数据 */
export function getMrpHistory(params?: {
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<PageResult<MrpHistoryRecord>>> {
  return request.get('/production/mrp-history', { params });
}

/** 指向桩 handler GET /mrp-history/{id}（missing_handlers），不返回真实数据 */
export function getMrpResult(id: number): Promise<ApiResponse<MrpCalculationResult>> {
  return request.get(`/production/mrp-history/${id}`);
}

/** 取消 MRP 计算；id 为 mrp_result.id（routes/production.rs:661 真实 handler） */
export function cancelMrpCalculation(id: number): Promise<ApiResponse<void>> {
  return request.put(`/production/mrp-history/${id}/cancel`);
}

/** 导出 MRP 结果为 xlsx；id 为 mrp_result.id（routes/production.rs:665 真实 handler） */
export function exportMrpResult(id: number): Promise<void> {
  return request.get(`/production/mrp-history/${id}/export`, {
    responseType: 'blob',
  }) as Promise<void>;
}

/** 获取物料需求明细（routes/production.rs:669 真实 handler） */
export function getMaterialRequirementDetail(
  calculationId: number,
  materialId: number
): Promise<ApiResponse<MrpMaterialRequirement & { supply_details: MrpSupplyDetail[] }>> {
  return request.get(`/production/mrp-history/${calculationId}/materials/${materialId}`);
}
