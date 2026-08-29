/**
 * UI-only 辅助函数：通过 Playwright 浏览器操作完成数据创建/查询
 * 所有操作均模拟真实用户在界面上的行为（点击、填写、提交）
 * 对应 ensureTestEntities 中各实体的 UI 创建流程
 */
import type { Page } from '@playwright/test';
import { BASE_URL, API_PREFIX } from './helpers';

// 简单的唯一 ID 生成（避免循环依赖）
function _genCode(prefix: string): string { return `${prefix}-${String(Date.now()).slice(-6)}`; }
function _genName(prefix: string): string { return `${prefix}-${String(Date.now()).slice(-6)}`; }

// ---------------------------------------------------------------------------
// 公共 UI 操作原语
// ---------------------------------------------------------------------------

/** 通用对话框字段 */
type UiField =
  | { kind: 'input'; label: string; value: string }
  | { kind: 'inputNumber'; label: string; value: number }
  | { kind: 'select'; label: string; value: string }
  | { kind: 'date'; label: string; value: string };

async function fillField(dialog: import('@playwright/test').Locator, page: Page, field: UiField): Promise<void> {
  const labelRegex = new RegExp(field.label);
  const formItem = dialog.locator('.el-form-item').filter({ has: dialog.locator('.el-form-item__label').filter({ hasText: labelRegex }) }).first();
  if (await formItem.count() === 0) {
    console.warn(`[uiCreate] 找不到字段 "${field.label}"`);
    return;
  }
  switch (field.kind) {
    case 'input': {
      const inp = formItem.locator('input:not([type])').first();
      await inp.waitFor({ state: 'visible', timeout: 10000 });
      await inp.click({ clickCount: 3 });
      await inp.fill(field.value);
      break;
    }
    case 'inputNumber': {
      const inp = formItem.locator('input[type="number"]').first();
      if (await inp.count() === 0) {
        // el-input-number 默认 input 可能不可见，点击 .el-input__inner 内的 input
        const inp2 = formItem.locator('.el-input__inner input').first();
        await inp2.waitFor({ state: 'visible', timeout: 10000 });
        await inp2.click({ clickCount: 3 });
        await inp2.fill(String(field.value));
      } else {
        await inp.waitFor({ state: 'visible', timeout: 10000 });
        await inp.click({ clickCount: 3 });
        await inp.fill(String(field.value));
      }
      break;
    }
    case 'date': {
      const inp = formItem.locator('input').first();
      await inp.waitFor({ state: 'visible', timeout: 10000 });
      await inp.click({ clickCount: 3 });
      await inp.fill(field.value);
      await inp.press('Escape');
      await page.waitForTimeout(200);
      break;
    }
    case 'select': {
      const wrapper = formItem.locator('.el-select__wrapper').first();
      if (await wrapper.count() === 0) {
        // el-tree-select 等自定义控件
        const inp = formItem.locator('.el-input__inner').first();
        if (await inp.count() > 0) {
          await inp.click();
          await page.waitForTimeout(300);
        }
        return;
      }
      await wrapper.click();
      await page.waitForTimeout(300);
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 15000 });
      const item = dropdown.locator('.el-select-dropdown__item').filter({ hasText: new RegExp(field.value, 'i') }).first();
      if (await item.count() > 0) {
        await item.click();
      } else {
        // fallback: 选第一个
        await dropdown.locator('.el-select-dropdown__item').first().click();
      }
      break;
    }
  }
}

async function waitCreateResponse(page: Page, apiPath: string, timeout = 25000): Promise<Record<string, unknown> | null> {
  try {
    const resp = await page.waitForResponse(
      (r) => r.url().includes(apiPath) && r.request().method() === 'POST',
      { timeout },
    );
    const json = await resp.json().catch(() => ({}));
    return (json?.data as Record<string, unknown>) ?? json;
  } catch {
    return null;
  }
}

async function waitListResponse(page: Page, apiPath: string, timeout = 15000): Promise<unknown[]> {
  try {
    const resp = await page.waitForResponse(
      (r) => r.url().includes(apiPath) && r.request().method() === 'GET',
      { timeout },
    );
    const json = await resp.json().catch(() => ({}));
    return (json?.data?.items ?? json?.data?.list ?? []) as unknown[];
  } catch {
    return [];
  }
}

