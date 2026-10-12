// P3-4 BI 多维分析 API 客户端（16 端点）
// 8 维度聚合 + 4 钻取 + 4 切片/上卷
// 创建时间: 2026-06-17

import { request } from './request';

// =====================================================
// 公共类型
// =====================================================

/**
 * 钻取：客户 → 订单的订单行。
 * 键集与类型对齐后端 backend/src/services/bi_analysis_ops/drilldown.rs 的 json! 构造点：
 * order_id 为整数、amount 经 dec_to_f64 出 JSON number（非 rust_decimal 字符串）、
 * date 为 YYYY-MM-DD 字符串（无值时为空串）。
 */
export interface DrilldownCustomerOrderItem {
  order_id: number;
  amount: number;
  date: string;
}

/**
 * 钻取：产品 → 订单的订单行。
 * 键集与类型对齐后端 backend/src/services/bi_analysis_ops/drilldown.rs 的 json! 构造点：
 * quantity 与 amount 均经 dec_to_f64 出 JSON number；此端点不输出日期键。
 */
export interface DrilldownProductOrderItem {
  order_id: number;
  quantity: number;
  amount: number;
}

/**
 * 钻取：客户 → 订单的内层载荷。
 * 后端出参是对象（含 customer_id 与 orders 数组），不是裸数组，消费方必须读 .orders。
 */
export interface CustomerOrderDrilldown {
  customer_id: number;
  orders: DrilldownCustomerOrderItem[];
}

/**
 * 钻取：产品 → 订单的内层载荷。
 * 后端出参是对象（含 product_id 与 orders 数组），不是裸数组，消费方必须读 .orders。
 */
export interface ProductOrderDrilldown {
  product_id: number;
  orders: DrilldownProductOrderItem[];
}

/** 切片/切块结果（通用聚合返回，由后端按维度动态返回） */
export interface SliceDiceResult {
  dimension: string;
  values: Array<Record<string, unknown>>;
  [key: string]: unknown;
}

/** 上卷结果 */
export interface RollupResult {
  from: string;
  to: string;
  values: Array<Record<string, unknown>>;
  [key: string]: unknown;
}

/** 透视结果 */
export interface PivotResult {
  row: string;
  col: string;
  measure: string;
  cells: Array<Record<string, unknown>>;
  [key: string]: unknown;
}

/** 时间序列点 */
export interface TimeSeriesPoint {
  period: string;
  total_amount: number;
  order_count: number;
  quantity: number;
  profit_amount: number;
}

/** 客户排行 */
export interface CustomerRank {
  customer_id: number;
  customer_name: string;
  total_amount: number;
  order_count: number;
  percentage: number;
}

/** 产品排行 */
export interface ProductRank {
  product_id: number;
  product_name: string;
  product_code: string;
  category: string;
  total_amount: number;
  quantity: number;
  order_count: number;
}

/** 区域统计 */
export interface RegionStat {
  region: string;
  total_amount: number;
  order_count: number;
  customer_count: number;
}

/** 品类统计 */
export interface CategoryStat {
  category: string;
  total_amount: number;
  percentage: number;
}

/** 利润分析 */
export interface ProfitAnalysis {
  total_revenue: number;
  total_cost: number;
  total_profit: number;
  gross_margin: number;
  order_count: number;
  avg_order_value: number;
}

/** KPI 概览 */
export interface KpiSummary {
  total_sales: number;
  order_count: number;
  customer_count: number;
  avg_order_value: number;
  /** 同比增长率百分比；后端无可比同期基期时为 null（非 0%） */
  yoy_growth: number | null;
  /** 环比增长率百分比；后端无可比上月基期时为 null（非 0%） */
  mom_growth: number | null;
}

/** BI 响应 */
export interface BiResponseData<T> {
  code: number;
  message: string;
  data: T;
}

/**
 * BI 端点是双层信封：外层 ApiResponse（request 拦截器原样返回）
 * 内层 services/bi_analysis_ops/types.rs 的 BiResponse。
 * 各页面直接把 ApiResponse.data 当业务数据用会拿到内层对象，
 * 表现为 `x.map is not a function` 或表格恒为空，故统一用 unwrapBi 剥层。
 */
