/**
 * UI-only 辅助函数：通过 Playwright 浏览器操作完成数据创建/查询
 * 所有操作均模拟真实用户在界面上的行为（点击、填写、提交）
 * 对应 ensureTestEntities 中各实体的 UI 创建流程
 *
 * 健壮性策略：
 * - safeGoto：处理 Vite 504 Outdated Optimize Dep，自动重试最多 3 次
 * - 所有 waitFor 超时提升到 30s（CI 16 shard 并发环境慢）
 * - 失败时截图 + DOM 快照 + 详细错误日志，不静默吞掉
 */
import type { Page } from '@playwright/test';
import { BASE_URL, API_PREFIX } from './helpers';

// 简单的唯一 ID 生成（避免循环依赖）
function _genCode(prefix: string): string {
  return `${prefix}-${String(Date.now()).slice(-6)}`;
}
function _genName(prefix: string): string {
  return `${prefix}-${String(Date.now()).slice(-6)}`;
}

// ---------------------------------------------------------------------------
// 公共 UI 操作原语
// ---------------------------------------------------------------------------

/** 通用对话框字段 */
type UiField =
  | { kind: 'input'; label: string; value: string }
  | { kind: 'inputNumber'; label: string; value: number }
  | { kind: 'select'; label: string; value: string }
  | { kind: 'date'; label: string; value: string };

/**
 * 安全导航：处理 Vite 504 + page 被关闭的情况
 * 最多重试 3 次，每次检测 504 后等 5s 重新加载
 */
async function safeGoto(page: Page, path: string): Promise<void> {
  const url = `${BASE_URL}${path}`;
  for (let attempt = 0; attempt < 3; attempt++) {
    const consoleLogs: string[] = [];
    const handler = (msg: { type(): string; text(): string }) =>
      consoleLogs.push(`[console.${msg.type()}] ${msg.text()}`);
    page.on('console', handler);
    try {
      await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30000 });

      // 等 1s 检测是否有 504
      await page.waitForTimeout(1000);
      const has504 = consoleLogs.some(log => log.includes('504'));

      if (has504 && attempt < 2) {
        console.log(`[safeGoto] 检测到 Vite 504，等待 5s 后重新加载 (attempt ${attempt + 1}/3)`);
        await page.waitForTimeout(5000);
        page.off('console', handler);
        continue; // 重试
      }

      // 设置 locale
      await page
        .evaluate(() => window.localStorage.setItem('bingxi.locale', 'zh-CN'))
        .catch(() => {});
      page.off('console', handler);
      return; // 成功
    } catch (e) {
      page.off('console', handler);
      const errMsg = (e as Error).message;
      if (
        attempt < 2 &&
        (errMsg.includes('504') ||
          errMsg.includes('Target') ||
          errMsg.includes('closed') ||
          errMsg.includes('net::ERR'))
      ) {
        console.warn(
          `[safeGoto] ${path} 导航失败 (attempt ${attempt + 1}/3): ${errMsg}，5s 后重试`
        );
        await page.waitForTimeout(5000);
        continue;
      }
      // 最后一次尝试也失败了，记录详细错误
      console.error(`[safeGoto] ${path} 导航最终失败: ${errMsg}`);
      throw e;
    }
  }
}

/**
 * 截图 + DOM 快照 + 错误日志（失败诊断用）
 */
async function diagnoseFailure(page: Page, label: string): Promise<void> {
  try {
    const screenshotPath = `test-results/ui-create-fail-${label}-${Date.now()}.png`;
    await page.screenshot({ path: screenshotPath, fullPage: true });
    console.error(`[UI诊断] ${label} 失败截图已保存: ${screenshotPath}`);
    const url = page.url();
    const bodyText = await page
      .locator('body')
      .innerText()
      .catch(() => '<无法获取>');
    const elMessages = await page
      .locator('.el-message__content')
      .allTextContents()
      .catch(() => []);
    const formErrors = await page
      .locator('.el-form-item__error')
      .allTextContents()
      .catch(() => []);
    console.error(`[UI诊断] ${label} 失败详情:`);
    console.error(`  URL: ${url}`);
    console.error(`  ElMessage: ${JSON.stringify(elMessages)}`);
    console.error(`  表单错误: ${JSON.stringify(formErrors)}`);
    console.error(`  页面文本(前500字): ${bodyText.slice(0, 500)}`);
  } catch {
    // 截图本身可能也会失败
  }
}

