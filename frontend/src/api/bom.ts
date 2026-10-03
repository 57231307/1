import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Bom {
  id: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  /** 后端 models/bom.rs:47 `pub version: i32`（JSON number，非字符串）；请求侧亦为 number，与后端 i32 双向一致 */
  version: number;
  is_default: boolean;
  status: 'draft' | 'active' | 'archived';
  remark?: string;
  items?: BomItem[];
  created_at?: string;
  updated_at?: string;
}

/**
 * BOM 明细行，键名对齐后端 handlers/bom_handler.rs::BomItemResponse
 * （id/bom_id/material_id/quantity/unit/scrap_rate/sort_order，snake_case 原样输出）。
 * 原声明的 material_name/loss_rate 后端从不存在：material_name 无出参键（恒空），
 * loss_rate 键名错位导致提交被 serde 忽略、损耗率静默丢失——按真实契约纠正。
 */
export interface BomItem {
  id?: number;
  bom_id?: number;
  material_id: number;
  quantity: number;
  /** DDL bom_items.unit VARCHAR(20)（m0007:40），后端 DTO 已同步 max=20 校验 */
  unit: string | null;
  /**
   * 损耗率（API 百分比数值口径：10 = 10%）。提交与回显同字段同口径：
   * 后端写/读边界经 BomService::scrap_percent_to_ratio / scrap_ratio_to_percent
   * 与 DECIMAL(5,4) 存储比率（0–1）换算，前端不再二次乘除。
   */
  scrap_rate: number | null;
  sort_order?: number | null;
}

/**
 * GET /boms 查询参数，对齐后端 handlers/bom_handler.rs::ListBomsQuery
 * （无 rename_all，字段保持 snake_case；全部 Option）。
 */
export interface BomQueryParams {
  product_id?: number;
  status?: string;
  is_default?: boolean;
  page?: number;
  page_size?: number;
}

// D14 Batch 5b：原 bomApi.list 转为风格 B 函数
export const getBomList = (params?: BomQueryParams) =>
  request.get<ApiResponse<{ items: Bom[]; total: number }>>('/boms', { params });

// D14 Batch 5b：原 bomApi.getById 转为风格 B 函数
export const getBomById = (id: number) => request.get<ApiResponse<Bom>>(`/boms/${id}`);

// D14 Batch 5b：原 bomApi.create 转为风格 B 函数
export const createBom = (data: Partial<Bom> & { items?: BomItem[] }) =>
  request.post<ApiResponse<Bom>>('/boms', data);

// D14 Batch 5b：原 bomApi.update 转为风格 B 函数
export const updateBom = (id: number, data: Partial<Bom> & { items?: BomItem[] }) =>
  request.put<ApiResponse<Bom>>(`/boms/${id}`, data);

// D14 Batch 5b：原 bomApi.delete 转为风格 B 函数
export const deleteBom = (id: number) => request.delete<ApiResponse<null>>(`/boms/${id}`);

// D14 Batch 5b：原 bomApi.copy 转为风格 B 函数
export const copyBom = (id: number) => request.post<ApiResponse<Bom>>(`/boms/${id}/copy`);

// D14 Batch 5b：原 bomApi.setDefault 转为风格 B 函数
export const setDefaultBom = (id: number) => request.put<ApiResponse<Bom>>(`/boms/${id}/default`);

// D14 Batch 5b：原 bomApi.getVersions 转为风格 B 函数（获取BOM版本历史）
// 后端真实路由：GET /boms/versions/{product_id}（catalog.rs boms()）
export const getBomVersionList = (productId: number) =>
  request.get<
    ApiResponse<{ id: number; version: number; created_at: string; is_default: boolean }[]>
  >(`/boms/versions/${productId}`);

// D14 Batch 5b：原 bomApi.submit 转为风格 B 函数（提交BOM审核）
export const submitBom = (id: number) => request.put<ApiResponse<void>>(`/boms/${id}/submit`);

// D14 Batch 5b：原 bomApi.approve 转为风格 B 函数（审核BOM）
export const approveBom = (id: number, data: { approved: boolean; remark?: string }) =>
  request.put<ApiResponse<void>>(`/boms/${id}/approve`, data);
