const http = require('http');
const url = require('url');

const PORT = 8082;

// Mock 数据
const mockData = {
  // 用户信息
  users: {
    admin: {
      id: 1,
      username: 'admin',
      real_name: '管理员',
      email: 'admin@example.com',
      phone: '13800138000',
      status: 'active',
      roles: ['admin'],
      permissions: ['*']
    }
  },
  
  // 产品列表
  products: [
    { id: 1, name: '棉布 A', code: 'MB001', category: '棉布', unit: '米', price: 25.50, status: 'active' },
    { id: 2, name: '涤纶布 B', code: 'DL001', category: '涤纶', unit: '米', price: 18.80, status: 'active' },
    { id: 3, name: '丝绸 C', code: 'SC001', category: '丝绸', unit: '米', price: 68.00, status: 'active' }
  ],
  
  // 客户列表
  customers: [
    { id: 1, name: '服装厂 A', contact: '张三', phone: '13900139000', address: '广东省深圳市', status: 'active' },
    { id: 2, name: '贸易公司 B', contact: '李四', phone: '13800138001', address: '上海市浦东新区', status: 'active' }
  ],
  
  // 供应商列表
  suppliers: [
    { id: 1, name: '纺织原料供应商 A', contact: '王五', phone: '13700137000', address: '江苏省苏州市', status: 'active' },
    { id: 2, name: '染料供应商 B', contact: '赵六', phone: '13600136000', address: '浙江省杭州市', status: 'active' }
  ],
  
  // 仓库列表
  warehouses: [
    { id: 1, name: '主仓库', code: 'WH001', address: '深圳市宝安区', capacity: 10000, status: 'active' },
    { id: 2, name: '分仓库', code: 'WH002', address: '上海市松江区', capacity: 5000, status: 'active' }
  ],
  
  // 销售订单
  salesOrders: [
    { id: 1, order_no: 'SO20260515001', customer_name: '服装厂 A', total_amount: 25500, status: 'completed', date: '2026-05-15' },
    { id: 2, order_no: 'SO20260515002', customer_name: '贸易公司 B', total_amount: 18800, status: 'pending', date: '2026-05-15' }
  ],
  
  // 采购订单
  purchaseOrders: [
    { id: 1, order_no: 'PO20260515001', supplier_name: '纺织原料供应商 A', total_amount: 15000, status: 'completed', date: '2026-05-15' },
    { id: 2, order_no: 'PO20260515002', supplier_name: '染料供应商 B', total_amount: 8000, status: 'pending', date: '2026-05-15' }
  ],
  
  // 库存
  inventory: [
    { id: 1, product_name: '棉布 A', warehouse: '主仓库', quantity: 1000, unit: '米', status: 'normal' },
    { id: 2, product_name: '涤纶布 B', warehouse: '主仓库', quantity: 500, unit: '米', status: 'low' },
    { id: 3, product_name: '丝绸 C', warehouse: '分仓库', quantity: 200, unit: '米', status: 'normal' }
  ],
  
  // 仪表盘数据
  dashboard: {
    overview: {
      total_sales: 1250000,
      total_purchase: 850000,
      total_inventory: 3200000,
      customer_count: 58,
      supplier_count: 32,
      product_count: 156
    },
    recent_orders: [
      { id: 1, type: 'sales', order_no: 'SO20260515001', amount: 25500, status: 'completed' },
      { id: 2, type: 'purchase', order_no: 'PO20260515001', amount: 15000, status: 'pending' }
    ],
    alerts: [
      { id: 1, type: 'low_stock', message: '涤纶布 B 库存不足', level: 'warning' },
      { id: 2, type: 'payment_due', message: '供应商 A 付款即将到期', level: 'info' }
    ]
  }
};

