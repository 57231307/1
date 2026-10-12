import { test, expect } from '../diagnose-fixture';
import { pickListArray } from './ui-helpers';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  tryCleanup,
  genCode,
  failureCode,
  APP_ERROR_CODES,
  type ApiFailureResult,
} from './helpers';

/**
 * 47 边界值 / 审批纵深 / 幂等 / 审计完整性（L3+L4+L5+审计防线合并）
 *
 * rule provenance：每条用例标注后端规则所在 文件::符号。
 */

/**
 * 精确拒绝契约（收紧原「status>=400（连 5xx 都算过）」与 code 子串匹配的假绿）：
 * HTTP 恰 400 + 机器码逐条等于调用点期望族；族别依据各调用点旁注释（error.rs:356-373
 * 客户端族全映射 400；business(_displayable)→BUSINESS_ERROR，error.rs::error_code）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

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

  test('47-B1 汇率=0.01 拒绝（P0-1 历史缺陷回归防线）', async ({ page }) => {
    await ensureTestEntities(page);
    const today = new Date().toISOString().slice(0, 10);
    const r001 = await apiCallExpectFail(page, 'POST', '/exchange-rates', {
      from_currency: 'USD',
      to_currency: 'CNY',
      rate: '0.01',
      effective_date: today,
    });
    // currency_service.rs::create_exchange_rate P0-1 门 → AppError::business（脱敏出参，只判状态+机器码）
    expectRejected(r001, '汇率 0.01 必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // 同函数非正汇率门 → AppError::business
    expectRejected(r0, '汇率 0 必须拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // reject 服务端必填理由（handler 先做 reason 校验再走状态门）：不带理由会先撞 400
    // VALIDATION_ERROR，触达不到 po/contract.rs::reject_order 的状态门。补理由让链真正走到状态门，
    // APPROVED 后拒绝仍被拦为 400 BUSINESS_ERROR（断言不变，非迁就）。
    const reject = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/reject`, {
      reason: 'E2E-47A1 状态门负例理由',
    });
    // po/contract.rs::reject_order 状态门（仅 PENDING_APPROVAL 可拒绝 → AppError::business）
    expectRejected(reject, 'APPROVED 后拒绝应被状态门拦截', APP_ERROR_CODES.BUSINESS_ERROR);
  });

  // ============ L5 幂等/唯一性 ============

  test('47-I1 用户名重复创建被拒（user_service.rs::create_user 先查后插）', async ({ page }) => {
    await ensureTestEntities(page);
    const username = `47dup${genCode('U').slice(-6)}`;
    // 前置角色
    const role = await apiCallRaw<{ roles: Array<{ id: number }> }>(
      page,
      'GET',
      '/roles?page=1&page_size=1'
    );
    // /roles -> RoleListResponse.roles（audit_log_handler 同款具名键，既不是 items 也不是裸数组）
    const roleList = pickListArray<{ id: number }>(role, 'roles', '47-I1 角色列表 /roles');
    expect(roleList.length, '角色表为空，无法构造用户').toBeGreaterThan(0);
    const roleId = roleList[0].id;
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
    // user_service.rs::create_user 唯一性门 → AppError::business_displayable（判码不判文案）
    expectRejected(r2, '重复用户名创建必须被拒', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // role_permission_service.rs::create_role 编码唯一门 → AppError::business
    expectRejected(r2, '重复角色编码必须被拒', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // GET /audit-logs 由 audit_log_handler 处理，响应结构是 { items, total, page, page_size }
    // （挂 analytics 域 /logs 的 audit_enhanced_handler 才用 list，两者不可混用）。
    // 部门创建走 record_async（mpsc channel 异步落库），需轮询等待写入完成。
    let items: Array<Record<string, unknown>> = [];
    for (let i = 0; i < 20; i++) {
      const logs = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
        page,
        'GET',
        `/audit-logs?page=1&page_size=100&table_name=department`
      );
      // /audit-logs -> AuditLogListResponse.items（audit_log_handler.rs:132）
      items = pickListArray<Record<string, unknown>>(logs, 'items', '47 审计日志 /audit-logs');
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
