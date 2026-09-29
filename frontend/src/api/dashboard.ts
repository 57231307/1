import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface DashboardQuery {
  start_date?: string;
  end_date?: string;
}

/**
 * 概览出参 = `dashboard_service.rs` 的 `DashboardOverview`（全部 i64 / String 金额）。
 * 真实列仅 7 项；下方带 `?` 的是当前 8 张卡片里后端并不提供的三项
 * （inventory_total / customer_count / today_orders）与 recent_activities 活动流，
 * 属需要后端补齐的聚合/关联项，暂恒缺。
 */
export interface DashboardOverview {
  total_products: number;
  total_warehouses: number;
  total_orders: number;
  total_sales: string;
  low_stock_count: number;
  pending_orders: number;
  monthly_sales: string;
  // 后端 overview 未提供，需服务端补充后才回显
  inventory_total?: number;
  customer_count?: number;
  today_orders?: number;
  recent_activities?: Activity[];
}

/** 最新活动流：DashboardOverview 目前无该数据源，需后端新增聚合端点（保留表格功能） */
export interface Activity {
  id: number;
  type: string;
  content: string;
  time: string;
  user: string;
}

/** 销售统计出参 = `dashboard_service.rs` 的 `SalesStatistics`（各维度分组序列） */
export interface SalesStatistics {
  daily_sales: SalesDataPoint[];
  weekly_sales: SalesDataPoint[];
  monthly_sales: SalesDataPoint[];
  by_customer: SalesByDimension[];
  by_product: SalesByDimension[];
  by_salesperson: SalesByDimension[];
}

/** 销售趋势点 = `SalesDataPoint`；`count` 后端未随 daily_sales 返回（需按日聚合订单数） */
export interface SalesDataPoint {
  date: string;
  amount: string;
  count?: number;
}

export interface SalesByDimension {
  name: string;
  amount: string;
  count: number;
}

/** 库存统计出参 = `dashboard_service.rs` 的 `InventoryStatistics` */
export interface InventoryStatistics {
  total_inventory: string;
  by_warehouse: InventoryByWarehouse[];
  by_category: InventoryByCategory[];
  turnover_rate: string;
  aging_analysis: AgingData[];
}

export interface InventoryByWarehouse {
  warehouse_name: string;
  quantity: string;
  value: string;
}

export interface InventoryByCategory {
  category_name: string;
  quantity: string;
  value: string;
}

export interface AgingData {
  age_range: string;
  quantity: string;
  percentage: number;
}

/** 低库存预警出参 = `dashboard_service.rs` 的 `LowStockAlert`（数量为字符串，无 id/单位/告警级别列） */
export interface LowStockAlert {
  product_id: number;
  product_name: string;
  warehouse_id: number;
  warehouse_name: string;
  current_quantity: string;
  min_stock: string;
  shortage: string;
}

export const getDashboardOverview = (
  params?: DashboardQuery
): Promise<ApiResponse<DashboardOverview>> => request.get('/dashboard/overview', { params });

export const getDashboardSalesStats = (
  params?: DashboardQuery
): Promise<ApiResponse<SalesStatistics>> => request.get('/dashboard/sales-stats', { params });

export const getDashboardInventoryStats = (
  params?: DashboardQuery
): Promise<ApiResponse<InventoryStatistics>> =>
  request.get('/dashboard/inventory-stats', { params });

export const getDashboardLowStockAlerts = (): Promise<ApiResponse<LowStockAlert[]>> =>
  request.get('/dashboard/low-stock-alerts');
