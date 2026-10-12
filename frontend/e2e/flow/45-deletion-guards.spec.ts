import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  expectBusinessRejection,
  tryCleanup,
  ensureTestEntities,
  getCtx,
} from './helpers';

/**
 * L2 删除约束矩阵（flow/45-deletion-guards）
 *
 * rule provenance：每条用例对应后端一条删除前置校验。
 *
 * 收紧假绿：原实现普遍用 `expect(status).toBeGreaterThanOrEqual(400)`，并在 45-1 里把
 * `catch { delStatus = 400 }` 硬编码——只要 apiCall 抛出就无条件记 400，等于恒真。裸 500
 * （后端缺前置校验、靠 DB FK 约束在 delete 阶段炸出 DATABASE_ERROR）会被 >=400 伪装成
 * "删除守卫生效"的绿灯。现统一改用 expectBusinessRejection：钉死 HTTP=400、code 为业务
 * 机器码且不属于 5xx 家族、响应含非空拒绝 message。若后端确为裸 500，用例会红——那是源码
 * 缺陷（应补前置业务校验返回 400 BUSINESS_ERROR），交后端修，禁止把断言放宽回 >=400。
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

  test('45-1 部门：存在子部门禁止删除（department_service.rs:266）', async ({ page }) => {
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

    // 删除父部门必须被业务拒绝：apiCallExpectFail 带回 CSRF 恢复，返回真实业务错误信封。
    const del = await apiCallExpectFail(page, 'DELETE', `/departments/${parentId}`);
    expectBusinessRejection(
      del,
      `有子部门的父部门 ${parentId} 删除应被拒（400 BUSINESS_ERROR + 拒绝原因）`
    );
  });

  test('45-2 产品类别：存在子类别禁止删除（product_category_service.rs:168）', async ({ page }) => {
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
    expectBusinessRejection(del, `有子类别的父分类 ${parentId} 删除应被拒（400 + 拒绝原因）`);
  });

  test('45-3 会计科目：父有子科目时禁止删除（account_subject_service.rs:289）', async ({
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

    // 父有子 → 删除必被业务拒绝（service delete："不能删除有子科目的科目"，bad_request）
    const delParent = await apiCallExpectFail(page, 'DELETE', `/subjects/${parentId}`);
    expectBusinessRejection(delParent, `有子科目的父科目 ${parentId} 删除应被拒（400 + 拒绝原因）`);

    // 对照组：无子的子科目删除应成功（apiCall 失败即抛真实响应），
    // 证明上一条 400 确由删除守卫产生，而非 CSRF/权限/路径错误
    const delChild = await apiCall(page, 'DELETE', `/subjects/${childId}`);
    expect(delChild.code, '无子科目的子科目删除应成功').toBe(200);
    // 子科目已在测试内删除，撤销其 cleanup 登记避免重复删除噪声
    const idx = CLEANUP.findIndex(c => c.path === `/subjects/${childId}`);
    if (idx >= 0) CLEANUP.splice(idx, 1);
  });

  test('45-4 供应商：有引用单据禁止删除（supplier_service.rs:516 business_displayable）', async ({
    page,
  }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
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

    // 删除被 PO 引用的供应商必须被业务拒绝（引用前置校验返回 400 BUSINESS_ERROR，
    // 而非靠 DB FK 在 delete 阶段裸抛 DATABASE_ERROR(500)——后者会被本断言判红=源码缺陷）
    const del = await apiCallExpectFail(page, 'DELETE', `/purchase/suppliers/${supId}`);
    expectBusinessRejection(
      del,
      `被 PO 引用的供应商 ${supId} 删除应被拒（400 + 拒绝原因，不得是裸 500）`
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
    // 直接删除应成功（无子部门无用户关联）：apiCall 失败即抛真实错误，无需 >=/<= 兜底
    const del = await apiCall(page, 'DELETE', `/departments/${id}`);
    expect(del.code, '无关联部门删除应成功（200）').toBe(200);
  });
});
