import { test, expect } from '../diagnose-fixture';
import type { Locator, Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  verifyBulkColorDeliveryBlock,
  verifyOutsourcingVoucher,
  verifyTrialBalance,
  verifyWeightConversion,
  verifyNetWeight,
  getCtx,
  genCode,
  ensureTestEntities,
  BASE_URL,
} from './helpers';

/**
 * 后端状态枚举原值的形态：全大写（purchase_order.order_status = DRAFT / PENDING_APPROVAL，
 * models/status/purchase_inventory.rs:13-31）或全小写下划线（sales_order.status =
 * partial_shipped，models/status/sales.rs:14-31）。两种都是"映射没命中、直接把库里的枚举
 * 值显示给用户"的形态；登录用例已把 locale 固定为 zh-CN，故本地化文案必然含非 ASCII 字符。
 */
const RAW_ENUM_TOKEN = /^[A-Za-z][A-Za-z0-9_]*$/;

/**
 * 取列表「状态」列渲染出的 el-tag 定位器（同时等待行与标签真正渲染出来）。
 *
 * 两个页面的表格实现不同，DOM 类名互不通用（原先共用 .el-table__body-wrapper .el-tag
 * 的选择器在销售页一条都匹配不到）：
 * - 采购：components/PurchaseTable.vue 用标准 el-table，每行有两个标签列
 *   （付款状态 + 订单状态），因此按表头文案锁定订单状态列本身；
 * - 销售：composables/useOlv.ts 的列经 components/V2Table 走 el-table-v2 虚拟表格，
 *   行类名是 .el-table-v2__row（不是 .el-table__row），且全表只有状态列渲染 ElTag。
 */
async function statusColumnTags(page: Page, table: 'purchase' | 'sales'): Promise<Locator> {
  if (table === 'sales') {
    const rows = page.locator('.el-table-v2__row');
    await expect(rows.first(), '销售列表未渲染出任何数据行').toBeVisible({ timeout: 30_000 });
    const tags = rows.locator('.el-tag');
    await expect(tags.first(), '销售列表状态列未渲染出 el-tag').toBeVisible({ timeout: 30_000 });
    return tags;
  }

  const headerCells = page.locator('.el-table__header th');
  await expect(headerCells.first(), '采购列表表头未渲染').toBeVisible({ timeout: 30_000 });
  const rows = page.locator('.el-table__body .el-table__row');
  await expect(rows.first(), '采购列表未渲染出任何数据行').toBeVisible({ timeout: 30_000 });
  const columnIndex = await headerCells.evaluateAll(cells =>
    cells.findIndex(c => /^(订单状态|Order Status)$/.test((c.textContent ?? '').trim()))
  );
  expect(columnIndex, '采购列表表头中找不到「订单状态」列').toBeGreaterThanOrEqual(0);
  const tags = rows.locator(`td:nth-child(${columnIndex + 1}) .el-tag`);
  await expect(tags.first(), '采购列表状态列未渲染出 el-tag').toBeVisible({ timeout: 30_000 });
  return tags;
}

/**
 * 状态列本地化校验：列表里渲染出的状态标签必须是"人读的文案"，
 * 而不是未映射的后端枚举原样输出（如 PENDING_APPROVAL / partial_shipped）。
 */
async function expectLocalizedStatusTags(page: Page, table: 'purchase' | 'sales') {
  const tags = await statusColumnTags(page, table);
  const texts = await tags.allTextContents();
  expect(texts.length, `${table} 列表状态列应渲染出状态标签`).toBeGreaterThan(0);
  for (const label of texts) {
    const raw = label.trim();
    expect(raw, `${table} 列表状态标签文案为空，映射函数没有产出文案`).not.toBe('');
    expect(
      RAW_ENUM_TOKEN.test(raw),
      `${table} 列表状态标签输出了未映射的后端枚举原值：「${raw}」`
    ).toBe(false);
  }
}

test.describe.serial('扩展: 状态显示映射/国际化', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('S1-1 验证采购订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    expect(new URL(page.url()).pathname, '页面被重定向出 /purchase 路由').toBe('/purchase');
    // 用例名承诺的是"状态中文显示"，原实现只断言 page.url() 非空
    await expectLocalizedStatusTags(page, 'purchase');
  });

  test('S1-2 验证销售订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`);
    expect(new URL(page.url()).pathname, '页面被重定向出 /sales 路由').toBe('/sales');
    await expectLocalizedStatusTags(page, 'sales');
  });

  test('S1-3 验证 el-tag 组件渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    // 原写法是固定 sleep + `count >= 0` 恒真：状态标签整体退化成纯文本也照样绿，
    // 数据慢于 3s 到达时又会假红。现由 statusColumnTags 等到状态列真的渲染出
    // el-tag 之后再计数（等待本身就是断言，超时即失败）。
    const tags = await statusColumnTags(page, 'purchase');
    expect(new URL(page.url()).pathname, '页面被重定向出 /purchase 路由').toBe('/purchase');
    expect(await tags.count(), '采购列表应渲染出状态标签（.el-tag）').toBeGreaterThan(0);
  });

  test('S1-4 验证仪表盘页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /dashboard 路由').toBe('/dashboard');
  });

  test('S1-5 验证库存页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /inventory 路由').toBe('/inventory');
  });

  test('S1-6 验证生产页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/production`);
    await page.waitForTimeout(3000);
    expect(page.url()).toContain('/production');
  });

  test('S1-7 验证财务页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/finance`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /finance 路由').toBe('/finance');
  });

  test('S1-8 验证系统管理页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/system`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /system 路由').toBe('/system');
  });
});
