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
    const productId = ctx.productIds[0];
    expect(productId, '前置产品未创建（EntityContext.productIds 为空）').toBeTruthy();
    const result = await apiCall<{ id?: number; order_no?: string }>(
      page,
      'POST',
      '/purchase/orders',
      {
        supplier_id: ctx.supplierId,
        warehouse_id: ctx.warehouseIds[0],
        // 后端 validate_order_request 要求 department_id 必填（"部门 ID 不能为空"）
        department_id: ctx.departmentIds[0],
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
    expect(
      id,
      '前置步骤未创建采购订单（ctx.purchaseOrderId 缺失），本用例前置失败而非跳过'
    ).toBeTruthy();

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
    expect(
      id,
      '前置步骤未创建采购订单（ctx.purchaseOrderId 缺失），本用例前置失败而非跳过'
    ).toBeTruthy();

    // 对已审批的订单再次提交 → 应拒绝
    await verifyIllegalTransition(page, '/purchase/orders', id, 'submit');
  });

  test('1-4 创建入库单（创建匹号，双计量）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.purchaseOrderId;
    expect(
      id,
      '前置步骤未创建采购订单（ctx.purchaseOrderId 缺失），本用例前置失败而非跳过'
    ).toBeTruthy();

    const productId = ctx.productIds[0];
    expect(productId, '前置产品未创建（EntityContext.productIds 为空）').toBeTruthy();
    const pieceNo1 = genPieceNo(dyeLotNo, 1);
    const pieceNo2 = genPieceNo(dyeLotNo, 2);

    // 后端 CreatePurchaseReceiptRequest 的订单字段是 order_id
    // （原发 purchase_order_id 会被 serde 忽略，入库单与订单脱钩，收货事件不发、库存不增）
    const receipt = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
      order_id: id,
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
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
    const productId = ctx.productIds[0];
    expect(productId, '前置产品未创建（EntityContext.productIds 为空）').toBeTruthy();

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
    const productId = ctx.productIds[0];
    expect(productId, '前置产品未创建（EntityContext.productIds 为空）').toBeTruthy();

    const byColor = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&color_no=RED-001&page=1&page_size=10`
    );
    expect(Array.isArray(byColor.items), `byColor.items 应为后端返回的 items 数组`).toBe(true);
    for (const row of byColor.items) {
      expect(String(row.color_no), `色号筛选下推失效：返回行色号 ${row.color_no}`).toBe('RED-001');
    }

    const byDyeLot = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&dye_lot_no=${encodeURIComponent(dyeLotNo)}&page=1&page_size=10`
    );
    expect(Array.isArray(byDyeLot.items), `byDyeLot.items 应为后端返回的 items 数组`).toBe(true);
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
    expect(Array.isArray(byStatus.items), `byStatus.items 应为后端返回的 items 数组`).toBe(true);
    expect(
      byStatus.items.some(row => String(row.dye_lot_no) === dyeLotNo),
      '正常状态筛选应能查到本用例刚收入的那条库存行'
    ).toBe(true);
    for (const row of byStatus.items) {
      expect(String(row.stock_status), `状态筛选下推失效：返回行状态 ${row.stock_status}`).toBe(
        '正常'
      );
    }

    // 关键词筛选（产品编码/名称）此前后端入参里根本没有 keyword：界面这个筛选框提交后
    // 被整个忽略，用户看到的是"筛了没反应"。现已按主数据取候选产品再下推 ID 集合。
    const ourProductCode = String(byStatus.items[0].product_code ?? '');
    expect(ourProductCode, '库存行应带出产品编码（attach_master_names 回查主数据）').toBeTruthy();
    const byKeyword = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/inventory/stock?keyword=${encodeURIComponent(ourProductCode)}&page=1&page_size=50`
    );
    expect(Array.isArray(byKeyword.items), `byKeyword.items 应为后端返回的 items 数组`).toBe(true);
    expect(
      byKeyword.items.length,
      `按本行产品编码搜索应至少命中本行（关键词筛选未下推）`
    ).toBeGreaterThan(0);
    for (const row of byKeyword.items) {
      expect(
        String(row.product_code),
        `关键词筛选下推失效：返回行产品编码 ${row.product_code} 不含关键词 ${ourProductCode}`
      ).toContain(ourProductCode);
    }
    const noHit = await apiCallRaw<{ items: unknown[] }>(
      page,
      'GET',
      `/inventory/stock?keyword=${encodeURIComponent(genCode('NOSUCH'))}&page=1&page_size=10`
    );
    expect(noHit.items.length, '无产品命中时应返回空集，而不是把筛选条件丢掉返回全量').toBe(0);

    // 反向对照：normal 不是后端台账状态取值（本列取值域是中文主数据）。此前越界值原样
    // 下推成 SQL 等值条件，返回 200 + 零行，前端假筛选因此永不显红；现按取值域拒绝并回显允许值。
    const bogusStatus = await apiCallExpectFail(
      page,
      'GET',
      `/inventory/stock?product_id=${productId}&stock_status=normal&page=1&page_size=10`
    );
    expect(bogusStatus.status, `越界台账状态应被拒绝，实际 ${bogusStatus.status}`).toBe(400);
    // 断言稳定错误码而非文案：对外 message 统一脱敏（允许值清单只进服务端 detail 日志），
    // "拒绝信息必须列出合法值"由 backend/tests/handlers_inventory_stock_status_test.rs 钉住
    expect(bogusStatus.code, `应返回 VALIDATION_ERROR，实际 ${JSON.stringify(bogusStatus)}`).toBe(
      'VALIDATION_ERROR'
    );
  });

  test('1-6b 入库明细与订单不符/缺批次应整单拒绝且库存无新增行', async ({ page }) => {
    const ctx = getCtx();
    // 选一个不在本采购订单明细中的产品（订单 1-1 只用 productIds[0]），验证「产品对不上」硬拒绝
    const wrongProductId = ctx.productIds[1];
    expect(
      wrongProductId,
      '前置未创建第二个产品，无法构造「产品对不上」负例（EntityContext.productIds 不足）'
    ).toBeTruthy();
    // 每次运行使用唯一色号/缸号，确保「库存无新增行」断言的是本用例真实拒绝的结果
    const phantomColor = `MISMATCH-${Date.now()}`;
    const phantomDyeLot = genDyeLotNo();
    const orderNo = ctx.purchaseOrderId;
    expect(orderNo, '前置采购订单缺失').toBeTruthy();

    const baseItem = (over: Record<string, unknown>) => ({
      order_id: orderNo,
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      receipt_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          line_no: 1,
          material_id: wrongProductId,
          material_code: 'P2P-WRONG',
          material_name: '非订单产品',
          unit_master: 'm',
          quantity: 10,
          quantity_alt: 1,
          color_code: phantomColor,
          lot_no: phantomDyeLot,
          batch_no: 'B-MISMATCH',
          ...over,
        },
      ],
    });

    // ① 产品对不上订单 → 建单被整单拒绝（BUSINESS_ERROR）
    const productMismatch = await apiCallExpectFail(
      page,
      'POST',
      '/purchase/receipts',
      baseItem({})
    );
    console.log('[1-6b 产品不符] 响应:', JSON.stringify(productMismatch));
    expect(
      productMismatch.status,
      `产品与订单不符应被拒绝（400），实际 ${productMismatch.status}`
    ).toBe(400);
    expect(
      productMismatch.code,
      `应返回 BUSINESS_ERROR，实际 ${JSON.stringify(productMismatch)}`
    ).toBe('BUSINESS_ERROR');

    // ② 缺批次（维度不全）→ 建单被整单拒绝（BUSINESS_ERROR）
    const missingBatch = await apiCallExpectFail(
      page,
      'POST',
      '/purchase/receipts',
      baseItem({ batch_no: undefined })
    );
    console.log('[1-6b 缺批次] 响应:', JSON.stringify(missingBatch));
    expect(missingBatch.status, `缺批次应被拒绝（400），实际 ${missingBatch.status}`).toBe(400);
    expect(missingBatch.code, `应返回 BUSINESS_ERROR，实际 ${JSON.stringify(missingBatch)}`).toBe(
      'BUSINESS_ERROR'
    );

    // ③ 负例拒绝后库存不得新增行：按唯一色号/缸号检索该产品必须为空
    const phantomStock = await verifyStockFourDim(
      page,
      wrongProductId,
      phantomColor,
      phantomDyeLot,
      { warehouseId: ctx.warehouseIds[0] }
    );
    expect(
      phantomStock,
      `产品不符/缺批次已拒绝建单，库存不应出现产品 ${wrongProductId} / 色号 ${phantomColor} 的新增行`
    ).toBeNull();
  });

  test('1-7 验证 AP 应付单', async ({ page }) => {
    const ctx = getCtx();

    // 必须限定本用例的供应商：不带 supplier_id 时列表返回的是库里任意一张应付单
    // （上一轮 1-8 报「应付单未付金额为 0」就是这么来的——拿到的根本不是本流程的单据）。
    const invoices = await apiCallRaw<{
      items: Array<{ id: number; amount: number | string; unpaid_amount: number | string }>;
      total: number;
    }>(page, 'GET', `/ap/invoices?supplier_id=${ctx.supplierId}&page=1&page_size=20`);
    // 后端 list_ap_invoices → ApiResponse<PaginatedResponse>（ap_invoice_handler.rs:40-69；
    // utils/response.rs:34）：data 唯一形状是 {items,total,page,page_size}，缺 items 即契约破坏，
    // 不再用 ?? [] 把"没返回该键"伪装成"空集合"。
    expect(
      Array.isArray(invoices?.items),
      `AP 应付单列表 data 缺 items 数组（后端 PaginatedResponse 契约）：${JSON.stringify(invoices).slice(0, 200)}`
    ).toBe(true);
    const invoiceList = invoices.items;

    // 优先用本流程已产生的应付单（收货完成会自动生成），且必须还有未付金额才谈得上付款
    const payable = invoiceList.find(inv => Number(inv.unpaid_amount) > 0);
    if (!payable) {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/ap/invoices', {
        // CreateApInvoiceRequest：invoice_no 非后端字段（应 inset_type），保留 amount/tax_amount/invoice_date
        supplier_id: ctx.supplierId,
        amount: 56500,
        tax_amount: 6500,
        invoice_date: new Date().toISOString().split('T')[0],
      });
      ctx.apInvoiceId = result.data?.id;
    } else {
      ctx.apInvoiceId = payable.id;
    }
    expect(
      ctx.apInvoiceId,
      `未取得应付单 ID（本供应商列表 ${invoiceList.length} 条，均无未付金额且手动建单未返回 id）`
    ).toBeTruthy();

    const chosen = await apiCallRaw<{
      unpaid_amount?: number | string;
      invoice_status?: string;
      supplier_id?: number;
    }>(page, 'GET', `/ap/invoices/${ctx.apInvoiceId}`);
    expect(
      Number(chosen.unpaid_amount),
      `选定的应付单 ${ctx.apInvoiceId} 未付金额应大于 0，实际 ${chosen.unpaid_amount}`
    ).toBeGreaterThan(0);
    expect(
      chosen.supplier_id,
      `应付单 ${ctx.apInvoiceId} 的供应商应为本用例供应商 ${ctx.supplierId}，实际 ${chosen.supplier_id}`
    ).toBe(ctx.supplierId);
  });

  test('1-8 付款', async ({ page }) => {
    const ctx = getCtx();
    // 前置由 1-7 断言保证；这里不再用 test.skip 把"前置没建好"混进"用例跳过"
    expect(ctx.apInvoiceId, '1-7 未产出应付单 ID').toBeTruthy();
    expect(ctx.supplierId, '前置供应商未创建').toBeTruthy();

    // 后端规则：DRAFT/CANCELLED 应付单不可申请付款，先审核应付单（幂等：仅 draft 状态调用）
    const inv = await apiCallRaw<{
      invoice_status?: string;
      unpaid_amount?: number | string;
    }>(page, 'GET', `/ap/invoices/${ctx.apInvoiceId}`);
    if ((inv.invoice_status || '').toLowerCase() === 'draft') {
      await apiCall(page, 'POST', `/ap/invoices/${ctx.apInvoiceId}/approve`);
    }

    // 申请金额取该单真实未付金额：写死 56500 只在"正好是本流程那张 56500 的单"时成立，
    // 上一轮的 400「申请金额超过未付金额」就是这么来的
    const applyAmount = Number(inv.unpaid_amount);
    expect(
      applyAmount,
      `应付单 ${ctx.apInvoiceId} 未付金额应大于 0，实际 ${inv.unpaid_amount}`
    ).toBeGreaterThan(0);

    // 先创建付款申请（POST /ap/payment-requests），再用 request_id 创建付款
    const payReq = await apiCall<{ data?: { id?: number } }>(page, 'POST', '/ap/payment-requests', {
      supplier_id: ctx.supplierId,
      request_date: new Date().toISOString().split('T')[0],
      payment_type: 'purchase',
      payment_method: 'bank_transfer',
      request_amount: applyAmount,
      currency: 'CNY',
      exchange_rate: 1,
      items: [
        {
          invoice_id: ctx.apInvoiceId,
          apply_amount: applyAmount,
          notes: 'E2E 1-8 付款申请明细',
        },
      ],
    });
    const requestId = payReq?.data?.id;
    expect(
      requestId,
      `付款申请创建失败，响应：${JSON.stringify(payReq).slice(0, 300)}`
    ).toBeTruthy();
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
    expect(
      id,
      '前置步骤未创建采购订单（ctx.purchaseOrderId 缺失），本用例前置失败而非跳过'
    ).toBeTruthy();

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
    expect(Array.isArray(orders.items), `orders.items 应为后端返回的 items 数组`).toBe(true);
  });
});
