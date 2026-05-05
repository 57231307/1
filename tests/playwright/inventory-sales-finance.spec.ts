import { test, expect } from '@playwright/test';
import { recordResult, generateTestId } from './helpers';

const MODULE = '库存销售采购财务';

let adminToken = '';

test.beforeAll(async ({ request }) => {
  const resp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
    data: { username: 'admin', password: 'admin123456' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await resp.json();
  adminToken = body.data.token;
});

function h() { return { Authorization: `Bearer ${adminToken}`, 'Content-Type': 'application/json' }; }

test.describe('库存管理 (Inventory Stock)', () => {
  test('库存列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/stock', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '库存列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '库存列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('库存摘要查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/stock/summary', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '库存摘要查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '库存摘要查询', 'fail', e.message, Date.now()-s); }
  });

  test('库存交易记录查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/stock/transactions', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '库存交易记录查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '库存交易记录查询', 'fail', e.message, Date.now()-s); }
  });

  test('面料库存查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/stock/fabric', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '面料库存查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '面料库存查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('库存转移 (Inventory Transfer)', () => {
  test('转移列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/transfers', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '转移列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '转移列表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('库存盘点 (Inventory Count)', () => {
  test('盘点列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/counts', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '盘点列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '盘点列表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('库存调整 (Inventory Adjustment)', () => {
  test('调整列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/adjustments', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '调整列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '调整列表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('销售订单 (Sales Orders)', () => {
  test('销售订单列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales/orders', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '销售订单列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '销售订单列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('面料订单列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales/fabric-orders', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '面料订单列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '面料订单列表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('采购订单 (Purchase Orders)', () => {
  test('采购订单列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchases/orders', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '采购订单列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '采购订单列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('采购收货列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchases/receipts', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '采购收货列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '采购收货列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('采购退货列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchases/returns', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '采购退货列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '采购退货列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('采购检验列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchases/inspections', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '采购检验列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '采购检验列表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('财务管理 (Finance)', () => {
  test('付款列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/finance/payments', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '付款列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '付款列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('发票列表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/finance/invoices', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '发票列表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '发票列表查询', 'fail', e.message, Date.now()-s); }
  });

  test('资产负债表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/finance/reports/balance-sheet', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '资产负债表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '资产负债表查询', 'fail', e.message, Date.now()-s); }
  });

  test('利润表查询', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/finance/reports/income-statement', { headers: h() });
      expect(r.status()).toBe(200);
      recordResult(MODULE, '利润表查询', 'pass', undefined, Date.now()-s);
    } catch(e:any) { recordResult(MODULE, '利润表查询', 'fail', e.message, Date.now()-s); }
  });
});

test.describe('性能测试', () => {
  test('API响应时间 < 1秒', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/health');
      const elapsed = Date.now() - s;
      expect(r.status()).toBe(200);
      expect(elapsed).toBeLessThan(1000);
      recordResult(MODULE, 'API响应时间<1s', 'pass', undefined, elapsed);
    } catch(e:any) { recordResult(MODULE, 'API响应时间<1s', 'fail', e.message, Date.now()-s); }
  });

  test('用户列表API响应时间 < 1秒', async ({ request }) => {
    const s = Date.now();
    try {
      const r = await request.get('http://127.0.0.1:8082/api/v1/erp/users', { headers: h() });
      const elapsed = Date.now() - s;
      expect(r.status()).toBe(200);
      expect(elapsed).toBeLessThan(1000);
      recordResult(MODULE, '用户列表API响应时间<1s', 'pass', undefined, elapsed);
    } catch(e:any) { recordResult(MODULE, '用户列表API响应时间<1s', 'fail', e.message, Date.now()-s); }
  });

  test('前端首页加载时间 < 5秒', async ({ page }) => {
    const s = Date.now();
    try {
      await page.goto('http://127.0.0.1:3000', { waitUntil: 'networkidle', timeout: 20000 });
      const elapsed = Date.now() - s;
      expect(elapsed).toBeLessThan(5000);
      recordResult(MODULE, '前端首页加载时间<5s', 'pass', undefined, elapsed);
    } catch(e:any) { recordResult(MODULE, '前端首页加载时间<5s', 'fail', e.message, Date.now()-s); }
  });
});
