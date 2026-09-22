import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

/**
 * 坯布实体类型。
 *
 * 真相源 backend/src/models/greige_fabric.rs（SeaORM Model 原样 snake_case 序列化）。
 * 下方「后端真实字段」区与后端 Model 逐字段对齐；/fabric 页面（views/fabric/**）
 * 的列表列 prop 一律绑定这些真实字段。
 *
 * 「遗留字段（仅供 /greige-fabrics 旧页过渡）」区为历史遗留、后端并不返回的字段
 * （fabric_code/supplier_name/weight/unit/quantity/min_order_quantity/description/...）：
 * 独立路由页 src/views/greige-fabrics/index.vue 仍在使用，属本轮改动范围之外
 * （用户明确禁止越界），暂保留以免其 vue-tsc 失败。该页存在与本任务相同的契约缺陷，
 * 待单独修复时应一并清理这些遗留字段。切勿在 /fabric 页面对这些字段做 `?? '-'` 兜底。
 */
export interface GreigeFabric {
  id: number;
  created_at: string;
  updated_at: string;

  // ---- 后端真实字段（对齐 greige_fabric::Model）----
  fabric_no?: string;
  fabric_name?: string;
  fabric_type?: string;
  color_code?: string;
  supplier_id?: number;
  warehouse_id?: number;
  product_id?: number;
  composition?: string;
  yarn_count?: string;
  density?: string;
  /** 幅宽（m） */
  width?: number;
  /** 幅宽（cm） */
  width_cm?: number;
  /** 克重（g/m²） */
  gram_weight?: number;
  /** 当前库存重量（kg） */
  weight_kg?: number;
  /** 当前库存长度（m） */
  length_m?: number;
  /** 累计入库重量（kg） */
  quantity_kg?: number;
  /** 累计入库米数（m） */
  quantity_meters?: number;
  batch_no?: string;
  location?: string;
  status?: string;
  quality_grade?: string;
  purchase_date?: string;
  production_date?: string;
  remarks?: string;
  /** V15 P2 21.3：缸号（染色批次追溯） */
  dye_lot_no?: string;
  /** V15 P2 21.3：色号（颜色批次追溯）；白胚 = color_no 为空 */
  color_no?: string;

  // ---- 遗留字段（仅供 /greige-fabrics 旧页过渡，后端不返回）----
  /** @deprecated 后端为 fabric_no；仅 /greige-fabrics 旧页仍读 */
  fabric_code?: string;
  /** @deprecated 后端列表仅返回 supplier_id，无 supplier_name */
  supplier_name?: string;
  /** @deprecated 后端为 weight_kg / gram_weight */
  weight?: number;
  /** @deprecated 后端无该列 */
  unit?: string;
  /** @deprecated 后端无该列（库存见 weight_kg / length_m） */
  quantity?: number;
  /** @deprecated 后端无该列 */
  min_order_quantity?: number;
  /** @deprecated 后端为 remarks */
  description?: string;
}

/**
 * 入库请求体：字段与后端 StockInRequest 逐一对应。
 * 真相源 backend/src/handlers/greige_fabric_handler.rs:110
 * 纺织坯布按重量交易、按长度核米：warehouse_id / weight_kg / length_m 均为必填。
 */
export interface GreigeStockInPayload {
  warehouse_id: number;
  weight_kg: number;
  length_m: number;
  location?: string;
  quality_grade?: string;
  remarks?: string;
  purchase_receipt_id?: number;
}

/**
 * 出库请求体：字段与后端 StockOutRequest 逐一对应。
 * 真相源 backend/src/handlers/greige_fabric_handler.rs:123
 * weight_kg / length_m 后端可选，但业务上至少填一项（前端校验保证）。
 */
export interface GreigeStockOutPayload {
  weight_kg?: number;
  length_m?: number;
  remarks?: string;
}

export function getGreigeFabricList(params?: QueryParams): Promise<ApiResponse<GreigeFabric[]>> {
  return request.get('/production/greige-fabrics', { params });
}

export function getGreigeFabric(id: number): Promise<ApiResponse<GreigeFabric>> {
  return request.get(`/production/greige-fabrics/${id}`);
}

export function createGreigeFabric(
  data: Partial<GreigeFabric>
): Promise<ApiResponse<GreigeFabric>> {
  return request.post('/production/greige-fabrics', data);
}

export function updateGreigeFabric(
  id: number,
  data: Partial<GreigeFabric>
): Promise<ApiResponse<GreigeFabric>> {
  return request.put(`/production/greige-fabrics/${id}`, data);
}

export function deleteGreigeFabric(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/production/greige-fabrics/${id}`);
}

export function stockInGreigeFabric(
  id: number,
  data: GreigeStockInPayload
): Promise<ApiResponse<GreigeFabric>> {
  return request.post(`/production/greige-fabrics/${id}/stock-in`, data);
}

export function stockOutGreigeFabric(
  id: number,
  data: GreigeStockOutPayload
): Promise<ApiResponse<GreigeFabric>> {
  return request.post(`/production/greige-fabrics/${id}/stock-out`, data);
}

export function getGreigeBySupplier(supplierId: number): Promise<ApiResponse<GreigeFabric[]>> {
  return request.get(`/production/greige-fabrics/by-supplier/${supplierId}`);
}
