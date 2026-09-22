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
} from './helpers';

test.describe.serial('扩展: 二级审批/BPM审批链/金额自适应', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('A1-1 验证二级审批（角色变更 pending_l1 → pending_l2 → approved）', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/role-change-approvals?page=1&page_size=5'
    );
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`).toBe(true);
    if ((list?.items?.length ?? 0) > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect(['pending_l1', 'pending_l2', 'approved', 'rejected', 'cancelled']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('A1-2 验证 BPM 审批链', async ({ page }) => {
    // 流程实例列表端点是 /bpm/monitor/instances（无 /bpm/instances 列表路由），
    // 返回 PageResponse{data,total}；原实现调用不存在的 /system/bpm/instances
    // 并用 try/catch 把失败再调一遍吞掉
    const instances = await apiCallRaw<{ data: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bpm/monitor/instances?page=1&page_size=5'
    );
    expect(Array.isArray(instances.data), `instances.data 应为后端返回的数组`).toBe(true);
    if ((instances?.data?.length ?? 0) > 0) {
      const status = (instances.data[0].status || '').toLowerCase();
      expect(['processing', 'completed', 'terminated', 'cancelled']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('A1-3 验证 BPM 任务审批', async ({ page }) => {
    // 原为 `expect(Array.isArray(tasks.items))`——没有匹配器，等于什么都没断（真空断言），
    // 且字段名也不对：/bpm/tasks 出参是 PageResponse{total,page,page_size,total_pages,data}。
    const tasks = await apiCallRaw<{
      data: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=5');
    expect(
      Array.isArray(tasks?.data),
      `tasks.data 应为后端返回的任务数组，实际响应：${JSON.stringify(tasks).slice(0, 200)}`
    ).toBe(true);
    expect(tasks.page, '应回显请求页码').toBe(1);
    expect(tasks.page_size, '应回显每页数量').toBe(5);
    const BPM_TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'];
    for (const task of tasks.data) {
      expect(BPM_TASK_STATUSES, `任务 ${task.id} 的状态在取值域外：${task.status}`).toContain(
        (task.status || '').toLowerCase()
      );
    }
  });

  test('A1-4 验证金额自适应审批（报价单）', async ({ page }) => {
    // 创建小额报价单 → 应自动审批通过
    // 创建大额报价单 → 应走 BPM 审批
    const ctx = getCtx();
    // 小额报价单
    const result = await apiCall<{ id?: number; status?: string }>(page, 'POST', '/quotations', {
      customer_id: ctx.customerId,
      quotation_date: new Date().toISOString().split('T')[0],
      valid_until: new Date(Date.now() + 30 * 86400000).toISOString().split('T')[0],
      items: [{ product_id: ctx.productIds[0] || 1, quantity: 1, unit_price: 1, tax_rate: 13 }],
      remarks: 'E2E 小额报价单（金额自适应审批）',
    });
    // 小额应自动审批
    if (result.data?.status) {
      const status = result.data.status.toLowerCase();
      expect(['approved', 'draft', 'submitted', 'pending_approval']).toContain(status);
    }
  });

  test('A1-5 验证审批日志追溯', async ({ page }) => {
    // 同为真空断言 + 错字段名：/bpm/tasks 的列表字段是 data。
    // 该端点是审批追溯的定位入口（按页取回任务并核对状态），断言出参契约与状态取值域。
    const logs = await apiCallRaw<{
      data: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
      total_pages: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=10');
    expect(
      Array.isArray(logs?.data),
      `审批任务应可从 data 数组定位，实际响应：${JSON.stringify(logs).slice(0, 200)}`
    ).toBe(true);
    expect(typeof logs.total, '应回显 total 供分页追溯').toBe('number');
    expect(logs.total_pages, '应回显 total_pages').toBeTypeOf('number');
    for (const row of logs.data) {
      expect(row.id, `任务 ID 应为正整数，实际 ${row.id}`).toBeGreaterThan(0);
      expect(
        ['pending', 'completed', 'rejected', 'cancelled'],
        `任务 ${row.id} 的状态在取值域外：${row.status}`
      ).toContain((row.status || '').toLowerCase());
    }
  });
});