async function fillField(
  dialog: import('@playwright/test').Locator,
  page: Page,
  field: UiField
): Promise<void> {
  const labelRegex = new RegExp(field.label);
  const formItem = dialog
    .locator('.el-form-item')
    .filter({ has: dialog.locator('.el-form-item__label').filter({ hasText: labelRegex }) })
    .first();
  if ((await formItem.count()) === 0) {
    // 兜底：用 label 文本直接找
    const altFormItem = dialog.locator('.el-form-item').filter({ hasText: labelRegex }).first();
    if ((await altFormItem.count()) === 0) {
      console.warn(`[uiCreate] 找不到字段 "${field.label}"`);
      return;
    }
    await fillInField(altFormItem, page, field);
    return;
  }
  await fillInField(formItem, page, field);
}

async function fillInField(
  formItem: import('@playwright/test').Locator,
  page: Page,
  field: UiField
): Promise<void> {
  switch (field.kind) {
    case 'input': {
      const inp = formItem.locator('input:not([type="number"])').first();
      if ((await inp.count()) === 0) {
        // 兜底：取任意 input
        const inp2 = formItem.locator('input').first();
        await inp2.waitFor({ state: 'visible', timeout: 20000 });
        await inp2.click({ clickCount: 3 });
        await inp2.fill(field.value);
        return;
      }
      await inp.waitFor({ state: 'visible', timeout: 20000 });
      await inp.click({ clickCount: 3 });
      await inp.fill(field.value);
      break;
    }
    case 'inputNumber': {
      const inp = formItem.locator('input[type="number"]').first();
      if ((await inp.count()) === 0) {
        const inp2 = formItem.locator('.el-input__inner input, input').first();
        await inp2.waitFor({ state: 'visible', timeout: 20000 });
        await inp2.click({ clickCount: 3 });
        await inp2.fill(String(field.value));
      } else {
        await inp.waitFor({ state: 'visible', timeout: 20000 });
        await inp.click({ clickCount: 3 });
        await inp.fill(String(field.value));
      }
      break;
    }
    case 'date': {
      const inp = formItem.locator('input').first();
      await inp.waitFor({ state: 'visible', timeout: 20000 });
      await inp.click({ clickCount: 3 });
      await inp.fill(field.value);
      await inp.press('Escape');
      await page.waitForTimeout(200);
      break;
    }
    case 'select': {
      const wrapper = formItem.locator('.el-select__wrapper').first();
      if ((await wrapper.count()) === 0) {
        const inp = formItem.locator('.el-input__inner').first();
        if ((await inp.count()) > 0) {
          await inp.click();
          await page.waitForTimeout(300);
        }
        return;
      }
      await wrapper.click({ timeout: 10_000 }).catch(() => {});
      await page.waitForTimeout(300);
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(() => {});
      if ((await dropdown.count()) === 0) break;
      const item = dropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: new RegExp(field.value, 'i') })
        .first();
      if ((await item.count()) > 0) {
        await item.click({ timeout: 10_000 }).catch(() => {});
      } else {
        // 无匹配项时选第一项（避免空点击报错拖到 120s 测试超时）
        const firstItem = dropdown.locator('.el-select-dropdown__item').first();
        if ((await firstItem.count()) > 0) {
          await firstItem.click({ timeout: 10_000 }).catch(() => {});
        }
      }
      break;
    }
  }
}

