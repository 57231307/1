// 生产管理 E2E 套件 — 05 定制订单 7 阶段逐步流转 + 非法跳变被拒
// 覆盖范围：创建(draft) → advance(lab_dip) → gate 拒绝(lab_dip→quotation 缺前置) →
//           终态不可再推进(completed/cancelled) → 非法跳变(draft直接到dyeing不可能，advance只能顺序)
// 状态机（7阶段）：draft→lab_dip→quotation→yarn_purchasing→dyeing→finishing→delivery→after_sales→completed
// 门控：lab_dip→quotation 需 lab_dip_request_id 且 approved_sample_id 非空；
//       quotation→yarn_purchasing 需 quotation_id 且报价单 status=approved
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  tryCleanup,
} from '../flow/helpers';

interface CustomOrderResponse {
  id: number;
  order_no: string;
  status: string;
  customer_id: number;
  product_id: number;
  quantity: number | string;
  unit: string;
  spec: string;
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function createCustomOrder(
  page: import('@playwright/test').Page
): Promise<CustomOrderResponse> {
  const ctx = getCtx();
  const created = await apiCallRaw<CustomOrderResponse>(page, 'POST', '/custom-orders', {
    customer_id: ctx.customerId || 1,
    product_id: ctx.productIds[0] || 1,
    spec: '65%棉35%涤 40S 150cm',
    quantity: '100',
    unit: '米',
    expected_delivery_date: new Date(Date.now() + 60 * 86400000).toISOString().slice(0, 10),
  });
  expect(created.id, '创建定制订单应返回 id').toBeTruthy();
  CLEANUP.push({ path: `/custom-orders/${created.id}`, label: 'custom_order' });
  return created;
}

test.describe('05 定制订单 7 阶段流转', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('05-01 新创建订单初始状态为 draft', async ({ page }) => {
    const order = await createCustomOrder(page);
    expect(order.status, `新建定制订单初始状态应为 draft，实际: ${order.status}`).toBe('draft');
  });

  test('05-02 draft → lab_dip 推进成功并回读精确状态', async ({ page }) => {
    const order = await createCustomOrder(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // advance: draft → lab_dip（无门控要求）
    const advanced = await apiCallRaw<CustomOrderResponse>(
      page,
      'POST',
      `/custom-orders/${order.id}/advance`,
      { operator_id: me.id, notes: 'E2E 测试推进到打样' }
    );
    expect(advanced.status, `推进后 status 应为 lab_dip，实际: ${advanced.status}`).toBe('lab_dip');

    // GET 回读二次确认
    const fetched = await apiCallRaw<CustomOrderResponse>(
      page,
      'GET',
      `/custom-orders/${order.id}`
    );
    expect(fetched.status, 'GET 回读状态应为 lab_dip').toBe('lab_dip');
  });

  test('05-03 lab_dip → quotation 门控拒绝（缺 lab_dip_request_id）', async ({ page }) => {
    const order = await createCustomOrder(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // 先推到 lab_dip
    await apiCall(page, 'POST', `/custom-orders/${order.id}/advance`, {
      operator_id: me.id,
    });

    // 再尝试推进 lab_dip → quotation：门控要求 lab_dip_request_id 且 approved_sample_id 非空
    // 新建订单无关联打样通知单，应被拒绝
    const fail = await apiCallExpectFail(page, 'POST', `/custom-orders/${order.id}/advance`, {
      operator_id: me.id,
    });
    expect(
      fail.status,
      `lab_dip→quotation 缺前置应返回 400+，实际 ${fail.status}: ${fail.message}`
    ).toBeGreaterThanOrEqual(400);
    // 验证具体为业务拒绝（非 500 内部错误）
    expect(fail.status, '应为业务级 4xx 而非 5xx').toBeLessThan(500);

    // 回读确认状态仍停留在 lab_dip（未被非法修改）
    const still = await apiCallRaw<CustomOrderResponse>(page, 'GET', `/custom-orders/${order.id}`);
    expect(still.status, '门控拒绝后状态应仍为 lab_dip').toBe('lab_dip');
  });

  test('05-04 推进到 lab_dip 后再次 advance 触发 quotation 门控，不跳级', async ({ page }) => {
    const order = await createCustomOrder(page);
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');

    // draft → lab_dip
    await apiCall(page, 'POST', `/custom-orders/${order.id}/advance`, {
      operator_id: me.id,
    });
    const afterFirst = await apiCallRaw<CustomOrderResponse>(
      page,
      'GET',
      `/custom-orders/${order.id}`
    );
    expect(afterFirst.status).toBe('lab_dip');

    // lab_dip 再推进 → 应触发 quotation 门控失败，绝不会直接跳到 dyeing 或更远状态
    const fail = await apiCallExpectFail(page, 'POST', `/custom-orders/${order.id}/advance`, {
      operator_id: me.id,
    });
    expect(fail.status).toBeGreaterThanOrEqual(400);
    expect(fail.status).toBeLessThan(500);

    // 状态仍为 lab_dip
    const still = await apiCallRaw<CustomOrderResponse>(page, 'GET', `/custom-orders/${order.id}`);
    expect(still.status, '门控失败不可产生跳变').toBe('lab_dip');
  });

  test('05-05 取消后终态不可再推进', async ({ page }) => {
    const order = await createCustomOrder(page);

    // DELETE = cancel
    await apiCall(page, 'DELETE', `/custom-orders/${order.id}`, {
      reason: 'E2E 测试取消',
    });

    const cancelled = await apiCallRaw<CustomOrderResponse>(
      page,
      'GET',
      `/custom-orders/${order.id}`
    );
    expect(cancelled.status, '取消后状态应为 cancelled').toBe('cancelled');

    // 移除 CLEANUP 中的取消路径（已取消不能重复取消）
    const cancelIdx = CLEANUP.findIndex(c => c.path === `/custom-orders/${order.id}`);
    if (cancelIdx >= 0) CLEANUP.splice(cancelIdx, 1);

    // 从 cancelled 推进 → 应被拒绝（终态）
    const me = await apiCallRaw<{ id: number }>(page, 'GET', '/auth/me');
    const fail = await apiCallExpectFail(page, 'POST', `/custom-orders/${order.id}/advance`, {
      operator_id: me.id,
    });
    expect(
      fail.status,
      `终态 cancelled 推进应返回 400+，实际 ${fail.status}`
    ).toBeGreaterThanOrEqual(400);
    expect(fail.status, '终态拒绝应为 4xx 非 5xx').toBeLessThan(500);
  });

  test('05-06 缺少 operator_id 的非法请求被拒绝', async ({ page }) => {
    const order = await createCustomOrder(page);

    // 后端 AdvanceRequest.operator_id 是必填 i32，缺失应 serde 拒绝 422
    const fail = await apiCallExpectFail(page, 'POST', `/custom-orders/${order.id}/advance`, {
      notes: 'missing operator_id',
    });
    expect(fail.status, `缺 operator_id 应返回 400+，实际 ${fail.status}`).toBeGreaterThanOrEqual(
      400
    );
  });
});
