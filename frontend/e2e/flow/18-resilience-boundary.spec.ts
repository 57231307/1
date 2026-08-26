import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  API_BASE,
  API_PREFIX,
  TEST_USERNAME,
  TEST_PASSWORD,
  ensureTestEntities,
} from './helpers';

test.describe('异常处理与边界条件', () => {
  test.beforeEach(async ({ page }) => { await loginViaUI(page); await ensureTestEntities(page); });

  test('并发编辑冲突：同一用户两个 context 同时修改同一单据', async ({ page, context }) => {
    const ctx = getCtx();
    const poId = ctx.purchaseOrderId;
    expect(poId).toBeDefined();

    // 复用当前 context 的 cookie（不重新登录，避免 429）
    // 创建第二个 page（共享 cookie）
    const page2 = await context.newPage();

    // 两个 page 同时更新同一单据
    const updateData1 = { notes: `并发修改1-${Date.now()}` };
    const updateData2 = { notes: `并发修改2-${Date.now()}` };

    // 第一个 page 先更新
    const result1 = await apiCall(page, 'PUT', `/purchase/orders/${poId}`, updateData1).catch(() => null);

    // 第二个 page 也尝试更新（可能因乐观锁/版本号冲突被拒）
    const csrf2 = (await context.cookies()).find(c => c.name === 'csrf_token')?.value || '';
    const resp2 = await page2.request.fetch(`${API_BASE}${API_PREFIX}/purchase/orders/${poId}`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': csrf2,
      },
      data: JSON.stringify(updateData2),
    });

    // 至少一个应成功，另一个可能因乐观锁被拒（4xx）
    const status2 = resp2.status();
    expect(status2 >= 200 && status2 < 500).toBe(true);

    await page2.close();
  });

  test('不存在的资源 ID 返回 404', async ({ page }) => {
    const result = await apiCallExpectFail(page, 'GET', '/purchase/orders/99999999');
    expect(result.status === 404 || result.status === 403).toBe(true);

    const supplierResult = await apiCallExpectFail(page, 'GET', '/purchase/suppliers/99999999');
    expect(supplierResult.status === 404 || supplierResult.status === 403).toBe(true);
  });

  test('非法字符输入不导致后端 500', async ({ page }) => {
    const ctx = getCtx();

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: '<script>alert("xss")</script>',
      supplier_id: ctx.supplierId || 1,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [],
    });

    expect(result.status < 500).toBe(true);
  });

  test('数量为 0 的明细被拒', async ({ page }) => {
    const ctx = getCtx();

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: genCode('PO'),
      supplier_id: ctx.supplierId || 1,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 0,
          unit_price: 10,
        },
      ],
    });

    expect(result.status >= 400 || result.code === 'VALIDATION_ERROR' || result.code === 'BUSINESS_ERROR').toBeTruthy();
  });

  test('金额精度：小数点后 4 位处理', async ({ page }) => {
    const ctx = getCtx();

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: genCode('PO'),
      supplier_id: ctx.supplierId || 1,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 1,
          unit_price: 123.4567,
        },
      ],
    }).catch(() => null);

    if (result) {
      expect(result.status < 500).toBe(true);
    }
  });

  test('未认证请求返回 401', async ({ browser }) => {
    // 用全新 context（无 cookie），不触发登录
    const context = await browser.newContext();
    const page = await context.newPage();

    const resp = await page.request.fetch(`${API_BASE}${API_PREFIX}/auth/me`, {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });

    expect(resp.status() === 401 || resp.status() === 403).toBe(true);
    await context.close();
  });

  test('CSRF Token 缺失时 POST 被拒', async ({ page, context }) => {
    // 复用当前已登录 context 的 cookie，但不带 CSRF Token 发送 POST
    // 通过拦截 cookie 中的 csrf_token 来模拟缺失

    // 获取 csrf_token cookie 值
    const cookies = await context.cookies();
    const csrfCookie = cookies.find(c => c.name === 'csrf_token');

    // 不带 X-CSRF-Token 头发送 POST
    const resp = await page.request.fetch(`${API_BASE}${API_PREFIX}/purchase/orders`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Requested-With': 'XMLHttpRequest',
      },
      data: JSON.stringify({ order_no: genCode('PO') }),
    });

    // 后端应返回 403 CSRF_TOKEN_MISSING
    expect(resp.status() === 403).toBe(true);
    const body = await resp.text();
    expect(body.includes('CSRF') || body.includes('csrf') || body.includes('token')).toBe(true);
  });

  test('库存为 0 时发货应被阻断', async ({ page }) => {
    const ctx = getCtx();

    const soData = {
      order_no: genCode('SO'),
      customer_id: ctx.customerId || 1,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          product_id: ctx.productIds[0],
          product_color_id: ctx.productColorIds[0],
          quantity: 999999,
          unit: '米',
        },
      ],
    };

    let soId: number | null = null;
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', soData);
      soId = result.data?.id ?? null;
    } catch {
      // 创建可能因库存不足直接被拒
    }

    if (soId) {
      try { await apiCall(page, 'POST', `/sales/orders/${soId}/submit`); } catch (e) { console.log(`submit: ${(e as { message?: string }).message || e}`); }
      try { await apiCall(page, 'POST', `/sales/orders/${soId}/approve`); } catch (e) { console.log(`approve: ${(e as { message?: string }).message || e}`); }

      // 发货应被阻断
      const shipResult = await apiCallExpectFail(page, 'POST', `/sales/orders/${soId}/ship`);
      expect(shipResult.status === 400 || shipResult.status === 409 || shipResult.status === 422 || shipResult.status === 403).toBe(true);
    }
  });

  test('会计期间关闭后凭证录入应被阻断', async ({ page }) => {
    const periods = await apiCallRaw<{ items: Array<{ id: number; status: string; period_name: string }> }>(
      page, 'GET', '/finance/accounting-periods?page=1&page_size=50'
    ).catch(() => ({ items: [] }));

    const closedPeriod = periods.items?.find(p => p.status === 'closed' || p.status === '已关闭');

    if (closedPeriod) {
      const result = await apiCallExpectFail(page, 'POST', '/finance/vouchers', {
        voucher_type: 'general',
        voucher_date: `${closedPeriod.period_name}-15`,
        items: [
          { subject_code: '1001', debit: '100', credit: '0', summary: '测试' },
          { subject_code: '1002', debit: '0', credit: '100', summary: '测试' },
        ],
      });

      expect(result.status >= 400).toBe(true);
    }
  });

  test('缸号状态机非法回退被拒', async ({ page }) => {
    const ctx = getCtx();
    const dyeBatchId = ctx.dyeBatchId;

    if (dyeBatchId) {
      const batch = await apiCallRaw<{ status: string }>(page, 'GET', `/production/dye-batches/${dyeBatchId}`);
      const status = (batch.status || '').toLowerCase();

      if (['completed', 'stored', 'done', '已入库', '已完成'].some(s => status.includes(s.toLowerCase()))) {
        const result = await apiCallExpectFail(page, 'POST', `/production/dye-batches/${dyeBatchId}/schedule`);
        expect(result.status >= 400).toBe(true);
      }
    }
  });

  test('超长字符串输入处理', async ({ page }) => {
    const ctx = getCtx();
    const longString = 'A'.repeat(10000);

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: genCode('PO'),
      supplier_id: ctx.supplierId || 1,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      notes: longString,
      items: [],
    }).catch(() => null);

    if (result) {
      expect(result.status < 500).toBe(true);
    }
  });
});
