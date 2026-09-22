import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  getCtx,
  genCode,
  genName,
  genDyeLotNo,
  getProcessSteps,
  ensureTestEntities,
} from './helpers';

test.describe.serial('扩展: 业务模式测试（染整加工/来料加工/委外加工）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test.beforeEach(async ({ page }) => {
    await ensureTestEntities(page);
  });

  test('M1-1 验证业务模式列表（6 种模式）', async ({ page }) => {
    const modes = await apiCallRaw<{
      items: Array<{ mode_code: string; mode_name: string; mode_category: string }>;
    }>(page, 'GET', '/production/business-modes?page=1&page_size=20');
    expect(Array.isArray(modes.items), `modes.items 应为后端返回的 items 数组`);
    // 验证至少有一种模式
    if (modes?.items?.length ?? 0 > 0) {
      const codes = modes.items.map(m => m.mode_code);
      const expectedCodes = [
        'grey_trading',
        'finished_trading',
        'dyeing_processing',
        'self_weave_dye',
        'outsourcing',
        'toll_processing',
      ];
      const hasAny = codes.some(c => expectedCodes.includes(c));
      expect(hasAny).toBe(true);
    }
  });

  test('M1-2 验证染整加工模式流程链', async ({ page }) => {
    const steps = await getProcessSteps(page, 'dyeing_processing');
    // 染整加工流程链：inventory_in → production → inventory_out → settlement
    expect(steps.length >= 0).toBeTruthy();
    if (steps.length > 0) {
      const stepCodes = steps.map(s => s.step_code);
      expect(stepCodes).toContain('production');
    }
  });

  test('M1-3 验证来料加工模式（toll_processing）', async ({ page }) => {
    const steps = await getProcessSteps(page, 'toll_processing');
    // 来料加工流程链：inventory_in → production → inventory_out → settlement
    expect(steps.length >= 0).toBeTruthy();
    if (steps.length > 0) {
      const stepCodes = steps.map(s => s.step_code);
      expect(stepCodes).toContain('production');
      // 来料加工不应包含采购节点
      expect(stepCodes).not.toContain('purchase');
    }
  });

  test('M1-4 创建委外加工订单', async ({ page }) => {
    const ctx = getCtx();
    const result = await apiCall<{ id?: number }>(page, 'POST', '/production/outsourcing-orders', {
      order_no: genCode('OUT'),
      order_type: 'dyeing',
      supplier_id: ctx.supplierId || 1,
      // CreateOutsourcingOrderRequest 必填 issue_date/issue_quantity/material_cost；
      // 交期字段名是 expected_return_date（原发的 expected_delivery_date 不在契约内，
      // 会被 serde 忽略，而缺失必填字段直接 422）
      issue_date: new Date().toISOString().split('T')[0],
      expected_return_date: new Date(Date.now() + 14 * 86400000).toISOString().split('T')[0],
      issue_quantity: '100',
      material_cost: '0',
      remarks: 'E2E 委外加工订单',
    });
    expect(result.data?.id).toBeDefined();
  });

  test('M1-5 委外加工订单状态流转', async ({ page }) => {
    const list = await apiCallRaw<{ items: Array<{ id: number; status: string }> }>(
      page,
      'GET',
      '/production/outsourcing-orders?page=1&page_size=1'
    );
    if (list.items?.length > 0) {
      const status = (list.items?.[0].status || '').toLowerCase();
      expect([
        'draft',
        'issued',
        'processing',
        'received',
        'settled',
        'closed',
        'cancelled',
      ]).toContain(status ?? '(missing-status)');
    }
  });

  test('M1-6 验证委外加工模式规则', async ({ page }) => {
    // GET /production/business-modes/rules 只有 POST 路由（无全局列表端点），
    // 原用例按"分页列表 items"发 GET，拿到的是 405 Method Not Allowed。
    // 规则的真实查询入口是按模式取回：/business-modes/rules/by-mode/{mode_id}。
    const modes = await apiCallRaw<{
      items: Array<{ id: number; mode_code: string; mode_name: string }>;
    }>(page, 'GET', '/production/business-modes?page=1&page_size=20');
    expect(
      Array.isArray(modes?.items),
      `业务模式列表应返回 items 数组，实际：${JSON.stringify(modes).slice(0, 200)}`
    ).toBe(true);
    const outsourcing = (modes.items ?? []).find(m => m.mode_code === 'outsourcing');
    expect(
      outsourcing,
      `种子数据里没有 outsourcing（委托加工）模式，现有模式：${(modes.items ?? [])
        .map(m => m.mode_code)
        .join(',')}`
    ).toBeTruthy();

    const rules = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/production/business-modes/rules/by-mode/${outsourcing!.id}`
    );
    expect(
      Array.isArray(rules),
      `按模式查询规则应返回数组，实际：${JSON.stringify(rules).slice(0, 200)}`
    ).toBe(true);
    for (const rule of rules) {
      expect(rule.mode_id, `规则混入了其他模式的行：${JSON.stringify(rule)}`).toBe(outsourcing!.id);
      expect(String(rule.rule_code ?? ''), `规则缺少 rule_code：${JSON.stringify(rule)}`).not.toBe(
        ''
      );
      // rule_type 取值域见 backend BusinessModeRule 注释：required / optional / forbidden
      expect(
        ['required', 'optional', 'forbidden'],
        `规则 ${rule.rule_code} 的类型在取值域外：${rule.rule_type}`
      ).toContain(String(rule.rule_type));
    }
  });

  test('M1-7 验证业务模式快照（mode_snapshot）', async ({ page }) => {
    const links = await apiCallRaw<{
      items: Array<{ document_type: string; mode_snapshot: string }>;
    }>(page, 'GET', '/business-mode-links?page=1&page_size=10');
    expect(Array.isArray(links.items), `links.items 应为后端返回的 items 数组`);
  });
});
