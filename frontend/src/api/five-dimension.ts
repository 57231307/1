import { request } from './request';
import type { ApiResponse } from '@/types/api';

/** 仓库库存分布项 = `five_dimension_service.rs` 的 `WarehouseStock`（数量为字符串/Decimal） */
export interface WarehouseDistributionItem {
  warehouse_id: number;
  warehouse_name: string;
  quantity_meters: string;
  quantity_kg: string;
}

/**
 * 五维统计行 = `five_dimension_service.rs` 的 `FiveDimensionStats`（扁平结构，无 `dimension` 嵌套）。
 * stats/search/list 均按此返回，前端不得再读 `item.dimension.xxx`。
 */
export interface FiveDimensionStats {
  product_id: number;
  product_name: string;
  batch_no: string;
  color_no: string;
  dye_lot_no?: string | null;
  grade: string;
  five_dimension_id: string;
  total_meters: string;
  total_kg: string;
  stock_count: number;
  warehouse_distribution: WarehouseDistributionItem[];
}

/** 详情钻取/表格行统一即扁平统计行（保留别名以兼容既有引用） */
export type FiveDimensionStatsResponse = FiveDimensionStats;
export type FiveDimensionItem = FiveDimensionStats;

/** 统计/列表查询参数 = `five_dimension_handler.rs` 的 `FiveDimensionStatsParams` */
export interface StatsQueryParams {
  product_id?: number;
  batch_no?: string;
  color_no?: string;
  dye_lot_no?: string;
  grade?: string;
  warehouse_id?: number;
  page?: number;
  page_size?: number;
}

/** 搜索查询参数 = `five_dimension_handler.rs` 的 `FiveDimensionSearchParams`（keyword 必填） */
export interface SearchQueryParams {
  keyword: string;
  search_type?: string;
  page?: number;
  page_size?: number;
}

interface PagedStats {
  items: FiveDimensionStats[];
  total: number;
}

export const getFiveDimensionStatsList = (params?: StatsQueryParams) =>
  request.get<ApiResponse<PagedStats>>('/crm/five-dimension/stats', { params });

// 后端真实路由：GET /five-dimension/{five_dimension_id}（Path<String>，五维编码）
export const getStatsByFiveDimensionId = (fiveDimensionId: string) =>
  request.get<ApiResponse<FiveDimensionStats>>(`/crm/five-dimension/${fiveDimensionId}`);

// 后端真实路由：POST /five-dimension/parse，body { five_dimension_id }
export const parseFiveDimensionId = (id: string) =>
  request.post<ApiResponse<{ success: boolean; dimension?: FiveDimensionStats; error?: string }>>(
    '/crm/five-dimension/parse',
    { five_dimension_id: id }
  );

export const searchFiveDimension = (params: SearchQueryParams) =>
  request.get<ApiResponse<PagedStats>>('/crm/five-dimension/search', { params });

export const getFiveDimensionById = (fiveDimensionId: string) =>
  request.get<ApiResponse<FiveDimensionStats>>(`/crm/five-dimension/${fiveDimensionId}`);
