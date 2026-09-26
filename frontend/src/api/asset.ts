import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface FixedAsset {
  id: number;
  asset_code: string;
  asset_name: string;
  category: string;
  department_id?: number;
  department_name?: string;
  purchase_date: string;
  purchase_amount: number;
  salvage_value: number;
  useful_life_months: number;
  depreciation_method: string;
  accumulated_depreciation: number;
  net_value: number;
  status: string;
  location?: string;
  custodian?: string;
  created_at: string;
  updated_at: string;
}

// 创建固定资产请求：字段集对齐后端 fixed_asset_handler::CreateAssetRequestDto。
// 说明：后端该 DTO 无 department_id / custodian / salvage_value 入参（残值仅从 DB 读），
// 故请求侧不再收集这些字段。useful_life 单位为“年”（后端直线法 (原值-残值)/(useful_life*12)）。
export interface FixedAssetCreateRequest {
  asset_no?: string;
  asset_name: string;
  asset_category?: string;
  specification?: string;
  location?: string;
  original_value: number;
  useful_life: number;
  depreciation_method: string;
  purchase_date: string;
  put_in_date?: string;
  supplier_id?: number;
  remark?: string;
}

// 更新固定资产请求：字段集对齐后端 fixed_asset_handler::UpdateAssetDto
// （department_id / location / custodian / status 均不被该 DTO 反序列化，已移除）。
export interface FixedAssetUpdateRequest {
  asset_name?: string;
  asset_category?: string;
  specification?: string;
  use_location?: string;
}

// 资产列表查询参数：字段集严格对齐后端 AssetQuery（handlers/fixed_asset_handler.rs）。
// keyword 匹配资产编码/名称；后端无 order_by/order_dir/supplier_name/customer_name 等通用键。
export interface FixedAssetListQuery {
  keyword?: string;
  status?: string;
  asset_category?: string;
  page?: number;
  page_size?: number;
}

export function getAssetList(params?: FixedAssetListQuery): Promise<ApiResponse<FixedAsset[]>> {
  return request.get('/fixed-assets', { params });
}

export function getAsset(id: number): Promise<ApiResponse<FixedAsset>> {
  return request.get(`/fixed-assets/${id}`);
}

export function createAsset(data: FixedAssetCreateRequest): Promise<ApiResponse<FixedAsset>> {
  return request.post('/fixed-assets', data);
}

export function updateAsset(
  id: number,
  data: FixedAssetUpdateRequest
): Promise<ApiResponse<FixedAsset>> {
  return request.put(`/fixed-assets/${id}`, data);
}

export function deleteAsset(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/fixed-assets/${id}`);
}

export function depreciateAsset(id: number, period: string): Promise<ApiResponse<FixedAsset>> {
  // 批次 88 PH-2：补传 period 参数（YYYY-MM 格式），后端 DepreciateRequest 必填
  return request.post(`/fixed-assets/${id}/depreciate`, { period });
}

// 资产处置请求（对齐后端 fixed_asset_handler::DisposalRequestDto）
// disposal_type：SALE 出售 / SCRAP 报废 / TRANSFER 转移
// v3 复审 P1-2：新增资产处置能力，支持出售/报废/转移
export interface DisposalRequest {
  disposal_type: string;
  disposal_value: number;
  disposal_date: string;
  reason: string;
  buyer_info?: string;
}

// 资产处置：将指定资产标记为已处置并记录处置信息
export function disposeAsset(id: number, data: DisposalRequest): Promise<ApiResponse<string>> {
  return request.post(`/fixed-assets/${id}/dispose`, data);
}

export const batchDepreciateAssets = (data: {
  asset_ids: number[];
  calculation_date: string;
  user_id: number;
}) => request.post('/fixed-assets/batch-depreciate', data);

// ===== 预算审批：统一出口（签名一致的重复实现收敛自 budget.ts）=====
export {
  getBudgetList,
  deleteBudget,
  approveBudget,
  approveBudgetAdjustment,
  rejectBudgetAdjustment,
} from './budget';
