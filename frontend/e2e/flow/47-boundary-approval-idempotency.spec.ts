import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  genCode,
} from './helpers';

/**
 * 47 边界值 / 审批纵深 / 幂等 / 审计完整性（L3+L4+L5+审计防线合并）
 *
 * rule provenance：每条用例标注后端规则代码位置。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) {
    await tryCleanup(page, 'DELETE', c.path, c.label);
  }
  CLEANUP.length = 0;
});

test.describe.serial('47 边界值/审批纵深/幂等/审计完整性', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ============ L3 边界值 ============

  test('47-B1 汇率=0.01 拒绝（P0-1 历史缺陷回归防线 ap_invoice_service.rs:87-95）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const today = new Date().toISOString().slice(0, 10);
    const r001 = await apiCallExpectFail(page, 'POST', '/exchange-rates', {
      from_currency: 'USD',
      to_currency: 'CNY',
      rate: '0.01',
      effective_date: today,
    });
    expect(r001.status, '汇率 0.01 必须拒绝').toBeGreaterThanOrEqual(400);
    expect(String(r001.message ?? ''), '拒绝消息应提示 P0-1 缺陷值').toContain('0.01');
  });

  test('47-B2 汇率=0 拒绝（汇率必须 > 0）', async ({ page }) => {
    await ensureTestEntities(page);
    const today = new Date().toISOString().slice(0, 10);
    const r0 = await apiCallExpectFail(page, 'POST', '/exchange-rates', {
      from_currency: 'USD',
      to_currency: 'CNY',
      rate: '0',
      effective_date: today,
    });
    expect(r0.status, '汇率 0 必须拒绝').toBeGreaterThanOrEqual(400);
  });

  // ============ L4 审批纵深 ============

  test('47-A1 PO 审批后拒绝被拒（contract.rs:218-221 仅 PENDING_APPROVAL 可拒绝）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const { getCtx } = await import('./helpers');
    const ctx = getCtx();
    const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      items: [{ material_id: ctx.productIds[0], quantity: 1, unit_price: '1.00' }],
    });
    const id = po?.data?.id;
    expect(id, 'PO 创建失败').toBeTruthy();
    CLEANUP.push({ path: `/purchase/orders/${id}`, label: '[47-A1] PO' });

    await apiCall(page, 'POST', `/purchase/orders/${id}/submit`);
    const approve = await apiCall(page, 'POST', `/purchase/orders/${id}/approve`);
    expect(approve, '审批应成功').toBeTruthy();
    const reject = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/reject`);
    expect(reject.status, 'APPROVED 后拒绝应被状态门拦截').toBeGreaterThanOrEqual(400);
  });

  // ============ L5 幂等/唯一性 ============

  test('47-I1 用户名重复创建被拒（user_service.rs:90-103 先查后插）', async ({ page }) => {
    await ensureTestEntities(page);
    const username = `47dup${genCode('U').slice(-6)}`;
    const { rolesResp } = await (async () => ({ rolesResp: null }))();
    // 前置角色
    const role = await apiCallRawSafe(page, 'GET', '/roles?page=1&page_size=1');
    const roleId = (role as { roles?: Array<{ id: number }> })?.roles?.[0]?.id;
    expect(roleId, '无可用角色').toBeTruthy();

    const r1 = await apiCall<{ id?: number }>(page, 'POST', '/users', {
      username,
      password: 'Dup47!Test#2026x',
      email: `${username}@test.com`,
      role_id: roleId,
    });
    const uid = r1?.data?.id;
    if (uid) CLEANUP.push({ path: `/users/${uid}`, label: '[47-I1] 用户' });
    expect(uid, '首个用户创建失败').toBeTruthy();

    const r2 = await apiCallExpectFail(page, 'POST', '/users', {
      username,
      password: 'Dup47!Test#2026x',
      email: `${username}2@test.com`,
      role_id: roleId,
    });
    expect(r2.status, '重复用户名创建必须被拒').toBeGreaterThanOrEqual(400);
  });

  test('47-I2 角色编码重复创建被拒（role_permission_service.rs:167-174）', async ({ page }) => {
    await ensureTestEntities(page);
    const code = `47R${genCode('C').slice(-5)}`;
    const r1 = await apiCall<{ id?: number }>(page, 'POST', '/roles', {
      name: `47幂等角色${code}`,
      code,
      description: '47-I2',
    });
    const rid = r1?.data?.id;
    if (rid) CLEANUP.push({ path: `/roles/${rid}`, label: '[47-I2] 角色' });
    expect(rid, '首个角色创建失败').toBeTruthy();

    const r2 = await apiCallExpectFail(page, 'POST', '/roles', {
      name: `47幂等角色重复${code}`,
      code,
      description: '47-I2 重复',
    });
    expect(r2.status, '重复角色编码必须被拒').toBeGreaterThanOrEqual(400);
  });

  // ============ 审计完整性（用户报障：审计日志记录不全）============

  test('47-AU1 创建操作必须产生审计记录（audit_log_service.update_with_audit 埋点链）', async ({
    page,
  }) => {
    const name = `47审计部门${Date.now().toString().slice(-8)}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/departments', { name });
    const id = r?.data?.id;
    expect(id, '部门创建失败').toBeTruthy();
    CLEANUP.push({ path: `/departments/${id}`, label: '[47-AU1] 部门' });

    // 回查审计日志（admin 可查 system.rs:264）
    const logs = await apiCallRawSafe(
      page,
      'GET',
      `/audit-logs?page=1&page_size=20&table_name=departments`
    );
    const items =
      (logs as { items?: Array<Record<string, unknown>> })?.items ??
      (logs as { data?: { items?: Array<Record<string, unknown>> } })?.data?.items ??
      (logs as unknown as Array<Record<string, unknown>>) ??
      [];
    const hit = JSON.stringify(items).includes(name);
    expect(
      hit,
      `审计日志必须包含刚创建的部门操作（记录不全缺陷防线）。响应样本: ${JSON.stringify(items).slice(0, 300)}`
    ).toBe(true);
  });
});

async function apiCallRawSafe(
  page: import('@playwright/test').Page,
  method: 'GET' | 'POST',
  path: string
): Promise<unknown> {
  try {
    const { apiCallRaw } = await import('./helpers');
    return await apiCallRaw(page, method, path);
  } catch {
    return null;
  }
}
