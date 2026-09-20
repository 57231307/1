import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
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
    // 后端 public_message 脱敏为'业务处理失败'，改断言 code=BUSINESS_ERROR
    expect(String(r001.code ?? ''), '0.01 汇率应被拒绝 code=BUSINESS_ERROR').toContain('BUSINESS');
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
    const role = await apiCallRaw(page, 'GET', '/roles?page=1&page_size=1');
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
    const code = `47r_${genCode('c').slice(-5)}`.toLowerCase().replace(/-/g, '_');
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

    // 回查审计日志：后端 list_audit_logs 把 table_name 映射到 resource_type 列，
    // 部门服务写的是 "department"（单数，department_service.rs:180）；
    // 响应结构是 { list, total, page, page_size }，无 items 键。
    // 部门创建走 record_async（mpsc channel 异步落库），需轮询等待写入完成。
    let items: Array<Record<string, unknown>> = [];
    for (let i = 0; i < 20; i++) {
      const logs = await apiCallRaw<{ list: Array<Record<string, unknown>> }>(
        page,
        'GET',
        `/audit-logs?page=1&page_size=100&table_name=department`
      );
      items = logs?.list ?? [];
      if (items.some(l => String(l.resource_name ?? '') === name)) break;
      await new Promise(r => setTimeout(r, 500));
    }
    expect(items.length, '按 table_name=department 应查到审计记录').toBeGreaterThan(0);
    const hit = items.some(
      l => String(l.resource_name ?? '') === name || String(l.description ?? '').includes(name)
    );
    expect(
      hit,
      `审计日志必须包含刚创建的部门（resource_name 断言）。响应样本: ${JSON.stringify(items).slice(0, 400)}`
    ).toBe(true);
  });
});