async function waitCreateResponse(
  page: Page,
  apiPath: string,
  timeout = 30000
): Promise<Record<string, unknown> | null> {
  try {
    const resp = await page.waitForResponse(
      r => r.url().includes(apiPath) && r.request().method() === 'POST',
      { timeout }
    );
    const json = await resp.json().catch(() => ({}));
    return (json?.data as Record<string, unknown>) ?? json;
  } catch (e) {
    console.warn(`[waitCreateResponse] 等待 POST ${apiPath} 超时: ${(e as Error).message}`);
    return null;
  }
}

async function waitListResponse(page: Page, apiPath: string, timeout = 20000): Promise<unknown[]> {
  try {
    const resp = await page.waitForResponse(
      r => r.url().includes(apiPath) && r.request().method() === 'GET',
      { timeout }
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
  fields: UiField[]
): Promise<number | undefined> {
  const entityLabel = route.replace(/^\//, '');
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(500);

    // 找新增按钮
    const addBtn = page.getByRole('button', { name: addButtonText, exact: false }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 30000 });
    await addBtn.click();

    // 等对话框出现
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 30000 });
    await page.waitForTimeout(300);

    // 填表
    for (const f of fields) {
      await fillField(dialog, page, f).catch(e => {
        console.warn(`[uiCreateDialog] 填表失败 "${f.label}": ${(e as Error).message}`);
      });
    }
    await page.waitForTimeout(300);

    // 提交
    const submitBtn = dialog.getByRole('button', { name: submitButtonText }).last();
    await submitBtn.waitFor({ state: 'visible', timeout: 20000 });
    await submitBtn.click();

    // 等待响应
    const data = await waitCreateResponse(page, createApiPath, 45000);
    if (data?.id !== undefined && typeof data.id === 'number') {
      return data.id;
    }
    // 创建失败：记录详细诊断
    await diagnoseFailure(page, entityLabel);
    console.error(`[uiCreateDialog] ${entityLabel} 创建失败: 响应数据=${JSON.stringify(data)}`);
    return undefined;
  } catch (e) {
    await diagnoseFailure(page, entityLabel);
    console.error(`[uiCreateDialog] ${entityLabel} 创建异常: ${(e as Error).message}`);
    return undefined;
  }
}

// ---------------------------------------------------------------------------
// 每个实体的专用 UI 创建函数
// ---------------------------------------------------------------------------

/** 创建仓库 */
export async function createWarehouseUI(page: Page): Promise<number | undefined> {
  // 仓库类型 select 选项为 原料仓/成品仓/半成品仓/退货仓（i18n 中文），
  // warehouse_type 必填（trigger:change），填错值会导致 select 选不上 →
  // formRef.validate 失败 → 提交请求不发 → waitCreateResponse 45s 超时
  const fields: UiField[] = [
    { kind: 'input', label: '仓库编码', value: _genCode('E2E-W') },
    { kind: 'input', label: '仓库名称', value: _genName('E2E仓库') },
    { kind: 'select', label: '类型', value: '原料仓' },
  ];
  return uiCreateDialog(
    page,
    '/warehouse',
    `${API_PREFIX}/warehouses`,
    /新建仓库/,
    /保存|确定/,
    fields
  );
}

/** 创建部门 */
export async function createDepartmentUI(page: Page): Promise<number | undefined> {
  // 部门表单 status 必填（trigger:change，select 选项 启用/禁用），
  // 缺该字段会触发 formRef.validate 失败 → 提交请求不发 → waitCreateResponse 超时
  const fields: UiField[] = [
    { kind: 'input', label: '部门名称', value: _genName('E2E部门') },
    { kind: 'input', label: '部门编码', value: _genCode('E2E-D') },
    { kind: 'select', label: '状态', value: '启用' },
  ];
  return uiCreateDialog(
    page,
    '/departments',
    `${API_PREFIX}/departments`,
    /新建部门/,
    /确认|保存/,
    fields
  );
}

