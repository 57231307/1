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

/**
 * 销售趋势出参行 —— 对齐后端 services/sales_analysis_service.rs::SalesTrendPoint
 * （按粒度分桶的时间序列；后端现算 sales_orders，聚合本体复用 BiAnalysisService::sales_by_time）。
 * amount/quantity/profit 为 rust_decimal 口径的两位小数字符串（如 "3001.25"、"5.00"），
 * 算术/比较/图表喂值前必须 Number() 归一，禁止当 number 直接运算；
 * 后端无 growth_rate 产出方，前端声明中不存在该键（自创假列即恒空缺陷形态）。
 */
export interface SalesTrendResult {
  /** 桶键：day `YYYY-MM-DD` / week `IYYY-IW` / month `YYYY-MM` / quarter 与后端分桶实现点当前形态一致 / year `YYYY` */
  period: string;
  /** 该桶销售额（字符串，Decimal=字符串口径） */
  amount: string;
  /** 该桶订单数 */
  order_count: number;
  /** 该桶销售数量（字符串） */
  quantity: string;
  /** 该桶利润＝销售额−成本（字符串） */
  profit: string;
}

/**
 * 趋势分桶粒度词表 —— 与后端 bi_analysis_ops/sales.rs::build_period_expr 的 match 分支同词表
 * （比照 api/bi.ts 的 granularity 联合字面量），非法值后端回落 month 并留痕，前端不应下发词表外值。
 */
export type SalesTrendGranularity = 'day' | 'week' | 'month' | 'quarter' | 'year';

/**
 * 趋势查询参数 —— 对齐后端 handler TrendQuery 四键。
 * period 语义为「桶键等值过滤」（如月粒度传 "2026-08"），缺省不过滤；
 * start_date/end_date 必须成对（YYYY-MM-DD），都缺省时后端按粒度回看 12 桶。
 */
export interface SalesTrendQueryParams {
  granularity?: SalesTrendGranularity;
  start_date?: string;
  end_date?: string;
  period?: string;
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

// 趋势查询：后端为按粒度分桶的时间序列端点（现算 sales_orders）。
// 路径收敛到复数 /trends（与同域 statistics/rankings/targets 命名一致；别名 /trend 仍挂同一 handler，
// 前端不再调用单数路径，别名仅由后端契约锁与 e2e 双路径一致性断言看守）。
export const getSalesTrendData = (params?: SalesTrendQueryParams) =>
  request.get<ApiResponse<SalesTrendResult[]>>('/crm/sales-analysis/trends', { params });

// D14 Batch 5b：原 salesAnalysisApi.exportReport 转为风格 B 函数
export const exportSalesAnalysisReport = (params?: SalesExportQueryParams) =>
  request.get<Blob>('/crm/sales-analysis/export', { params, responseType: 'blob' });
