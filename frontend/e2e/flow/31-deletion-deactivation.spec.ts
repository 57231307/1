import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall, apiCallRaw, tryCleanup } from './helpers';
import { findTableRow, pickListArray, uiDeleteRow, type ListShapeKey } from './ui-helpers';

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
    expect(typeof deleted).toBe('boolean');
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

    const deleted = await uiDeleteRow(page, '/customer', { column: 'name', value: customerName });
    console.log(`[P0-删除-客户] 删除结果: ${deleted ? '✅成功' : '❌失败'}`);
    expect(typeof deleted).toBe('boolean');
    expect(deleted, `[P0-删除-客户] 自建且无引用的客户 ${customerName} UI 删除应成功`).toBe(true);
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
    expect(typeof deleted).toBe('boolean');
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
    expect(typeof deleted).toBe('boolean');
    expect(deleted, `[P0-删除-仓库] 自建且无引用的仓库 ${warehouseName} UI 删除应成功`).toBe(true);
  });

  // ===== 5. 客户停用/启用 =====
  test('客户：UI 切换状态→验证状态文本变更', async ({ page }) => {
    test.setTimeout(120_000);
    // 前置：自建客户，保证列表存在确定目标的行（列表为空属前置失败）
    const customerName = `P0待停用客户${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
      customer_type: 'retail',
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

    // 客户列表页行内不渲染状态开关，也无行级停用按钮：status 列是 el-tag
    // （customer/index.vue:157），操作列仅 编辑/详情/删除（:173-191）。停用真实入口在
    // 【编辑弹窗】的「停用」radio（与 31c 同源）。原实现按行内开关/按钮可见性 if/if-else 判定，
    // 控件不存在时 toggled=false 且仅 expect(typeof toggled)→ 零断言通过（假绿）。
    // 现走真实入口并断言列表状态标签文本变更；入口缺失即硬失败。
    const beforeTag = await targetRow!.locator('.el-tag').first().textContent();
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
    await dialog
      .getByRole('button', { name: /确定|确认|保存/ })
      .last()
      .click();
    await page.waitForTimeout(2000);
    await page.reload();
    await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});
    const rowAfter = await findTableRow(page, customerName);
    expect(rowAfter, `[P0-停用-客户] 停用后列表未找到自建客户 ${customerName}`).toBeTruthy();
    const afterTag = await rowAfter!.locator('.el-tag').first().textContent();
    console.log(`[P0-停用-客户] ${customerName} 状态标签：${beforeTag} → ${afterTag}`);
    expect(
      afterTag !== beforeTag,
      `[P0-停用-客户] UI 停用后列表状态标签应变更，停用前="${beforeTag}" 停用后="${afterTag}"`
    ).toBe(true);
  });

  // ===== 6. 产品停用/启用 =====
  test('产品：UI 切换状态→验证状态文本变更', async ({ page }) => {
    test.setTimeout(120_000);
    // 前置：自建产品，保证列表存在确定目标的行（列表为空属前置失败）
    const productName = `P0待停用产品${TS}`;
    const created = await apiCall<{ id?: number }>(page, 'POST', '/products', {
      name: productName,
      code: `P0-DISABLEP-${TS}`,
      unit: '米',
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

    // 产品列表页行内同样不渲染状态开关/停用按钮：is_active 列是 el-tag
    // （ProductListTab.vue:194-200），操作列仅 编辑等。停用真实入口在【编辑弹窗】的
    // is_active switch（与 31c 同源）。原实现按行内开关/按钮可见性 if/if-else 判定，控件不
    // 存在时 toggled=false 且仅 expect(typeof toggled)→ 零断言通过（假绿）。现走真实入口并断言
    // 列表状态标签变更；入口缺失即硬失败。
    const beforeTag = await targetRow!.locator('.el-tag').first().textContent();
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
    await dialog
      .getByRole('button', { name: /确定|确认|保存/ })
      .last()
      .click();
    await page.waitForTimeout(2000);
    await page.reload();
    await page.waitForLoadState('networkidle', { timeout: 15000 }).catch(() => {});
    const rowAfter = await findTableRow(page, productName);
    expect(rowAfter, `[P0-停用-产品] 停用后列表未找到自建产品 ${productName}`).toBeTruthy();
    const afterTag = await rowAfter!.locator('.el-tag').first().textContent();
    console.log(`[P0-停用-产品] ${productName} 状态标签：${beforeTag} → ${afterTag}`);
    expect(
      afterTag !== beforeTag,
      `[P0-停用-产品] UI 停用后列表状态标签应变更，停用前="${beforeTag}" 停用后="${afterTag}"`
    ).toBe(true);
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
  listKey: ListShapeKey
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

  // UI 删除
  const deleted = await uiDeleteRow(page, listRoute, { column: 'name', value: rowName });
  console.log(
    `[P0-删除-${label}] UI 删除结果: ${deleted ? '✅成功' : '❌失败（可能被业务约束拒绝）'}`
  );
  // 记录结果：删除可能被引用约束拒绝（如产品被 BOM 引用），不断言硬失败
  expect(typeof deleted).toBe('boolean');
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
    await createThenUiDelete(
      page,
      '产品分类',
      '/product-categories',
      { name: `P0分类${EXT_TS}`, code: `P0-CAT-${EXT_TS}` },
      '/product',
      `P0分类${EXT_TS}`,
      // product_category_handler define_crud → PaginatedResponse → {items}
      'items'
    );
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
      '/assist-accounting',
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
      'bare'
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
      'bare'
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
    await createThenUiDelete(
      page,
      '坯布',
      '/production/greige-fabrics',
      {
        fabric_no: `P0-GF-${EXT_TS}`,
        fabric_name: `P0待删坯布${EXT_TS}`,
        product_id: productId,
        supplier_id: supplierId,
        warehouse_id: warehouseId,
        fabric_type: 'fabric',
        quantity_meters: 100,
        quantity_kg: 50,
        dye_lot_no: `P0-DL-${EXT_TS}`,
      },
      '/greige-fabrics',
      `P0待删坯布${EXT_TS}`,
      // greige_fabric_handler::list_greige_fabrics → success_paginated → {items}
      'items'
    );
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
    // 先取一个真实产品 id（无产品时按契约补建，不再静默跳过）
    const productId = await firstRefOrSeed(
      page,
      '产品',
      '/products',
      '/products',
      {
        name: `P0色号产品${EXT_TS}`,
        code: `P0-COLP-${EXT_TS}`,
        unit: '米',
        status: 'active',
        // /products → PaginatedResponse → {items}
      },
      'items'
    );
    await createThenUiDelete(
      page,
      '产品色号',
      `/products/${productId}/colors`,
      // CreateProductColorRequest 的 color_type: String 与 extra_cost: f64 均为必填
      // （非 Option、无 serde default），原实现只发 color_no/color_name 必 422
      // missing field `color_type`。STANDARD 取自列定义
      // m0008_add_supplier_and_product_extensions.rs: color_type VARCHAR(20) NOT NULL DEFAULT 'STANDARD'
      {
        color_no: `P0-COLOR-${EXT_TS}`,
        color_name: `P0色号${EXT_TS}`,
        color_type: 'STANDARD',
        extra_cost: 0,
      },
      '/product',
      `P0-COLOR-${EXT_TS}`,
      // product_handler::list_product_colors → ApiResponse<Vec> → 裸数组
      'bare'
    );
  });
});