export interface BiEnvelope<T> {
  code: number;
  message: string;
  data: BiResponseData<T>;
}

export function unwrapBi<T>(res: BiEnvelope<T>): T {
  return res.data.data;
}

// =====================================================
// 8 个维度聚合端点
// =====================================================

/** 按时间聚合销售 */
export function getSalesByTime(
  startDate: string,
  endDate: string,
  granularity: 'day' | 'week' | 'month' | 'quarter' | 'year'
) {
  return request.get<BiEnvelope<TimeSeriesPoint[]>>('/bi/sales/by-time', {
    params: { start_date: startDate, end_date: endDate, granularity },
  });
}

/** 按客户聚合 */
export function getSalesByCustomer(limit = 10) {
  return request.get<BiEnvelope<CustomerRank[]>>('/bi/sales/by-customer', {
    params: { limit },
  });
}

/** 按产品聚合 */
export function getSalesByProduct(limit = 10) {
  return request.get<BiEnvelope<ProductRank[]>>('/bi/sales/by-product', {
    params: { limit },
  });
}

/** 按区域聚合 */
export function getSalesByRegion() {
  return request.get<BiEnvelope<RegionStat[]>>('/bi/sales/by-region');
}

/** 按品类聚合 */
export function getSalesByCategory() {
  return request.get<BiEnvelope<CategoryStat[]>>('/bi/sales/by-category');
}

/** 销售趋势 */
export function getSalesTrend(days = 30) {
  return request.get<BiEnvelope<TimeSeriesPoint[]>>('/bi/sales/trend', {
    params: { days },
  });
}

/** 利润分析 */
export function getProfitAnalysis() {
  return request.get<BiEnvelope<ProfitAnalysis>>('/bi/sales/profit');
}

/** 核心 KPI */
export function getKpiSummary() {
  return request.get<BiEnvelope<KpiSummary>>('/bi/sales/kpi');
}

// =====================================================
// 4 个钻取端点
// =====================================================

/** 钻取：年 → 月 */
export function getDrilldownYearToMonth(year: number) {
  return request.get<BiEnvelope<TimeSeriesPoint[]>>('/bi/sales/drilldown/year-to-month', {
    params: { year },
  });
}

/** 钻取：月 → 日 */
export function getDrilldownMonthToDay(year: number, month: number) {
  return request.get<BiEnvelope<TimeSeriesPoint[]>>('/bi/sales/drilldown/month-to-day', {
    params: { year, month },
  });
}

/** 钻取：客户 → 订单（内层载荷为对象，消费方读 .orders） */
export function getDrilldownCustomerToOrder(customerId: number) {
  return request.get<BiEnvelope<CustomerOrderDrilldown>>(
    `/bi/sales/drilldown/customer-to-order/${customerId}`
  );
}

/** 钻取：产品 → 订单（内层载荷为对象，消费方读 .orders） */
export function getDrilldownProductToOrder(productId: number) {
  return request.get<BiEnvelope<ProductOrderDrilldown>>(
    `/bi/sales/drilldown/product-to-order/${productId}`
  );
}

// =====================================================
// 4 个切片/上卷端点
// =====================================================

/** 切片 */
export function postSlice(dimension: string, filters: Record<string, unknown>) {
  return request.post<BiEnvelope<SliceDiceResult>>('/bi/sales/slice', { dimension, filters });
}

/** 切块 */
export function postDice(filters: Record<string, unknown>) {
  return request.post<BiEnvelope<SliceDiceResult>>('/bi/sales/dice', { filters });
}

/** 上卷 */
export function postRollup(from: string, to: string) {
  return request.post<BiEnvelope<RollupResult>>('/bi/sales/rollup', { from, to });
}

/** 透视 */
export function postPivot(row: string, col: string, measure: string) {
  return request.post<BiEnvelope<PivotResult>>('/bi/sales/pivot', { row, col, measure });
}