function firstId(items: unknown[]): number | undefined {
  const item = items[0] as Record<string, unknown> | undefined;
  return item?.id as number | undefined;
}

// ---------------------------------------------------------------------------
// 通用 UI 创建入口（对话框表单）
// ---------------------------------------------------------------------------

export async function uiCreateDialog(
  page: Page,
  route: string,
  createApiPath: string,
  addButtonText: RegExp,
  submitButtonText: RegExp,
  fields: UiField[],
): Promise<number | undefined> {
  await page.goto(`${BASE_URL}${route}`, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(400);
  const addBtn = page.getByRole('button', { name: addButtonText, exact: false }).first();
  await addBtn.waitFor({ state: 'visible', timeout: 15000 });
  await addBtn.click();
  const dialog = page.locator('.el-dialog:visible').last();
  await dialog.waitFor({ state: 'visible', timeout: 15000 });
  await page.waitForTimeout(300);
  for (const f of fields) {
    await fillField(dialog, page, f).catch((e) => console.warn(`[uiCreateDialog] 填表失败 "${f.label}": ${(e as Error).message}`));
  }
  await page.waitForTimeout(300);
  const submitBtn = dialog.getByRole('button', { name: submitButtonText }).last();
  await submitBtn.waitFor({ state: 'visible', timeout: 10000 });
  await submitBtn.click();
  const data = await waitCreateResponse(page, createApiPath, 25000);
  return typeof data?.id === 'number' ? data.id : undefined;
}

// ---------------------------------------------------------------------------
// 每个实体的专用 UI 创建函数
// ---------------------------------------------------------------------------

/** 创建仓库：新建仓库，必填 code/name/type；默认类型选第一个 */
export async function createWarehouseUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '仓库编码', value: _genCode('E2E-W') },
    { kind: 'input', label: '仓库名称', value: _genName('E2E仓库') },
    { kind: 'select', label: '类型', value: '普通' }, // 第一个选项
  ];
  return uiCreateDialog(page, '/warehouse', `${API_PREFIX}/warehouses`, /新建仓库/, /保存|确定/, fields);
}

/** 创建部门：新建部门，必填 name/code；status 默认启用 */
export async function createDepartmentUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '部门名称', value: _genName('E2E部门') },
    { kind: 'input', label: '部门编码', value: _genCode('E2E-D') },
  ];
  return uiCreateDialog(page, '/departments', `${API_PREFIX}/departments`, /新建部门/, /确认|保存/, fields);
}

/** 创建供应商：新建供应商，必填 supplier_code/supplier_name/contact_phone */
export async function createSupplierUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '供应商编码', value: _genCode('E2E-S') },
    { kind: 'input', label: '供应商名称', value: _genName('E2E供应商') },
    { kind: 'input', label: '联系电话', value: '13800000001' },
  ];
  return uiCreateDialog(page, '/supplier', `${API_PREFIX}/purchase/suppliers`, /新建供应商/, /确定|保存/, fields);
}

/** 创建产品：新建产品，必填 code/name/category/unit */
export async function createProductUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '产品编码', value: _genCode('E2E-P') },
    { kind: 'input', label: '产品名称', value: _genName('E2E产品') },
    { kind: 'select', label: '分类', value: '面料' }, // 第一个选项
    { kind: 'input', label: '单位', value: '米' },
  ];
  return uiCreateDialog(page, '/product', `${API_PREFIX}/products`, /新建产品/, /确定|保存/, fields);
}

