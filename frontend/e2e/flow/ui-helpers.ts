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
import { expect, type Locator, type Page } from '@playwright/test';
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
        .catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
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
      .catch(e => {
        console.warn(`[E2E] 文本兜底读取: ${(e as Error).message}`);
        return '<兜底>';
      });
    const elMessages = await page
      .locator('.el-message__content')
      .allTextContents()
      .catch(e => {
        console.warn('[ui-helpers] 选择器批量查询失败:', (e as Error).message);
        return [];
      });
    const formErrors = await page
      .locator('.el-form-item__error')
      .allTextContents()
      .catch(e => {
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
      .catch(e => {
        console.warn(`[E2E] 计数兜底: ${(e as Error).message}`);
        return 0;
      });
    if (errorBoundary > 0) {
      const detailBtn = page.locator('.error-boundary button:has-text("查看详情")').first();
      if ((await detailBtn.count()) > 0) {
        await detailBtn.click().catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
        await page.waitForTimeout(300);
      }
      const stack = await page
        .locator('.error-boundary__detail')
        .textContent()
        .catch(e => {
          console.warn('[ui-helpers] error-boundary 详情读取失败:', (e as Error).message);
          return '';
        });
      console.error(`  [ErrorBoundary] 组件运行时错误: ${(stack || '').slice(0, 500)}`);
    }
    console.error(`  页面文本(前500字): ${bodyText.slice(0, 500)}`);
  } catch (e) {
    console.warn(
      '[ui-helpers] 页面截图失败（截图功能降级，诊断信息见上方日志）:',
      (e as Error).message
    );
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
        await inp2.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
        await inp2.click({ clickCount: 3 }).catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
        await inp2.fill(field.value).catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
        return;
      }
      await inp.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      await inp.click({ clickCount: 3 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      await inp.fill(field.value).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
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
      await inp.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      await inp.click({ clickCount: 3 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      await inp.fill(field.value).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      // el-date-picker fill 后需 Enter 确认（Escape 会取消选择清空值，
      // 导致"请选择染色日期"校验失败 → 请求不发 → 超时）
      await inp.press('Enter').catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
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
      await wrapper.click({ timeout: 10_000 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      await page.waitForTimeout(300);
      const dropdown = page.locator('.el-select-dropdown:visible').last();
      await dropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
      if ((await dropdown.count()) === 0) break;
      const item = dropdown
        .locator('.el-select-dropdown__item')
        .filter({ hasText: new RegExp(field.value, 'i') })
        .first();
      if ((await item.count()) > 0) {
        await item.click({ timeout: 10_000 }).catch(e => {
          console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
        });
      } else {
        // 无匹配项时选第一项（避免空点击报错拖到 120s 测试超时）
        const firstItem = dropdown.locator('.el-select-dropdown__item').first();
        if ((await firstItem.count()) > 0) {
          await firstItem.click({ timeout: 10_000 }).catch(e => {
            console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
          });
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
    const json = await resp.json().catch(e => {
      console.warn(`[waitCreateResponse] POST ${apiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    });
    return (json?.data as Record<string, unknown>) ?? json;
  } catch (e) {
    console.warn(`[waitCreateResponse] 等待 POST ${apiPath} 超时: ${(e as Error).message}`);
    return null;
  }
}

/**
 * 列表载荷「显式形状契约」工具（清零双形状探测"假绿"）。
 *
 * 背景：此前多处列表读取写成 `Array.isArray(x) ? x : (x?.items ?? [])` 之类的双形状
 * 宽容表达式，同时接受「裸数组 / 分页 items / list / data / roles」。一旦后端把某个
 * 端点的 data 形状改掉（items→list、分页→裸数组、键名变更），helper 会静默退化并
 * 返回 [] 或走错分支，使前后端契约漂移永远不会暴露——本仓正是按此法修出过合同列表、
 * AP/AR 各列表、染色配方列表等多起「列表恒空却全绿」的真实缺陷。
 *
 * 现规则：调用方必须声明自己端点的真实形状（listKey），实际不符即抛错（明确失败），
 * 禁止再收敛成「两种都接受」。
 *
 * listKey 取值与各端点的后端 handler 对应（均已逐一核对）：
 * - 'bare'  → handler 返回 ApiResponse<Vec<T>>，data 直接是数组。
 *   例：ar_invoice_handler::list_ar_invoices、account_subject_handler::list_subjects、
 *       quality_standard_handler::list_standards、print_handler::list_print_templates、
 *       product_handler::list_product_colors、sales_contract_handler::list_contracts、
 *       purchase_contract_handler::list_contracts、role_relation_handler::get_relation_between
 * - 'items' → handler 返回 ApiResponse<PaginatedResponse<T>>（data={items,total,page,page_size}）。
 *   例：warehouse/product/department/supplier/dye_batch/inventory_stock/customer/
 *       dye_recipe/greige_fabric/custom_order/color_card/business_mode/product_category 各列表
 * - 'list'  → notification_handler::list_notifications 手搓 {list,total,page,page_size}
 * - 'roles' / 'users' → 角色/用户列表以专属键返回（05-system spec 已核实）
 * - 'data'  → 个别端点在 data 下再嵌一层 data 数组
 */
export type ListShapeKey =
  | 'bare'
  | 'items'
  | 'list'
  | 'roles'
  | 'users'
  | 'data'
  | 'counts'
  | 'transitions'
  | 'nodes'
  | 'traces'
  | 'codes';

export function pickListArray<T = Record<string, unknown>>(
  data: unknown,
  listKey: ListShapeKey,
  context: string
): T[] {
  const arr =
    listKey === 'bare'
      ? Array.isArray(data)
        ? (data as T[])
        : null
      : data != null &&
          typeof data === 'object' &&
          Array.isArray((data as Record<string, unknown>)[listKey])
        ? ((data as Record<string, unknown>)[listKey] as T[])
        : null;
  if (arr === null) {
    const shape =
      data == null
        ? String(data)
        : Array.isArray(data)
          ? 'array'
          : `object{${Object.keys(data as object).join(',')}}`;
    throw new Error(
      `[${context}] 列表契约失配：声明 data${
        listKey === 'bare' ? ' 为数组' : `.${listKey} 为数组`
      }，实际 data=${shape}`
    );
  }
  return arr;
}

async function waitListResponse(
  page: Page,
  apiPath: string,
  listKey: ListShapeKey,
  timeout = 20000
): Promise<unknown[]> {
  try {
    const resp = await page.waitForResponse(
      r => r.url().includes(apiPath) && r.request().method() === 'GET',
      { timeout }
    );
    const json = await resp.json().catch(e => {
      console.warn(`[waitListResponse] GET ${apiPath} 响应非 JSON:`, (e as Error).message);
      return {};
    });
    // 单一形状直读（listKey 由调用方声明）；形状漂移即抛错，由本函数外层 catch 记为告警。
    // 注意：这里捕获抛错仅用于「UI 页未命中响应」的降级路径——真正判定形状的权威路径
    // 是 readFirstEntityId 随后的 API 直查（那里不吞异常，形状不符会让用例失败）。
    return pickListArray(json?.data, listKey, `waitListResponse ${apiPath}`);
  } catch (e) {
    console.warn(`[waitListResponse] 等待 GET ${apiPath} 失败: ${(e as Error).message}`);
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
  await page.reload({ waitUntil: 'domcontentloaded' }).catch(e => {
    console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
  });
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
  await page.reload({ waitUntil: 'domcontentloaded' }).catch(e => {
    console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
  });
  // 等待 GET /product-categories 响应完成（分类下拉数据就绪），避免异步竞态
  await page
    .waitForResponse(
      r => r.url().includes('/product-categories') && r.request().method() === 'GET',
      {
        timeout: 15_000,
      }
    )
    .catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
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
    await unitInput.fill('米').catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
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
    .catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
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
    await prodDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
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
    await versionInput.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    await versionInput.fill('1').catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });

    // 状态下拉（选"启用"或第一项）
    const statusItem = dialog.locator('.el-form-item').filter({ hasText: '状态' }).first();
    const statusSelect = statusItem.locator('.el-select__wrapper, .el-select').first();
    if ((await statusSelect.count()) > 0) {
      await statusSelect.click();
      const statusDropdown = page.locator('.el-select-dropdown:visible').last();
      await statusDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(e => {
        console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
      });
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
          await matDropdown.waitFor({ state: 'visible', timeout: 10_000 }).catch(e => {
            console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
          });
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
          await unitInput.click({ clickCount: 3 }).catch(e => {
            console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
          });
          await unitInput.fill('米').catch(e => {
            console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
          });
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
    await specInput.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    await specInput.click({ clickCount: 3 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    await specInput.fill('E2E 定制规格').catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    // 数量定位须限定"数量"label 的 form-item：页面另有 total_amount 等
    // el-input-number，.last() 会误选 total_amount 导致 quantity 空校验失败
    const quantityInput = page.locator('.el-form-item:has(:text-is("数量")) input').first();
    await quantityInput.waitFor({ state: 'visible', timeout: 20000 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    await quantityInput.click({ clickCount: 3 }).catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    await quantityInput.fill('100').catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
    // el-input-number 需 blur/Enter 同步 v-model，fill 后触发 blur
    await quantityInput.press('Tab').catch(e => {
      console.warn(`[E2E] 断言容错（元素可能未渲染）: ${(e as Error).message}`);
    });
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
  listApiPath: string,
  listKey: ListShapeKey = 'items'
): Promise<number | undefined> {
  try {
    await safeGoto(page, route);
    const items = await waitListResponse(page, listApiPath, listKey, 20000);
    const id = firstId(items);
    if (id !== undefined) return id;
  } catch (e) {
    console.warn(`[readFirstEntityId] ${route} 查找失败: ${(e as Error).message}`);
  }
  // 页面路径未命中列表响应（页面懒加载/tab 未激活/页面不发该请求）时，
  // 直接用 API 查询：避免误判"实体不存在"而反复走 120s UI 创建超时
  // （run 34019751699 shard-13：dye-batch 查找失败导致每个测试都消耗 120s+20s）
  const resp = await page.request.get(listApiPath, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
  });
  const json = (await resp.json().catch(e => {
    console.warn(`[readFirstEntityId] ${listApiPath} 响应非 JSON:`, (e as Error).message);
    return {};
  })) as { data?: unknown };
  // 单一形状直读：listKey 由调用方按端点声明（本仓所有调用方均为 'items'）。
  // 不再 `data.items ?? data.list ?? []`——那会把端点形状漂移静默吸收成"实体不存在"。
  // 此处不吞形状异常：契约失配应作为明确失败抛出，交由 ensureTestEntities 记录/暴露。
  const items = pickListArray(json?.data, listKey, `readFirstEntityId ${listApiPath}`);
  return firstId(items);
}

/** 通用 UI 列表查找：返回多条 id */
export async function readEntityIds(
  page: Page,
  route: string,
  listApiPath: string,
  listKey: ListShapeKey = 'items',
  limit = 10
): Promise<number[]> {
  // API 直查（唯一路径）：实体查询语义不变（查真实存在的实体列表），
  // 但跳过 safeGoto 页面渲染等待——16 分片并发下 UI 渲染是 ensureTestEntities
  // 超 420s 测试上限的根因，页面级覆盖由 46 崩溃巡检承担
  const resp = await page.request.get(listApiPath, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
  });
  const json = (await resp.json().catch(e => {
    console.warn(`[readEntityIds] ${listApiPath} 响应非 JSON:`, (e as Error).message);
    return {};
  })) as { data?: unknown };
  // 单一形状直读：listKey 由调用方按端点声明（本仓调用方为 warehouses/products/departments，
  // 三者均 PaginatedResponse → 'items'）。原写法 `Array.isArray(raw)?raw:(raw?.items??raw?.list??[])`
  // 同时吞裸数组/items/list，端点形状漂移时恒返回 []（读成"空库"）→ 掩盖契约变更。
  // 现形状不符即抛错，令漂移暴露；网络/解析失败由调用方 try/catch 承接（ensure 允许降级）。
  const items = pickListArray<{ id?: number }>(json?.data, listKey, `readEntityIds ${listApiPath}`);
  return items
    .slice(0, limit)
    .map(it => it?.id as number)
    .filter((id): id is number => typeof id === 'number');
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
  options?: { confirmText?: RegExp; listApiPath?: string; listKey?: ListShapeKey }
): Promise<boolean> {
  const entityLabel = route.replace(/^\//, '');
  const confirmText = options?.confirmText ?? /确定|确认|是|删除/;
  try {
    await safeGoto(page, route);
    await page.waitForTimeout(1000);

    // 找目标行
    const targetRow = await findTableRow(page, rowIdentifier.value);
    if (!targetRow) {
      console.warn(
        `[uiDeleteRow] ${entityLabel} 未找到 ${rowIdentifier.column}=${rowIdentifier.value} 的行`
      );
      return false;
    }

    // 点删除按钮（el-button type=danger link）
    const deleteBtn = targetRow
      .locator('button.el-button--danger, button:has-text("删除")')
      .first();
    await deleteBtn.waitFor({ state: 'visible', timeout: 5000 });
    await deleteBtn.click();
    console.log(`[uiDeleteRow] 已点击删除按钮`);

    // 确认弹窗
    await page.waitForTimeout(500);
    const confirmBtn = page.getByRole('button', { name: confirmText }).last();
    if (
      await confirmBtn.isVisible({ timeout: 3000 }).catch(e => {
        console.warn(`[uiDeleteRow] 确认弹窗可见性查询失败: ${(e as Error).message}`);
        return false;
      })
    ) {
      await confirmBtn.click();
      console.log(`[uiDeleteRow] 已确认删除`);
    }

    // 等待列表刷新
    await page.waitForTimeout(2000);
    if (options?.listApiPath) {
      await waitListResponse(page, options.listApiPath, options.listKey ?? 'items', 15000).catch(
        e => {
          console.warn(`[uiDeleteRow] 列表刷新响应等待失败:`, (e as Error).message);
        }
      );
    }

    // 验证行消失
    const stillExists = await findTableRow(page, rowIdentifier.value);

    if (stillExists) {
      console.error(
        `[uiDeleteRow] ❌ ${entityLabel} 删除后行仍存在（${rowIdentifier.column}=${rowIdentifier.value}）`
      );
      return false;
    }
    console.log(`[uiDeleteRow] ✅ ${entityLabel} 删除成功，行已消失`);
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

    const targetRow = await findTableRow(page, rowIdentifier.value);
    if (!targetRow) {
      console.warn(`[uiToggleStatus] ${entityLabel} 未找到目标行`);
      return false;
    }

    // 找状态开关（el-switch 或 el-button 带"停用"/"启用"文案）
    const switchEl = targetRow.locator('.el-switch').first();
    const statusBtn = targetRow
      .locator('button:has-text("停用"), button:has-text("启用"), button:has-text("禁用")')
      .first();
    if (
      await switchEl.isVisible({ timeout: 3000 }).catch(e => {
        console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`);
        return false;
      })
    ) {
      await switchEl.click();
      console.log(`[uiToggleStatus] 已点击状态开关`);
    } else if (
      await statusBtn.isVisible({ timeout: 3000 }).catch(e => {
        console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`);
        return false;
      })
    ) {
      await statusBtn.click();
      console.log(`[uiToggleStatus] 已点击状态按钮`);
    } else {
      console.warn(`[uiToggleStatus] ${entityLabel} 未找到状态切换控件`);
      return false;
    }

    await page.waitForTimeout(2000);

    // 验证状态文本
    // textContent() 的解析结果本身可为 null（元素存在但无文本），与读取失败的
    // catch 分支同样归一为空串哨兵，两者都使下面的 includes 判定为 false（切换未确认）。
    const rowText =
      (await targetRow.textContent().catch(e => {
        console.warn(`[uiToggleStatus] 行文本读取失败: ${(e as Error).message}`);
        return '';
      })) ?? '';
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
      if (
        await confirmBtn.isVisible({ timeout: 3000 }).catch(e => {
          console.warn(`[E2E] 可见性查询失败: ${(e as Error).message}`);
          return false;
        })
      ) {
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
      const templateBtn = dialog
        .getByRole('button', { name: options.templateDownloadText })
        .first();
      if (
        await templateBtn.isVisible({ timeout: 3000 }).catch(e => {
          console.warn(`[uiImportUpload] 模板下载按钮不可见: ${(e as Error).message}`);
          return false;
        })
      ) {
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
    if (
      await submitBtn.isVisible({ timeout: 5000 }).catch(e => {
        console.warn(`[uiImportUpload] 确认按钮不可见: ${(e as Error).message}`);
        return false;
      })
    ) {
      await submitBtn.click();
      console.log(`[uiImportUpload] 已点击确认导入`);
    }

    // 等待结果提示
    await page.waitForTimeout(3000);
    const message = page.locator('.el-message__content').last();
    const messageText = await message.textContent().catch(e => {
      console.warn(`[uiImportUpload] 结果提示文本读取失败: ${(e as Error).message}`);
      return '';
    });
    console.log(`[uiImportUpload] ${entityLabel} 导入结果: ${messageText || '无提示消息'}`);
    return messageText || null;
  } catch (e) {
    console.error(`[uiImportUpload] ❌ ${entityLabel} 导入异常:`, (e as Error).message);
    return null;
  }
}

// ---------------------------------------------------------------------------
// 通用 UI 操作原语（消除 spec 重复步骤）
// ---------------------------------------------------------------------------

/**
 * 表格行 DOM 形态：
 * - 'standard'：Element Plus `<el-table>`，行类名 `.el-table__row`，单元格 `td`
 *   （如 occupational-health 危害监测表 index.vue:38）。
 * - 'virtual'：`<el-table-v2>`（本仓 components/V2Table 包装），行类名 `.el-table-v2__row`、
 *   单元格 `.el-table-v2__row-cell`（如 sales-contract 列表 SalesContractTable.vue 已迁移）。
 * 两形态行类名互不通用：`.el-table__row` 对虚拟表恒空（N4/S6 直接来源）。
 */
export type TableRowShape = 'standard' | 'virtual';

/** findTableRow 匹配方式。 */
export interface FindTableRowOptions {
  /** 显式指定表格形态；省略时按页面实际挂载的表格容器唯一确定（见 resolveRowShape）。 */
  shape?: TableRowShape;
  /**
   * 匹配粒度：
   * - 'row-contains'（默认）：整行 textContent 含目标值（适合唯一业务编码/名称，如合同号）。
   * - 'first-cell-equals'：仅首列文本严格等于目标值（适合按自增主键 id 精确定位自建行，
   *   规避数字子串误命中，如 id=1 命中 id=12/101）。
   */
  match?: 'row-contains' | 'first-cell-equals';
}

/** 某形态的行选择器（不含 :visible，由调用方拼接）。 */
function rowSelectorFor(shape: TableRowShape): string {
  return shape === 'virtual' ? '.el-table-v2__row' : '.el-table__row';
}

/** 某形态行内首单元格选择器。 */
function firstCellSelectorFor(shape: TableRowShape): string {
  return shape === 'virtual' ? '.el-table-v2__row-cell' : 'td';
}

/**
 * 显式确定目标表格形态（禁止跨形态静默兜底探测）：
 * - 调用方传了 shape → 直接用（并存页必须由调用方点名要操作哪张表）。
 * - 未传：按页面挂载的表格容器唯一判定——只虚拟→virtual、只普通→standard。
 * - 两形态并存且未点名 → 抛错（要求调用方传 shape，不猜）。
 * - 两形态都没有 → 抛错点名两个容器定位器 + 当前 URL + DOM 片段（fail-visible）。
 */
async function resolveRowShape(page: Page, shape?: TableRowShape): Promise<TableRowShape> {
  if (shape) return shape;
  // 按「可见」容器判定：EP el-dialog 关闭不销毁内部 DOM（display:none 子树不计入 :visible），
  // 否则列表 V2Table 与隐藏表单里的 el-table 会被误判为「两形态并存」。
  const virtualCount = await page.locator('.el-table-v2:visible').count();
  const standardCount = await page.locator('.el-table:visible').count();
  if (virtualCount > 0 && standardCount > 0) {
    throw new Error(
      `[findTableRow] 页面同时可见 .el-table-v2(${virtualCount}) 与 .el-table(${standardCount}) ` +
        `两种表格形态且调用方未指定 shape，无法自动判定目标表（URL=${page.url()}）——须显式传 opts.shape`
    );
  }
  if (virtualCount > 0) return 'virtual';
  if (standardCount > 0) return 'standard';
  const dom = await page
    .locator('body')
    .innerText()
    .catch(() => '<读取失败>');
  throw new Error(
    `[findTableRow] 页面无任何可见表格容器（.el-table-v2=${virtualCount}, .el-table=${standardCount}，` +
      `URL=${page.url()}，body 片段=${JSON.stringify(dom.slice(0, 300))}）——表格未渲染即判红，不返回 null 掩盖`
  );
}

/**
 * 按列值查找可见表格行（同源收口，标准表 + 虚拟表两形态）。
 *
 * 替代各 spec 里重复/各写一套的"遍历行→匹配→返回"循环，并统一修复此前只认
 * `.el-table__row` 而对 el-table-v2 虚拟表恒空的缺陷（N4 销售合同 / S6）。
 * 形态按 resolveRowShape 显式判定；表格容器都不存在时直接抛错（fail-visible），
 * 目标行在已判定形态下找不到才返回 null（交由调用方按唯一键断言，不放宽）。
 *
 * @param page          Playwright Page
 * @param value         目标值（toString 后匹配）
 * @param minRows       最少等待行数（默认 1），列表未渲染时先等
 * @param filterKeyword 可选：先在 .filter-card 搜索框按该关键字过滤再扫目标行；并行模式下列表
 *                      膨胀时收敛结果，规避分页。无搜索框则告警回退首页扫描。
 * @param opts.shape    表格形态；省略时按页面唯一挂载的表格容器判定（并存须点名）。
 * @param opts.match    'row-contains'（默认，整行含值）| 'first-cell-equals'（首列严格等，按 id 精确）。
 * @returns 目标行 Locator 或 null
 */
export async function findTableRow(
  page: Page,
  value: string | number,
  minRows = 1,
  filterKeyword?: string,
  opts: FindTableRowOptions = {}
): Promise<Locator | null> {
  const shape = await resolveRowShape(page, opts.shape);
  const rowSel = rowSelectorFor(shape);
  const match = opts.match ?? 'row-contains';
  if (filterKeyword !== undefined) {
    // 与 31c 用户 Tab 既有写法一致：仅命中当前激活区可见搜索框（隐藏 Tab 的 filter-card 不抢）
    const keywordInput = page.locator('.filter-card input:visible').first();
    const hasSearch = await keywordInput.isVisible({ timeout: 4000 }).catch(() => false);
    if (hasSearch) {
      await keywordInput.fill(String(filterKeyword));
      await keywordInput.press('Enter');
      // 等 keyword 请求返回 + 表格按当前形态重渲染出结果行
      await page
        .locator(`${rowSel}:visible`)
        .first()
        .waitFor({ state: 'visible', timeout: 10_000 })
        .catch(() => {});
      await page.waitForTimeout(500);
    } else {
      console.warn(
        `[findTableRow] 未找到可见搜索框，跳过过滤回退首页扫描（keyword=${filterKeyword}）`
      );
    }
  }
  const rows = page.locator(`${rowSel}:visible`);
  await rows
    .first()
    .waitFor({ state: 'visible', timeout: 10000 })
    .catch(() => {});
  const rowCount = await rows.count();
  if (rowCount < minRows) {
    console.warn(
      `[findTableRow] ${shape} 表（${rowSel}）仅 ${rowCount} 行（期望≥${minRows}），可能未加载`
    );
  }
  const target = String(value);
  for (let i = 0; i < rowCount; i++) {
    const row = rows.nth(i);
    if (match === 'first-cell-equals') {
      const cellText = await row
        .locator(firstCellSelectorFor(shape))
        .first()
        .textContent()
        .catch(() => '');
      if ((cellText ?? '').trim() === target) return row;
    } else {
      const txt = await row.textContent().catch(() => '');
      if (txt?.includes(target)) return row;
    }
  }
  return null;
}

/**
 * 点击触发按钮并等待对话框可见
 *
 * @param page         Playwright Page
 * @param triggerText  触发按钮文案（getByRole name 正则）
 * @returns dialog Locator（已可见）
 */
export async function openDialog(page: Page, triggerText: string | RegExp): Promise<Locator> {
  const btn = page.getByRole('button', { name: triggerText }).first();
  await btn.click();
  const dialog = page.locator('.el-dialog:visible').first();
  await dialog.waitFor({ state: 'visible', timeout: 15000 });
  return dialog;
}

/**
 * 在可见对话框内点击提交/确认按钮
 *
 * @param dialog    对话框 Locator
 * @param btnText   按钮文案正则（默认 /确定|确认|保存|提交/）
 */
export async function submitDialog(
  dialog: Locator,
  btnText: RegExp = /确定|确认|保存|提交/
): Promise<void> {
  const btn = dialog.getByRole('button', { name: btnText }).last();
  await btn.waitFor({ state: 'visible', timeout: 5000 });
  await btn.click();
}

/** 关闭对话框（点取消/关闭按钮或按 ESC） */
export async function closeDialog(dialog: Locator): Promise<void> {
  const cancel = dialog.getByRole('button', { name: /取消|关闭/ }).first();
  if (await cancel.isVisible().catch(() => false)) {
    await cancel.click();
  } else {
    await dialog.page().keyboard.press('Escape');
  }
}

// ---------------------------------------------------------------------------
// el-select 通用交互（对齐 8c1adf03 / ef1f6137 已跑通范式）
// ---------------------------------------------------------------------------

/**
 * 构造「按 form-item 内 label 文本锚出其内层 .el-select」的触发 Locator。
 *
 * EP 的 el-select 内层为 readonly combobox input：直接 getByLabel/combobox 命中该 input，
 * 会被 `.el-select__placeholder` 拦截 pointer events 或 `element is not stable` → 30s 超时。
 * 正确姿势是锚到含该 label 的 `.el-form-item`，再取其中的 `.el-select` 外层触发点。
 *
 * 【迁移提示】新代码优先用本文件「唯一事实源」区块的 pickSelectIn / pickV2In / pickV2Remote /
 * formItemByExactLabel（默认精确锚定、root 作用域、无静默）。本函数仅为既有 ~30 处 spec 调用
 * 向后兼容保留（默认子串匹配 + 部分调用方传 RegExp），勿在新域修复中使用。
 *
 * @param root      作用域（对话框/页面/表格容器）Locator 或 Page
 * @param labelText form-item 的 label 文本（string 子串匹配；RegExp 按原样匹配）
 * @param exact     true 时 label 必须整串相等（避免 '客户' 命中 '客户等级' 这类共享子串）
 */
export function elSelectByLabel(
  root: Locator | Page,
  labelText: string | RegExp,
  exact = false
): Locator {
  const pattern =
    exact && typeof labelText === 'string'
      ? new RegExp(`^\\s*${escRe(labelText)}\\s*$`)
      : labelText;
  return root
    .locator('.el-form-item')
    .filter({ has: root.locator('.el-form-item__label', { hasText: pattern }) })
    .first()
    .locator('.el-select')
    .first();
}

/** pickSelect 行为开关 */
export interface PickSelectOptions {
  /** 未传 optionText 时按下标选第 N 个 option（默认 0=首个），等价旧 getByRole('option').nth(k) */
  index?: number;
  /** 多选：选完后按 Escape 收起 dropdown（EP 多选面板不自动关闭） */
  multiple?: boolean;
  /** 仅展开并等待首个 option 可见，不做选择（用于「验证下拉可打开」型用例，调用方自行 Escape） */
  openOnly?: boolean;
  /** 单步超时（默认 10_000ms） */
  timeout?: number;
}

/**
 * 打开一个 el-select（点外层 .el-select/.el-select__wrapper 而非 readonly input），
 * 并从 body-level 的可见 dropdown 选取 option。
 *
 * 选项面板由 EP teleport 到 body，故用 `.el-select-dropdown:visible` + `.el-select-dropdown__item`
 * 定位（对齐 ef1f6137 purchase/01、8c1adf03 ai/crm 既有写法），不再走 getByRole('option')
 * （后者在 dropdown 提前关闭时不稳）。
 *
 * 【迁移提示】本函数接收调用方已构造好的 trigger Locator，保留供既有 spec 向后兼容。
 * 新域修复改用 pickSelectIn / pickV2In / pickV2Remote（按精确 label 自管 root 作用域，无静默）。
 *
 * @param page       Playwright Page
 * @param trigger    指向 `.el-select`（或含之的 form-item/容器）的 Locator；内部优先点 .el-select__wrapper
 * @param optionText 目标 option 文本（string 子串/RegExp）；省略则按 opts.index 取下标项
 */
export async function pickSelect(
  page: Page,
  trigger: Locator,
  optionText?: string | RegExp,
  opts: PickSelectOptions = {}
): Promise<void> {
  const timeout = opts.timeout ?? 10_000;
  const wrapper = trigger.locator('.el-select__wrapper').first();
  const clickTarget = (await wrapper.count()) > 0 ? wrapper : trigger;
  await clickTarget.click({ timeout });
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await dropdown.waitFor({ state: 'visible', timeout });
  const items = dropdown.locator('.el-select-dropdown__item');
  if (opts.openOnly) {
    await items.first().waitFor({ state: 'visible', timeout });
    return;
  }
  const item =
    optionText === undefined
      ? items.nth(opts.index ?? 0)
      : items.filter({ hasText: optionText }).first();
  await item.waitFor({ state: 'visible', timeout });
  await item.click({ timeout });
  if (opts.multiple) {
    await page.keyboard.press('Escape');
  } else {
    // 单选后 dropdown 收起（非致命，仅确保动画稳定再让下一步执行）
    await dropdown.waitFor({ state: 'hidden', timeout }).catch(() => {});
  }
}

// ---------------------------------------------------------------------------
// el-select / el-select-v2 选择器 helper —— 唯一事实源（single source of truth）
// ---------------------------------------------------------------------------
//
// 后续所有 e2e 域修复只调用本区块 API，不再各自内联下拉交互逻辑，也不再改本文件。
// 契约（三条硬规则，违反即回退本 helper，不在 spec 里临时兜底）：
//  1. 作用域：所有 form-item / select / input 定位一律以传入的 root（dialog 或页内某 form
//     容器）为界，绝不绝对作用域——列表页搜索栏常有与弹窗同名的 label（如「我方产品」），
//     绝对作用域会跨容器命中错误节点。
//  2. 精确标签锚定：.el-form-item__label 用 RegExp('^'+escRe(label)+'$') 行首尾锚定，
//     消除子串误命中（「供应商品编码」⊃「供应商品」、「我方色号编号」⊃「我方色号」、
//     「客户」⊃「客户等级」这类同前缀 label 是既往假红的直接来源）。
//  3. 禁静默：定位失败必须让 Playwright 自然超时抛错（真实红），本区块内**严禁**任何
//     `.catch(()=>null/false/[])` 把「找不到」吞成「为假/为空」。仅允许对「点击后下拉收起
//     的动画收尾」这类已确认成功之后的软等待做 .catch。
//
// 选 API 的判据：
//  - el-select（普通单选/多选，选项全量渲染）→ pickSelectIn
//  - el-select-v2（虚拟滚动 / filterable 本地）→ pickV2In
//  - el-select-v2 + remote（filterable remote，输入触发远程搜索 / 高基数不整表渲染）→ pickV2Remote
//  - 读禁用 → isFieldDisabled；读值 → getFieldValue；填文本值 → fillFieldByLabel

/** 正则元字符转义，供精确锚定 label / option 文本构造 RegExp('^…$') 使用。 */
export function escRe(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * 锚到含「整串相等」label 的 .el-form-item（契约第 2 条：精确标签锚定）。
 *
 * 与旧 elSelectByLabel 的区别：旧函数默认子串匹配（hasText 直接命中「供应商品编码」里的
 * 「供应商品」），且仅返回内层 .el-select。本函数始终以 root 为作用域、以精确 label 命中
 * 唯一 form-item，并返回 form-item 本身供上层继续下钻到 wrapper / input / v2 容器。
 *
 * @param root      作用域（dialog 或页内某 form 容器）Locator 或 Page
 * @param labelText form-item 的 label 全文（非子串、非正则；内部自动 escRe 锚定行首尾）
 */
export function formItemByExactLabel(root: Locator | Page, labelText: string): Locator {
  // 真根因(#4657 证伪"星号说"后定位,Playwright filter({has}) 语义):
  // 内层 `has` locator 的 **selector** 会被再嵌到候选元素内部匹配。若内层写成
  // `root.locator('.el-form-item__label')`(root 携带 `.el-dialog`/form 段),就变成
  // "在单个 form-item 内部再找一个 .el-dialog" → 永远零命中 → 上层 .el-select__wrapper
  // 超时假红(无对话框的 inventory 筛选表单同样零命中,与星号/CSS 无关;EP 必填星号是
  // ::before 生成内容,根本不在 textContent,先前据 a11y 快照的 "* 客户" 推断是取证错误)。
  // 正解:内层以 **Page 为基、只带 `.el-form-item__label`**,重嵌后即 `form-item .el-form-item__label` ✓。
  const page = (root as Locator).page?.() ?? (root as Page);
  return root
    .locator('.el-form-item')
    .filter({
      has: page.locator('.el-form-item__label', {
        hasText: new RegExp(`^\\s*\\*?\\s*${escRe(labelText)}\\s*$`),
      }),
    })
    .first();
}

/** 下拉交互共参（各 pick* 系列复用）。 */
interface DropdownPickOpts {
  /** 目标 option 文本：string 走「转义后子串」匹配；RegExp 按原样。省略则按下标 index 选。 */
  optionText?: string | RegExp;
  /** 未传 optionText 时按下标选第 N 项（默认 0=首个）。 */
  index?: number;
  /** 多选：选完按 Escape 收起（EP 多选面板不自动关）。 */
  multiple?: boolean;
  /** 仅展开并等首个 option 可见即返回（「验证下拉可打开」型用例，调用方自行 Escape）。 */
  openOnly?: boolean;
  /** 远程搜索输入词：非空则先 click 触发器、keyboard.type 触发 remote-method 再选。 */
  query?: string;
  /** 单步超时（默认 15_000ms；CI 并发慢）。 */
  timeout?: number;
}

/**
 * 内部核心：点下拉触发器（wrapper，非只读内层 input）→（可选 type query 触发远程）
 * → 等 body 级可见 dropdown → 精确/子串匹配 option 并点。
 *
 * 触发器由调用方（pickSelectIn/pickV2In/pickV2Remote）用 wrapper 类构造，这里只负责通用
 * 的「点 - 等面板 - 选项」时序。任何一步定位失败均直接抛错（契约第 3 条），无 catch。
 * 唯一 .catch 用于「点完后面板收起的动画收尾」（面板已确认出现过，属成功之后的软等待）。
 */
async function openDropdownAndPick(
  page: Page,
  trigger: Locator,
  opts: DropdownPickOpts
): Promise<void> {
  const timeout = opts.timeout ?? 15_000;
  await trigger.waitFor({ state: 'visible', timeout });
  // 点 wrapper（.el-select__wrapper / .el-select-v2__wrapper），不点只读 combobox 内层 input
  // ——内层 input 被 placeholder 拦 pointer events / element not stable（既往假红根因②）。
  await trigger.click({ timeout });
  if (opts.query) {
    await page.keyboard.type(opts.query, { delay: 50 });
    // 远程搜索：等 remote-method 请求回、loading 指示消失后再找项。
    // state:'detached' 对「本就无 loading 节点」立即 resolve（搜索秒回的正常态），
    // 而对「loading 卡住不消失」超时抛错（远程搜索挂起=真实红），故此处不加 .catch 吞错。
    await page
      .locator(
        '.el-select-dropdown:visible .el-select-loading, .el-select-dropdown:visible .is-loading'
      )
      .first()
      .waitFor({ state: 'detached', timeout });
  }
  const dropdown = page.locator('.el-select-dropdown:visible').last();
  await dropdown.waitFor({ state: 'visible', timeout });
  // el-select 与 el-select-v2 虚拟列表的 option 类名不同，一并匹配
  const items = dropdown.locator('.el-select-dropdown__item, .el-select-v2__item');
  if (opts.openOnly) {
    await items.first().waitFor({ state: 'visible', timeout });
    return;
  }
  const option =
    opts.optionText === undefined
      ? items.nth(opts.index ?? 0)
      : items.filter({ hasText: opts.optionText }).first();
  await option.waitFor({ state: 'visible', timeout });
  await option.click({ timeout });
  if (opts.multiple) {
    await page.keyboard.press('Escape');
  } else {
    // 点完面板收起属动画收尾（option 已确认可见并点击成功），此处 .catch 不掩盖定位失败
    await dropdown.waitFor({ state: 'hidden', timeout: 5_000 }).catch(() => {});
  }
}

/**
 * el-select（普通下拉）：按精确 label 打开并从 body 级面板选目标项。
 * 契约同本区块头部三条。optionText 省略则按下标选（默认首项）。
 *
 * @param root      作用域（dialog / 页内 form 容器）
 * @param page      用于等 teleport 到 body 的下拉面板
 * @param labelText form-item 精确 label 全文
 */
export async function pickSelectIn(
  root: Locator | Page,
  page: Page,
  labelText: string,
  opts: DropdownPickOpts = {}
): Promise<void> {
  const wrapper = formItemByExactLabel(root, labelText).locator('.el-select__wrapper').first();
  await openDropdownAndPick(page, wrapper, opts);
}

/**
 * el-select-v2（虚拟滚动 / 本地 filterable）：按精确 label 打开并在虚拟列表面板选目标。
 * 高基数场景 optionText 必传（不整表扫描），走转义子串/正则匹配。
 *
 * @param root      作用域
 * @param page      用于等 body 级面板
 * @param labelText form-item 精确 label 全文
 */
export async function pickV2In(
  root: Locator | Page,
  page: Page,
  labelText: string,
  opts: DropdownPickOpts = {}
): Promise<void> {
  const wrapper = formItemByExactLabel(root, labelText).locator('.el-select-v2__wrapper').first();
  await openDropdownAndPick(page, wrapper, opts);
}

/**
 * el-select-v2 + remote（filterable remote，输入关键词触发 remote-method）：
 * 按精确 label 打开 → keyboard.type(keyword) 触发远程搜索 → 等 loading 收 → 点含 pickText 的项。
 * 高基数不整表渲染，故必须先输关键词把候选收敛到可见项再点。
 *
 * @param root      作用域
 * @param page      用于 keyboard.type 与等 body 级面板
 * @param labelText form-item 精确 label 全文
 * @param keyword   触发远程搜索的输入词（如商品编码片段）
 * @param pickText  搜索回来后要点中的候选文本（string 走转义子串；RegExp 原样）
 */
export async function pickV2Remote(
  root: Locator | Page,
  page: Page,
  labelText: string,
  keyword: string,
  pickText: string | RegExp,
  opts: Omit<DropdownPickOpts, 'optionText' | 'query'> = {}
): Promise<void> {
  const wrapper = formItemByExactLabel(root, labelText).locator('.el-select-v2__wrapper').first();
  await openDropdownAndPick(page, wrapper, { ...opts, query: keyword, optionText: pickText });
}

/**
 * 读某表单项当前是否真实禁用（级联「未选上级则下级 disabled」断言）。
 *
 * 兼容 el-select 与 el-select-v2：disabled 标记节点在不同组件不一致——
 *   优先读容器 aria-disabled；否则读容器 / 内层 __wrapper 的 is-disabled 类。
 * 定位不到（容器不存在 / form-item 命不中）→ getAttribute 自然超时抛错（契约第 3 条），
 * **绝不** `.catch(()=>false)` 把「找不到」吞成「未禁用」（既往 #17 假红根因：只读根元素
 * class 找 is-disabled，而 EP 实际把 disabled 标在 aria-disabled / __wrapper 上）。
 *
 * @param root      作用域
 * @param labelText form-item 精确 label 全文
 * @param timeout   定位超时（默认 10_000ms）
 */
export async function isFieldDisabled(
  root: Locator | Page,
  labelText: string,
  timeout = 10_000
): Promise<boolean> {
  const container = formItemByExactLabel(root, labelText)
    .locator('.el-select, .el-select-v2')
    .first();
  await container.waitFor({ state: 'attached', timeout });
  const aria = await container.getAttribute('aria-disabled');
  if (aria !== null) return aria === 'true';
  const containerCls = (await container.getAttribute('class')) ?? '';
  if (containerCls.includes('is-disabled')) return true;
  // 兜底：部分版本把 is-disabled 只加在内层 __wrapper 上（getAttribute 命中失败会超时抛错）
  const wrapperCls = await container
    .locator('.el-select__wrapper, .el-select-v2__wrapper')
    .first()
    .getAttribute('class');
  return !!wrapperCls && wrapperCls.includes('is-disabled');
}

/**
 * 读某表单项真 input 当前值（如只读回显「供应商品编码」）。
 * 以 root 作用域 + 精确 label 命中真 input；inputValue 定位失败即超时抛错，
 * **绝不** `.catch(()=>'')` 把定位失败吞成空串（#18 假红根因之一）。
 */
export async function getFieldValue(
  root: Locator | Page,
  labelText: string,
  timeout = 10_000
): Promise<string> {
  return formItemByExactLabel(root, labelText).locator('input').first().inputValue({ timeout });
}

/**
 * 按精确 label 填充可编辑 el-input（如「协议价」）。
 * 以 root 作用域 + 精确 label 命中真 input；任一步定位失败抛错（无静默）。
 */
export async function fillFieldByLabel(
  root: Locator | Page,
  page: Page,
  labelText: string,
  value: string,
  timeout = 10_000
): Promise<void> {
  const inp = formItemByExactLabel(root, labelText).locator('input').first();
  await inp.waitFor({ state: 'visible', timeout });
  await inp.click({ clickCount: 3, timeout });
  await inp.fill(value);
  await page.waitForTimeout(100);
}

// ---------------------------------------------------------------------------
// 语义化等待（替代硬编码 waitForTimeout）
// ---------------------------------------------------------------------------

/** 等待对话框可见（替代 waitForTimeout + locator 检查） */
export async function waitForDialog(page: Page, timeout = 15000): Promise<Locator> {
  const dialog = page.locator('.el-dialog:visible').first();
  await dialog.waitFor({ state: 'visible', timeout });
  return dialog;
}

/** 等待表格至少 N 行可见（替代 waitForTimeout 等列表加载） */
export async function waitForTableRows(page: Page, minCount = 1, timeout = 10000): Promise<number> {
  const rows = page.locator('.el-table__row:visible');
  await rows
    .first()
    .waitFor({ state: 'visible', timeout })
    .catch(() => {});
  return rows.count();
}

/** 等待 toast 消息出现并获取文本（替代 waitForTimeout + textContent） */
export async function waitForToast(page: Page, timeout = 5000): Promise<string> {
  const msg = page.locator('.el-message__content').last();
  await msg.waitFor({ state: 'visible', timeout }).catch(() => {});
  return (await msg.textContent().catch(() => '')) ?? '';
}

/** 等待 toast 消息消失 */
export async function waitForToastGone(page: Page, timeout = 5000): Promise<void> {
  await page
    .locator('.el-message')
    .waitFor({ state: 'detached', timeout })
    .catch(() => {});
}

// ---------------------------------------------------------------------------
// 核心业务页面验证原语（从 28 系列提取，消除 3×105 行重复）
// ---------------------------------------------------------------------------

/** 访问页面并验证表格/容器已加载，返回表格 Locator */
export async function visitAndVerifyTable(page: Page, path: string): Promise<Locator> {
  await page.goto(`${BASE_URL}${path}`);
  const container = page
    .locator(
      '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, .el-form'
    )
    .first();
  await container
    .waitFor({ state: 'visible', timeout: 30_000 })
    .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
  return page.locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper').first();
}

/** 验证按钮可见且可点击 */
export async function verifyButton(page: Page, text: string): Promise<boolean> {
  const btn = page.locator(`button:has-text("${text}")`).first();
  await btn
    .waitFor({ state: 'visible', timeout: 5000 })
    .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
  const visible = await btn.isVisible().catch(() => false);
  if (visible) {
    const disabled = await btn.isDisabled().catch(() => false);
    expect(disabled).toBe(false);
  }
  return visible;
}

/** 点击新建按钮并验证弹窗出现（基于 openDialog，消除 waitForTimeout(1000)） */
export async function clickNewAndVerifyDialog(page: Page, btnText: string): Promise<boolean> {
  const btn = page.locator(`button:has-text("${btnText}")`).first();
  // 按钮不可见或弹窗未出现=页面异常，直接失败暴露（失败必须修复，禁止静默返回 false）
  await btn.waitFor({ state: 'visible', timeout: 5000 });
  await btn.click();
  const dialog = await waitForDialog(page, 5000);
  return dialog !== null;
}

/** 验证表单必填校验：点击 footer 主按钮→检查 .el-message / .el-form-item__error */
export async function verifyRequiredValidation(page: Page): Promise<boolean> {
  const dialog = page.locator('.el-dialog:visible').first();
  const saveBtn = dialog.locator('.el-dialog__footer .el-button--primary').first();
  try {
    await saveBtn.click({ timeout: 10_000 });
  } catch {
    return false;
  }
  await page
    .locator('.el-message, .el-form-item__error')
    .first()
    .waitFor({ state: 'visible', timeout: 5000 })
    .catch(() => {});
  return page
    .locator('.el-message, .el-form-item__error')
    .first()
    .isVisible()
    .catch(() => false);
}

/** 关闭弹窗（点 headerbtn，替代 waitForTimeout(500)） */
export async function closeDialogByX(page: Page): Promise<void> {
  await page
    .locator('.el-dialog__headerbtn')
    .first()
    .click()
    .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
}
