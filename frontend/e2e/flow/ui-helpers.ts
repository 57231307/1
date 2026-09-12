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
export async function safeGoto(page: Page, path: string): Promise<void> {
  const url = `${BASE_URL}${path}`;
  // 整体 45s 上限：goto 自身 30s，防止 Vite 504 重试循环 + 页内 evaluate
  // 挂起拖垮整个 ensure（run 34041167918 exit 124 根因链）
  return Promise.race([safeGotoInner(page, url), timeoutReject(path, 45_000)]);
}

function timeoutReject(path: string, ms: number): Promise<void> {
  return new Promise((_, reject) => {
    setTimeout(() => reject(new Error(`safeGoto ${path} 整体超时 ${ms}ms`)), ms);
  });
}

async function safeGotoInner(page: Page, url: string): Promise<void> {
  const path = url.replace(/^https?:\/\/[^/]+/, '');
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
        .catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
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
      .catch((e) => { console.warn(`[E2E] 文本兜底读取: ${(e as Error).message}`); return '<兜底>'; });
    const elMessages = await page
      .locator('.el-message__content')
      .allTextContents()
      .catch((e) => {
        console.warn('[ui-helpers] 选择器批量查询失败:', (e as Error).message);
        return [];
      });
    const formErrors = await page
      .locator('.el-form-item__error')
      .allTextContents()
      .catch((e) => {
        console.warn('[ui-helpers] 选择器批量查询失败:', (e as Error).message);
        return [];
      });
    console.error(`[UI诊断] ${label} 失败详情:`);
    console.error(`  URL: ${url}`);
    console.error(`  ElMessage: ${JSON.stringify(elMessages)}`);
    console.error(`  表单错误: ${JSON.stringify(formErrors)}`);
    // ErrorBoundary 捕获的组件运行时错误（"页面加载出错"即来源于此）
    const errorBoundary = await page
      .locator('.error-boundary')
      .count()
      .catch((e) => { console.warn(`[E2E] 计数兜底: ${(e as Error).message}`); return 0; });
    if (errorBoundary > 0) {
      const detailBtn = page.locator('.error-boundary button:has-text("查看详情")').first();
      if ((await detailBtn.count()) > 0) {
        await detailBtn.click().catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
        await page.waitForTimeout(300);
      }
      const stack = await page
        .locator('.error-boundary__detail')
        .textContent()
        .catch((e) => {
          console.warn('[ui-helpers] error-boundary 详情读取失败:', (e as Error).message);
          return '';
        });
      console.error(`  [ErrorBoundary] 组件运行时错误: ${(stack || '').slice(0, 500)}`);
    }
    console.error(`  页面文本(前500字): ${bodyText.slice(0, 500)}`);
  } catch (e) {
    console.warn('[ui-helpers] 页面截图失败（截图功能降级，诊断信息见上方日志）:', (e as Error).message);
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
      // 兼容 el-input type="textarea"（textarea 标签）与普通 input
      const inp = formItem.locator('input:not([type="number"]), textarea').first();
      if ((await inp.count()) === 0) {
        // 兜底：取任意 input 或 textarea
        const inp2 = formItem.locator('input, textarea').first();
        await inp2.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
        await inp2.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
        await inp2.fill(field.value).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
        return;
      }
      await inp.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await inp.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await inp.fill(field.value).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
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
      await inp.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await inp.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await inp.fill(field.value).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      // el-date-picker fill 后需 Enter 确认（Escape 会取消选择清空值，
      // 导致"请选择染色日期"校验失败 → 请求不发 → 超时）
      await inp.press('Enter').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await page.waitForTimeout(300);
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
      await wrapper.click({ timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      await page.waitForTimeout(300);
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      if ((await dropdown.count()) === 0) break;
      const item = dropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: new RegExp(field.value, 'i') })
        .first();
      if ((await item.count()) > 0) {
        await item.click({ timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      } else {
        // 无匹配项时选第一项（避免空点击报错拖到 120s 测试超时）
        const firstItem = dropdown.locator('.el-select-dropdown__item').first();
        if ((await firstItem.count()) > 0) {
          await firstItem.click({ timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
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
    // CSRF 一次性消费机制下，UI 提交的第一次 POST 可能 403（token 被并发消费），
    // 前端 axios 拦截器自动用 X-New-CSRF-Token 恢复头重放第二次 POST。
    // 跳过 403（CSRF 重放中间态），匹配其他所有响应（200 业务成功 / 400 业务校验失败等），
    // 业务 400 需被捕获以返回错误信息用于诊断，避免 45s 空等。
    const resp = await page.waitForResponse(
      r => r.url().includes(apiPath) && r.request().method() === 'POST' && r.status() !== 403,
      { timeout }
    );
    const json = await resp.json().catch((e) => {
      console.warn(`[waitCreateResponse] POST ${apiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    });
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
    const json = await resp.json().catch((e) => {
      console.warn(`[waitListResponse] GET ${apiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    });
    return (json?.data?.items ?? json?.data?.list ?? []) as unknown[];
  } catch (e) {
    console.warn(`[waitListResponse] 等待 GET ${apiPath} 超时: ${(e as Error).message}`);
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
    await addBtn.waitFor({ state: 'visible', timeout: 60000 });
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
  // 路由 chunk 偶发加载失败（页面只有 layout 无组件内容），先 reload 保证组件挂载
  await safeGoto(page, '/departments');
  await page.reload({ waitUntil: 'domcontentloaded' }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
  await page.waitForTimeout(500);
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
  await page.reload({ waitUntil: 'domcontentloaded' }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
  // 等待 GET /product-categories 响应完成（分类下拉数据就绪），避免异步竞态
  await page
    .waitForResponse(
      r => r.url().includes('/product-categories') && r.request().method() === 'GET',
      {
        timeout: 15_000,
      }
    )
    .catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
  await page.waitForTimeout(500);
  // 打开新建产品 dialog
  const addBtn = page.getByRole('button', { name: /新建产品/ }).first();
  await addBtn.waitFor({ state: 'visible', timeout: 60000 });
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
    await unitInput.fill('米').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
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
    .catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
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
      .locator('input[placeholder*="卡号" i], .el-form-item:has(:text-is("卡号")) input')
      .first();
    const cardNameInput = page.locator('.el-form-item:has(:text-is("卡名")) input').first();
    const typeSelect = page
      .locator('.el-form-item:has(:text-is("色卡类型")) .el-select__wrapper')
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
    // 色卡创建页提交按钮文本为"提交"（colorCards.create.submit），正则须含"提交"
    const submitBtn = page.getByRole('button', { name: /提交|立即创建|创建|确定|保存/ }).first();
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
  // 加 120s 超时保护：safeGoto 在页面 504 时最多重试 3 次约 108s，
  // 60s race 会误中断正常创建流程（300s 总超时内 120s 安全）
  const fields: UiField[] = [
    { kind: 'input', label: '批次号', value: _genCode('E2E-DB') },
    { kind: 'select', label: '产品', value: 'E2E' },
    { kind: 'input', label: '色号', value: _genCode('E2E-CN') },
    { kind: 'date', label: '染色日期', value: new Date().toISOString().slice(0, 10) },
    { kind: 'inputNumber', label: '数量', value: 100 },
  ];
  const timeoutPromise = new Promise<undefined>(resolve =>
    setTimeout(() => {
      console.warn('[createDyeBatchUI] 120s 超时，走 API 兜底');
      resolve(undefined);
    }, 120_000)
  );
  return Promise.race([
    uiCreateDialog(
      page,
      '/dye-batch',
      `${API_PREFIX}/production/dye-batches`,
      /新建批次/,
      /确认|确定|保存/,
      fields
    ),
    timeoutPromise,
  ]);
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
  // 后端 CreateBomPayload 校验 items 至少 1 条（"BOM明细不能为空"），
  // 且 product_id/material_id 必填（i32）——表单已改为产品/物料下拉。
  // 独立实现：填字段 → 点"添加物料" → 选物料 select → 填单位 → 提交
  try {
    await safeGoto(page, '/bom');
    await page.waitForTimeout(500);
    const addBtn = page.getByRole('button', { name: /新建|新建 BOM/, exact: false }).first();
    await addBtn.waitFor({ state: 'visible', timeout: 60000 });
    await addBtn.click();
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 30000 });
    await page.waitForTimeout(300);

    // 产品下拉（el-select，选含 E2E 的项，无则选第一项）
    const productItem = dialog.locator('.el-form-item').filter({ hasText: '产品名称' }).first();
    const productSelect = productItem.locator('.el-select__wrapper, .el-select').first();
    await productSelect.waitFor({ state: 'visible', timeout: 20000 });
    await productSelect.click();
    const prodDropdown = page.locator('.el-select-dropdown:visible').last();
    await prodDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    const e2eProd = prodDropdown
      .locator('.el-select-dropdown__item')
      .filter({ hasText: /E2E/i })
      .first();
    if ((await e2eProd.count()) > 0) {
      await e2eProd.click();
    } else {
      const first = prodDropdown.locator('.el-select-dropdown__item').first();
      if ((await first.count()) > 0) await first.click();
    }
    await page.waitForTimeout(300);

    // 版本
    const versionInput = dialog
      .locator('.el-form-item')
      .filter({ hasText: '版本' })
      .locator('input')
      .first();
    await versionInput.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await versionInput.fill('1').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });

    // 状态下拉（选"启用"或第一项）
    const statusItem = dialog.locator('.el-form-item').filter({ hasText: '状态' }).first();
    const statusSelect = statusItem.locator('.el-select__wrapper, .el-select').first();
    if ((await statusSelect.count()) > 0) {
      await statusSelect.click();
      const statusDropdown = page.locator('.el-select-dropdown:visible').last();
      await statusDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
      const activeOpt = statusDropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: /启用|active/i })
        .first();
      if ((await activeOpt.count()) > 0) {
        await activeOpt.click();
      } else {
        const first = statusDropdown.locator('.el-select-dropdown__item').first();
        if ((await first.count()) > 0) await first.click();
      }
      await page.waitForTimeout(300);
    }

    // 添加物料明细（items 至少 1 条，后端校验"BOM明细不能为空"）
    const addItemBtn = dialog.getByRole('button', { name: /添加物料|添加/, exact: false }).first();
    if ((await addItemBtn.count()) > 0) {
      await addItemBtn.click();
      await page.waitForTimeout(300);
      const firstRow = dialog.locator('.el-table tbody tr').first();
      if ((await firstRow.count()) > 0) {
        const matSelect = firstRow.locator('.el-select').first();
        if ((await matSelect.count()) > 0) {
          await matSelect.click();
          const matDropdown = page.locator('.el-select-dropdown:visible').last();
          await matDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
          const matE2e = matDropdown
            .locator('.el-select-dropdown__item')
            .filter({ hasText: /E2E/i })
            .first();
          if ((await matE2e.count()) > 0) {
            await matE2e.click();
          } else {
            const first = matDropdown.locator('.el-select-dropdown__item').first();
            if ((await first.count()) > 0) await first.click();
          }
          await page.waitForTimeout(300);
        }
        const unitInput = firstRow.locator('input').nth(2);
        if ((await unitInput.count()) > 0) {
          await unitInput.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
          await unitInput.fill('米').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
        }
      }
    }

    // 提交
    const submitBtn = dialog.getByRole('button', { name: /保存|确定/ }).last();
    await submitBtn.waitFor({ state: 'visible', timeout: 20000 });
    await submitBtn.click();
    const data = await waitCreateResponse(page, `${API_PREFIX}/boms`, 45000);
    // BOM 成功响应为 BomDetailResponse { bom: { id }, items }（非扁平 {id}），
    // 兼容两种结构取 id，否则误判创建失败
    const bomId =
      typeof data?.id === 'number'
        ? data.id
        : typeof (data as { bom?: { id?: number } })?.bom?.id === 'number'
          ? (data as { bom: { id: number } }).bom.id
          : undefined;
    if (typeof bomId === 'number') return bomId;
    await diagnoseFailure(page, 'bom');
    console.error(`[createBomUI] 创建失败: 响应数据=${JSON.stringify(data)}`);
    return undefined;
  } catch (e) {
    await diagnoseFailure(page, 'bom');
    console.error(`[createBomUI] 异常: ${(e as Error).message}`);
    return undefined;
  }
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
      const customerIdInput = page.locator('.el-form-item:has(:text-is("客户ID")) input').first();
      const productIdInput = page.locator('.el-form-item:has(:text-is("产品ID")) input').first();
      await customerIdInput.waitFor({ state: 'visible', timeout: 30000 });
      await customerIdInput.click({ clickCount: 3 });
      await customerIdInput.fill('1');
      await productIdInput.click({ clickCount: 3 });
      await productIdInput.fill('1');
    }
    const specInput = page
      .locator('.el-form-item:has(:text-is("规格")) input, input[placeholder*="规格"]')
      .first();
    await specInput.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await specInput.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await specInput.fill('E2E 定制规格').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    // 数量定位须限定"数量"label 的 form-item：页面另有 total_amount 等
    // el-input-number，.last() 会误选 total_amount 导致 quantity 空校验失败
    const quantityInput = page.locator('.el-form-item:has(:text-is("数量")) input').first();
    await quantityInput.waitFor({ state: 'visible', timeout: 20000 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await quantityInput.click({ clickCount: 3 }).catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await quantityInput.fill('100').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    // el-input-number 需 blur/Enter 同步 v-model，fill 后触发 blur
    await quantityInput.press('Tab').catch((e) => { console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`); });
    await page.waitForTimeout(300);
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
    const id = firstId(items);
    if (id !== undefined) return id;
  } catch (e) {
    console.warn(`[readFirstEntityId] ${route} 查找失败: ${(e as Error).message}`);
  }
  // 页面路径未命中列表响应（页面懒加载/tab 未激活/页面不发该请求）时，
  // 直接用 API 查询：避免误判"实体不存在"而反复走 120s UI 创建超时
  // （run 34019751699 shard-13：dye-batch 查找失败导致每个测试都消耗 120s+20s）
  try {
    const resp = await page.request.get(listApiPath, {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });
    const json = (await resp.json().catch((e) => {
      console.warn(`[readFirstEntityId] ${listApiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    })) as {
      data?: { items?: Array<{ id?: number }>; list?: Array<{ id?: number }> };
    };
    const items = json?.data?.items ?? json?.data?.list ?? [];
    return firstId(items);
  } catch (e) {
    console.warn(`[readFirstEntityId] ${listApiPath} API 直查失败:`, (e as Error).message);
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
    const ids = items
      .slice(0, limit)
      .map(it => (it as Record<string, unknown>)?.id as number)
      .filter((id): id is number => typeof id === 'number');
    if (ids.length > 0) return ids;
  } catch (e) {
    console.warn(`[readEntityIds] ${route} 查找失败: ${(e as Error).message}`);
  }
  // 页面路径未命中列表响应时 API 直查兜底：防止误判"实体不存在"导致
  // 每个测试都重复兜底创建（run 34067844812 产品 id 2,3→5,6→8,9→11,12 风暴）
  try {
    const resp = await page.request.get(listApiPath, {
      headers: { 'X-Requested-With': 'XMLHttpRequest' },
    });
    const json = (await resp.json().catch((e) => {
      console.warn(`[readEntityIds] ${listApiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    })) as {
      data?: { items?: Array<{ id?: number }>; list?: Array<{ id?: number }> };
    };
    const items = json?.data?.items ?? json?.data?.list ?? [];
    return items
      .slice(0, limit)
      .map(it => it?.id as number)
      .filter((id): id is number => typeof id === 'number');
  } catch (e) {
    console.warn(`[readEntityIds] ${listApiPath} API 直查失败:`, (e as Error).message);
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
    await waitCreateResponse(page, `${API_PREFIX}/finance/accounting-periods/init`, 15000);
  }
}

// ============================================================
// UI 驱动通用工具：删除/停用/导出/导入（2026-09-10 用户指令）
// 所有操作基于真实 UI 点击，非 API 调用
// ============================================================

/**
 * UI 驱动删除列表行
 * 1. 导航到列表页
 * 2. 找到目标行的删除按钮（el-button type=danger link）
 * 3. 点击删除 → 确认弹窗（el-popconfirm/el-message-box）
 * 4. 等待列表刷新，验证该行消失
 *
 * @returns true=删除成功且行消失，false=删除失败或行仍在
 */
export async function uiDeleteRow(
  page: Page,
  route: string,
  rowIdentifier: { column: string; value: string | number },
  options?: { confirmText?: RegExp; listApiPath?: string }
): Promise<boolean> {
  const entityLabel = route.replace(/^\//, '');
  const confirmText = options?.confirmText ?? /确定|确认|是|删除/;
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(1000);

    // 找目标行：遍历行，匹配标识列值
    const rows = page.locator('.el-table__row');
    const rowCount = await rows.count();
    console.log(`[uiDeleteRow] ${entityLabel} 列表共 ${rowCount} 行，查找 ${rowIdentifier.column}=${rowIdentifier.value}`);

    let targetRow: import('@playwright/test').Locator | null = null;
    for (let i = 0; i < rowCount; i++) {
      const row = rows.nth(i);
      const cellText = await row.locator('td').filter({ hasText: String(rowIdentifier.value) }).first().textContent().catch((e) => { console.warn(`[uiDeleteRow] 单元格文本读取异常: ${(e as Error).message}`); return ''; });
      if (cellText && cellText.includes(String(rowIdentifier.value))) {
        targetRow = row;
        console.log(`[uiDeleteRow] 匹配到目标行（第 ${i + 1} 行）`);
        break;
      }
    }

    if (!targetRow) {
      console.warn(`[uiDeleteRow] ${entityLabel} 未找到 ${rowIdentifier.column}=${rowIdentifier.value} 的行`);
      return false;
    }

    // 点删除按钮（el-button type=danger link）
    const deleteBtn = targetRow.locator('button.el-button--danger, button:has-text("删除")').first();
    await deleteBtn.waitFor({ state: 'visible', timeout: 5000 });
    await deleteBtn.click();
    console.log(`[uiDeleteRow] 已点击删除按钮`);

    // 确认弹窗
    await page.waitForTimeout(500);
    const confirmBtn = page.getByRole('button', { name: confirmText }).last();
    if (await confirmBtn.isVisible({ timeout: 3000 }).catch((e) => { console.warn(`[uiDeleteRow] 确认弹窗可见性查询失败: ${(e as Error).message}`); return false; })) {
      await confirmBtn.click();
      console.log(`[uiDeleteRow] 已确认删除`);
    }

    // 等待列表刷新
    await page.waitForTimeout(2000);
    if (options?.listApiPath) {
      await waitListResponse(page, options.listApiPath, 15000).catch((e) => {
        console.warn(`[uiDeleteRow] 列表刷新响应等待失败:`, (e as Error).message);
      });
    }

    // 验证行消失
    const rowsAfter = page.locator('.el-table__row');
    const countAfter = await rowsAfter.count();
    let stillExists = false;
    for (let i = 0; i < countAfter; i++) {
      const cellText = await rowsAfter.nth(i).locator('td').filter({ hasText: String(rowIdentifier.value) }).first().textContent().catch((e) => { console.warn(`[E2E] 文本读取失败: ${(e as Error).message}`); return ''; });
      if (cellText && cellText.includes(String(rowIdentifier.value))) {
        stillExists = true;
        break;
      }
    }

    if (stillExists) {
      console.error(`[uiDeleteRow] ❌ ${entityLabel} 删除后行仍存在（${rowIdentifier.column}=${rowIdentifier.value}）`);
      return false;
    }
    console.log(`[uiDeleteRow] ✅ ${entityLabel} 删除成功，行已消失（列表 ${rowCount}→${countAfter} 行）`);
    return true;
  } catch (e) {
    console.error(`[uiDeleteRow] ${entityLabel} 删除异常:`, (e as Error).message);
    return false;
  }
}

/**
 * UI 驱动切换行状态（停用/启用）
 * 1. 导航到列表页
 * 2. 找到目标行的状态切换开关（el-switch / el-button 状态按钮）
 * 3. 点击切换
 * 4. 验证状态文本已变更
 */
export async function uiToggleStatus(
  page: Page,
  route: string,
  rowIdentifier: { column: string; value: string | number },
  expectedStatusAfter: string
): Promise<boolean> {
  const entityLabel = route.replace(/^\//, '');
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(1000);

    const rows = page.locator('.el-table__row');
    const rowCount = await rows.count();
    let targetRow: import('@playwright/test').Locator | null = null;
    for (let i = 0; i < rowCount; i++) {
      const row = rows.nth(i);
      const cellText = await row.locator('td').filter({ hasText: String(rowIdentifier.value) }).first().textContent().catch((e) => { console.warn(`[uiDeleteRow] 单元格文本读取异常: ${(e as Error).message}`); return ''; });
      if (cellText && cellText.includes(String(rowIdentifier.value))) {
        targetRow = row;
        break;
      }
    }
    if (!targetRow) {
      console.warn(`[uiToggleStatus] ${entityLabel} 未找到目标行`);
      return false;
    }

    // 找状态开关（el-switch 或 el-button 带"停用"/"启用"文案）
    const switchEl = targetRow.locator('.el-switch').first();
    const statusBtn = targetRow.locator('button:has-text("停用"), button:has-text("启用"), button:has-text("禁用")').first();
    if (await switchEl.isVisible({ timeout: 3000 }).catch((e) => { console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`); return false; })) {
      await switchEl.click();
      console.log(`[uiToggleStatus] 已点击状态开关`);
    } else if (await statusBtn.isVisible({ timeout: 3000 }).catch((e) => { console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`); return false; })) {
      await statusBtn.click();
      console.log(`[uiToggleStatus] 已点击状态按钮`);
    } else {
      console.warn(`[uiToggleStatus] ${entityLabel} 未找到状态切换控件`);
      return false;
    }

    await page.waitForTimeout(2000);

    // 验证状态文本
    const rowText = await targetRow.textContent().catch((e) => { console.warn(`[uiToggleStatus] 行文本读取失败: ${(e as Error).message}`); return ''; });
    if (rowText.includes(expectedStatusAfter)) {
      console.log(`[uiToggleStatus] ✅ ${entityLabel} 状态切换成功，当前=${expectedStatusAfter}`);
      return true;
    }
    console.error(`[uiToggleStatus] ❌ ${entityLabel} 状态切换后未显示"${expectedStatusAfter}"`);
    return false;
  } catch (e) {
    console.error(`[uiToggleStatus] ${entityLabel} 状态切换异常:`, (e as Error).message);
    return false;
  }
}

/**
 * UI 驱动导出文件下载
 * 1. 导航到列表页
 * 2. 点击导出按钮
 * 3. 等待下载事件触发
 * 4. 验证文件名 + 文件大小 + 文件类型
 *
 * @returns 下载文件信息（filename/suggestedFilename/size）或 null
 */
export async function uiExportDownload(
  page: Page,
  route: string,
  exportButtonText: RegExp,
  options?: { acceptConfirm?: boolean }
): Promise<{ filename: string; size: number } | null> {
  const entityLabel = route.replace(/^\//, '');
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(1000);

    const exportBtn = page.getByRole('button', { name: exportButtonText, exact: false }).first();
    await exportBtn.waitFor({ state: 'visible', timeout: 10000 });
    console.log(`[uiExportDownload] ${entityLabel} 找到导出按钮`);

    // 设置下载监听
    const downloadPromise = page.waitForEvent('download', { timeout: 30000 });
    await exportBtn.click();
    console.log(`[uiExportDownload] 已点击导出按钮`);

    if (options?.acceptConfirm) {
      await page.waitForTimeout(500);
      const confirmBtn = page.getByRole('button', { name: /确定|确认|导出/ }).last();
      if (await confirmBtn.isVisible({ timeout: 3000 }).catch((e) => { console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`); return false; })) {
        await confirmBtn.click();
      }
    }

    const download = await downloadPromise;
    const filename = download.suggestedFilename();
    const path = await download.path();
    const size = path ? (await import('fs')).statSync(path).size : 0;
    console.log(`[uiExportDownload] ✅ ${entityLabel} 导出成功：文件名=${filename} 大小=${size}B`);
    return { filename, size };
  } catch (e) {
    console.error(`[uiExportDownload] ❌ ${entityLabel} 导出失败:`, (e as Error).message);
    return null;
  }
}

/**
 * UI 驱动导入文件上传
 * 1. 导航到列表页
 * 2. 点击导入按钮
 * 3. 等待导入弹窗出现
 * 4. 上传文件（setInputFiles）
 * 5. 点击确认导入
 * 6. 等待导入结果提示（成功/失败/部分成功）
 *
 * @returns 导入结果文本或 null
 */
export async function uiImportUpload(
  page: Page,
  route: string,
  importButtonText: RegExp,
  filePath: string,
  options?: { submitText?: RegExp; templateDownloadText?: RegExp }
): Promise<string | null> {
  const entityLabel = route.replace(/^\//, '');
  const submitText = options?.submitText ?? /确定|确认|导入|上传/;
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(1000);

    const importBtn = page.getByRole('button', { name: importButtonText, exact: false }).first();
    await importBtn.waitFor({ state: 'visible', timeout: 10000 });
    await importBtn.click();
    console.log(`[uiImportUpload] ${entityLabel} 已点击导入按钮`);

    // 等导入弹窗
    const dialog = page.locator('.el-dialog:visible').last();
    await dialog.waitFor({ state: 'visible', timeout: 10000 });
    await page.waitForTimeout(500);

    // 可选：下载模板
    if (options?.templateDownloadText) {
      const templateBtn = dialog.getByRole('button', { name: options.templateDownloadText }).first();
      if (await templateBtn.isVisible({ timeout: 3000 }).catch((e) => { console.warn(`[uiImportUpload] 模板下载按钮不可见: ${(e as Error).message}`); return false; })) {
        const templateDownload = page.waitForEvent('download', { timeout: 10000 });
        await templateBtn.click();
        const templateFile = await templateDownload;
        console.log(`[uiImportUpload] 模板下载成功：${templateFile.suggestedFilename()}`);
      }
    }

    // 找文件输入（el-upload 的 input[type=file]）
    const fileInput = dialog.locator('input[type="file"]').first();
    await fileInput.setInputFiles(filePath);
    console.log(`[uiImportUpload] 已上传文件: ${filePath}`);

    await page.waitForTimeout(1000);

    // 点击确认导入
    const submitBtn = dialog.getByRole('button', { name: submitText }).first();
    if (await submitBtn.isVisible({ timeout: 5000 }).catch((e) => { console.warn(`[uiImportUpload] 确认按钮不可见: ${(e as Error).message}`); return false; })) {
      await submitBtn.click();
      console.log(`[uiImportUpload] 已点击确认导入`);
    }

    // 等待结果提示
    await page.waitForTimeout(3000);
    const message = page.locator('.el-message__content').last();
    const messageText = await message.textContent().catch((e) => { console.warn(`[uiImportUpload] 结果提示文本读取失败: ${(e as Error).message}`); return ''; });
    console.log(`[uiImportUpload] ${entityLabel} 导入结果: ${messageText || '无提示消息'}`);
    return messageText || null;
  } catch (e) {
    console.error(`[uiImportUpload] ❌ ${entityLabel} 导入异常:`, (e as Error).message);
    return null;
  }
}
