/**
 * 仪表盘 store 测试夹具
 * batch-20 P3: fixtures 测试夹具补充
 */

export const mockDashboardOverview = {
  fabricCount: 150,
  inventoryTotal: 50000,
  monthOrders: 120,
  customerCount: 80,
  todayOrders: 5,
  pendingOrders: 15,
  lowStockProducts: 8,
  monthSales: 250000,
  recentActivities: [
    { id: 1, type: 'order', description: '新订单 #1001', time: '2026-08-01T10:00:00Z' },
    { id: 2, type: 'delivery', description: '发货 #1002', time: '2026-08-01T09:00:00Z' },
  ],
};

export const mockSalesStatistics = {
  total_sales: 250000,
  total_orders: 120,
  average_order_value: 2083,
  top_products: [
    { name: '纯棉坯布', quantity: 5000, amount: 75000 },
    { name: '涤纶面料', quantity: 3000, amount: 45000 },
  ],
};

export const mockInventoryStatistics = {
  total_items: 50000,
  low_stock_items: 8,
  out_of_stock_items: 2,
  categories: [
    { name: '坯布', count: 20000 },
    { name: '成品布', count: 25000 },
    { name: '辅料', count: 5000 },
  ],
};