/** 创建供应商 */
export async function createSupplierUI(page: Page): Promise<number | undefined> {
  // 后端 CreateSupplierRequest 校验：
  // - supplier_short_name: Option<String> length(min=2)，表单空串触发校验失败
  // - credit_code: Option<String> length(equal=18)，表单空串触发校验失败
  // 故 UI 必须填这两个字段（label 以 i18n 中文翻译为准：供应商简称 / 信用代码）
  const creditCode = `91${String(Date.now()).slice(-8).padStart(8, '0')}MA${String(
    Math.floor(Math.random() * 900000) + 100000
  )}`;
  const fields: UiField[] = [
    { kind: 'input', label: '供应商编码', value: _genCode('E2E-S') },
    { kind: 'input', label: '供应商名称', value: _genName('E2E供应商') },
    { kind: 'input', label: '供应商简称', value: 'E2E供' },
    { kind: 'input', label: '联系电话', value: '13800000001' },
    { kind: 'input', label: '信用代码', value: creditCode },
  ];
  return uiCreateDialog(
    page,
    '/supplier',
    `${API_PREFIX}/purchase/suppliers`,
    /新建供应商/,
    /确定|保存/,
    fields
  );
}

/** 创建产品 */
export async function createProductUI(page: Page): Promise<number | undefined> {
  // 关键：先整页加载 /product（safeGoto 触发 Vue Router 导航 + 组件挂载），
  // 确保分类下拉的 categories prop 在“面料”分类创建之后加载。
  // 若页面此前已挂载（分类列表为旧缓存），reload 强制刷新。
  await safeGoto(page, '/product');
  await page.reload({ waitUntil: 'domcontentloaded' }).catch(() => {});
  // 等待 GET /product-categories 响应完成（分类下拉数据就绪），避免异步竞态
  await page
    .waitForResponse(
      r => r.url().includes('/product-categories') && r.request().method() === 'GET',
      {
        timeout: 15_000,
      }
    )
    .catch(() => {});
  await page.waitForTimeout(500);
  // 打开新建产品 dialog
  const addBtn = page.getByRole('button', { name: /新建产品/ }).first();
  await addBtn.waitFor({ state: 'visible', timeout: 30000 });
  await addBtn.click();
  const dialog = page.locator('.el-dialog:visible').last();
  await dialog.waitFor({ state: 'visible', timeout: 30000 });
  await page.waitForTimeout(300);
  // 填普通输入字段
  const codeInput = dialog
    .locator('.el-form-item')
    .filter({ hasText: '产品编码' })
    .locator('input')
    .first();
  await codeInput.waitFor({ state: 'visible', timeout: 20000 });
  await codeInput.fill(_genCode('E2E-P'));
  const nameInput = dialog
    .locator('.el-form-item')
    .filter({ hasText: '产品名称' })
    .locator('input')
    .first();
  await nameInput.waitFor({ state: 'visible', timeout: 20000 });
  await nameInput.fill(_genName('E2E产品'));
  const unitInput = dialog
    .locator('.el-form-item')
    .filter({ hasText: '单位' })
    .locator('input')
    .first();
  if ((await unitInput.count()) > 0) {
    await unitInput.fill('米').catch(() => {});
  }
  // 分类下拉：用 placeholder 定位 select，点击后从全局 dropdown 选“面料”
  const categorySelect = dialog
    .locator('.el-form-item:has(.el-select)')
    .filter({ hasText: '分类' })
    .locator('.el-select__wrapper, .el-select')
    .first();
  await categorySelect.waitFor({ state: 'visible', timeout: 20000 });
  await categorySelect.click();
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await dropdown.waitFor({ state: 'visible', timeout: 20000 });
  // 等待 option 项真正渲染（排除 loading/空 dropdown 暂态）
  await dropdown
    .locator('.el-select-dropdown__item')
    .first()
    .waitFor({ state: 'visible', timeout: 10_000 })
    .catch(() => {});
  const fabricItem = dropdown
    .locator('.el-select-dropdown__item')
    .filter({ hasText: /面料/i })
    .first();
  if ((await fabricItem.count()) > 0) {
    await fabricItem.click();
  } else {
    // 无“面料”选项 → 取第一项（避免空值提交），并告警
    console.warn('[createProductUI] 分类下拉无“面料”选项，回退选第一项');
    await dropdown.locator('.el-select-dropdown__item').first().click();
  }
  // 验证分类已选中（select 显示非 placeholder 文本），未选中则重试一次
  await page.waitForTimeout(500);
  const selectText = (await categorySelect.textContent()) || '';
  if (selectText.includes('选择分类') || selectText.trim() === '') {
    console.warn('[createProductUI] 分类未选中，重试一次');
    await categorySelect.click();
    const dropdown2 = page.locator('.el-select-dropdown:visible').last();
    await dropdown2.waitFor({ state: 'visible', timeout: 20000 });
    const firstItem = dropdown2.locator('.el-select-dropdown__item').first();
    await firstItem.waitFor({ state: 'visible', timeout: 10_000 });
    await firstItem.click();
    await page.waitForTimeout(500);
  }
  // 重试后再次确认分类已选中：未选中则提前返回，避免提交触发必填校验失败后
  // waitCreateResponse 空等 45s（请求不会发出），让 API 兜底接管
  const selectText2 = (await categorySelect.textContent()) || '';
  if (selectText2.includes('选择分类') || selectText2.trim() === '') {
    console.warn('[createProductUI] 分类仍未选中，跳过 UI 提交（走 API 兜底）');
    await diagnoseFailure(page, 'product');
    return undefined;
  }
  // 提交
  const submitBtn = dialog.getByRole('button', { name: /确定|保存/ }).last();
  await submitBtn.waitFor({ state: 'visible', timeout: 20000 });
  await submitBtn.click();
  // 等待响应
  const data = await waitCreateResponse(page, `${API_PREFIX}/products`, 45000);
  if (data?.id !== undefined && typeof data.id === 'number') {
    return data.id;
  }
  await diagnoseFailure(page, 'product');
  console.error(`[createProductUI] 创建失败: 响应数据=${JSON.stringify(data)}`);
  return undefined;
}

