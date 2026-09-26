import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 出参键逐字段对齐 `backend/src/handlers/assist_accounting_handler.rs` 的响应 DTO。
 *
 * 金额/数量列在后端为 `Decimal`，而 `backend/Cargo.toml:60` 的 rust_decimal 仅开启 `serde` feature，
 * 因此序列化为 JSON 字符串 → 前端建模为 `string`；可空列（`Option<T>`）建模为 `T | null`，
 * 非空列不得标可选（否则掩盖后端不返回的键）。
 */

/** 维度：`assist_accounting_handler.rs:17-24` */
export interface AssistDimensionResponse {
  id: number;
  dimension_code: string;
  dimension_name: string;
  description: string | null;
  is_active: boolean;
  sort_order: number;
}

/** 记录：`assist_accounting_handler.rs:29-51`（无 record_no/amount/accounting_period 列） */
export interface AssistRecordResponse {
  id: number;
  business_type: string;
  business_no: string;
  business_id: number;
  account_subject_id: number;
  debit_amount: string;
  credit_amount: string;
  // 复合串，形如 `P{product}|B{batch}|C{color}|D{dye_lot}|G{grade}`（five_dimension_service.rs:156-163）
  five_dimension_id: string;
  product_id: number;
  batch_no: string;
  color_no: string;
  dye_lot_no: string | null;
  grade: string;
  warehouse_id: number;
  quantity_meters: string;
  quantity_kg: string;
  workshop_id: number | null;
  customer_id: number | null;
  supplier_id: number | null;
  remarks: string | null;
  created_at: string;
}

/** 记录列表载荷：`/records` 与穿透查询返回的封装体（`assist_accounting_handler.rs:73-78`） */
export interface AssistRecordListResponse {
  records: AssistRecordResponse[];
  total: number;
  page: number;
  page_size: number;
}

/** 汇总：`assist_accounting_handler.rs:56-68`（total_* 为 Decimal 字符串） */
export interface AssistSummaryResponse {
  id: number;
  accounting_period: string;
  dimension_code: string;
  dimension_value_id: number;
  dimension_value_name: string;
  account_subject_id: number;
  total_debit: string;
  total_credit: string;
  total_quantity_meters: string;
  total_quantity_kg: string;
  record_count: number;
}

/**
 * 记录列表查询参数，对齐 `assist_accounting_handler.rs:83-90` 的 `Query<AssistRecordQueryParams>`
 * （全为 Option，均可缺省）。
 */
export interface AssistRecordQueryParams {
  accounting_period?: string;
  dimension_code?: string;
  business_type?: string;
  warehouse_id?: number;
  page?: number;
  page_size?: number;
}

/**
 * 汇总查询参数，对齐 `assist_accounting_handler.rs:321-324`：后端 `accounting_period: String` 为必填，
 * 不得在前端标可选（缺它后端反序列化即失败）。
 */
export interface AssistSummaryQueryParams {
  accounting_period: string;
  dimension_code?: string;
}

/** 维度列表：`/dimensions` handler 无 Query 提取器，不接受任何查询参数。裸数组信封。 */
export const getAssistDimensionList = () =>
  request.get<ApiResponse<AssistDimensionResponse[]>>('/assist-accounting/dimensions');

/** 记录列表：`/records` 返回 `{records,total,page,page_size}` 封装体。 */
export const getAssistRecordList = (params?: AssistRecordQueryParams) =>
  request.get<ApiResponse<AssistRecordListResponse>>('/assist-accounting/records', { params });

/** 按五维 ID 查询：路径参数为复合串（含 `|`），必须 encodeURIComponent；data 是裸数组。 */
export const getAssistRecordsByFiveDimension = (fiveDimensionId: string) =>
  request.get<ApiResponse<AssistRecordResponse[]>>(
    `/assist-accounting/records/five-dimension/${encodeURIComponent(fiveDimensionId)}`
  );

/** 汇总：`/summary` data 是裸数组。 */
export const getAssistSummary = (params: AssistSummaryQueryParams) =>
  request.get<ApiResponse<AssistSummaryResponse[]>>('/assist-accounting/summary', { params });
