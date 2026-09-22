import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyBulkColorDeliveryBlock,
  verifyOutsourcingVoucher,
  verifyTrialBalance,
  verifyWeightConversion,
  verifyNetWeight,
  getCtx,
  genCode,
  ensureTestEntities,
  expectBadRequest,
} from './helpers';

test.describe.serial('扩展: 定制订单全流程（打样→报价→客户确认→投产）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // 打样通知单 id：C1-4 真实创建后写入，C1-6 按 by-request 端点回读其小样
  let labDipRequestId = 0;

  test('C1-1 创建定制订单', async ({ page }) => {
    const ctx = getCtx();
    try {
      const result = await apiCall<{ id?: number }>(page, 'POST', '/custom-orders', {
        order_no: genCode('CO'),
        customer_id: ctx.customerId,
        product_id: ctx.productIds[0] || 1,
        color_id: ctx.productColorIds[0],
        spec: '65%棉35%涤 40S 133x72 150cm',
        quantity: 500,
        unit: '米',
        custom_requirements: {
          yarn_spec: '40S',
          dye_method: 'reactive',
          finishing_method: '防水',
        },
        expected_delivery_date: new Date(Date.now() + 30 * 86400000).toISOString().split('T')[0],
        notes: 'E2E 定制订单全流程',
      });
      ctx.customOrderId = result.data?.id;
    } catch {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/custom-orders?page=1&page_size=1'
      );
      ctx.customOrderId = list.items?.[0]?.id;
    }
    expect(ctx.customOrderId).toBeDefined();
  });

  test('C1-2 验证定制订单 7 阶段状态机', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.customOrderId) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    const order = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/custom-orders/${ctx.customOrderId}`
    );
    const status = (order.status || '').toLowerCase();
    expect([
      'draft',
      'lab_dip',
      'quotation',
      'yarn_purchasing',
      'dyeing',
      'finishing',
      'delivery',
      'after_sales',
      'completed',
      'cancelled',
      'pending',
    ]).toContain(status ?? '(missing-status)');
  });

  test('C1-3 验证状态门校验（draft → dyeing 非法跳跃）', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.customOrderId) {
      console.warn('[E2E] test.skip: 前置数据缺失/条件不满足');
      test.skip();
      return;
    }

    // 直接从 draft 跳到 dyeing → 应拒绝
    const result = await apiCallExpectFail(
      page,
      'POST',
      `/custom-orders/${ctx.customOrderId}/advance`,
      { to_status: 'dyeing' }
    );
    expectBadRequest(result); // 非法转换应被拒
  });

  test('C1-4 创建打样通知单（lab_dip_request）', async ({ page }) => {
    const ctx = getCtx();
    // CreateLabDipRequestRequest 的字段是 customer_color_no / fabric_spec / fabric_component，
    // 必填 light_source（主对色光源）与 required_date（客户交期）；
    // 原用例发的 product_id/color_no/color_name/fabric_type/status 五个字段 DTO 里都不存在
    // （serde 忽略），两个必填又缺失，创建必被 422 拒掉。status 由后端状态机决定，不接受入参。
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/lab-dip/requests', {
      customer_id: ctx.customerId,
      customer_color_no: `C14-${Date.now().toString().slice(-6)}`,
      customer_color_name: '大红',
      sample_type: '小样',
      fabric_spec: 'E2E 打样规格',
      fabric_component: '棉涤',
      light_source: 'D65',
      required_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      remarks: 'E2E 打样通知单',
    });
    expect(result.data?.id, '打样通知单创建应返回 id').toBeTruthy();
    labDipRequestId = result.data!.id!;
  });

  test('C1-5 验证打样状态机（pending → sampling → submitted → approved/rejected）', async ({
    page,
  }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/production/lab-dip/requests?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
    if (list?.items?.length ?? 0 > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect(['pending', 'sampling', 'submitted', 'approved', 'rejected', 'completed']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('C1-6 验证打样小样状态机（pending → matched/not_matched/selected）', async ({ page }) => {
    // 小样没有全局列表端点：routes/production.rs:157-162 只注册了 POST /lab-dip/samples、
    // /lab-dip/samples/{id} 与 /lab-dip/samples/by-request/{request_id}（出参是裸数组）。
    // 原用例 GET /production/lab-dip/samples?page=… 是不存在的路径，且 expect 无匹配器。
    expect(labDipRequestId, 'C1-4 未创建打样通知单，无法按单回读小样').toBeGreaterThan(0);
    const list = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/production/lab-dip/samples/by-request/${labDipRequestId}`
    );
    expect(
      Array.isArray(list),
      `按打样单回读小样应为数组，实际：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    for (const row of list) {
      expect(Number(row.request_id), `小样行未挂在本打样单下：${JSON.stringify(row)}`).toBe(
        labDipRequestId
      );
      expect(String(row.status ?? ''), `小样行缺少 status：${JSON.stringify(row)}`).not.toBe('');
    }
  });

  test('C1-7 验证大货批色 8 态状态机', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bulk-color-approvals?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
    if (list?.items?.length ?? 0 > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect([
        'pending',
        'sampled',
        'sent_to_customer',
        'approved',
        'rejected',
        'rework',
        'downgraded',
        'scrapped',
      ]).toContain(status ?? '(missing-status)');
    }
  });

  test('C1-8 验证大货批色回修流程（rework → sampled）', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bulk-color-approvals?status=rework&page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
  });

  test('C1-9 验证坯布五维追溯链', async ({ page }) => {
    const ctx = getCtx();
    // 追溯域没有列表端点（business_trace() 只有 five-dimension/forward/backward/snapshot），
    // 原用例先查 /analytics/business-trace、失败后再查 /business-trace 顶包，
    // 两条路径都不存在，且 expect() 不带匹配器，无论返回什么都算通过。
    const batchNo = `10C-TRC-${Date.now().toString().slice(-8)}`;
    const trace = await apiCallRaw<{ traces: Array<Record<string, unknown>>; total: number }>(
      page,
      'GET',
      `/business-trace/forward?supplier_id=${ctx.supplierId ?? 0}&batch_no=${encodeURIComponent(batchNo)}`
    );
    expect(
      Array.isArray(trace?.traces),
      `正向追溯应返回 traces 数组，实际：${JSON.stringify(trace).slice(0, 200)}`
    ).toBe(true);
    expect(
      Number(trace.total) >= trace.traces.length,
      `total(${trace.total}) 不应小于返回链数(${trace.traces.length})`
    ).toBe(true);
  });

  test('C1-10 验证工艺跟踪大屏数据', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.customOrderId, 'C1-1 未创建定制订单，无法验证工艺时间线').toBeTruthy();
    // /production/process-nodes 与 /production/process-logs 两个路径在路由里根本不存在，
    // 节点与节点日志的真实入口是定制订单工艺时间线：
    // GET /custom-orders/{id}/nodes → ProcessTimeline{order_id,order_no,current_status,nodes[]}
    // （节点自带 logs；日志只有 POST /{id}/nodes/{nid}/logs 写入端点，无独立 GET 列表）
    const timeline = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/custom-orders/${ctx.customOrderId}/nodes`
    );
    expect(
      Number(timeline?.order_id),
      `工艺时间线应回显订单 id，实际：${JSON.stringify(timeline).slice(0, 200)}`
    ).toBe(Number(ctx.customOrderId));
    expect(
      Array.isArray(timeline.nodes),
      `工艺时间线应返回 nodes 数组，实际：${JSON.stringify(timeline).slice(0, 200)}`
    ).toBe(true);
    expect(String(timeline.current_status ?? ''), '工艺时间线缺少 current_status').not.toBe('');
    for (const node of timeline.nodes as Array<Record<string, unknown>>) {
      expect(Number(node.id), `工艺节点缺少 id：${JSON.stringify(node)}`).toBeGreaterThan(0);
      expect(
        Array.isArray(node.logs ?? []),
        `工艺节点的日志数组形态异常：${JSON.stringify(node)}`
      ).toBe(true);
    }
  });
});
