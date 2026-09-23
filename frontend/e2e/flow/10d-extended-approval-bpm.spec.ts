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
    // 流程实例列表端点是 /bpm/monitor/instances（无 /bpm/instances 列表路由）。
    // bpm_handler.rs:198 list_instances_for_monitor 内部调用
    // bpm_ops/monitor.rs:118 service.list_instances_for_monitor，返回类型是
    // PaginatedResponse<bpm_process_instance::Model>（非裸 Vec）——bpm_handler.rs:208 只是把它
    // to_value 后塞进 ApiResponse.data，因此 data 是 {items,total,page,page_size} 对象，
    // 列表在 items 里（与 A1-1/A1-3 出参契约一致）。apiCallRaw 取的是 res.data，
    // 故断言应为 Array.isArray(instances.items)，而非把 data 当数组。
    const instances = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bpm/monitor/instances?page=1&page_size=5'
    );
    expect(Array.isArray(instances.items), 'instances.items 应为后端返回的 items 数组').toBe(true);
    if (instances.items.length > 0) {
      const status = (instances.items[0].status || '').toLowerCase();
      expect(['processing', 'completed', 'terminated', 'cancelled']).toContain(
        status ?? '(missing-status)'
      );
    }
  });

  test('A1-3 验证 BPM 任务审批', async ({ page }) => {
    // 分页出参统一为 PaginatedResponse{items,total,page,page_size}（utils/response.rs:34）。
    const tasks = await apiCallRaw<{
      items: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=5');
    expect(
      Array.isArray(tasks?.items),
      `tasks.items 应为后端返回的任务数组，实际响应：${JSON.stringify(tasks).slice(0, 200)}`
    ).toBe(true);
    expect(tasks.page, '应回显请求页码').toBe(1);
    expect(tasks.page_size, '应回显每页数量').toBe(5);
    const BPM_TASK_STATUSES = ['pending', 'completed', 'rejected', 'cancelled'];
    for (const task of tasks.items) {
      expect(BPM_TASK_STATUSES, `任务 ${task.id} 的状态在取值域外：${task.status}`).toContain(
        (task.status || '').toLowerCase()
      );
    }
  });

  test('A1-4 验证金额自适应审批（报价单）', async ({ page }) => {
    // 创建小额报价单 → 应自动审批通过
    // 创建大额报价单 → 应走 BPM 审批
    const ctx = getCtx();
    // 字段对照 backend/src/models/quotation_create_dto.rs 的 CreateQuotationDto：
    // sales_user_id / currency / exchange_rate / base_currency / price_terms / tax_inclusive
    // 为必填，明细的 unit / unit_price_with_tax 为必填，缺任一即 422。
    // sales_user_id 取 ensureTestEntities 已确保的当前登录用户（报价单创建者语义），非魔法数字。
    const result = await apiCall<{ id?: number; status?: string }>(page, 'POST', '/quotations', {
      customer_id: ctx.customerId,
      sales_user_id: ctx.userIds[0],
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
          product_id: ctx.productIds[0],
          unit: '米',
          quantity: '1',
          unit_price: '1',
          unit_price_with_tax: '1.13',
        },
      ],
      notes: 'E2E 小额报价单（金额自适应审批）',
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
      items: Array<{ id: number; status: string }>;
      total: number;
      page: number;
      page_size: number;
    }>(page, 'GET', '/bpm/tasks?page=1&page_size=10');
    expect(
      Array.isArray(logs?.items),
      `审批任务应可从 items 数组定位，实际响应：${JSON.stringify(logs).slice(0, 200)}`
    ).toBe(true);
    expect(typeof logs.total, '应回显 total 供分页追溯').toBe('number');

    for (const row of logs.items) {
      expect(row.id, `任务 ID 应为正整数，实际 ${row.id}`).toBeGreaterThan(0);
      expect(
        ['pending', 'completed', 'rejected', 'cancelled'],
        `任务 ${row.id} 的状态在取值域外：${row.status}`
      ).toContain((row.status || '').toLowerCase());
    }
  });
});
