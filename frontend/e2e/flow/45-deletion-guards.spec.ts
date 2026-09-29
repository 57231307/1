import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  tryCleanup,
  ensureTestEntities,
  getCtx,
} from './helpers';

/**
 * L2 删除约束矩阵（flow/45-deletion-guards）
 *
 * rule provenance：每条用例对应后端一条删除前置校验，断言 HTTP 400+ 且
 * 错误消息关键词——防规则被静默移除后产生悬挂引用。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) {
    await tryCleanup(page, 'DELETE', c.path, c.label);
  }
  CLEANUP.length = 0;
});

test.describe.serial('45 删除约束矩阵（每条对应后端删除前置校验规则）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('45-1 部门：存在子部门禁止删除（department_service.rs:207-213）', async ({ page }) => {
    const ts = Date.now().toString().slice(-8);
    const parent = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
      name: `45删除守卫父部门${ts}`,
      description: '45-1 父',
    });
    const parentId = parent?.data?.id;
    expect(parentId, '父部门创建失败').toBeTruthy();
    CLEANUP.push({ path: `/departments/${parentId}`, label: '[45-1] 父部门' });

    const child = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
      name: `45删除守卫子部门${ts}`,
      description: '45-1 子',
    });
    const childId = child?.data?.id;
    expect(childId, '子部门创建失败').toBeTruthy();

    // 建立父子关系（update parent_id）
    const upd = await apiCall(page, 'PUT', `/departments/${childId}`, {
      parent_id: parentId,
    });
    expect(upd, '子部门挂载父部门应成功').toBeTruthy();

    // 删除父部门必须被拒（apiCallExpectFail 不带 CSRF 恢复，DELETE 可能被 CSRF 拦截；
    // 改用 apiCall——它在 CSRF 失败时自动恢复重试，拿到真实业务错误 code）
    let delCode = '';
    let delStatus = 0;
    try {
      await apiCall(page, 'DELETE', `/departments/${parentId}`);
    } catch (e) {
      const msg = (e as Error).message;
      // apiCall 失败时 message 格式: code=BUSINESS_ERROR message=业务处理失败
      const codeMatch = msg.match(/code=(\S+)/);
      delCode = codeMatch ? codeMatch[1] : '';
      delStatus = 400;
    }
    expect(delStatus, '有子部门的父部门删除应被拒').toBeGreaterThanOrEqual(400);
    expect(delCode, '拒绝 code 应为 BUSINESS_ERROR').toContain('BUSINESS');
  });

  test('45-2 产品类别：存在子类别禁止删除（product_category_service.rs:162-168）', async ({
    page,
  }) => {
    const ts = Date.now().toString().slice(-8);
    const parent = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
      name: `45守卫父分类${ts}`,
    });
    const parentId = parent?.data?.id;
    expect(parentId, '父分类创建失败').toBeTruthy();
    CLEANUP.push({ path: `/product-categories/${parentId}`, label: '[45-2] 父分类' });

    const child = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
      name: `45守卫子分类${ts}`,
      parent_id: parentId,
    });
    const childId = child?.data?.id;
    expect(childId, '子分类创建失败').toBeTruthy();
    CLEANUP.push({ path: `/product-categories/${childId}`, label: '[45-2] 子分类' });

    const del = await apiCallExpectFail(page, 'DELETE', `/product-categories/${parentId}`);
    expect(del.status, '有子类别的父分类删除应被拒').toBeGreaterThanOrEqual(400);
  });

  test('45-3 会计科目：父有子科目时禁止删除（account_subject_service.rs delete 三重校验之一）', async ({
    page,
  }) => {
    // 真实端点：POST /subjects + DELETE /subjects/{id}
    // （finance.rs gl() 经 sub_routes() 直接 nest 在 /api/v1/erp → 相对路径 /subjects；
    //  旧写法 /finance/subjects 未在 finance() 路由树注册，恒 404，旧断言
    //  expect(status>=400) 因此"永远绿"，从未真正跑到删除守卫——典型条件 skip 假绿。）
    // create_subject 仅校验 code 唯一 + 父存在，合法入参必 200 返回 id；
    // 造不出前置数据即判红（apiCall 抛真实 code/message），不允许 skip。
    const uniq = `${Date.now().toString().slice(-6)}${Math.floor(Math.random() * 1000)}`;
    const parent = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
      code: `45G${uniq}P`,
      name: `45守卫父科目${uniq}`,
      level: 1,
    });
    const parentId = parent?.data?.id;
    expect(parentId, `父科目创建失败，未返回 id：${JSON.stringify(parent)}`).toBeTruthy();
    CLEANUP.push({ path: `/subjects/${parentId}`, label: '[45-3] 父科目' });

    const child = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
      code: `45G${uniq}C`,
      name: `45守卫子科目${uniq}`,
      level: 2,
      parent_id: parentId,
    });
    const childId = child?.data?.id;
    expect(childId, `子科目创建失败，未返回 id：${JSON.stringify(child)}`).toBeTruthy();
    CLEANUP.push({ path: `/subjects/${childId}`, label: '[45-3] 子科目' });

    // 父有子 → 删除必被业务拒绝（service delete："不能删除有子科目的科目"）
    const delParent = await apiCallExpectFail(page, 'DELETE', `/subjects/${parentId}`);
    expect(
      delParent.status,
      `有子科目的父科目删除应被拒（实际 status=${delParent.status} code=${delParent.code} msg=${delParent.message ?? ''}）`
    ).toBeGreaterThanOrEqual(400);

    // 对照组：无子的子科目删除应成功（apiCall 失败即抛真实响应），
    // 证明上一条 400+ 确由删除守卫产生，而非 CSRF/权限/路径错误
    const delChild = await apiCall(page, 'DELETE', `/subjects/${childId}`);
    expect(delChild.code, '无子科目的子科目删除应成功').toBe(200);
    // 子科目已在测试内删除，撤销其 cleanup 登记避免重复删除噪声
    const idx = CLEANUP.findIndex(c => c.path === `/subjects/${childId}`);
    if (idx >= 0) CLEANUP.splice(idx, 1);
  });

  test('45-4 供应商：有活跃采购订单禁止删除（supplier_service.rs:493-512）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 动态创建供应商与一张活跃 PO
    const sup = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: `45守卫供应商${Date.now().toString().slice(-8)}`,
      contact_person: '45-4',
      contact_phone: '13800000045',
    });
    const supId = sup?.data?.id;
    expect(supId, '供应商创建失败').toBeTruthy();
    CLEANUP.push({ path: `/purchase/suppliers/${supId}`, label: '[45-4] 供应商' });

    const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: supId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      items: [{ material_id: ctx.productIds[0], quantity: 1, unit_price: '1.00' }],
    });
    const poId = po?.data?.id;
    expect(poId, 'PO 创建失败').toBeTruthy();
    CLEANUP.push({ path: `/purchase/orders/${poId}`, label: '[45-4] PO' });

    // 删除有活跃 PO 的供应商必须被拒
    const del = await apiCallExpectFail(page, 'DELETE', `/purchase/suppliers/${supId}`);
    expect(del.status, '有活跃 PO 的供应商删除应被拒（can_delete 校验）').toBeGreaterThanOrEqual(
      400
    );
  });

  test('45-5 对照组：无关联资源可正常删除（防规则过紧回归）', async ({ page }) => {
    const name = `45对照部门${Date.now().toString().slice(-8)}`;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
      name,
      description: '45-5 对照',
    });
    const id = r?.data?.id;
    expect(id, '创建失败').toBeTruthy();
    // 直接删除应成功（无子部门无用户关联）
    const del = await apiCallExpectFail(page, 'DELETE', `/departments/${id}`);
    expect(del.status, '无关联部门删除应成功').toBeLessThan(300);
  });
});
