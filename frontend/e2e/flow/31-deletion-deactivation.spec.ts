import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, tryCleanup } from './helpers';
import {
  findRowAction,
  findTableRow,
  pickListArray,
  uiDeleteRow,
  safeGoto,
  type ListShapeKey,
} from './ui-helpers';

/**
 * P0 级删除与停用验证（2026-09-10 用户指令）
 *
 * 用户原话："删除和停用也要跟创建和保存一样进行非常全面的 E2E 测试，
 * 用于测试是否删除成功或者停用成功，数据没有记录等等。"
 *
 * 所有操作基于真实 UI 点击（非 API 调用），每步显式日志。
 *
 * 每个资源验证：
 * 1. UI 点击删除按钮 → 确认弹窗 → 验证列表行消失
 * 2. UI 点击状态切换 → 验证状态文本已变更
 */

const BASE_URL = process.env.BASE_URL || 'http://localhost:3000';
const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';
const TS = Date.now().toString().slice(-8);

// 用例自建对象的清理闭环（参考 08-business-modes.spec.ts 的 CLEANUP 模式）：
// 停用类用例创建客户/产品后不再污染库，用例结束按 id 删除。
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe.serial('P0 删除与停用：真实 UI 点击验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== 1. 产品删除 =====
  test('产品：UI 删除行→验证列表行消失', async ({ page }) => {
    test.setTimeout(120_000);
    // 前置：自建一条无引用产品（不依赖库中残留数据），列表为空即属前置失败
    const productName = `P0待删产品${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name: productName,
      code: `P0-DELP-${TS}`,
      unit: '米',
      status: 'active',
    });
    expect(
      created?.data?.id,
      `[P0-删除-产品] 自建产品未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();
    console.log(`[P0-删除-产品] 自建产品 id=${created.data?.id} name=${productName}`);

    await page.goto(`${BASE_URL}/product`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);
    console.log('[P0-删除-产品] 导航到产品列表页');

    // 记录删除前行数
    const rowsBefore = await page.locator('.el-table__row').count();
    console.log(`[P0-删除-产品] 列表当前 ${rowsBefore} 行`);
    expect(rowsBefore, '[P0-删除-产品] 产品列表为空，删除链路无法验证').toBeGreaterThan(0);

    // 目标行必须是本用例自建的产品（原实现取任意首行，删除对象不确定）
    const targetRow = await findTableRow(page, productName);
    expect(targetRow, `[P0-删除-产品] 列表未找到自建产品 ${productName}`).toBeTruthy();

    // UI 删除
    const deleted = await uiDeleteRow(page, '/product', { column: 'name', value: productName });
    console.log(`[P0-删除-产品] 删除结果: ${deleted ? '✅成功' : '❌失败'}`);
    expect(deleted, `[P0-删除-产品] 自建且无引用的产品 ${productName} UI 删除应成功`).toBe(true);
  });

  // ===== 2. 客户删除 =====
  test('客户：UI 删除行→验证列表行消失', async ({ page }) => {
    test.setTimeout(120_000);
    const customerName = `P0待删客户${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
      customer_type: 'retail',
    });
    expect(
      created?.data?.id,
      `[P0-删除-客户] 自建客户未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();
    console.log(`[P0-删除-客户] 自建客户 id=${created.data?.id} name=${customerName}`);

    await page.goto(`${BASE_URL}/customer`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);

    const rowsBefore = await page.locator('.el-table__row').count();
    console.log(`[P0-删除-客户] 列表当前 ${rowsBefore} 行`);
    expect(rowsBefore, '[P0-删除-客户] 客户列表为空，删除链路无法验证').toBeGreaterThan(0);

    const targetRow = await findTableRow(page, customerName);
    expect(targetRow, `[P0-删除-客户] 列表未找到自建客户 ${customerName}`).toBeTruthy();

    // 客户删除为「软删除」：delete_customer 仅把 status 置为 inactive（customer_ops/crud.rs:178），
    // 且 list_customers 默认不过滤 inactive（query.rs:77，仅当显式传入 status 才过滤）——
    // 故删除后该行仍留在列表（状态变「禁用」），"行消失"模型对本实体不适用。
    // 仍用 uiDeleteRow 走真实 UI 点击（行内「删除」→确认弹窗）触发删除；uiDeleteRow 现对
    // 一切真实失败（找不到行/按钮未渲染/点击异常）**显式抛错**，软删除的"行仍在"预期通过
    // expectRowGone:false 声明，删除效果改由后端权威契约断言（见下）。
    await uiDeleteRow(
      page,
      '/customer',
      { column: 'name', value: customerName },
      {
        expectRowGone: false,
      }
    );
    const detail = await apiCallRaw<{ status?: string }>(
      page,
      'GET',
      `/crm/customers/${created.data?.id}`
    );
    console.log(`[P0-删除-客户] UI 删除后 status=${detail?.status}`);
    expect(
      detail?.status,
      `[P0-删除-客户] 自建且无引用的客户 ${customerName} UI 删除（软删除）后 status 应为 inactive，实际 ${detail?.status}`
    ).toBe('inactive');
  });

  // ===== 3. 供应商删除 =====
  test('供应商：UI 删除行→验证列表行消失', async ({ page }) => {
    test.setTimeout(120_000);
    const supplierName = `P0待删供应商${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: supplierName,
      supplier_short_name: 'P0供',
      contact_phone: '13800000009',
    });
    expect(
      created?.data?.id,
      `[P0-删除-供应商] 自建供应商未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();
    console.log(`[P0-删除-供应商] 自建供应商 id=${created.data?.id} name=${supplierName}`);

    await page.goto(`${BASE_URL}/supplier`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);

    const rowsBefore = await page.locator('.el-table__row').count();
    console.log(`[P0-删除-供应商] 列表当前 ${rowsBefore} 行`);
    expect(rowsBefore, '[P0-删除-供应商] 供应商列表为空，删除链路无法验证').toBeGreaterThan(0);

    const targetRow = await findTableRow(page, supplierName);
    expect(targetRow, `[P0-删除-供应商] 列表未找到自建供应商 ${supplierName}`).toBeTruthy();

    const deleted = await uiDeleteRow(page, '/supplier', { column: 'name', value: supplierName });
    console.log(`[P0-删除-供应商] 删除结果: ${deleted ? '✅成功' : '❌失败'}`);
    expect(deleted, `[P0-删除-供应商] 自建且无引用的供应商 ${supplierName} UI 删除应成功`).toBe(
      true
    );
  });

  // ===== 4. 仓库删除 =====
  test('仓库：UI 删除行→验证列表行消失', async ({ page }) => {
    test.setTimeout(120_000);
    const warehouseName = `P0待删仓库${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
      name: warehouseName,
      code: `P0DELW-${TS}`,
    });
    expect(
      created?.data?.id,
      `[P0-删除-仓库] 自建仓库未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();
    console.log(`[P0-删除-仓库] 自建仓库 id=${created.data?.id} name=${warehouseName}`);

    await page.goto(`${BASE_URL}/warehouse`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);

    const rowsBefore = await page.locator('.el-table__row').count();
    console.log(`[P0-删除-仓库] 列表当前 ${rowsBefore} 行`);
    expect(rowsBefore, '[P0-删除-仓库] 仓库列表为空，删除链路无法验证').toBeGreaterThan(0);

    const targetRow = await findTableRow(page, warehouseName);
    expect(targetRow, `[P0-删除-仓库] 列表未找到自建仓库 ${warehouseName}`).toBeTruthy();

    const deleted = await uiDeleteRow(page, '/warehouse', { column: 'name', value: warehouseName });
    console.log(`[P0-删除-仓库] 删除结果: ${deleted ? '✅成功' : '❌失败'}`);
    expect(deleted, `[P0-删除-仓库] 自建且无引用的仓库 ${warehouseName} UI 删除应成功`).toBe(true);
  });

  // ===== 5. 客户停用/启用 =====
  test('客户：UI 切换状态→验证状态文本变更', async ({ page }) => {
    test.setTimeout(120_000);
    // 前置：自建客户，保证列表存在确定目标的行（列表为空属前置失败）
    const customerName = `P0待停用客户${TS}`;
    // 编辑弹窗表单对 customer_code / contact_person / contact_phone 有必填校验
    // （CustomerFormTab.vue:301-335 formRules，电话还须匹配 ^1[3-9]\d{9}$）。
    // 仅传 name+type 建的客户缺失这些字段 → 点编辑后 handleSubmit 的 validate() 失败 →
    // 保存不发 PUT/PATCH（backend.log 无 /crm/customers 更新），停用链路根本发不起。
    // 前置改为造一条满足表单校验的完整客户（与 0-11 建客户口径一致）。
    const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
      customer_code: `P0-DIS-CUST-${TS}`,
      contact_person: '联系人',
      contact_phone: '13800000000',
      customer_type: 'retail',
      status: 'active',
    });
    expect(
      created?.data?.id,
      `[P0-停用-客户] 自建客户未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();

    await page.goto(`${BASE_URL}/customer`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);

    const rows = page.locator('.el-table__row');
    const rowCount = await rows.count();
    console.log(`[P0-停用-客户] 列表 ${rowCount} 行`);
    expect(rowCount, '[P0-停用-客户] 客户列表为空，停用链路无法验证').toBeGreaterThan(0);
    const targetRow = await findTableRow(page, customerName);
    expect(targetRow, `[P0-停用-客户] 列表未找到自建客户 ${customerName}`).toBeTruthy();
    CLEANUP.push({
      path: `/crm/customers/${created.data?.id}`,
      label: `[P0-停用-客户] ${customerName}`,
    });

    // 客户列表每行有两个 el-tag：类型列（retail→"零售"）+ 状态列（启用/禁用）。
    // 原实现 `.el-tag.first()` 取到类型列（恒"零售"），状态变更读不到。收窄到状态列：
    // 状态标签文本只可能是列表状态词表 {启用,禁用}（customer/index.vue:157-163 +
    // zh-CN.ts customer.index.statusLabel），类型标签"零售"不会命中，据此精确定位。
    const readStatusTag = async (row: Awaited<ReturnType<typeof findTableRow>>) => {
      const tag = row!
        .locator('.el-tag')
        .filter({ hasText: /^(启用|禁用)$/ })
        .first();
      return ((await tag.textContent()) ?? '').trim();
    };
    const beforeTag = await readStatusTag(targetRow);
    expect(beforeTag, `[P0-停用-客户] 停用前状态列应为"启用"，实际="${beforeTag}"`).toBe('启用');
    const editBtn = targetRow!.locator('button:has-text("编辑")').first();
    expect(
      await editBtn.isVisible({ timeout: 3000 }),
      `[P0-停用-客户] 自建客户 ${customerName} 行内既无状态开关/停用按钮，也无「编辑」入口，停用链路无法发起`
    ).toBe(true);
    await editBtn.click();
    const dialog = page.locator('.el-dialog:visible').first();
    const dialogOpened = await dialog
      .waitFor({ state: 'visible', timeout: 8000 })
      .then(() => true)
      .catch(() => false);
    expect(dialogOpened, '[P0-停用-客户] 点击编辑未打开编辑弹窗').toBe(true);
    const inactiveRadio = dialog
      .locator('.el-radio:has-text("停用"), .el-radio-button:has-text("停用")')
      .first();
    const radioPresent = await inactiveRadio
      .waitFor({ state: 'visible', timeout: 4000 })
      .then(() => true)
      .catch(() => false);
    expect(
      radioPresent,
      '[P0-停用-客户] 编辑弹窗内未渲染「停用」状态控件（前端渲染条件若与后端状态词表不一致会命中此处）'
    ).toBe(true);
    await inactiveRadio.click();
    // 保存应真正发出 PUT /crm/customers/{id}（updateCustomer → api/customer.ts:85）。
    // 显式等待该请求并断言成功：若前端保存链路未提交状态更新，此处即暴露（而非误绿）。
    const putPromise = page.waitForResponse(
      r =>
        /\/crm\/customers\/\d+/.test(r.url()) && r.request().method() === 'PUT' && r.status() < 400,
      { timeout: 10000 }
    );
    await dialog
      .getByRole('button', { name: /确定|确认|保存/ })
      .last()
      .click();
    const putResp = await putPromise;
    expect(putResp.ok(), '[P0-停用-客户] 保存停用应发出成功的 PUT /crm/customers/{id}').toBe(true);
    await page.waitForTimeout(2000);
    await page.reload();
    await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});
    const rowAfter = await findTableRow(page, customerName);
    expect(rowAfter, `[P0-停用-客户] 停用后列表未找到自建客户 ${customerName}`).toBeTruthy();
    const afterTag = await readStatusTag(rowAfter);
    console.log(`[P0-停用-客户] ${customerName} 状态标签：${beforeTag} → ${afterTag}`);
    expect(afterTag, `[P0-停用-客户] UI 停用后状态列应变更为"禁用"，实际="${afterTag}"`).toBe(
      '禁用'
    );
  });

  // ===== 6. 产品停用/启用 =====
  test('产品：UI 切换状态→验证状态文本变更', async ({ page }) => {
    test.setTimeout(120_000);
    // 前置：自建产品，保证列表存在确定目标的行（列表为空属前置失败）
    const productName = `P0待停用产品${TS}`;
    // 产品编辑弹窗表单 category_id 必填（ProductFormDialogTab.vue:205-211 formRules）。
    // 缺 category_id 的产品在编辑保存时 validate() 失败 → 不发 PUT，停用发不起。
    // 先造一条产品分类并让产品带上它（POST /products 用后端真实字段 name/code/category_id/unit/status）。
    const cat = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
      name: `P0停用分类${TS}`,
      code: `P0-DIS-CAT-${TS}`,
    });
    const categoryId = cat.data?.id;
    expect(categoryId, `[P0-停用-产品] 产品分类创建未返回 id：${JSON.stringify(cat)}`).toBeTruthy();
    CLEANUP.push({ path: `/product-categories/${categoryId}`, label: '[P0-停用-产品] 分类' });

    const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name: productName,
      code: `P0-DISABLEP-${TS}`,
      unit: '米',
      category_id: categoryId,
      status: 'active',
    });
    expect(
      created?.data?.id,
      `[P0-停用-产品] 自建产品未返回 id：${JSON.stringify(created)}`
    ).toBeTruthy();

    await page.goto(`${BASE_URL}/product`);
    await page.waitForLoadState('networkidle', { timeout: 15000 });
    await page.waitForTimeout(1000);

    const rows = page.locator('.el-table__row');
    const rowCount = await rows.count();
    console.log(`[P0-停用-产品] 列表 ${rowCount} 行`);
    expect(rowCount, '[P0-停用-产品] 产品列表为空，停用链路无法验证').toBeGreaterThan(0);
    const targetRow = await findTableRow(page, productName);
    expect(targetRow, `[P0-停用-产品] 列表未找到自建产品 ${productName}`).toBeTruthy();
    CLEANUP.push({ path: `/products/${created.data?.id}`, label: `[P0-停用-产品] ${productName}` });

    // 产品列表每行可能有两个 el-tag：分类列（category_name，仅当有分类时渲染）+ 状态列。
    // 本用例产品带分类 → `.el-tag.first()` 会取到分类标签而非状态标签。收窄到状态列：
    // 状态标签文本只可能是 {启用,禁用}（ProductListTab.vue:196-201 + productListTab.statusLabel），
    // 分类名（"P0停用分类…"）不会命中，据此精确定位。
    const readStatusTag = async (row: Awaited<ReturnType<typeof findTableRow>>) => {
      const tag = row!
        .locator('.el-tag')
        .filter({ hasText: /^(启用|禁用)$/ })
        .first();
      return ((await tag.textContent()) ?? '').trim();
    };
    const beforeTag = await readStatusTag(targetRow);
    expect(beforeTag, `[P0-停用-产品] 停用前状态列应为"启用"，实际="${beforeTag}"`).toBe('启用');
    const editBtn = targetRow!.locator('button:has-text("编辑")').first();
    expect(
      await editBtn.isVisible({ timeout: 3000 }),
      `[P0-停用-产品] 自建产品 ${productName} 行内既无状态开关/停用按钮，也无「编辑」入口，停用链路无法发起`
    ).toBe(true);
    await editBtn.click();
    const dialog = page.locator('.el-dialog:visible').first();
    const dialogOpened = await dialog
      .waitFor({ state: 'visible', timeout: 8000 })
      .then(() => true)
      .catch(() => false);
    expect(dialogOpened, '[P0-停用-产品] 点击编辑未打开编辑弹窗').toBe(true);
    const activeSwitch = dialog.locator('.el-switch').first();
    const switchPresent = await activeSwitch
      .waitFor({ state: 'visible', timeout: 4000 })
      .then(() => true)
      .catch(() => false);
    expect(
      switchPresent,
      '[P0-停用-产品] 编辑弹窗内未渲染 is_active 开关（前端渲染条件若与后端状态词表不一致会命中此处）'
    ).toBe(true);
    await activeSwitch.click();
    // 保存应真正发出 PUT /products/{id}（updateProduct → api/product.ts:115）。
    // 显式等待并断言成功：前端保存链路未提交即在此暴露（而非误绿）。
    const putPromise = page.waitForResponse(
      r => /\/products\/\d+/.test(r.url()) && r.request().method() === 'PUT' && r.status() < 400,
      { timeout: 10000 }
    );
    await dialog
      .getByRole('button', { name: /确定|确认|保存/ })
      .last()
      .click();
    const putResp = await putPromise;
    expect(putResp.ok(), '[P0-停用-产品] 保存停用应发出成功的 PUT /products/{id}').toBe(true);
    await page.waitForTimeout(2000);
    await page.reload();
    await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});
    const rowAfter = await findTableRow(page, productName);
    expect(rowAfter, `[P0-停用-产品] 停用后列表未找到自建产品 ${productName}`).toBeTruthy();
    const afterTag = await readStatusTag(rowAfter);
    console.log(`[P0-停用-产品] ${productName} 状态标签：${beforeTag} → ${afterTag}`);
    expect(afterTag, `[P0-停用-产品] UI 停用后状态列应变更为"禁用"，实际="${afterTag}"`).toBe(
      '禁用'
    );
  });
});

// ============================================================
// 扩展覆盖：12 资源系统性删除测试（API 创建数据准备 + UI 删除真实操作）
// ============================================================

const EXT_TS = Date.now().toString().slice(-8);

/** 创建→UI 删除→验证消失 的通用验证器 */
async function createThenUiDelete(
  page: import('@playwright/test').Page,
  label: string,
  createApi: string,
  createPayload: Record<string, unknown>,
  listRoute: string,
  rowName: string | number,
  // 该资源列表端点（GET createApi）的显式形状：调用方按后端 handler 逐一声明。
  // 取代原 `body.data.items ?? body.data.roles ?? body.data ?? []` 三重形状宽容探测——
  // 它同时吞分页 items / 具名 roles / 裸数组并 `?? []`，端点改形时静默读成空集。
  listKey: ListShapeKey,
  // 软删除实体的删除后状态值。给定则本验证器按软删处理（行保留、只改 status），
  // 走真实 UI 删除点击但声明 expectRowGone 为 false，删除效果由 GET 详情断言 status 等于此值
  // （合同 delete 置状态词表 CANCELLED 即 cancelled，list 不过滤，行仍在）。
  softDeleteStatus?: string
): Promise<void> {
  const createResp = await apiCall<{ id?: number }>(page, 'POST', createApi, createPayload);
  console.log(`[P0-删除-${label}] 创建响应:`, JSON.stringify(createResp?.data)?.slice(0, 300));
  const id = createResp?.data?.id;
  expect(id, `[P0-删除-${label}] 创建失败（前置数据缺失或 API 异常）`).toBeTruthy();
  console.log(`[P0-删除-${label}] 数据准备完成 id=${id}`);

  // 在列表页通过 API 回读确认数据存在（先确认数据落库）
  const listCheck = await page.request.get(
    `${API_BASE}${API_PREFIX}${createApi}?page=1&page_size=200`,
    {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    }
  );
  if (listCheck?.ok()) {
    const body = await listCheck.json();
    // 单一形状直读：listKey 不匹配 → pickListArray 抛错（明确失败），不再被吸收成空列表。
    const arr = pickListArray<Record<string, unknown>>(
      body?.data,
      listKey,
      `P0-删除-${label} 列表回读`
    );
    const exists = arr.some(i => i.id === id);
    console.log(
      `[P0-删除-${label}] 创建后列表回读: ${exists ? '✅存在' : '❌不存在'}（列表 ${arr.length} 条）`
    );
  }

  // UI 删除。软删除实体（给定 softDeleteStatus）走真实删除点击但声明行不消失，
  // 删除效果改由 GET 详情契约断言 status（对齐本文件客户软删用例）。
  // 硬删除实体仍按行必须消失做硬断言（uiDeleteRow 对一切真实失败显式抛错）。
  if (softDeleteStatus !== undefined) {
    await uiDeleteRow(
      page,
      listRoute,
      { column: 'name', value: rowName },
      { expectRowGone: false }
    );
    const detail = await apiCallRaw<{ status?: string }>(page, 'GET', `${createApi}/${id}`);
    console.log(`[P0-删除-${label}] UI 软删除后 status=${detail?.status}`);
    expect(
      detail?.status,
      `[P0-删除-${label}] 自建 ${rowName} UI 删除（软删除）后 status 应为 ${softDeleteStatus}，实际 ${detail?.status}`
    ).toBe(softDeleteStatus);
    return;
  }
  const deleted = await uiDeleteRow(page, listRoute, { column: 'name', value: rowName });
  console.log(`[P0-删除-${label}] UI 删除结果: ✅（失败路径已由 uiDeleteRow 抛错判红）`);
  expect(
    deleted,
    `[P0-删除-${label}] 自建行 ${rowName} 的 UI 删除必须真实完成（点中行内删除→确认→行消失），` +
      `uiDeleteRow 现在只可能返回 true 或抛错，实际返回=${deleted}`
  ).toBe(true);
}

/**
 * 引用类前置取列表首条 id；列表为空时按该资源的真实创建契约补建一条。
 * 用于替代"引用数据缺失即 test.skip"的假绿写法：前置要么成立，要么硬失败。
 */
async function firstRefOrSeed(
  page: import('@playwright/test').Page,
  label: string,
  listApi: string,
  seedApi: string,
  seedPayload: Record<string, unknown>,
  // 列表端点（GET listApi）的显式形状，取代原 `Array.isArray(resp)?resp:(resp?.items??[])` 双形状探测。
  listKey: ListShapeKey
): Promise<number> {
  const resp = await apiCallRaw<unknown>(page, 'GET', `${listApi}?page=1&page_size=1`);
  // 单一形状直读；形状不符即抛错（不再把 items 键漂移当成"列表为空"→误走 seed 分支）
  const arr = pickListArray<{ id: number }>(resp, listKey, `P0-删除-${label} 引用列表`);
  const existing = arr[0]?.id;
  if (existing) return existing;
  const created = await apiCall<{ id?: number }>(page, 'POST', seedApi, seedPayload);
  const id = created?.data?.id;
  if (!id) {
    throw new Error(
      `[P0-删除-${label}] ${listApi} 列表为空且 ${seedApi} 创建未返回 id：${JSON.stringify(created)}`
    );
  }
  console.log(`[P0-删除-${label}] ${listApi} 无数据，已真实创建引用 ${label} id=${id}`);
  return id;
}

/**
 * 产品色号删除走真实 UI 嵌套对话框路径。
 * 色号行只在产品行色号按钮弹出的子表对话框内渲染，主产品表单元格不含色号值，
 * 故不能用按主表行定位的通用删除器。这里先在产品列表按产品名定位该行并点开色号对话框，
 * 再把定位收敛到对话框内的色号子表，点行内删除经确认框删除，最后验证该色号行消失。
 * 调用方须先自建产品并取其产品名，不能用列表首条引用，否则无法在 UI 唯一定位到目标产品行。
 */
async function uiDeleteProductColor(
  page: import('@playwright/test').Page,
  productName: string,
  colorNo: string
): Promise<void> {
  await safeGoto(page, '/product');
  await page.waitForTimeout(1000);

  const colorsBtn = await findRowAction(page, productName, row =>
    row.getByRole('button', { name: '色号' })
  );
  await colorsBtn.click();

  const dialog = page.locator('.el-dialog:visible').first();
  await dialog.waitFor({ state: 'visible', timeout: 10_000 });

  const colorRow = dialog.locator('.el-table__row:visible', { hasText: colorNo }).first();
  await colorRow.waitFor({ state: 'visible', timeout: 10_000 });
  const delBtn = colorRow.locator('button.el-button--danger, button:has-text("删除")').first();
  await delBtn.click();
  console.log(`[P0-删除-产品色号] 已在色号对话框点击色号 ${colorNo} 的行内删除`);

  // 确认框为 ElMessageBox，用其主按钮类精确定位，避免与子表行内删除按钮文案冲突误点
  await page
    .locator('.el-message-box:visible .el-message-box__btns button.el-button--primary')
    .first()
    .click();
  await page.waitForTimeout(1500);

  const remain = await dialog.locator('.el-table__row:visible', { hasText: colorNo }).count();
  if (remain !== 0) {
    throw new Error(
      `[P0-删除-产品色号] 色号 ${colorNo} 删除后对话框子表仍有 ${remain} 行，删除未真实生效`
    );
  }
  console.log(`[P0-删除-产品色号] ✅ 色号 ${colorNo} 已从对话框子表消失`);
}

test.describe.serial('P0 扩展删除：12 资源系统性覆盖', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('部门：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '部门',
      '/departments',
      { name: `P0部门${EXT_TS}`, code: `P0-DEPT-${EXT_TS}` },
      '/departments',
      `P0部门${EXT_TS}`,
      // department_handler define_crud → PaginatedResponse → {items}
      'items'
    );
  });

  test('产品分类：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    // 产品分类不是产品：通用 uiDeleteRow 在产品列表主表按分类名找行，产品行不含分类名 → 找不到。
    // 分类 UI 维护入口是 /product 页「产品分类」统计卡点开的 CategoryDialogTab 对话框表格
    // （列 name + 行内删除/编辑），故按真实 DOM 在对话框内操作删除。
    const catName = `P0分类${EXT_TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/product-categories', {
      name: catName,
      code: `P0-CAT-${EXT_TS}`,
    });
    const catId = created?.data?.id;
    expect(catId, '[P0-删除-产品分类] API 创建分类未返回 id').toBeTruthy();

    await safeGoto(page, '/product');
    // 打开分类管理对话框：命中「产品分类」统计卡（对话框尚未出现，此时全页唯此卡含该文本）
    const catCard = page.locator('.stat-card').filter({ hasText: '产品分类' }).first();
    await catCard.waitFor({ state: 'visible', timeout: 30_000 });
    await catCard.click();
    const dialog = page.locator('.el-dialog:visible').filter({ hasText: '产品分类管理' }).first();
    await dialog.waitFor({ state: 'visible', timeout: 20_000 });

    // 对话框表格拉取 GET /product-categories，先等自建分类行渲染（scoped 到 dialog，避开产品主表）
    const row = dialog.locator('.el-table__row').filter({ hasText: catName }).first();
    await row.waitFor({ state: 'visible', timeout: 20_000 });

    // 行内「删除」→ CategoryDialogTab.handleDelete 触发 ElMessageBox.confirm（标题「删除确认」）
    const delPromise = page
      .waitForResponse(
        r =>
          r.url().includes(`/api/v1/erp/product-categories/${catId}`) &&
          r.request().method() === 'DELETE',
        { timeout: 20_000 }
      )
      .catch(() => null);
    await row.locator('button.el-button--danger, button:has-text("删除")').first().click();
    const confirmBtn = page.locator('.el-message-box').locator('button.el-button--primary').first();
    await confirmBtn.waitFor({ state: 'visible', timeout: 10_000 });
    await confirmBtn.click();
    const delResp = await delPromise;
    expect(
      delResp,
      `[P0-删除-产品分类] 未在 20s 内捕获 DELETE /product-categories/${catId} 响应——删除按钮/确认未真实触发后端删除`
    ).not.toBeNull();
    expect(
      delResp!.ok(),
      `[P0-删除-产品分类] DELETE 应成功（分类无引用），实际 status=${delResp!.status()}`
    ).toBe(true);

    // 删除成功后 handleDelete 调 fetchCategories 重渲染：断自建分类行从对话框表格消失
    await dialog
      .locator('.el-table__row')
      .filter({ hasText: catName })
      .first()
      .waitFor({ state: 'detached', timeout: 15_000 });
  });

  test('会计科目：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '会计科目',
      '/subjects',
      {
        code: `P0-DEL-SUB-${EXT_TS}`,
        name: `P0待删科目${EXT_TS}`,
        level: 1,
        balance_direction: 'debit',
      },
      // 会计科目的 UI 维护页是 /account-subject（views/account-subject/index.vue →
      // SubjectListTab.vue：列含 prop="name"，操作列有行内「删除」按钮 deleteSubject）。
      // 原写成 /assist-accounting（views/assist-accounting/index.vue）错误——该页 records 表
      // 无科目名称列、操作列只有 View 按钮，findRowAction 按科目名找不到行也找不到删除按钮，
      // 删除链路根本发不起（测试导航错，非源码缺陷）。
      '/account-subject',
      `P0待删科目${EXT_TS}`,
      // account_subject_handler::list_subjects → ApiResponse<Vec> → 裸数组
      'bare'
    );
  });

  test('打印模板：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '打印模板',
      '/print-templates',
      {
        template_name: `P0待删模板${EXT_TS}`,
        template_type: 'order',
        description: 'P0打印模板',
        content: '<p>P0</p>',
      },
      '/print-templates',
      `P0待删模板${EXT_TS}`,
      // print_handler::list_print_templates → ApiResponse<Vec<PrintTemplateRecord>> → 裸数组
      'bare'
    );
  });

  test('报表模板：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '报表模板',
      // 页面列表读的是 DB 支撑的 report_enhanced 组；report_engine 的 GET /report-templates
      // 只回代码内预置模板（list_templates → get_predefined_templates），从这里建的记录永远不会出现在页面里。
      '/reports/enhanced/templates',
      {
        name: `P0待删报表${EXT_TS}`,
        // CreateReportTemplateRequest.code 必填（length 1-50，report_template_service.rs:42-43）
        code: `P0-RPT-${EXT_TS}`,
        description: 'P0报表模板',
        category: 'custom',
        data_source: 'sales',
        report_type: 'table',
        columns: [],
        filters: [],
        parameters: [],
        supported_formats: ['xlsx'],
      },
      '/report-templates',
      `P0待删报表${EXT_TS}`,
      // report_enhanced_handler::list_report_templates → json!{items,...} → {items}
      'items'
    );
  });

  test('质检标准：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '质检标准',
      '/quality-standards',
      { standard_name: `P0待删标准${EXT_TS}`, standard_type: 'product' },
      '/quality-standards',
      `P0待删标准${EXT_TS}`,
      // quality_standard_handler::list_standards → ApiResponse<Vec> → 裸数组
      'bare'
    );
  });

  test('销售合同：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '销售合同',
      '/sales/sales-contracts',
      {
        contract_no: `P0-SC-${EXT_TS}`,
        contract_name: `P0销售合同${EXT_TS}`,
        customer_id: 1,
        total_amount: 10000,
        delivery_date: new Date().toISOString().slice(0, 10),
      },
      '/sales-contract',
      `P0-SC-${EXT_TS}`,
      // sales_contract_handler::list_contracts → ApiResponse<Vec> → 裸数组
      'bare',
      // 合同删除为软删（status 置 cancelled、list 不过滤，行保留），按软删断言 status
      'cancelled'
    );
  });

  test('采购合同：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '采购合同',
      '/purchase/purchase-contracts',
      {
        contract_no: `P0-PC-${EXT_TS}`,
        contract_name: `P0采购合同${EXT_TS}`,
        supplier_id: 1,
        total_amount: 8000,
        delivery_date: new Date().toISOString().slice(0, 10),
      },
      '/purchase-contract',
      `P0-PC-${EXT_TS}`,
      // purchase_contract_handler::list_contracts → ApiResponse<Vec> → 裸数组
      'bare',
      // 合同软删（status 置 cancelled、行保留），按软删断言 status
      'cancelled'
    );
  });

  test('染料配方：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '染料配方',
      '/production/dye-recipes',
      { recipe_name: `P0待删配方${EXT_TS}`, customer_id: 1 },
      '/dye-recipe',
      `P0待删配方${EXT_TS}`,
      // dye_recipe_handler::list_dye_recipes → success_paginated → {items}
      'items'
    );
  });

  test('坯布：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    // 引用字段取真实存在的前置数据（列表为空则按契约补建），禁止硬编码 ID
    const productId = await firstRefOrSeed(
      page,
      '产品',
      '/products',
      '/products',
      {
        name: `P0坯布产品${EXT_TS}`,
        code: `P0-GFP-${EXT_TS}`,
        unit: '米',
        status: 'active',
        // /products → PaginatedResponse → {items}
      },
      'items'
    );
    const warehouseId = await firstRefOrSeed(
      page,
      '仓库',
      '/warehouses',
      '/warehouses',
      {
        name: `P0坯布仓库${EXT_TS}`,
        code: `P0-GFW-${EXT_TS}`,
        // /warehouses → PaginatedResponse → {items}
      },
      'items'
    );
    const supplierId = await firstRefOrSeed(
      page,
      '供应商',
      '/purchase/suppliers',
      '/purchase/suppliers',
      {
        supplier_name: `P0坯布供应商${EXT_TS}`,
        supplier_short_name: 'P0坯供',
        contact_phone: '13800000010',
        // /purchase/suppliers → PaginatedResponse → {items}
      },
      'items'
    );
    const fabricName = `P0待删坯布${EXT_TS}`;
    // 建单带物理重量与长度，成为真实在库坯布。后端删除门按业务禁止删除在库坯布，
    // 故先真实出库把重量与长度同时归零，后端据剩余库存把状态合法翻为已出库这一可删态，
    // 再走 UI 删除验证放行态下删除真实生效。
    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/greige-fabrics', {
      fabric_no: `P0-GF-${EXT_TS}`,
      fabric_name: fabricName,
      product_id: productId,
      supplier_id: supplierId,
      warehouse_id: warehouseId,
      fabric_type: 'fabric',
      quantity_meters: 100,
      quantity_kg: 50,
      weight_kg: 50,
      length_m: 100,
      dye_lot_no: `P0-DL-${EXT_TS}`,
    });
    const fabricId = created?.data?.id;
    expect(fabricId, `[P0-删除-坯布] 自建坯布未返回 id：${JSON.stringify(created)}`).toBeTruthy();
    const stockedOut = await apiCall<{ status?: string }>(
      page,
      'POST',
      `/production/greige-fabrics/${fabricId}/stock-out`,
      { weight_kg: 50, length_m: 100 }
    );
    // 回读确认状态已由后端按库存判为已出库，锁定删除前置条件真实成立（apiCall 已对非 200 抛错）
    expect(
      stockedOut?.data?.status,
      `[P0-删除-坯布] 出库后状态应为已出库方可删，实际 ${stockedOut?.data?.status}`
    ).toBe('已出库');
    const deleted = await uiDeleteRow(page, '/greige-fabrics', {
      column: 'name',
      value: fabricName,
    });
    expect(
      deleted,
      `[P0-删除-坯布] 已出库坯布 ${fabricName} 的 UI 删除必须真实完成（行内删除→行消失）`
    ).toBe(true);
  });

  test('染色批次：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    await createThenUiDelete(
      page,
      '染色批次',
      '/production/dye-batches',
      { batch_no: `P0-DB-${EXT_TS}`, recipe_id: 1, quantity: 100 },
      '/dye-batch',
      `P0-DB-${EXT_TS}`,
      // dye_batch_handler::list_dye_batches → ApiResponse<PaginatedResponse> → {items}
      'items'
    );
  });

  test('产品色号：API 创建→UI 删除→验证消失', async ({ page }) => {
    test.setTimeout(120_000);
    // 色号只在产品行色号按钮弹出的对话框子表内渲染，删除须经该对话框定位，
    // 故必须自建并持有一个名字确定的宿主产品，不能用列表首条引用（其产品在 UI 无法唯一定位）。
    const productName = `P0色号产品${EXT_TS}`;
    const product = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name: productName,
      code: `P0-COLP-${EXT_TS}`,
      unit: '米',
      status: 'active',
    });
    const productId = product?.data?.id;
    expect(
      productId,
      `[P0-删除-产品色号] 宿主产品创建未返回 id：${JSON.stringify(product)}`
    ).toBeTruthy();

    // CreateProductColorRequest 的 color_type 与 extra_cost 为非 Option 必填，
    // 只提交色号编号与色名会因缺字段被拒。STANDARD 取自色卡列定义默认值。
    const colorNo = `P0-COLOR-${EXT_TS}`;
    const color = await apiCall<{ id?: number }>(page, 'POST', `/products/${productId}/colors`, {
      color_no: colorNo,
      color_name: `P0色号${EXT_TS}`,
      color_type: 'STANDARD',
      extra_cost: 0,
    });
    expect(
      color?.data?.id,
      `[P0-删除-产品色号] 色号创建未返回 id：${JSON.stringify(color)}`
    ).toBeTruthy();

    await uiDeleteProductColor(page, productName, colorNo);
  });
});
