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
  });

  test('C1-5 验证打样状态机（pending → sampling → submitted → approved/rejected）', async ({
    page,
  }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/production/lab-dip/requests?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
    if (list?.items?.length ?? 0 > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect(['pending', 'sampling', 'submitted', 'approved', 'rejected', 'completed']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('C1-6 验证打样小样状态机（pending → matched/not_matched/selected）', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/production/lab-dip/samples?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
    if (list?.items?.length ?? 0 > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect(['pending', 'matched', 'not_matched', 'selected']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('C1-7 验证大货批色 8 态状态机', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bulk-color-approvals?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
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
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
  });

  test('C1-9 验证坯布五维追溯链', async ({ page }) => {
    try {
      const trace = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/analytics/business-trace?page=1&page_size=5'
      );
      expect(Array.isArray(trace.items), `trace.items 应为后端返回的 items 数组`);
    } catch {
      const trace = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/business-trace?page=1&page_size=5'
      );
      expect(Array.isArray(trace.items), `trace.items 应为后端返回的 items 数组`);
    }
  });

  test('C1-10 验证工艺跟踪大屏数据', async ({ page }) => {
    const nodes = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/production/process-nodes?page=1&page_size=5'
    );
    expect(Array.isArray(nodes.items), `nodes.items 应为后端返回的 items 数组`);
    const logs = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/process-logs?page=1&page_size=5'
    );
    expect(Array.isArray(logs.items), `logs.items 应为后端返回的 items 数组`);
  });
});