/** 创建色卡 */
export async function createColorCardUI(page: Page): Promise<number | undefined> {
  try {
    await safeGoto(page, '/color-cards/create');
    await page.waitForTimeout(800);
    const cardNoInput = page
      .locator('input[placeholder*="卡号" i], .el-form-item:has(span:text-is("卡号")) input')
      .first();
    const cardNameInput = page.locator('.el-form-item:has(span:text-is("卡名")) input').first();
    const typeSelect = page
      .locator('.el-form-item:has(span:text-is("色卡类型")) .el-select__wrapper')
      .first();
    await cardNoInput.waitFor({ state: 'visible', timeout: 30000 });
    await cardNoInput.click({ clickCount: 3 });
    await cardNoInput.fill(_genCode('E2E-CC'));
    await cardNameInput.waitFor({ state: 'visible', timeout: 20000 });
    await cardNameInput.click({ clickCount: 3 });
    await cardNameInput.fill(_genName('E2E色卡'));
    if ((await typeSelect.count()) > 0) {
      await typeSelect.click();
      await page.waitForTimeout(300);
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 20000 });
      const item = dropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: /自定义|CUSTOM/i })
        .first();
      if ((await item.count()) > 0) await item.click();
      else await dropdown.locator('.el-select-dropdown__item').first().click();
    }
    const submitBtn = page.getByRole('button', { name: /立即创建|创建|确定|保存/ }).first();
    await submitBtn.waitFor({ state: 'visible', timeout: 20000 });
    await submitBtn.click();
    const data = await waitCreateResponse(page, `${API_PREFIX}/color-cards`, 45000);
    if (data?.id !== undefined && typeof data.id === 'number') return data.id;
    await diagnoseFailure(page, 'color-card');
    console.error(`[createColorCardUI] 创建失败: 响应=${JSON.stringify(data)}`);
    return undefined;
  } catch (e) {
    await diagnoseFailure(page, 'color-card');
    console.error(`[createColorCardUI] 异常: ${(e as Error).message}`);
    return undefined;
  }
}

