import { test, expect } from '../diagnose-fixture';
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
  ensureStockInWarehouse,
  expectBadRequest,
  failureCode,
  type ApiFailureBody,
  APP_ERROR_CODES,
  CSRF_ERROR_CODES,
} from './helpers';

test.describe('异常处理与边界条件', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

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
    const result1 = await apiCall(page, 'PUT', `/purchase/orders/${poId}`, updateData1);

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
      supplier_id: ctx.supplierId,
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
      supplier_id: ctx.supplierId,
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

    // 拒绝判据：HTTP 状态码，或 utils/error.rs:143-148 直出的字符串机器码
    const rejectCode = failureCode(result);
    expect(
      result.status >= 400 ||
        rejectCode === APP_ERROR_CODES.VALIDATION_ERROR ||
        rejectCode === APP_ERROR_CODES.BUSINESS_ERROR
    ).toBeTruthy();
  });

  test('金额精度：小数点后 4 位处理', async ({ page }) => {
    const ctx = getCtx();

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: genCode('PO'),
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 1,
          unit_price: 123.4567,
        },
      ],
    });

    expect(result.status < 500).toBe(true);
  });

  test('未认证请求返回 401', async ({ browser }) => {
    // 用全新 context（无 cookie），不触发登录
    // 注意：playwright.config 全局配置了 storageState，browser.newContext() 默认
    // 继承该登录态（实测被认证为 e2e_admin 返回 200）。必须显式传
    // storageState: undefined 才能拿到真正无 cookie 的未认证 context。
    const context = await browser.newContext({ storageState: undefined });
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

    // 后端应返回 403 + CSRF 机器码（middleware/csrf.rs:234-242 直出体，
    // code 为字符串机器码而非 ApiResponse 的数字码；常量出处 csrf.rs:39-45）
    expect(resp.status() === 403).toBe(true);
    const body: ApiFailureBody = await resp.json();
    expect(failureCode(body), `CSRF 拒绝机器码，实际响应：${JSON.stringify(body)}`).toBe(
      CSRF_ERROR_CODES.MISSING
    );
  });

  test('销售建单不锁库存：可用量不足仅在发货时门控（不预占 reservation）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '缺产品 id，无法验证建单/发货门控').toBeTruthy();
    expect(warehouseId, '缺仓库 id，无法验证建单/发货门控').toBeTruthy();

    // 预置一行带全四维（色号+缸号+批次）的真实库存作为可用量基准，
    // 并取真实仓库编码（ship.rs 按 warehouse_code 查仓）供发货请求使用。
    const stockRow = await ensureStockInWarehouse(page, productId, warehouseId);
    const wh = await apiCallRaw<{ warehouse_code?: string }>(
      page,
      'GET',
      `/warehouses/${warehouseId}`
    );
    const warehouseCode = wh?.warehouse_code;
    expect(warehouseCode, `仓库 ${warehouseId} 应返回 warehouse_code`).toBeTruthy();
    expect(stockRow.batch_no, '库存行应带批次号（四维出库入参来源）').toBeTruthy();
    expect(stockRow.dye_lot_no, '库存行应带缸号（四维出库入参来源）').toBeTruthy();

    // 发货量刻意远超在库可用量（基准 + 巨大缺口）。
    //
    // A5 决策：纺织 ERP 按可用量经营销售，建单【不锁物理库存、不引入 reservation 预占】，
    // 非负/可用量校验只在发货出库时门控。因此建单本身不因可用量不足被拒。
    // ⚠️ 现状后端与此决策相悖：create_order（order_crud.rs:123）在建单事务内调用
    //   lock_inventory（inventory.rs:227），其 build_and_lock_reservations 在无库存行时报
    //   「没有库存记录，无法锁定」（inventory.rs:313）、有行但可用量不足时经
    //   check_stock_sufficient 报 BUSINESS_ERROR「库存不足，无法锁定」（inventory.rs:329-336），
    //   即【建单期硬锁库存】。已作为后端缺陷上报。该缺陷修复前，本用例在 build 步骤会如实变红
    //   （apiCall 遇 code!=200 抛错）——这是暴露缺陷，不得为迁就后端把建单改成"被拒也算过"。
    const created = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [
        {
          product_id: productId,
          quantity: '999999',
          unit_price: '100',
        },
      ],
    });
    const soId = created.data?.id;
    expect(soId, '销售建单应成功：可用量不足不阻断建单（A5 不锁库存）').toBeTruthy();

    // 推进到可发货状态（APPROVED，ship.rs:111 仅此态可发货）
    await apiCall(page, 'POST', `/sales/orders/${soId}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${soId}/approve`);

    // 发货门控：按真实出库四维请求远超可用量的发货量，可用量校验（check_inventory，
    // ship.rs:145 → inventory.rs:153 decide_item_stock）应判 Insufficient/NoStockRows 拒发。
    // 与旧版的两处假绿不同：
    //  ①旧版用 if (soId) 包裹发货断言——建单未返回 id 时整段被跳过 = 空转假绿；现显式断言 soId。
    //  ②旧版 ship 传空 body → 因缺 order_id/warehouse_code/items 被 serde 判 4xx，
    //    "看似被阻断"实为请求格式错误（并非可用量门控），也是假绿；此处传结构合法请求体，
    //    令拒绝只能来自可用量门控（status<500 且携带业务/校验机器码，非 500 崩溃、非 401/403）。
    const ship = await apiCallExpectFail(page, 'POST', `/sales/orders/${soId}/ship`, {
      order_id: soId,
      warehouse_code: warehouseCode,
      items: [
        {
          product_id: productId,
          quantity: 999999,
          color_no: stockRow.color_no,
          batch_no: stockRow.batch_no,
          dye_lot_no: stockRow.dye_lot_no,
        },
      ],
    });
    const shipReject = failureCode(ship);
    const rejectedByGate =
      ship.status >= 400 &&
      ship.status < 500 &&
      (shipReject === APP_ERROR_CODES.BUSINESS_ERROR ||
        shipReject === APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      rejectedByGate,
      `发货应因可用量不足被业务门控拒绝，实际 status=${ship.status} code=${ship.code ?? ''} message=${ship.message ?? ''}`
    ).toBeTruthy();
  });

  test('会计期间关闭后凭证录入应被阻断', async ({ page }) => {
    const periods = await apiCallRaw<{
      items: Array<{ id: number; status: string; period_name: string }>;
    }>(page, 'GET', '/finance/accounting-periods?page=1&page_size=50');

    const closedPeriod = periods.items?.find(p => p.status === 'closed' || p.status === '已关闭');

    if (closedPeriod) {
      const result = await apiCallExpectFail(page, 'POST', '/vouchers', {
        voucher_type: 'general',
        voucher_date: `${closedPeriod.period_name}-15`,
        items: [
          { subject_code: '1001', debit: '100', credit: '0', summary: '测试' },
          { subject_code: '1002', debit: '0', credit: '100', summary: '测试' },
        ],
      });

      expectBadRequest(result);
    }
  });

  test('缸号状态机非法回退被拒', async ({ page }) => {
    const ctx = getCtx();
    const dyeBatchId = ctx.dyeBatchId;

    if (dyeBatchId) {
      const batch = await apiCallRaw<{ status?: string }>(
        page,
        'GET',
        `/production/dye-batches/${dyeBatchId}`
      );
      const status = (batch.status || '').trim();

      // 后端 6 态中文状态机：已完成/已取消为终态，任何流转都应被拒
      if (status === '已完成' || status === '已取消') {
        const result = await apiCallExpectFail(
          page,
          'PUT',
          `/production/dye-batches/${dyeBatchId}`,
          {
            status: '生产中',
          }
        );
        expectBadRequest(result);
      }
    }
  });

  test('超长字符串输入处理', async ({ page }) => {
    const ctx = getCtx();
    const longString = 'A'.repeat(10000);

    const result = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      order_no: genCode('PO'),
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      notes: longString,
      items: [],
    });

    expect(result.status < 500).toBe(true);
  });
});
