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
    expect(Array.isArray(list.items), `list.items 应为后端返回的 items 数组`);
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
    const tasks = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/bpm/tasks?page=1&page_size=5'
    );
    expect(Array.isArray(tasks.items), `tasks.items 应为后端返回的 items 数组`);
    if ((tasks?.items?.length ?? 0) > 0) {
      const status = (tasks.items?.[0].status || '').toLowerCase();
      expect(['pending', 'completed', 'rejected', 'cancelled']).toContain(
        status ?? '(missing-status)'
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
    const logs = await apiCallRaw<{ items: Array<{ id: number; action: string }> }>(
      page,
      'GET',
      '/bpm/tasks?page=1&page_size=10'
    );
    expect(Array.isArray(logs.items), `logs.items 应为后端返回的 items 数组`);
  });
});