/** 创建染色批次 */
export async function createDyeBatchUI(page: Page): Promise<number | undefined> {
  // uiCreateDialog 内部已有 safeGoto('/dye-batch')，页面挂载会触发 getProductList（GET /products），
  // 此处不再重复导航（避免与 uiCreateDialog 内 safeGoto 叠加导致耗时接近 120s 测试超时）。
  // 产品下拉数据在 addBtn/dialog waitFor 期间异步加载，fillField select 时已就绪。
  const fields: UiField[] = [
    { kind: 'input', label: '批次号', value: _genCode('E2E-DB') },
    { kind: 'select', label: '产品', value: 'E2E' },
    { kind: 'input', label: '色号', value: _genCode('E2E-CN') },
    { kind: 'date', label: '染色日期', value: new Date().toISOString().slice(0, 10) },
    { kind: 'inputNumber', label: '数量', value: 100 },
  ];
  return uiCreateDialog(
    page,
    '/dye-batch',
    `${API_PREFIX}/production/dye-batches`,
    /新建批次/,
    /确定|保存/,
    fields
  );
}

/** 创建染色配方 */
export async function createDyeRecipeUI(page: Page): Promise<number | undefined> {
  const fields: UiField[] = [
    { kind: 'input', label: '配方编号', value: _genCode('E2E-DR') },
    { kind: 'input', label: '配方名称', value: _genName('E2E配方') },
    { kind: 'input', label: '色号', value: _genCode('E2E-CN') },
    { kind: 'input', label: '颜色名称', value: '测试色' },
    { kind: 'input', label: '配方内容', value: 'E2E测试内容' },
  ];
  return uiCreateDialog(
    page,
    '/dye-recipe',
    `${API_PREFIX}/production/dye-recipes`,
    /新建配方/,
    /确认|确定|保存/,
    fields
  );
}

/** 创建 BOM */
export async function createBomUI(page: Page): Promise<number | undefined> {
  const dialogResult = await uiCreateDialog(
    page,
    '/bom',
    `${API_PREFIX}/boms`,
    /新建|新建 BOM/,
    /保存|确定/,
    [
      { kind: 'input', label: '产品名称', value: _genName('E2E BOM') },
      { kind: 'input', label: '版本', value: '1' },
      { kind: 'select', label: '状态', value: '启用' },
    ]
  );
  if (dialogResult !== undefined) return dialogResult;
  // 可能需要添加物料明细
  const dialog = page.locator('.el-dialog:visible').last();
  if ((await dialog.count()) === 0) return undefined;
  const hasItemsSection = (await dialog.locator('.items-section, [class*="items"]').count()) > 0;
  if (hasItemsSection) {
    const addItemBtn = dialog.getByRole('button', { name: /添加物料|添加/, exact: false }).first();
    if ((await addItemBtn.count()) > 0) {
      await addItemBtn.click();
      await page.waitForTimeout(300);
      const firstRow = dialog.locator('.el-table tbody tr').first();
      if ((await firstRow.count()) > 0) {
        const matNameInput = firstRow.locator('input').first();
        await matNameInput.waitFor({ state: 'visible', timeout: 20000 });
        await matNameInput.click({ clickCount: 3 });
        await matNameInput.fill('E2E 原料');
        const unitInput = firstRow.locator('input').nth(2);
        if ((await unitInput.count()) > 0) {
          await unitInput.click({ clickCount: 3 });
          await unitInput.fill('米');
        }
      }
    }
    const submitBtn = dialog.getByRole('button', { name: /保存|确定/ }).last();
    if ((await submitBtn.count()) > 0) {
      await submitBtn.click();
      const data = await waitCreateResponse(page, `${API_PREFIX}/boms`, 45000);
      return typeof data?.id === 'number' ? data.id : undefined;
    }
  }
  return undefined;
}

