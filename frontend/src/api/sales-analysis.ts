import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface SalesStats {
  monthOrders: number;
  monthAmount: number;
  grossProfitRate: number;
  activeCustomers: number;
  orderTrend: number;
  amountTrend: number;
  profitTrend: number;
  customerTrend: number;
}

/**
 * 产品销售排名行 —— 对齐后端 models/dto/sales_analysis_dto.rs::ProductRankingItem。
 * amount/quantity/percentage 均为 rust_decimal，出参序列化为字符串（如 "1250.00"），
 * 算术/比较/格式化前必须 Number() 归一，禁止当 number 直接运算。
 */
export interface ProductRanking {
  product_name: string;
  amount: string;
  quantity: string;
  percentage: string;
}

/**
 * 客户销售排名行 —— 对齐 CustomerRankingItem（order_count 为 i32 → number，其余 Decimal → 字符串）
 */
export interface CustomerRanking {
  customer_name: string;
  amount: string;
  order_count: number;
  percentage: string;
}

/**
 * 销售目标行 —— 对齐 SalesTargetDto；四个金额/比率列均为 rust_decimal 出参字符串。
 */
export interface SalesTarget {
  period: string;
  target_amount: string;
  actual_amount: string;
  completion_rate: string;
  variance: string;
  status: string;
}

/**
 * 排名查询参数 —— 对齐 ProductRankingParams/CustomerRankingParams 的 period/limit 两键。
 * 后端第三键 dimension_type 带字段级 #[serde(rename = "type")]：契约线上键名是 `type`，
 * 且它是**统计维度**（默认 product/customer），不是「按金额/数量排序」的开关——
 * 把排序值（amount/quantity/orders）当 type 下发会按不存在维度过滤 ⇒ 榜单恒空（静默假绿形态）。
 * 前端展示层用默认维度即正确，故本类型不携带该键（排序切换在消费侧本地完成）。
 */
export interface RankingQueryParams {
  period?: string;
  limit?: number;
}

/**
 * 更新销售目标载荷 —— 对齐 UpdateSalesTargetRequest（全部 Option）。
 * target_amount 为 Decimal 入参（JSON number 提交）；SalesTarget 是响应模型
 * （actual_amount/completion_rate/variance 为后端计算派生列，非更新契约字段），禁止再当载荷用。
 */
export interface UpdateSalesTargetPayload {
  target_amount?: number;
  status?: string;
  remarks?: string;
}

// P2-9c 修复（批次 82 v1 复审）：销售统计列表查询参数强类型化
export interface SalesStatsQueryParams {
  period?: string;
  start_date?: string;
  end_date?: string;
  category_id?: number;
}

// P2-9c 修复（批次 82 v1 复审）：销售报表导出查询参数强类型化
// 对齐后端 sales_analysis_handler::ExportParams（仅 period/format；start_date/end_date/category_id
// 后端不读，此前写在类型里属"传了被丢"的假筛选）
export interface SalesExportQueryParams {
  period?: string;
  format?: string;
}

// P2-16 修复（批次 86 v2 复审）：销售趋势 ApiResponse<any> → SalesTrendResult
export interface SalesTrendResult {
  period: string;
  amount: number;
  order_count: number;
  profit: number;
  growth_rate: number;
  [key: string]: unknown;
}

// 后端 sales_analysis_handler::get_stats 无 Query<T> 提取器（概览统计固定全量），
// 传入 period/日期等都会被 Axum 丢弃，故不再发送 params。
export const getSalesAnalysisStats = () =>
  request.get<ApiResponse<SalesStats>>('/crm/sales-analysis/stats');

// D14 Batch 5b：原 salesAnalysisApi.getProductRanking 转为风格 B 函数
export const getProductRanking = (params?: RankingQueryParams) =>
  request.get<ApiResponse<ProductRanking[]>>('/crm/sales-analysis/product-ranking', { params });

// D14 Batch 5b：原 salesAnalysisApi.getCustomerRanking 转为风格 B 函数
export const getCustomerRanking = (params?: RankingQueryParams) =>
  request.get<ApiResponse<CustomerRanking[]>>('/crm/sales-analysis/customer-ranking', { params });

// D14 Batch 5b：原 salesAnalysisApi.getSalesTargets 转为风格 B 函数
export const getSalesTargetList = () =>
  request.get<ApiResponse<SalesTarget[]>>('/crm/sales-analysis/targets');

// D14 Batch 5b：原 salesAnalysisApi.updateSalesTarget 转为风格 B 函数
export const updateSalesTarget = (period: string, data: UpdateSalesTargetPayload) =>
  request.put<ApiResponse<SalesTarget>>(`/crm/sales-analysis/targets/${period}`, data);

// D14 Batch 5b：原 salesAnalysisApi.getTrendData 转为风格 B 函数
export const getSalesTrendData = (params?: { period?: string }) =>
  request.get<ApiResponse<SalesTrendResult[]>>('/crm/sales-analysis/trend', { params });

// D14 Batch 5b：原 salesAnalysisApi.exportReport 转为风格 B 函数
export const exportSalesAnalysisReport = (params?: SalesExportQueryParams) =>
  request.get<Blob>('/crm/sales-analysis/export', { params, responseType: 'blob' });
