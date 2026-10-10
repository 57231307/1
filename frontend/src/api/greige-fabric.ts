import { request } from './request';
import type { ApiResponse } from '@/types/api';

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
  fabric_type: string;
  color_code?: string;
  supplier_id?: number;
  warehouse_id?: number;
  product_id?: number;
  composition?: string;
  yarn_count?: string;
  density?: string;
  // rust_decimal 出参一律为字符串（展示/计算前 Number() 归一，禁止对字符串直接 .toFixed）
  /** 幅宽（m） */
  width?: string | null;
  /** 幅宽（cm） */
  width_cm?: string | null;
  /** 克重（g/m²） */
  gram_weight?: string | null;
  /** 当前库存重量（kg） */
  weight_kg?: string | null;
  /** 当前库存长度（m） */
  length_m?: string | null;
  /** 累计入库重量（kg） */
  quantity_kg?: string | null;
  /** 累计入库米数（m） */
  quantity_meters?: string | null;
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
 * 坯布库存状态权威取值（中文 token 即落库值，禁止英文化）。
 *
 * 真相源 backend/src/handlers/greige_fabric_handler.rs：
 *   :222 新建默认 `在库`；:405 入库置 `在库`；
 *   :504-507 出库后重量与米数耗尽置 `已出库`，否则回 `在库`；
 *   :349 删除门控 `status == Some("在库")` 拒绝删除。
 * 后端只会写入这两个中文值，故比较点与门控值必须与此逐字符相同。
 * 展示范式复用 views/fabric/tabs/GreigeTab.vue（直接渲染中文原值，不再套一层英文 code）。
 */
export const GREIGE_STATUS = {
  IN_STOCK: '在库',
  OUT_OF_STOCK: '已出库',
} as const;

export type GreigeStatusValue = (typeof GREIGE_STATUS)[keyof typeof GREIGE_STATUS];

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

/**
 * 坯布列表查询参数——严格对齐后端 greige_fabric_handler.rs::GreigeFabricListQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * 支持真实筛选：坯布编号/名称/类型、供应商、仓库、状态、等级。
 */
export interface GreigeFabricListParams {
  page?: number;
  page_size?: number;
  fabric_no?: string;
  fabric_name?: string;
  fabric_type?: string;
  supplier_id?: number;
  warehouse_id?: number;
  status?: string;
  quality_grade?: string;
}

/**
 * 创建坯布载荷：逐字段对齐 handlers/greige_fabric_handler.rs::CreateGreigeFabricRequest。
 * 后端字段全为 Option，但 handler 要求 fabric_type 非空（DB NOT NULL，缺失返回 422）；
 * fabric_no 留空由后端自动生成。禁止夹带 id/created_at 等实体回显键。
 * 建单人 created_by 不是该后端请求结构体的入参（handler 按服务端会话派生），前端不得上送。
 * 纺织四维口径（services/inv/fabric_class.rs::validate_fabric_trace）：
 * color_no 白坯可空但禁止提交空串——空值整键省略；染色布必须带 dye_lot_no。
 */
export interface CreateGreigeFabricPayload {
  fabric_no?: string;
  fabric_name?: string;
  fabric_type: string;
  color_code?: string;
  width_cm?: number;
  weight_kg?: number;
  length_m?: number;
  supplier_id?: number;
  batch_no?: string;
  warehouse_id?: number;
  location?: string;
  /** 坯布状态中文 token（GREIGE_STATUS 词表），省略由后端默认「在库」 */
  status?: GreigeStatusValue;
  quality_grade?: string;
  /** YYYY-MM-DD */
  purchase_date?: string;
  remarks?: string;
  product_id?: number;
  composition?: string;
  yarn_count?: string;
  density?: string;
  width?: number;
  gram_weight?: number;
  structure?: string;
  /** YYYY-MM-DD */
  production_date?: string;
  quantity_meters?: number;
  quantity_kg?: number;
  purchase_order_id?: number;
  purchase_receipt_id?: number;
  safety_stock?: number;
  reorder_point?: number;
  max_stock_point?: number;
  reorder_quantity?: number;
  dye_lot_no?: string;
  color_no?: string;
}

/**
 * 更新坯布载荷：对齐 UpdateGreigeFabricRequest。
 * 注意：该 DTO 无 fabric_no/product_id/composition/yarn_count/density/width/
 * structure/production_date/quantity_* 等键（创建后不可改），旧代码用
 * Partial<实体> 提交会被 serde 静默丢弃；status 必须为 GREIGE_STATUS 中文 token。
 */
export interface UpdateGreigeFabricPayload {
  fabric_name?: string;
  fabric_type?: string;
  color_code?: string;
  width_cm?: number;
  weight_kg?: number;
  length_m?: number;
  supplier_id?: number;
  batch_no?: string;
  warehouse_id?: number;
  location?: string;
  status?: GreigeStatusValue;
  quality_grade?: string;
  remarks?: string;
  safety_stock?: number;
  reorder_point?: number;
  max_stock_point?: number;
  reorder_quantity?: number;
  dye_lot_no?: string;
  color_no?: string;
}

export function getGreigeFabricList(
  params?: GreigeFabricListParams
): Promise<ApiResponse<{ items: GreigeFabric[]; total: number; page: number; page_size: number }>> {
  return request.get('/production/greige-fabrics', { params });
}

export function getGreigeFabric(id: number): Promise<ApiResponse<GreigeFabric>> {
  return request.get(`/production/greige-fabrics/${id}`);
}

export function createGreigeFabric(
  data: CreateGreigeFabricPayload
): Promise<ApiResponse<GreigeFabric>> {
  return request.post('/production/greige-fabrics', data);
}

export function updateGreigeFabric(
  id: number,
  data: UpdateGreigeFabricPayload
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
