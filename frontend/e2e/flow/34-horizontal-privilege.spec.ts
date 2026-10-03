import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  genName,
  ensureTestEntities,
  getCtx,
  loginInIsolatedContext,
  getRoleCredential,
} from './helpers';

/**
 * P5.4 水平越权测试
 *
 * 真越权（收紧假绿）：原实现只"创建资源 + 断言创建成功"，从未真正以另一账号凭证去改/删
 * A 的资源——等于没测越权。现改为：以分片账号 A（默认登录态，即资源的 owner）创建资源，
 * 再用一个独立身份账号 B 的真实会话对同一资源发起 PUT/DELETE，断言被拒（403/404）。
 *
 * 判责口径（2026-10-02 用户裁定**方案 A** 收口后）：
 * - **水平越权用例必须用对等非 admin 用户**（裁定原文：读可 All 跨 owner；写必须
 *   owner 本人或显式持 crm/cross_owner_write 代表键+留痕；admin 跨 owner 写经
 *   is_admin_role 放行属预期通道，不属"越权"，禁止再拿 admin 当越权者断 403、
 *   也禁止拿"All=可写"当契约断 200——两拨互相矛盾的断言统一为本口径）。
 *   B 固定取 salesperson 角色账号（非 admin、self 数据范围）；凭证缺失即前置失败判红，
 *   不得静默降级为其它账号。
 * - CRM 客户域（写门已接入 crm_write_guard）：非 owner 跨 owner 改/删必为 **403**；
 *   断言只许 status + 信封 code（FORBIDDEN），**不断言拒绝原因**——权限文案永久脱敏是硬令。
 * - 其它域（供应商/采购单）：owner 过滤后不可见可能表现为 404，维持 403/404 双合法拒绝集，
 *   同样只断 status。若 B 竟能 2xx 改动/删除 A 的资源 → 用例红 = 水平越权真缺陷，交后端，
 *   禁止放宽断言蒙过。
 *
 * 依赖：global-setup ensureRoleUsers 已建出 salesperson 角色账号（role-credentials.json）。
 */

/** 取对等非 admin 越权者 B（裁定口径：salesperson，self 数据范围，非 admin）。 */
function pickNonAdminPeerCredential(): { username: string; password: string } {
  const cred = getRoleCredential('salesperson');
  if (!cred) {
    throw new Error(
      '水平越权用例前置失败：role-credentials.json 无 salesperson 角色凭证（裁定口径要求对等非 admin 用户，不得改用其它身份兜底）'
    );
  }
  return cred;
}

/** 非 CRM 域水平越权被拒的合法状态集合：403（鉴权拒绝）/404（owner 过滤后不可见）。 */
const DENIED_STATUSES = [403, 404] as const;

function expectHorizontallyDenied(
  res: { status: number; code?: string | number; message?: string | null },
  context: string
): void {
  expect(
    (DENIED_STATUSES as readonly number[]).includes(res.status),
    `${context}：越权改/删应被拒（403 无权限 / 404 owner 过滤后不可见），实际 status=${res.status} code=${res.code ?? '(none)'}——若为 2xx 说明缺 owner 校验（水平越权缺陷），若为 5xx 属后端崩溃，均不得放宽本断言`
  ).toBe(true);
}

/** CRM 客户域被拒锁（裁定方案 A + 权限文案永久脱敏）：只断 status=403 与信封 code，不断原因。 */
function expectCrmWriteDenied(
  res: { status: number; code?: string | number; message?: string | null },
  context: string
): void {
  expect(
    res.status,
    `${context}：跨 owner 写他人客户必须 403（写门 crm_write_guard），实际 status=${res.status} code=${res.code ?? '(none)'}`
  ).toBe(403);
  expect(String(res.code ?? ''), `${context}：失败信封机器码必须是 FORBIDDEN`).toBe('FORBIDDEN');
}

test.describe('P5.4 水平越权（以 B 账号真实改/删 A 账号资源）', () => {
  test.beforeEach(async ({ page }) => {
    // 默认登录态 = 用户 A（分片账号），A 是下面所创建资源的 owner
    await loginViaUI(page);
  });

  test('B 无法修改/删除 A 创建的客户', async ({ page, browser }) => {
    const customerName = genName('HozCust');
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
      customer_type: 'wholesale',
      contact_person: '测试',
      contact_phone: '13800000000',
    });
    const customerId = createResp.data?.id;
    expect(
      customerId,
      `客户创建未返回 id（POST /crm/customers 响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();

    const b = pickNonAdminPeerCredential();
    const sessionB = await loginInIsolatedContext(browser, b.username, b.password);
    try {
      const upd = await apiCallExpectFail(sessionB.page, 'PUT', `/crm/customers/${customerId}`, {
        customer_name: `${customerName}-被篡改`,
      });
      expectCrmWriteDenied(upd, `B(${b.username}) 修改 A 的客户 ${customerId}`);

      const del = await apiCallExpectFail(sessionB.page, 'DELETE', `/crm/customers/${customerId}`);
      expectCrmWriteDenied(del, `B(${b.username}) 删除 A 的客户 ${customerId}`);
    } finally {
      await sessionB.close();
    }
  });

  test('B 无法修改/删除 A 创建的供应商', async ({ page, browser }) => {
    const supplierName = genName('HozSup');
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: supplierName,
      supplier_type: 'material',
      contact_person: '测试',
      contact_phone: '13800000000',
    });
    const supplierId = createResp.data?.id;
    expect(
      supplierId,
      `供应商创建未返回 id（POST /purchase/suppliers 响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();

    const b = pickNonAdminPeerCredential();
    const sessionB = await loginInIsolatedContext(browser, b.username, b.password);
    try {
      const upd = await apiCallExpectFail(
        sessionB.page,
        'PUT',
        `/purchase/suppliers/${supplierId}`,
        { supplier_name: `${supplierName}-被篡改` }
      );
      expectHorizontallyDenied(upd, `B(${b.username}) 修改 A 的供应商 ${supplierId}`);

      const del = await apiCallExpectFail(
        sessionB.page,
        'DELETE',
        `/purchase/suppliers/${supplierId}`
      );
      expectHorizontallyDenied(del, `B(${b.username}) 删除 A 的供应商 ${supplierId}`);
    } finally {
      await sessionB.close();
    }
  });

  test('B 无法删除 A 创建的采购订单', async ({ page, browser }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          material_id: ctx.productIds[0],
          quantity_ordered: '1',
          unit_price: '1',
        },
      ],
    });
    const poId = createResp.data?.id;
    expect(
      poId,
      `采购订单创建未返回 id（响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();

    const b = pickNonAdminPeerCredential();
    const sessionB = await loginInIsolatedContext(browser, b.username, b.password);
    try {
      const del = await apiCallExpectFail(sessionB.page, 'DELETE', `/purchase/orders/${poId}`);
      expectHorizontallyDenied(del, `B(${b.username}) 删除 A 的采购订单 ${poId}`);
    } finally {
      await sessionB.close();
    }
  });
});
