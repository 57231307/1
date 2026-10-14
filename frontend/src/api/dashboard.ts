import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface DashboardQuery {
  start_date?: string;
  end_date?: string;
}

/**
 * 概览出参 = `dashboard_service.rs` 的 `DashboardOverview`（计数为 i64、金额为 String）。
 * `inventory_total` 是库存米数合计（Decimal 以 String 承载精度，同 total_sales/monthly_sales），
 * 故类型为 number | string；`customer_count`/`today_orders` 为计数。
 * `fabric_count` 为面料总数（product_type 属坯布/成品布的产品行计数，排除辅料），供"面料总数"卡片。
 * recent_activities 后端 overview 仍未提供，属活动流数据源缺口，保留可选。
 */
export interface DashboardOverview {
  total_products: number;
  total_warehouses: number;
  total_orders: number;
  total_sales: string;
  low_stock_count: number;
  pending_orders: number;
  monthly_sales: string;
  inventory_total?: number | string;
  inventory_weight_kg?: number | string;
  customer_count?: number;
  today_orders?: number;
  fabric_count?: number;
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
