// P3-4 BI 多维分析 API 客户端（16 端点）
// 8 维度聚合 + 4 钻取 + 4 切片/上卷
// 创建时间: 2026-06-17

import { request } from './request'

// =====================================================
// 公共类型
// =====================================================

/** 时间序列点 */
export interface TimeSeriesPoint {
  period: string
  total_amount: number
  order_count: number
  quantity: number
  profit_amount: number
}

/** 客户排行 */
export interface CustomerRank {
  customer_id: number
  customer_name: string
  total_amount: number
  order_count: number
  percentage: number
}

/** 产品排行 */
export interface ProductRank {
  product_id: number
  product_name: string
  product_code: string
  category: string
  total_amount: number
  quantity: number
  order_count: number
}

/** 区域统计 */
export interface RegionStat {
  region: string
  total_amount: number
  order_count: number
  customer_count: number
}

/** 品类统计 */
export interface CategoryStat {
  category: string
  total_amount: number
  percentage: number
}

/** 利润分析 */
export interface ProfitAnalysis {
  total_revenue: number
  total_cost: number
  total_profit: number
  gross_margin: number
  order_count: number
  avg_order_value: number
}

/** KPI 概览 */
export interface KpiSummary {
  total_sales: number
  order_count: number
  customer_count: number
  avg_order_value: number
  yoy_growth: number
  mom_growth: number
}

/** BI 响应 */
export interface BiResponseData<T> {
  code: number
  message: string
  data: T
}

// =====================================================
// 8 个维度聚合端点
// =====================================================

/** 按时间聚合销售 */
export function getSalesByTime(
  startDate: string,
  endDate: string,
  granularity: 'day' | 'week' | 'month' | 'quarter' | 'year',
) {
  return request.get<BiResponseData<TimeSeriesPoint[]>>('/bi/sales/by-time', {
    params: { start_date: startDate, end_date: endDate, granularity },
  })
}

/** 按客户聚合 */
export function getSalesByCustomer(limit = 10) {
  return request.get<BiResponseData<CustomerRank[]>>('/bi/sales/by-customer', {
    params: { limit },
  })
}

/** 按产品聚合 */
export function getSalesByProduct(limit = 10) {
  return request.get<BiResponseData<ProductRank[]>>('/bi/sales/by-product', {
    params: { limit },
  })
}

/** 按区域聚合 */
export function getSalesByRegion() {
  return request.get<BiResponseData<RegionStat[]>>('/bi/sales/by-region')
}

/** 按品类聚合 */
export function getSalesByCategory() {
  return request.get<BiResponseData<CategoryStat[]>>('/bi/sales/by-category')
}

/** 销售趋势 */
export function getSalesTrend(days = 30) {
  return request.get<BiResponseData<TimeSeriesPoint[]>>('/bi/sales/trend', {
    params: { days },
  })
}

/** 利润分析 */
export function getProfitAnalysis() {
  return request.get<BiResponseData<ProfitAnalysis>>('/bi/sales/profit')
}

/** 核心 KPI */
export function getKpiSummary() {
  return request.get<BiResponseData<KpiSummary>>('/bi/sales/kpi')
}

// =====================================================
// 4 个钻取端点
// =====================================================

/** 钻取：年 → 月 */
export function getDrilldownYearToMonth(year: number) {
  return request.get<BiResponseData<TimeSeriesPoint[]>>(
    '/bi/sales/drilldown/year-to-month',
    { params: { year } },
  )
}

/** 钻取：月 → 日 */
export function getDrilldownMonthToDay(year: number, month: number) {
  return request.get<BiResponseData<TimeSeriesPoint[]>>(
    '/bi/sales/drilldown/month-to-day',
    { params: { year, month } },
  )
}

/** 钻取：客户 → 订单 */
export function getDrilldownCustomerToOrder(customerId: number) {
  return request.get<BiResponseData<any>>(
    `/bi/sales/drilldown/customer-to-order/${customerId}`,
  )
}

/** 钻取：产品 → 订单 */
export function getDrilldownProductToOrder(productId: number) {
  return request.get<BiResponseData<any>>(
    `/bi/sales/drilldown/product-to-order/${productId}`,
  )
}

// =====================================================
// 4 个切片/上卷端点
// =====================================================

/** 切片 */
export function postSlice(dimension: string, filters: Record<string, any>) {
  return request.post<BiResponseData<any>>('/bi/sales/slice', { dimension, filters })
}

/** 切块 */
export function postDice(filters: Record<string, any>) {
  return request.post<BiResponseData<any>>('/bi/sales/dice', { filters })
}

/** 上卷 */
export function postRollup(from: string, to: string) {
  return request.post<BiResponseData<any>>('/bi/sales/rollup', { from, to })
}

/** 透视 */
export function postPivot(row: string, col: string, measure: string) {
  return request.post<BiResponseData<any>>('/bi/sales/pivot', { row, col, measure })
}