/** 创建色卡：跳转到新建色卡页面，必填 card_no/card_name/card_type */
export async function createColorCardUI(page: Page): Promise<number | undefined> {
  await page.goto(`${BASE_URL}/color-cards/create`, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(600);
  const cardNoInput = page.locator('input[placeholder*="卡号" i], .el-form-item:has(span:text-is("卡号")) input').first();
  const cardNameInput = page.locator('.el-form-item:has(span:text-is("卡名")) input').first();
  const typeSelect = page.locator('.el-form-item:has(span:text-is("色卡类型")) .el-select__wrapper').first();
  await cardNoInput.waitFor({ state: 'visible', timeout: 15000 });
  await cardNoInput.click({ clickCount: 3 });
  await cardNoInput.fill(_genCode('E2E-CC'));
  await cardNameInput.waitFor({ state: 'visible', timeout: 10000 });
  await cardNameInput.click({ clickCount: 3 });
  await cardNameInput.fill(_genName('E2E色卡'));
  if (await typeSelect.count() > 0) {
    await typeSelect.click();
    await page.waitForTimeout(300);
    const dropdown = page.locator('.el-select-dropdown:visible').last();
    await dropdown.waitFor({ state: 'visible', timeout: 10000 });
    // 选 CUSTOM/自定义
    const item = dropdown.locator('.el-select-dropdown__item').filter({ hasText: /自定义|CUSTOM/i }).first();
    if (await item.count() > 0) await item.click();
    else await dropdown.locator('.el-select-dropdown__item').first().click();
  }
  const submitBtn = page.getByRole('button', { name: /立即创建|创建|确定|保存/ }).first();
  await submitBtn.waitFor({ state: 'visible', timeout: 10000 });
  await submitBtn.click();
  const data = await waitCreateResponse(page, `${API_PREFIX}/color-cards`, 25000);
  return typeof data?.id === 'number' ? data.id : undefined;
}

/** 创建染色批次：必填 batch_no/product_id/color_no/dye_date/quantity */
export async function createDyeBatchUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '批次号', value: _genCode('E2E-DB') },
    { kind: 'select', label: '产品', value: 'E2E产品' },
    { kind: 'input', label: '色号', value: _genCode('E2E-CN') },
    { kind: 'date', label: '染色日期', value: new Date().toISOString().slice(0, 10) },
    { kind: 'inputNumber', label: '数量', value: 100 },
  ];
  return uiCreateDialog(page, '/dye-batch', `${API_PREFIX}/production/dye-batches`, /新建批次/, /确定|保存/, fields);
}

/** 创建染色配方：必填 recipe_no/recipe_name/color_no/color_name/content */
export async function createDyeRecipeUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '配方编号', value: _genCode('E2E-DR') },
    { kind: 'input', label: '配方名称', value: _genName('E2E配方') },
    { kind: 'input', label: '色号', value: _genCode('E2E-CN') },
    { kind: 'input', label: '颜色名称', value: '测试色' },
    { kind: 'input', label: '配方内容', value: 'E2E测试内容' },
  ];
  return uiCreateDialog(page, '/dye-recipe', `${API_PREFIX}/production/dye-recipes`, /新建配方/, /确认|确定|保存/, fields);
}

/** 创建 BOM：必填 product_name/version/status，并添加 1 条物料明细（material_name/quantity/unit） */
export async function createBomUI(page: Page): Promise<number | undefined> {
  const dialogResult = await uiCreateDialog(
    page, '/bom', `${API_PREFIX}/boms`, /新建|新建 BOM/, /保存|确定/,
    [
      { kind: 'input', label: '产品名称', value: _genName('E2E BOM') },
      { kind: 'input', label: '版本', value: '1' },
      { kind: 'select', label: '状态', value: '启用' },
    ],
  );
  if (dialogResult === undefined) return undefined;
  // BOM 对话框打开后，需要添加至少一条物料明细再提交
  // 当前 uiCreateDialog 已提交，说明 BOM 不需要明细（默认有 1 行）
  // 若对话框尚未提交（如先填了表头但还没提交），这里处理添加物料行
  // 等待对话框出现
  const dialog = page.locator('.el-dialog:visible').last();
  const hasItemsSection = await dialog.locator('.items-section, [class*="items"]').count() > 0;
  if (hasItemsSection) {
    // 查找"添加物料"按钮并点击
    const addItemBtn = dialog.getByRole('button', { name: /添加物料|添加/, exact: false }).first();
    if (await addItemBtn.count() > 0) {
      await addItemBtn.click();
      await page.waitForTimeout(300);
      // 填写第一行物料：物料名称、数量、单位
      const firstRow = dialog.locator('.el-table tbody tr').first();
      if (await firstRow.count() > 0) {
        const matNameInput = firstRow.locator('input').first();
        await matNameInput.waitFor({ state: 'visible', timeout: 10000 });
        await matNameInput.click({ clickCount: 3 });
        await matNameInput.fill('E2E 原料');
        const unitInput = firstRow.locator('input').nth(2);
        if (await unitInput.count() > 0) {
          await unitInput.click({ clickCount: 3 });
          await unitInput.fill('米');
        }
      }
    }
    // 提交
    const submitBtn = dialog.getByRole('button', { name: /保存|确定/ }).last();
    if (await submitBtn.count() > 0) {
      await submitBtn.click();
      const data = await waitCreateResponse(page, `${API_PREFIX}/boms`, 25000);
      return typeof data?.id === 'number' ? data.id : undefined;
    }
  }
  return dialogResult;
}

