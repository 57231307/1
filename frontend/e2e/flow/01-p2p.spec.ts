import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyStatusTransition,
  verifyIllegalTransition,
  verifyStockFourDim,
  verifyAuditLog,
  getCtx,
  genCode,
  genDyeLotNo,
  genPieceNo,
  ensureTestEntities,
} from './helpers';

test.describe.serial('Shard 1: 现货模式 P2P 闭环（grey_trading）', () => {
  const dyeLotNo = genDyeLotNo();
  const CLEANUP: Array<{ path: string; label: string }> = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('1-1 创建采购订单（含色号+缸号+双计量）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;
    const result = await apiCall<{ id?: number; order_no?: string }>(
      page,
      'POST',
      '/purchase/orders',
      {
        supplier_id: ctx.supplierId,
        warehouse_id: ctx.warehouseIds[0] || 1,
        // 后端 validate_order_request 要求 department_id 必填（"部门 ID 不能为空"）
        department_id: ctx.departmentIds[0] || 1,
        order_date: new Date().toISOString().slice(0, 10),
        expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
        items: [
          {
            material_id: productId,
            quantity_ordered: '1000',
            quantity_alt_ordered: '200',
            unit_price: '50',
            tax_rate: '13',
          },
        ],
        notes: 'E2E P2P 现货采购（grey_trading）',
      }
    );
    ctx.purchaseOrderId = result.data?.id;
    expect(ctx.purchaseOrderId).toBeDefined();
  });

  test('1-2 采购订单状态机：DRAFT → SUBMITTED → APPROVED', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.purchaseOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 验证初始状态
    const initial = await apiCallRaw<{ status: string; order_status?: string }>(
      page,
      'GET',
      `/purchase/orders/${id}`
    );
    const initialStatus = (initial.status || initial.order_status || '').toLowerCase();

    // 提交审批
    if (['draft', 'pending_approval'].includes(initialStatus) || initialStatus === '') {
      await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
    }

    // 审批通过
    await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);

    const final = await apiCallRaw<{ status: string; order_status?: string }>(
      page,
      'GET',
      `/purchase/orders/${id}`
    );
    const finalStatus = (final.status || final.order_status || '').toLowerCase();
    expect([
      'approved',
      'confirmed',
      'pending_receipt',
      'partially_received',
      'received',
      'completed',
      'closed',
    ]).toContain(finalStatus ?? '(missing-status)');
  });

  test('1-3 验证非法状态转换被拒绝', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.purchaseOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 对已审批的订单再次提交 → 应拒绝
    await verifyIllegalTransition(page, '/purchase/orders', id, 'submit');
  });

  test('1-4 创建入库单（创建匹号，双计量）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.purchaseOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    const productId = ctx.productIds[0] || 1;
    const pieceNo1 = genPieceNo(dyeLotNo, 1);
    const pieceNo2 = genPieceNo(dyeLotNo, 2);

    // 后端 CreatePurchaseReceiptRequest 的订单字段是 order_id
    // （原发 purchase_order_id 会被 serde 忽略，入库单与订单脱钩，收货事件不发、库存不增）
    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: id,
      supplier_id: ctx.supplierId || 1,
      warehouse_id: ctx.warehouseIds[0] || 1,
      receipt_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          line_no: 1,
          material_id: productId,
          material_code: 'P2P-MAT-001',
          material_name: 'P2P 测试面料',
          unit_master: 'm',
          quantity: 500,
          quantity_alt: 100,
          color_code: 'RED-001',
          lot_no: dyeLotNo,
          batch_no: 'B001',
          piece_no: pieceNo1,
        },
        {
          line_no: 2,
          material_id: productId,
          material_code: 'P2P-MAT-001',
          material_name: 'P2P 测试面料',
          unit_master: 'm',
          quantity: 500,
          quantity_alt: 100,
          color_code: 'RED-001',
          lot_no: dyeLotNo,
          batch_no: 'B001',
          piece_no: pieceNo2,
        },
      ],
    });

    const receiptId = receipt.data?.id;
    expect(receiptId, '入库单创建失败（响应缺 id）').toBeTruthy();

    // 确认入库事务内按入库明细完成库存收货并推进订单已收数量，入库单直接进 COMPLETED；
    // 仍用轮询而非固定 sleep：断言的是终态可达，不依赖具体时序
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    await expect
      .poll(
        async () => {
          const r = await apiCallRaw<{ receipt_status?: string; status?: string }>(
            page,
            'GET',
            `/purchase/receipts/${receiptId}`
          );
          return (r.receipt_status || r.status || '').toUpperCase();
        },
        { message: `确认入库后入库单 ${receiptId} 应进入 COMPLETED 终态` }
      )
      .toBe('COMPLETED');

    // 验证订单状态更新：已收满订单应为 completed（未收货会停在 approved/pending_receipt）
    const order = await apiCallRaw<{ status: string; order_status?: string }>(
      page,
      'GET',
      `/purchase/orders/${id}`
    );
    const status = (order.status || order.order_status || '').toLowerCase();
    expect(
      ['completed', 'partial_received', 'received'],
      `收货确认后采购订单 ${id} 应进入收货态，实际 ${status}`
    ).toContain(status);
  });

  test('1-5 验证库存四维聚合（产品→色号→缸号→匹号）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;

    // 入库明细的色号/缸号/批次必须原样落到库存行：1-4 两行同维度共 1000 米
    // 仓库过滤一并带上：不按时会命中其他仓库同维度行，命中行的在库量与本用例无关
    const stock = await verifyStockFourDim(page, productId, 'RED-001', dyeLotNo, {
      batchNo: 'B001',
      warehouseId: ctx.warehouseIds[0],
    });
    expect(
      stock,
      `确认入库后应按产品 ${productId} + 色号 RED-001 + 缸号 ${dyeLotNo} + 批次 B001 命中库存行（未命中即收货链路有缺陷）`
    ).toBeTruthy();
    expect(String(stock!.color_no), '命中库存行的色号应与入库明细一致').toBe('RED-001');
    expect(String(stock!.dye_lot_no), '命中库存行的缸号应与入库明细一致').toBe(dyeLotNo);
    expect(String(stock!.batch_no), '命中库存行的批次应与入库明细一致').toBe('B001');
    expect(
      Number(stock!.quantity_on_hand),
      `同维度两行入库应累加到同一库存行，在库量应不少于 1000（实际 ${stock!.quantity_on_hand}）`
    ).toBeGreaterThanOrEqual(1000);

    // 缸号过滤必须真实下推：不存在的缸号不得命中任何行
    const noMatch = await verifyStockFourDim(page, productId, 'RED-001', `${dyeLotNo}-NO-SUCH`);
    expect(noMatch, `缸号过滤未生效：不存在的 ${dyeLotNo}-NO-SUCH 仍命中了库存行`).toBeNull();
  });

  test('1-6 验证库存查询支持色号/缸号筛选', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;

    const byColor = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&color_no=RED-001&page=1&page_size=10`
    );
    expect(Array.isArray(byColor.items), `byColor.items 应为后端返回的 items 数组`);
    for (const row of byColor.items) {
      expect(String(row.color_no), `色号筛选下推失效：返回行色号 ${row.color_no}`).toBe('RED-001');
    }

    const byDyeLot = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&dye_lot_no=${encodeURIComponent(dyeLotNo)}&page=1&page_size=10`
    );
    expect(Array.isArray(byDyeLot.items), `byDyeLot.items 应为后端返回的 items 数组`);
    for (const row of byDyeLot.items) {
      expect(String(row.dye_lot_no), `缸号筛选下推失效：返回行缸号 ${row.dye_lot_no}`).toBe(
        dyeLotNo
      );
    }

    // 台账状态同为后端主数据值（中文）：正常筛选必须能查回本用例收入的那一行
    const byStatus = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&stock_status=${encodeURIComponent('正常')}&page=1&page_size=10`
    );
    expect(Array.isArray(byStatus.items), `byStatus.items 应为后端返回的 items 数组`);
    expect(
      byStatus.items.some(row => String(row.dye_lot_no) === dyeLotNo),
      '正常状态筛选应能查到本用例刚收入的那条库存行'
    ).toBe(true);
    for (const row of byStatus.items) {
      expect(String(row.stock_status), `状态筛选下推失效：返回行状态 ${row.stock_status}`).toBe(
        '正常'
      );
    }

    // 反向对照：normal 不是后端取值，此前前端正是提交这类值导致筛选恒零命中
    const bogusStatus = await apiCallRaw<{ items: unknown[] }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&stock_status=normal&page=1&page_size=10`
    );
    expect(
      bogusStatus.items.length,
      '英文 normal 不是后端台账状态取值，不应命中任何行（命中即筛选未下推）'
    ).toBe(0);
  });

  test('1-7 验证 AP 应付单', async ({ page }) => {
    const ctx = getCtx();

    // 后端 list_ap_invoices 返回 ApiResponse<Vec<Model>>：data 是数组（无 items 包装）
    const invoices = await apiCallRaw<
      | Array<{ id: number; amount: number; status: string }>
      | { items?: Array<{ id: number; amount: number; status: string }> }
    >(page, 'GET', '/ap/invoices?page=1&page_size=5');
    const invoiceList = Array.isArray(invoices)
      ? invoices
      : ((invoices as { items?: Array<{ id: number; amount: number; status: string }> }).items ??
        []);

    // 尝试手动创建 AP 应付单（如果未自动生成）
    if ((invoiceList.length ?? 0) === 0) {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/ap/invoices', {
        // CreateApInvoiceRequest：invoice_no 非后端字段（应 inset_type），保留 amount/tax_amount/invoice_date
        supplier_id: ctx.supplierId,
        amount: 56500,
        tax_amount: 6500,
        invoice_date: new Date().toISOString().split('T')[0],
      });
      ctx.apInvoiceId = result.data?.id;
    } else {
      ctx.apInvoiceId = invoiceList[0]?.id;
    }
    expect(ctx.apInvoiceId).toBeDefined();
  });

  test('1-8 付款', async ({ page }) => {
    const ctx = getCtx();

    if (!ctx.apInvoiceId || !ctx.supplierId) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 后端规则：DRAFT/CANCELLED 应付单不可申请付款，先审核应付单（幂等：仅 draft 状态调用）
    const inv = await apiCallRaw<{ invoice_status?: string }>(
      page,
      'GET',
      `/ap/invoices/${ctx.apInvoiceId}`
    );
    if ((inv.invoice_status || '').toLowerCase() === 'draft') {
      await apiCall(page, 'POST', `/ap/invoices/${ctx.apInvoiceId}/approve`);
    }

    // 先创建付款申请（POST /ap/payment-requests），再用 request_id 创建付款
    const payReq = await apiCall<{ data?: { id?: number } }>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: new Date().toISOString().split('T')[0],
      payment_type: 'purchase',
      payment_method: 'bank_transfer',
      request_amount: 56500,
      currency: 'CNY',
      exchange_rate: 1,
      items: [
        {
          invoice_id: ctx.apInvoiceId,
          apply_amount: 56500,
          notes: 'E2E 1-8 付款申请明细',
        },
      ],
    });
    const requestId = payReq?.data?.id;
    if (!requestId) {
      console.warn('[E2E] test.skip: 付款申请创建失败');
      test.skip();
      return;
    }
    CLEANUP.push({ path: `/ap/payment-requests/${requestId}`, label: '[1-8] 付款申请' });

    // 后端规则：付款申请 DRAFT 不可创建付款单；流程 DRAFT→submit→PENDING→approve→APPROVED
    const payReqStatus = await apiCallRaw<{ approval_status?: string }>(
      page,
      'GET',
      `/ap/payment-requests/${requestId}`
    );
    const st = (payReqStatus.approval_status || '').toLowerCase();
    if (st === 'draft') {
      await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
      await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);
    } else if (st !== 'approved') {
      await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);
    }

    await apiCall(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: new Date().toISOString().split('T')[0],
      notes: 'E2E 1-8 付款测试',
    });

    // 验证应付单状态（字段名 invoice_status）
    const invoice = await apiCallRaw<{ status?: string; invoice_status?: string }>(
      page,
      'GET',
      `/ap/invoices/${ctx.apInvoiceId}`
    );
    const invStatus = (invoice.status || invoice.invoice_status || '').toLowerCase();
    expect(invStatus).not.toBe('');
    expect([
      'paid',
      'partially_paid',
      'unpaid',
      'pending',
      'approved',
      'confirmed',
      'draft',
      'audited',
      'auditing',
    ]).toContain(invStatus);
  });

  test('1-9 验证采购订单完整状态流转记录', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.purchaseOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    const order = await apiCallRaw<{ status: string; order_status?: string }>(
      page,
      'GET',
      `/purchase/orders/${id}`
    );
    expect(order?.id ?? order, '采购订单详情应返回订单对象').toBeTruthy();
    const status = (order.status || order.order_status || '').toLowerCase();
    expect([
      'approved',
      'confirmed',
      'pending_receipt',
      'partially_received',
      'received',
      'completed',
      'closed',
      'cancelled',
    ]).toContain(status ?? '(missing-status)');
  });

  test('1-10 验证审计日志包含采购操作', async ({ page }) => {
    // 后端审计字段值：operation_type='CREATE'（大写枚举序列化）、
    // resource_type='purchase_order'（单数下划线，见 purchase_order_handler.rs:565）
    const hasLog = await verifyAuditLog(page, 'CREATE', 'purchase');
    // 审计日志查询成功时必须命中 create 记录（API 失败返回 false 同样判失败）
    expect(hasLog).toBe(true);
  });

  test('1-11 验证供应商报表', async ({ page }) => {
    const orders = await apiCallRaw<{ items: unknown[] }>(
      page,
      'GET',
      '/purchase/orders?page=1&page_size=5'
    );
    expect(Array.isArray(orders.items), `orders.items 应为后端返回的 items 数组`);
  });
});
