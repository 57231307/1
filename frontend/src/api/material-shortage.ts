import { request } from './request';
import type { ApiResponse, PageResult } from '@/types/api';
import type {
  ReplenishmentPriorityValue,
  ShortageAlertStatusValue,
  ShortageLevelValue,
} from '@/constants/shortage';

/** 实时检测出的缺料项（后端 MaterialShortageItem） */
export interface ShortageCheckItem {
  material_id: number;
  material_name: string;
  material_code: string;
  required_quantity: number;
  available_quantity: number;
  shortage_quantity: number;
  /** 缺口率（百分比，>=100 为 Critical） */
  deficit_rate: number;
  level: ShortageLevelValue;
  affected_orders: ShortageAffectedOrder[];
  unit?: string | null;
}

/** 受影响的生产订单 */
export interface ShortageAffectedOrder {
  order_id: number;
  order_no: string;
  demand_quantity: number;
  planned_end_date?: string | null;
}

/**
 * 缺料预警列表行：实时缺料结果 + 该物料未解决预警的落库状态。
 * status 为空表示该条实时缺料尚未落库成预警记录（persist_alerts 降级失败的例外）。
 */
export interface MaterialShortageAlert extends ShortageCheckItem {
  alert_id: number | null;
  /** 缺料单号 MS-YYYYMMDD-NNN，由后端自动生成 */
  alert_no: string | null;
  status: ShortageAlertStatusValue | null;
  identified_at?: string | null;
}

/** 缺料汇总（后端 ShortageSummary，级别计数与 level 同源） */
export interface MaterialShortageSummary {
  total_materials_checked: number;
  shortage_count: number;
  critical_count: number;
  severe_count: number;
  warning_count: number;
  affected_orders_count: number;
  items: ShortageCheckItem[];
}

/** 持久化的缺料预警记录（material_shortage_alerts 行，状态更新接口出参） */
export interface ShortageAlertRecord {
  id: number;
  alert_no: string;
  material_id: number;
  material_name: string;
  material_code?: string | null;
  required_quantity: number;
  available_quantity: number;
  shortage_quantity: number;
  deficit_rate: number;
  level: ShortageLevelValue;
  status: ShortageAlertStatusValue;
  affected_orders_count: number;
  purchase_request_id?: number | null;
  purchase_order_id?: number | null;
  unit?: string | null;
  identified_at: string;
  resolved_at?: string | null;
  created_at: string;
  updated_at: string;
}

/** 缺料列表查询参数：level / status 取值域见 @/constants/shortage，越界后端直接 400 */
export interface MaterialShortageQueryParams {
  level?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

/** 手动检查入参（对齐后端 ShortageCheckRequest），全部缺省表示不额外限定范围 */
export interface ShortageCheckPayload {
  product_ids?: number[];
  date_from?: string;
  date_to?: string;
}

// D14 Batch 5b：原 materialShortageApi.getSummary 转为风格 B 函数
export const getMaterialShortageSummary = (params?: ShortageCheckPayload) =>
  request.get<ApiResponse<MaterialShortageSummary>>('/material-shortage/summary', { params });

// D14 Batch 5b：原 materialShortageApi.listShortages 转为风格 B 函数
export const getMaterialShortageList = (params?: MaterialShortageQueryParams) =>
  request.get<ApiResponse<PageResult<MaterialShortageAlert>>>('/material-shortage/list', {
    params,
  });

// D14 Batch 5b：原 materialShortageApi.triggerCheck 转为风格 B 函数
// 后端为 Json<ShortageCheckRequest>，必须带请求体（空体会被判为解析失败）
export const triggerMaterialShortageCheck = (data: ShortageCheckPayload = {}) =>
  request.post<ApiResponse<MaterialShortageSummary>>('/material-shortage/check', data);

/**
 * 推进缺料预警状态。
 * 路径参数是 material_id（同物料至多一条未解决预警），status 必须落在状态机内。
 */
export const updateMaterialShortageStatus = (
  materialId: number,
  status: ShortageAlertStatusValue
) =>
  request.put<ApiResponse<ShortageAlertRecord>>(`/material-shortage/${materialId}/status`, {
    status,
  });

/** 补货建议（后端按缺口量加 20% 余量给出，优先级由缺料级别映射） */
export interface ReplenishmentSuggestion {
  material_id: number;
  material_name: string;
  material_code: string;
  shortage_quantity: number;
  suggested_quantity: number;
  unit?: string | null;
  priority: ReplenishmentPriorityValue;
  affected_orders_count: number;
}

export const getReplenishmentSuggestions = (params?: ShortageCheckPayload) =>
  request.get<ApiResponse<{ suggestions: ReplenishmentSuggestion[]; total: number }>>(
    '/material-shortage/replenishment',
    { params }
  );