/** 创建定制订单：跳转到新建页，必填 customer_id/product_id/spec/quantity */
export async function createCustomOrderUI(page: Page): Promise<number | undefined> {
  await page.goto(`${BASE_URL}/custom-orders/new`, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(600);
  // customer_id / product_id 是 el-input-number（手填数字 ID）
  const customerIdInput = page.locator('.el-form-item:has(span:text-is("客户ID")) input[type="number"], .el-input-number input').first();
  const productIdInput = page.locator('.el-form-item:has(span:text-is("产品ID")) input[type="number"], .el-input-number input').nth(1);
  const specInput = page.locator('.el-form-item:has(span:text-is("规格")) input').first();
  const quantityInput = page.locator('.el-form-item:has(span:text-is("数量")) input[type="number"], .el-input-number input').first();
  if (await customerIdInput.count() === 0) {
    // 兜底：用 placeholder 匹配
    const inputs = page.locator('input[type="number"]');
    await inputs.nth(0).waitFor({ state: 'visible', timeout: 15000 });
    await inputs.nth(0).click({ clickCount: 3 });
    await inputs.nth(0).fill('1');
    await inputs.nth(1).waitFor({ state: 'visible', timeout: 10000 });
    await inputs.nth(1).click({ clickCount: 3 });
    await inputs.nth(1).fill('1');
  } else {
    await customerIdInput.waitFor({ state: 'visible', timeout: 15000 });
    await customerIdInput.click({ clickCount: 3 });
    await customerIdInput.fill('1');
    await productIdInput.waitFor({ state: 'visible', timeout: 10000 });
    await productIdInput.click({ clickCount: 3 });
    await productIdInput.fill('1');
  }
  await specInput.waitFor({ state: 'visible', timeout: 10000 });
  await specInput.click({ clickCount: 3 });
  await specInput.fill('E2E 定制规格');
  await quantityInput.waitFor({ state: 'visible', timeout: 10000 });
  await quantityInput.click({ clickCount: 3 });
  await quantityInput.fill('100');
  const submitBtn = page.getByRole('button', { name: /保存草稿|保存|确定/ }).first();
  await submitBtn.waitFor({ state: 'visible', timeout: 10000 });
  await submitBtn.click();
  const data = await waitCreateResponse(page, `${API_PREFIX}/custom-orders`, 25000);
  return typeof data?.id === 'number' ? data.id : undefined;
}

/**
 * 通过 UI 读取列表第一行实体的 id
 * 适用于"查找已有实体"（列表不为空时复用已有数据）
 */
export async function readFirstEntityId(
  page: Page,
  route: string,
  listApiPath: string,
): Promise<number | undefined> {
  const items = await waitListResponse(page, listApiPath, 15000);
  return firstId(items);
}

/** 通用 UI 列表查找：给定路由和 API 路径，返回第一条实体的 id 列表 */
export async function readEntityIds(
  page: Page,
  route: string,
  listApiPath: string,
  limit = 10,
): Promise<number[]> {
  const items = await waitListResponse(page, listApiPath, 15000);
  return items.slice(0, limit).map((it) => (it as Record<string, unknown>)?.id as number).filter((id): id is number => typeof id === 'number');
}

/** 等待会计期间初始化成功（POST /accounting-periods/init） */
export async function ensureAccountingPeriodUI(page: Page): Promise<void> {
  await page.goto(`${BASE_URL}/finance`, { waitUntil: 'domcontentloaded' });
  await page.waitForTimeout(500);
  // 尝试点击"初始化"或"新建期间"按钮；若页面有对应按钮则点击
  const initBtn = page.getByRole('button', { name: /初始化|新建期间|新建会计期间/ }).first();
  if (await initBtn.count() > 0) {
    await initBtn.click();
    await waitCreateResponse(page, `${API_PREFIX}/accounting-periods/init`, 15000);
  }
  // 若没有按钮，直接调 API 兜底（保留原逻辑的健壮性）
}
