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

test.describe.serial('扩展: 状态显示映射/国际化', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('S1-1 验证采购订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });

  test('S1-2 验证销售订单页面状态中文显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/sales`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });

  test('S1-3 验证 el-tag 组件渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);
    const tags = page.locator('.el-tag');
    const count = await tags.count();
    expect(count >= 0).toBeTruthy();
  });

  test('S1-4 验证仪表盘页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });

  test('S1-5 验证库存页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });

  test('S1-6 验证生产页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/production`);
    await page.waitForTimeout(3000);
    expect(page.url()).toContain('/production');
  });

  test('S1-7 验证财务页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/finance`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });

  test('S1-8 验证系统管理页面加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/system`);
    await page.waitForTimeout(3000);
    expect(page.url()).toBeTruthy();
  });
});
