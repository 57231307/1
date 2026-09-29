import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  genName,
  ensureTestEntities,
  getCtx,
  pickDifferentUserCredential,
  loginInIsolatedContext,
} from './helpers';

/**
 * P5.4 水平越权测试
 *
 * 真越权（收紧假绿）：原实现只"创建资源 + 断言创建成功"，从未真正以另一账号凭证去改/删
 * A 的资源——等于没测越权。现改为：以分片账号 A（默认登录态，即资源的 owner）创建资源，
 * 再用一个独立身份账号 B 的真实会话对同一资源发起 PUT/DELETE，断言被拒（403/404）。
 *
 * 判责口径：
 * - 后端按 owner 隔离正确生效 → B 改/删得 403（无权限触达他人资源）或 404（按 owner 过滤后
 *   资源对 B 不可见）。两种拒绝都满足"无法越权"的契约。
 * - 若 B 竟能 2xx 改动/删除 A 的资源 → 本用例会红：这是水平越权真实缺陷（缺 owner 校验），
 *   属源码问题交后端，禁止把断言放宽（例如改成"只要不 500 就算过"）蒙过。
 *
 * 依赖：global-setup ensureRoleUsers 至少建出一个与分片账号不同身份的账号。
 */

/** 水平越权被拒的合法状态集合：403（鉴权拒绝）/404（owner 过滤后不可见）。 */
const DENIED_STATUSES = [403, 404] as const;

function expectHorizontallyDenied(
  res: { status: number; code?: string | number; message?: string | null },
  context: string
): void {
  expect(
    (DENIED_STATUSES as readonly number[]).includes(res.status),
    `${context}：越权改/删应被拒（403 无权限 / 404 owner 过滤后不可见），实际 status=${res.status} code=${res.code ?? '(none)'} message=${res.message ?? '(none)'}——若为 2xx 说明缺 owner 校验（水平越权缺陷），若为 5xx 属后端崩溃，均不得放宽本断言`
  ).toBe(true);
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

    const b = pickDifferentUserCredential();
    const sessionB = await loginInIsolatedContext(browser, b.username, b.password);
    try {
      const upd = await apiCallExpectFail(sessionB.page, 'PUT', `/crm/customers/${customerId}`, {
        customer_name: `${customerName}-被篡改`,
      });
      expectHorizontallyDenied(upd, `B(${b.username}) 修改 A 的客户 ${customerId}`);

      const del = await apiCallExpectFail(sessionB.page, 'DELETE', `/crm/customers/${customerId}`);
      expectHorizontallyDenied(del, `B(${b.username}) 删除 A 的客户 ${customerId}`);
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

    const b = pickDifferentUserCredential();
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

    const b = pickDifferentUserCredential();
    const sessionB = await loginInIsolatedContext(browser, b.username, b.password);
    try {
      const del = await apiCallExpectFail(sessionB.page, 'DELETE', `/purchase/orders/${poId}`);
      expectHorizontallyDenied(del, `B(${b.username}) 删除 A 的采购订单 ${poId}`);
    } finally {
      await sessionB.close();
    }
  });
});
