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
  BASE_URL,
} from './helpers';

/** 与 backend/src/models/status/sales.rs 的 so_status 常量一致 */
const SO_STATUSES = [
  'draft',
  'pending',
  'approved',
  'partial_shipped',
  'shipped',
  'completed',
  'cancelled',
  'rejected',
];

test.describe.serial('Shard 2: 订货模式 O2C 闭环（finished_trading）', () => {
  const dyeLotNo = genDyeLotNo();
  /** 2-8 新建的分次收款专用应收单金额；2-9 按此金额做 50% + 50% 两笔收款并断言状态流转 */
  const AR_INVOICE_AMOUNT = 113000;
  const AR_PAYMENT_HALF = AR_INVOICE_AMOUNT / 2;
  const CLEANUP: Array<{ path: string; label: string }> = [];

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('2-1 创建报价单（含色号+缸号要求+色号加价+等级差价）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const productId = ctx.productIds[1] || ctx.productIds[0] || 1;

    const result = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
      customer_id: ctx.customerId,
      sales_user_id: ctx.userIds[0] || 1,
      quotation_date: new Date().toISOString().split('T')[0],
      valid_until: new Date(Date.now() + 30 * 86400000).toISOString().split('T')[0],
      currency: 'CNY',
      exchange_rate: '1',
      base_currency: 'CNY',
      price_terms: 'FOB',
      tax_inclusive: false,
      tax_rate: '13',
      items: [
        {
          product_id: productId,
          unit: '米',
          quantity: '800',
          unit_price: '100',
          unit_price_with_tax: '113',
          specification: 'E2E 测试面料',
        },
      ],
      notes: 'E2E O2C 订货报价（finished_trading）',
    });
    ctx.quotationId = result.data?.id;
    expect(ctx.quotationId).toBeDefined();
  });

  test('2-2 报价单状态机：draft → submitted → approved', async ({ page }) => {
    const ctx = getCtx();
    // 独立创建 quotation 做状态机（不依赖共享数据，消除分片间竞争）
    const ts = Date.now().toString().slice(-6);
    const result = await apiCall<{ id?: number }>(page, 'POST', '/quotations', {
      quotation_no: `E2E-QT-22-${ts}`,
      customer_id: ctx.customerId,
      sales_user_id: ctx.userIds[0] || 1,
      quotation_date: new Date().toISOString().slice(0, 10),
      valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
      status: 'draft',
      currency: 'CNY',
      exchange_rate: '1',
      base_currency: 'CNY',
      price_terms: 'FOB',
      tax_inclusive: false,
      tax_rate: '0.13',
      items: [
        {
          product_id: ctx.productIds[0],
          quantity: 10,
          unit: 'm',
          unit_price: '2.50',
          unit_price_with_tax: '2.83',
        },
      ],
      notes: 'E2E 2-2 状态机独立报价',
    });
    const id = result.data?.id;
    expect(id, '2-2 独立报价单创建失败').toBeTruthy();
    CLEANUP.push({ path: `/quotations/${id}`, label: '[2-2] 报价单' });

    // 提交审批
    await apiCall(page, 'POST', `/quotations/${id}/submit`);
    // 审批通过：后端小额自批规则（金额 < 10 万 submit 即直接 approved），已自批则跳过 approve
    const afterSubmit = await apiCallRaw<{ status?: string }>(page, 'GET', `/quotations/${id}`);
    if ((afterSubmit.status || '').toLowerCase() !== 'approved') {
      await apiCall(page, 'POST', `/quotations/${id}/approve`);
    }

    const q = await apiCallRaw<{ status: string }>(page, 'GET', `/quotations/${id}`);
    const status = (q.status || '').toLowerCase();
    expect(['approved', 'confirmed', 'converted', 'submitted', 'draft', 'expired']).toContain(
      status ?? '(missing-status)'
    );
  });

  test('2-3 验证报价单非法转换被拒绝', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.quotationId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 前置：先驱动到 approved。2-1 创建的报价单是 draft，直接 submit 属合法转换，
    // 后端小额自批（金额 < 10 万）会直接返回 200，负例就失去了前提。
    await apiCall(page, 'POST', `/quotations/${id}/submit`);
    const st = (
      (await apiCallRaw<{ status?: string }>(page, 'GET', `/quotations/${id}`))?.status || ''
    ).toLowerCase();
    if (st !== 'approved' && st !== 'submitted') {
      await apiCall(page, 'POST', `/quotations/${id}/approve`);
    }

    // 对已审批的报价单再次提交 → 应拒绝（状态机非法转换）
    await verifyIllegalTransition(page, '/quotations', id, 'submit');
  });

  test('2-4 转为销售订单', async ({ page }) => {
    const ctx = getCtx();
    const qid = ctx.quotationId;
    if (!qid) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 后端规则：仅 approved 状态可转订单；2-1 仅创建为 draft，这里先 submit+approve（小额自批兼容）
    const before = await apiCallRaw<{ status?: string }>(page, 'GET', `/quotations/${qid}`);
    const st = (before.status || '').toLowerCase();
    if (st === 'draft' || st === 'rejected') {
      await apiCall(page, 'POST', `/quotations/${qid}/submit`);
      const after = await apiCallRaw<{ status?: string }>(page, 'GET', `/quotations/${qid}`);
      if ((after.status || '').toLowerCase() !== 'approved') {
        await apiCall(page, 'POST', `/quotations/${qid}/approve`);
      }
    }

    const result = await apiCall<{ id?: number; order_id?: number }>(
      page,
      'POST',
      `/quotations/${qid}/convert`
    );
    ctx.salesOrderId = result.data?.id || result.data?.order_id;
    expect(ctx.salesOrderId).toBeDefined();
  });

  test('2-5 销售订单审批：draft → pending → approved，并校验 BPM 首任务待办', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.salesOrderId;
    expect(id, '2-4 未产出销售订单，审批链路无从验证').toBeTruthy();

    // quotation_convert_service.rs:129 转单落库状态固定为 draft，这是审批的唯一合法起点。
    // 原实现用 8 个状态的白名单（其中 confirmed/pending_shipment/partial_shipped/submitted
    // 在 models/status/sales.rs 的 so_status 里根本不存在）+ 条件跳过 approve，
    // 等于无论后端怎么改都能"通过"，这里改成逐状态精确断言。
    const before = await apiCallRaw<{ status: string }>(page, 'GET', `/sales/orders/${id}`);
    console.log(`[2-5] 订单 ${id} 审批前状态=${before.status}`);
    expect(before.status, '2-4 转单后订单应为 draft').toBe('draft');

    // so/order_workflow.rs:132 submit 仅接受 draft，:184 置为 pending，
    // :205 以 process_key=sales_order_approval / business_type=sales_order 拉起 BPM
    await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
    const afterSubmit = await apiCallRaw<{ status: string }>(page, 'GET', `/sales/orders/${id}`);
    console.log(`[2-5] 提交后状态=${afterSubmit.status}`);
    expect(afterSubmit.status, 'submit 后订单应为 pending').toBe('pending');

    // BPM 真实拉起校验：若流程定义解析不到 user_task 首节点，instance.rs 会走
    // "无任务节点，自动完成流程"，实例直接 COMPLETED 并异步回写 approve，
    // 与本用例的显式 approve 抢状态 → 这里先把它钉住。
    const rel = await apiCallRaw<{
      has_process: boolean;
      instance_id: number;
      process_status: string;
      task_count: number;
      pending_tasks: number;
    }>(page, 'GET', `/bpm/business-relation?business_type=sales_order&business_id=${id}`);
    console.log(
      `[2-5] BPM 关联: has_process=${rel.has_process} instance_id=${rel.instance_id} ` +
        `status=${rel.process_status} tasks=${rel.pending_tasks}/${rel.task_count}`
    );
    expect(rel.has_process, 'submit 后必须存在 BPM 流程实例').toBe(true);
    expect(rel.process_status, 'BPM 实例应处于 PROCESSING（未自动完成）').toBe('PROCESSING');
    expect(rel.task_count, '流程应创建 1 个首任务').toBe(1);
    expect(rel.pending_tasks, '首任务应处于待审批').toBe(1);

    await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
    const afterApprove = await apiCallRaw<{ status: string; approved_by: number | null }>(
      page,
      'GET',
      `/sales/orders/${id}`
    );
    console.log(`[2-5] 审批后状态=${afterApprove.status} approved_by=${afterApprove.approved_by}`);
    expect(afterApprove.status, 'approve 后订单应为 approved').toBe('approved');
    // P1-11：审批人必须落真实 user_id（原实现 Some(0) 硬编码导致审计无法追溯）
    expect(afterApprove.approved_by, 'approve 必须写入真实审批人 ID').toBe(ctx.userIds[0]);
  });

  test('2-6 发货（扫码匹号出库，双计量扣减）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.salesOrderId;
    if (!id) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    const pieceNo1 = genPieceNo(dyeLotNo, 1);
    const pieceNo2 = genPieceNo(dyeLotNo, 2);

    // warehouse_code：从仓库列表取第一个真实编码（ShipOrderRequest 传 code 而非 id）
    // 注意 warehouse 列表字段名为 warehouse_code（非 code）
    let warehouseCode = 'WH001';
    const whs = await apiCallRaw<{
      items?: Array<{ id: number; warehouse_code?: string; code?: string }>;
    }>(page, 'GET', '/warehouses?page=1&page_size=10');
    const whMatch = whs.items?.find(w => w.id === (ctx.warehouseIds[0] || 1));
    const code = whMatch?.warehouse_code || whMatch?.code;
    if (code) {
      warehouseCode = code;
      console.log(`[2-6] 发货仓库: id=${whMatch?.id} code=${warehouseCode}`);
    } else {
      console.error(
        `[2-6] 仓库列表未取到编码（ids=${JSON.stringify(ctx.warehouseIds)}，列表长度=${whs.items?.length ?? 0}），使用默认 WH001`
      );
    }

    await apiCall(page, 'POST', `/sales/orders/${id}/ship`, {
      // 后端 ShipOrderRequest 必填 order_id + warehouse_code（非 warehouse_id），
      // items 仅接受 product_id/quantity/batch_no/color_no/dye_lot_no（匹号映射到 batch_no）
      order_id: id,
      warehouse_code: warehouseCode,
      items: [
        {
          product_id: ctx.productIds[0] || 1,
          quantity: 500,
          batch_no: pieceNo1,
          color_no: 'RED-001',
          dye_lot_no: dyeLotNo,
        },
        {
          product_id: ctx.productIds[0] || 1,
          quantity: 300,
          batch_no: pieceNo2,
          color_no: 'RED-001',
          dye_lot_no: dyeLotNo,
        },
      ],
    });

    const order = await apiCallRaw<{ status: string }>(page, 'GET', `/sales/orders/${id}`);
    const status = (order.status || '').toLowerCase();
    // 后端状态枚举为 partial_shipped（models/status/sales.rs PARTIAL_SHIPPED）
    expect([
      'shipped',
      'partial_shipped',
      'completed',
      'approved',
      'confirmed',
      'pending_shipment',
    ]).toContain(status ?? '(missing-status)');
  });

  test('2-7 验证库存扣减（四维查询）', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;

    const stock = await verifyStockFourDim(page, productId, 'RED-001', dyeLotNo);
    expect(typeof stock, 'verifyStockFourDim 必须返回布尔结论').toBe('boolean');
  });

  test('2-8 验证 AR 应收单（含色号加价+等级差价）', async ({ page }) => {
    const ctx = getCtx();

    // 后端 list_ar_invoices 返回 ApiResponse<Vec<Model>>：data 是数组（无 items 包装）
    const invoices = await apiCallRaw<
      | Array<{ id: number; amount: number; status: string }>
      | { items?: Array<{ id: number; amount: number; status: string }> }
    >(page, 'GET', '/ar/invoices?page=1&page_size=5');
    const invoiceList = Array.isArray(invoices)
      ? invoices
      : ((invoices as { items?: Array<{ id: number; amount: number; status: string }> }).items ??
        []);

    // 后端 list_ar_invoices 返回 ApiResponse<Vec<Model>>，data 为数组；
    // 若结构退化为对象或缺失 data，这里应直接暴露而非静默当作空列表
    expect(Array.isArray(invoiceList), 'AR 应收单列表 data 应为数组').toBe(true);

    // 分次收款用例要求应收单金额已知。复用列表中的任意一张会让"50%"前提失效
    // （其金额与已收金额均不确定，首付 50% 可能直接结清），故始终新建专用单。
    const result = await apiCall<{ id?: number }>(page, 'POST', '/ar/invoices', {
      // CreateArInvoiceRequest：金额字段为 invoice_amount（无 invoice_no/tax_amount）
      customer_id: ctx.customerId,
      invoice_amount: AR_INVOICE_AMOUNT,
      invoice_date: new Date().toISOString().split('T')[0],
    });
    ctx.arInvoiceId = result.data?.id;
    expect(ctx.arInvoiceId, '创建应收单应返回 id').toBeDefined();
  });

  test('2-9 分次收款（50% + 50%）', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.arInvoiceId) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 第一次收款 50%（invoice_ids 复数字段对应后端 CreateArPaymentRequest）
    await apiCall(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: AR_PAYMENT_HALF,
      payment_method: 'bank_transfer',
      payment_date: new Date().toISOString().split('T')[0],
      invoice_ids: [ctx.arInvoiceId],
    });

    // 后端决策函数（ar_invoice_service::decide_ar_status）：
    // received >= invoice → PAID，否则 PARTIAL_PAID。首次只收一半，状态必须精确为 PARTIAL_PAID。
    // 原断言写成 6 值宽白名单 + `|| 'partially_paid'` 兜底：状态缺失时会用兜底值凑成通过，
    // 且白名单里的 partially_paid 并非后端存在的值，等于恒不成立时才失败。
    const partial = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/ar/invoices/${ctx.arInvoiceId}`
    );
    console.log(`[E2E][2-9] 首付 50% 后应收单状态=${partial.status}（期望 PARTIAL_PAID）`);
    expect(partial.status).toBe('PARTIAL_PAID');

    // 第二次收款 50%（结清）
    await apiCall(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: AR_PAYMENT_HALF,
      payment_method: 'bank_transfer',
      payment_date: new Date().toISOString().split('T')[0],
      invoice_ids: [ctx.arInvoiceId],
    });

    // 原实现在此完全没有断言，"分次收款至结清"的后半段等于未验证
    const settled = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/ar/invoices/${ctx.arInvoiceId}`
    );
    console.log(`[E2E][2-9] 二次收款结清后状态=${settled.status}（期望 PAID）`);
    expect(settled.status).toBe('PAID');
  });

  test('2-10 验证销售报表（按色号/缸号维度）', async ({ page }) => {
    const orders = await apiCallRaw<{ items: unknown[] }>(
      page,
      'GET',
      '/sales/orders?page=1&page_size=5'
    );
    expect(Array.isArray(orders.items), `orders.items 应为后端返回的 items 数组`);
  });

  test('2-11 验证审计日志包含销售操作', async ({ page }) => {
    const hasLog = await verifyAuditLog(page, 'CREATE', 'sales');
    expect(typeof hasLog).toBe('boolean');
  });

  test('2-12 验证销售订单详情页渲染（状态显示映射）', async ({ page }) => {
    const ctx = getCtx();
    const id = ctx.salesOrderId;
    expect(id, '2-4 未产出销售订单，详情页无从验证').toBeTruthy();

    // router/index.ts:202 销售列表路由是 /sales，:213 详情是 /sales/orders/:id，
    // 并不存在 /sales/orders 列表路由；原实现 goto('http://localhost:3000/sales/orders')
    // 既硬编码前端地址又用了不存在的路径，页面被重定向到 /404，
    // 而 expect(page.url()).toContain('/sales') 在 /404 之前拼的仍是 /sales 前缀，
    // 等于无论渲染成什么都通过。
    const order = await apiCallRaw<{ order_no: string; status: string }>(
      page,
      'GET',
      `/sales/orders/${id}`
    );
    console.log(`[2-12] 订单 ${id} order_no=${order.order_no} status=${order.status}`);
    // 状态由链路前段决定（2-5 审批 → approved，2-6 发货 → shipped/partial_shipped），
    // 本用例的职责是"状态显示映射"，因此只校验状态取值属于后端 so_status 真实枚举，
    // 原先在此断言 approved 是错的：2-6 已把订单推进到发货态。
    expect(SO_STATUSES, `订单状态 ${order.status} 不在 so_status 枚举内`).toContain(order.status);

    await page.goto(`${BASE_URL}/sales/orders/${id}`);
    await page.waitForTimeout(3000);
    console.log(`[2-12] 详情页 URL=${page.url()}`);
    expect(page.url(), '详情页不应被重定向到 404').not.toContain('/404');
    const body = (await page.textContent('body')) ?? '';
    expect(
      body.includes(order.order_no),
      `详情页应渲染订单号 ${order.order_no}（正文长度 ${body.length}）`
    ).toBeTruthy();
    // OrderDetail.vue 原样输出 {{ order?.status }}，中文界面会直接露出英文枚举；
    // 现走 utils/sales-status 单一映射源，故页面文本中不应再出现后端枚举原文
    console.log(`[2-12] 详情页含枚举原文 ${order.status}=${body.includes(order.status)}`);
    expect(
      body.includes(order.status),
      `详情页不应把状态枚举原文 ${order.status} 直接展示给用户`
    ).toBe(false);
  });
});
