import { test, expect } from '../diagnose-fixture';
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
 * 状态列本地化校验：列表里渲染出的状态标签必须是"人读的文案"，
 * 而不是未映射的后端枚举原样输出（如 PENDING_APPROVAL / draft）。
 */
async function expectLocalizedStatusTags(page: import('@playwright/test').Page) {
  const texts = await page.locator('.el-table__body-wrapper .el-tag').allTextContents();
  expect(texts.length, '列表状态列应渲染出状态标签').toBeGreaterThan(0);
  for (const label of texts) {
    const raw = label.trim();
    expect(/^[A-Z][A-Z0-9_]*$/.test(raw), `状态标签输出了未映射的后端枚举原值：「${raw}」`).toBe(
      false
    );
  }
}

test.describe.serial('扩展: 状态显示映射/国际化', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('S1-1 验证采购订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /purchase 路由').toBe('/purchase');
    // 用例名承诺的是"状态中文显示"，原实现只断言 page.url() 非空
    await expectLocalizedStatusTags(page);
  });

  test('S1-2 验证销售订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`);
    await page.waitForTimeout(3000);
    expect(new URL(page.url()).pathname, '页面被重定向出 /sales 路由').toBe('/sales');
    await expectLocalizedStatusTags(page);
  });

  test('S1-3 验证 el-tag 组件渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);
    const tags = page.locator('.el-tag');
    const count = await tags.count();
    // 采购列表有种子数据，状态列必然渲染 el-tag；原写法 `count >= 0` 恒真，
    // 状态标签整体退化成纯文本也照样绿。
    expect(count, '采购列表应渲染出状态标签（.el-tag）').toBeGreaterThan(0);
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