/** 创建定制订单 */
export async function createCustomOrderUI(page: Page): Promise<number | undefined> {
  try {
    await safeGoto(page, '/custom-orders/new');
    await page.waitForTimeout(800);
    // customer_id / product_id 是 el-input-number（手填数字 ID）
    const inputs = page.locator('input[type="number"]');
    const inputCount = await inputs.count();
    if (inputCount >= 2) {
      await inputs.nth(0).waitFor({ state: 'visible', timeout: 30000 });
      await inputs.nth(0).click({ clickCount: 3 });
      await inputs.nth(0).fill('1');
      await inputs.nth(1).click({ clickCount: 3 });
      await inputs.nth(1).fill('1');
    } else {
      // 兜底：用 label 定位
      const customerIdInput = page
        .locator('.el-form-item:has(span:text-is("客户ID")) input')
        .first();
      const productIdInput = page
        .locator('.el-form-item:has(span:text-is("产品ID")) input')
        .first();
      await customerIdInput.waitFor({ state: 'visible', timeout: 30000 });
      await customerIdInput.click({ clickCount: 3 });
      await customerIdInput.fill('1');
      await productIdInput.click({ clickCount: 3 });
      await productIdInput.fill('1');
    }
    const specInput = page
      .locator('.el-form-item:has(span:text-is("规格")) input, input[placeholder*="规格"]')
      .first();
    await specInput.waitFor({ state: 'visible', timeout: 20000 });
    await specInput.click({ clickCount: 3 });
    await specInput.fill('E2E 定制规格');
    const quantityInput = page
      .locator('.el-form-item:has(span:text-is("数量")) input, input[placeholder*="数量"]')
      .last();
    await quantityInput.waitFor({ state: 'visible', timeout: 20000 });
    await quantityInput.click({ clickCount: 3 });
    await quantityInput.fill('100');
    const submitBtn = page.getByRole('button', { name: /保存草稿|保存|确定/ }).first();
    await submitBtn.waitFor({ state: 'visible', timeout: 20000 });
    await submitBtn.click();
    const data = await waitCreateResponse(page, `${API_PREFIX}/custom-orders`, 45000);
    if (data?.id !== undefined && typeof data.id === 'number') return data.id;
    await diagnoseFailure(page, 'custom-order');
    console.error(`[createCustomOrderUI] 创建失败: 响应=${JSON.stringify(data)}`);
    return undefined;
  } catch (e) {
    await diagnoseFailure(page, 'custom-order');
    console.error(`[createCustomOrderUI] 异常: ${(e as Error).message}`);
    return undefined;
  }
}

/**
 * 通过 UI 读取列表第一行实体的 id
 */
export async function readFirstEntityId(
  page: Page,
  route: string,
  listApiPath: string
): Promise<number | undefined> {
  try {
    await safeGoto(page, route);
    const items = await waitListResponse(page, listApiPath, 20000);
    return firstId(items);
  } catch (e) {
    console.warn(`[readFirstEntityId] ${route} 查找失败: ${(e as Error).message}`);
    return undefined;
  }
}

/** 通用 UI 列表查找：返回多条 id */
export async function readEntityIds(
  page: Page,
  route: string,
  listApiPath: string,
  limit = 10
): Promise<number[]> {
  try {
    await safeGoto(page, route);
    const items = await waitListResponse(page, listApiPath, 20000);
    return items
      .slice(0, limit)
      .map(it => (it as Record<string, unknown>)?.id as number)
      .filter((id): id is number => typeof id === 'number');
  } catch (e) {
    console.warn(`[readEntityIds] ${route} 查找失败: ${(e as Error).message}`);
    return [];
  }
}

/** 等待会计期间初始化 */
export async function ensureAccountingPeriodUI(page: Page): Promise<void> {
  await safeGoto(page, '/finance');
  await page.waitForTimeout(500);
  const initBtn = page.getByRole('button', { name: /初始化|新建期间|新建会计期间/ }).first();
  if ((await initBtn.count()) > 0) {
    await initBtn.click();
    await waitCreateResponse(page, `${API_PREFIX}/accounting-periods/init`, 15000);
  }
}