// 处理请求
function handleRequest(req, res) {
  const parsedUrl = url.parse(req.url, true);
  const path = parsedUrl.pathname;
  const method = req.method;
  
  // 设置 CORS 头
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, PUT, DELETE, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');
  
  // 处理 OPTIONS 请求
  if (method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }
  
  // 设置响应头
  res.setHeader('Content-Type', 'application/json');
  
  // 路由处理
  let response = { code: 200, message: 'success', data: null };
  
  // 登录接口
  if (path === '/api/v1/erp/auth/login' && method === 'POST') {
    let body = '';
    req.on('data', chunk => { body += chunk; });
    req.on('end', () => {
      try {
        const { username } = JSON.parse(body);
        const user = mockData.users[username] || mockData.users.admin;
        // 生成标准 JWT 格式的 mock token
        const header = Buffer.from(JSON.stringify({ alg: 'HS256', typ: 'JWT' })).toString('base64');
        const payload = Buffer.from(JSON.stringify({
          sub: user.id,
          username: user.username,
          iat: Math.floor(Date.now() / 1000),
          exp: Math.floor(Date.now() / 1000) + 86400 // 24小时后过期
        })).toString('base64');
        const signature = 'mock-signature';
        const token = `${header}.${payload}.${signature}`;
        
        response.data = {
          token: token,
          refresh_token: 'mock-refresh-token',
          user: user
        };
        res.writeHead(200);
        res.end(JSON.stringify(response));
      } catch (e) {
        response.code = 400;
        response.message = 'Invalid request';
        res.writeHead(400);
        res.end(JSON.stringify(response));
      }
    });
    return;
  }
  
  // 获取用户信息
  if (path === '/api/v1/erp/auth/me') {
    response.data = mockData.users.admin;
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 仪表盘数据
  if (path === '/api/v1/erp/dashboard/overview') {
    response.data = mockData.dashboard.overview;
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  if (path === '/api/v1/erp/dashboard/recent-orders') {
    response.data = mockData.dashboard.recent_orders;
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  if (path === '/api/v1/erp/dashboard/alerts') {
    response.data = mockData.dashboard.alerts;
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  if (path === '/api/v1/erp/dashboard/sales-stats') {
    response.data = {
      totalAmount: 1250000,
      orderCount: 89,
      customerCount: 45,
      avgOrderAmount: 14044,
      trends: [
        { date: '2026-05-01', amount: 45000, count: 3 },
        { date: '2026-05-02', amount: 52000, count: 4 },
        { date: '2026-05-03', amount: 38000, count: 2 },
        { date: '2026-05-04', amount: 61000, count: 5 },
        { date: '2026-05-05', amount: 48000, count: 3 },
        { date: '2026-05-06', amount: 55000, count: 4 },
        { date: '2026-05-07', amount: 42000, count: 3 }
      ]
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  if (path === '/api/v1/erp/dashboard/inventory-stats') {
    response.data = {
      totalItems: 156,
      totalValue: 3200000,
      warehouseCount: 2,
      categoryDistribution: [
        { label: '棉布', value: 45 },
        { label: '涤纶', value: 38 },
        { label: '丝绸', value: 28 },
        { label: '混纺', value: 25 },
        { label: '其他', value: 20 }
      ]
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  if (path === '/api/v1/erp/dashboard/low-stock-alerts') {
    response.data = [
      { id: 1, productId: 2, productName: '涤纶布 B', productCode: 'DL001', warehouseId: 1, warehouseName: '主仓库', currentQuantity: 500, minQuantity: 1000, unit: '米', alertLevel: 'warning' },
      { id: 2, productId: 3, productName: '丝绸 C', productCode: 'SC001', warehouseId: 2, warehouseName: '分仓库', currentQuantity: 200, minQuantity: 500, unit: '米', alertLevel: 'danger' }
    ];
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 产品列表
  if (path === '/api/v1/erp/products') {
    response.data = { list: mockData.products, total: mockData.products.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 客户列表
  if (path === '/api/v1/erp/customers') {
    response.data = { list: mockData.customers, total: mockData.customers.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 供应商列表
  if (path === '/api/v1/erp/suppliers') {
    response.data = { list: mockData.suppliers, total: mockData.suppliers.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 仓库列表
  if (path === '/api/v1/erp/warehouses') {
    response.data = { list: mockData.warehouses, total: mockData.warehouses.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 销售订单
  if (path === '/api/v1/erp/sales/orders') {
    response.data = { list: mockData.salesOrders, total: mockData.salesOrders.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 采购订单
  if (path === '/api/v1/erp/purchase/orders') {
    response.data = { list: mockData.purchaseOrders, total: mockData.purchaseOrders.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 库存
  if (path === '/api/v1/erp/inventory/stock') {
    response.data = { list: mockData.inventory, total: mockData.inventory.length };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 部门列表
  if (path === '/api/v1/erp/departments') {
    response.data = [
      { id: 1, name: '总经办', code: 'CEO', status: 'active' },
      { id: 2, name: '销售部', code: 'SALES', status: 'active' },
      { id: 3, name: '采购部', code: 'PURCHASE', status: 'active' },
      { id: 4, name: '生产部', code: 'PRODUCTION', status: 'active' },
      { id: 5, name: '财务部', code: 'FINANCE', status: 'active' }
    ];
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 角色列表
  if (path === '/api/v1/erp/roles') {
    response.data = [
      { id: 1, name: '超级管理员', code: 'admin', status: 'active' },
      { id: 2, name: '销售经理', code: 'sales_manager', status: 'active' },
      { id: 3, name: '采购经理', code: 'purchase_manager', status: 'active' }
    ];
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 财务数据
  if (path === '/api/v1/erp/finance/vouchers') {
    response.data = {
      list: [
        { id: 1, voucher_no: 'PZ20260515001', date: '2026-05-15', type: '收款', amount: 25500, status: 'approved' },
        { id: 2, voucher_no: 'PZ20260515002', date: '2026-05-15', type: '付款', amount: 15000, status: 'pending' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 用户列表
  if (path === '/api/v1/erp/users') {
    response.data = {
      list: [
        { id: 1, username: 'admin', real_name: '管理员', email: 'admin@example.com', phone: '13800138000', status: 'active', department_name: '总经办' },
        { id: 2, username: 'sales01', real_name: '张三', email: 'zhangsan@example.com', phone: '13900139000', status: 'active', department_name: '销售部' },
        { id: 3, username: 'purchase01', real_name: '李四', email: 'lisi@example.com', phone: '13700137000', status: 'active', department_name: '采购部' }
      ],
      total: 3
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 销售合同
  if (path === '/api/v1/erp/sales-contracts') {
    response.data = {
      list: [
        { id: 1, contract_no: 'SC20260515001', customer_name: '服装厂 A', contract_date: '2026-05-15', total_amount: 50000, status: 'active' },
        { id: 2, contract_no: 'SC20260515002', customer_name: '贸易公司 B', contract_date: '2026-05-15', total_amount: 30000, status: 'draft' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 采购合同
  if (path === '/api/v1/erp/purchase-contracts') {
    response.data = {
      list: [
        { id: 1, contract_no: 'PC20260515001', supplier_name: '纺织原料供应商 A', contract_date: '2026-05-15', total_amount: 40000, status: 'active' },
        { id: 2, contract_no: 'PC20260515002', supplier_name: '染料供应商 B', contract_date: '2026-05-15', total_amount: 20000, status: 'draft' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 销售退货
  if (path === '/api/v1/erp/sales-returns') {
    response.data = {
      list: [
        { id: 1, return_no: 'SR20260515001', customer_name: '服装厂 A', return_date: '2026-05-15', total_amount: 5000, status: 'pending' },
        { id: 2, return_no: 'SR20260515002', customer_name: '贸易公司 B', return_date: '2026-05-15', total_amount: 3000, status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 采购退货
  if (path === '/api/v1/erp/purchase-returns') {
    response.data = {
      list: [
        { id: 1, return_no: 'PR20260515001', supplier_name: '纺织原料供应商 A', return_date: '2026-05-15', total_amount: 4000, status: 'pending' },
        { id: 2, return_no: 'PR20260515002', supplier_name: '染料供应商 B', return_date: '2026-05-15', total_amount: 2000, status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 库存盘点
  if (path === '/api/v1/erp/inventory-counts') {
    response.data = {
      list: [
        { id: 1, count_no: 'IC20260515001', warehouse_name: '主仓库', count_date: '2026-05-15', status: 'draft' },
        { id: 2, count_no: 'IC20260515002', warehouse_name: '分仓库', count_date: '2026-05-15', status: 'completed' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 库存调拨
  if (path === '/api/v1/erp/inventory-transfers') {
    response.data = {
      list: [
        { id: 1, transfer_no: 'IT20260515001', from_warehouse_name: '主仓库', to_warehouse_name: '分仓库', transfer_date: '2026-05-15', status: 'draft' },
        { id: 2, transfer_no: 'IT20260515002', from_warehouse_name: '分仓库', to_warehouse_name: '主仓库', transfer_date: '2026-05-15', status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 库存调整
  if (path === '/api/v1/erp/inventory-adjustments') {
    response.data = {
      list: [
        { id: 1, adjust_no: 'IA20260515001', warehouse_name: '主仓库', adjust_date: '2026-05-15', reason: '盘点差异', status: 'draft' },
        { id: 2, adjust_no: 'IA20260515002', warehouse_name: '分仓库', adjust_date: '2026-05-15', reason: '损耗', status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 应付发票
  if (path === '/api/v1/erp/ap/invoices') {
    response.data = {
      list: [
        { id: 1, invoice_no: 'API20260515001', supplier_name: '纺织原料供应商 A', invoice_date: '2026-05-15', invoice_amount: 40000, status: 'pending' },
        { id: 2, invoice_no: 'API20260515002', supplier_name: '染料供应商 B', invoice_date: '2026-05-15', invoice_amount: 20000, status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 应收发票
  if (path === '/api/v1/erp/ar/invoices') {
    response.data = {
      list: [
        { id: 1, invoice_no: 'ARI20260515001', customer_name: '服装厂 A', invoice_date: '2026-05-15', invoice_amount: 50000, status: 'pending' },
        { id: 2, invoice_no: 'ARI20260515002', customer_name: '贸易公司 B', invoice_date: '2026-05-15', invoice_amount: 30000, status: 'approved' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 固定资产
  if (path === '/api/v1/erp/fixed-assets') {
    response.data = {
      list: [
        { id: 1, asset_code: 'FA001', asset_name: '电脑', category: '电子设备', original_value: 5000, net_value: 3000, status: 'in_use' },
        { id: 2, asset_code: 'FA002', asset_name: '打印机', category: '电子设备', original_value: 2000, net_value: 1200, status: 'in_use' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 审批任务
  if (path === '/api/v1/erp/bpm/tasks') {
    response.data = {
      list: [
        { id: 1, task_name: '销售订单审批', process_name: '销售流程', assignee_name: '张三', status: 'pending', priority: 'high' },
        { id: 2, task_name: '采购订单审批', process_name: '采购流程', assignee_name: '李四', status: 'pending', priority: 'medium' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 质量标准
  if (path === '/api/v1/erp/quality/standards') {
    response.data = {
      list: [
        { id: 1, standard_code: 'QS001', standard_name: '面料质量标准', type: 'product', version: '1.0', status: 'approved' },
        { id: 2, standard_code: 'QS002', standard_name: '染色工艺标准', type: 'process', version: '1.0', status: 'draft' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 通知列表
  if (path === '/api/v1/erp/notifications') {
    response.data = {
      list: [
        { id: 1, title: '系统更新通知', content: '系统将于今晚进行维护更新', notificationType: 'SYSTEM', status: 'UNREAD', createdAt: '2026-05-15 10:00:00' },
        { id: 2, title: '新订单提醒', content: '您有新的销售订单待处理', notificationType: 'INTERNAL', status: 'READ', createdAt: '2026-05-15 09:00:00' }
      ],
      total: 2
    };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 通用列表接口
  if (path.match(/^\/api\/v1\/erp\/\w+$/)) {
    response.data = { list: [], total: 0 };
    res.writeHead(200);
    res.end(JSON.stringify(response));
    return;
  }
  
  // 默认响应
  response.code = 404;
  response.message = 'Not Found';
  res.writeHead(404);
  res.end(JSON.stringify(response));
}

// 创建服务器
const server = http.createServer(handleRequest);

server.listen(PORT, () => {
  console.log(`Mock server running at http://localhost:${PORT}`);
  console.log('Available endpoints:');
  console.log('  POST /api/v1/erp/auth/login');
  console.log('  GET  /api/v1/erp/auth/me');
  console.log('  GET  /api/v1/erp/dashboard/overview');
  console.log('  GET  /api/v1/erp/products');
  console.log('  GET  /api/v1/erp/customers');
  console.log('  GET  /api/v1/erp/suppliers');
  console.log('  GET  /api/v1/erp/warehouses');
  console.log('  GET  /api/v1/erp/sales/orders');
  console.log('  GET  /api/v1/erp/purchase/orders');
  console.log('  GET  /api/v1/erp/inventory/stock');
  console.log('  GET  /api/v1/erp/departments');
  console.log('  GET  /api/v1/erp/roles');
  console.log('  GET  /api/v1/erp/finance/vouchers');
});
