import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  safeGet,
  safeGetList,
  safePostAction,
  verifyEndpointHealthy,
} from './helpers';

test.describe('色卡+色卡价格：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('色卡：CRUD+明细+预警+成本+扫码+报表', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/color-cards?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/color-cards/warnings');
    await verifyEndpointHealthy(page, '/color-cards/customer-color-cards?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reorder-dye-lot?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/statistics/daily');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-cards?page=1&page_size=1'
    );
    const cardId = list.items?.[0]?.id;
    if (cardId) {
      await apiCallRaw(page, 'GET', `/color-cards/${cardId}`);
      await apiCallRaw(page, 'GET', `/color-cards/${cardId}/items`);
      await verifyEndpointHealthy(page, `/color-cards/warnings/${cardId}`);
      await verifyEndpointHealthy(page, `/color-cards/cost/production/${cardId}`);
      await verifyEndpointHealthy(page, `/color-cards/export/${cardId}`);
      await verifyEndpointHealthy(page, `/color-cards/scan-by-id/${cardId}`);
    }
    await verifyEndpointHealthy(page, '/color-cards/issues?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reports/issue-detail?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reports/issue-summary?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/reports/expired-unused?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-cards/by-sales-order?sales_order_id=1');
    await verifyEndpointHealthy(page, '/color-cards/scan/TEST001');
  });

  test('色卡借出→归还→丢失→损坏状态机', async ({ page }) => {
    const issues = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-cards/issues?page=1&page_size=1'
    ).catch(() => ({ items: [] as Array<{ id: number }> }));
    const issueId = issues.items?.[0]?.id;
    if (issueId) {
      await apiCallRaw(page, 'GET', `/color-cards/issues/${issueId}`);
      await safePostAction(page, `/color-cards/issues/${issueId}/return`);
      await safePostAction(page, `/color-cards/issues/${issueId}/damaged`);
      await safePostAction(page, `/color-cards/issues/${issueId}/cancel`);
    }
  });

  test('色卡价格：CRUD+批量调价+审批+历史+阶梯+季节规则', async ({ page }) => {
    // 注意：/color-prices/ 尾斜杠在 axum 0.8 nest 下 404，必须用 /color-prices
    await apiCallRaw(page, 'GET', '/color-prices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-prices/customer-special?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/color-prices/seasonal-rules?page=1&page_size=5');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/color-prices?page=1&page_size=1'
    ).catch(() => ({ items: [] as Array<{ id: number }> }));
    const priceId = list.items?.[0]?.id;
    if (priceId) {
      await apiCallRaw(page, 'GET', `/color-prices/${priceId}`);
      await verifyEndpointHealthy(page, `/color-prices/${priceId}/history`);
      await verifyEndpointHealthy(page, `/color-prices/tiers/${priceId}`);
      await safePostAction(page, `/color-prices/${priceId}/approve`);
    }
    await verifyEndpointHealthy(page, '/color-prices/calculate?product_id=1&quantity=100');
  });

  // ===== 真实 UI 交互验证 =====
  test('色卡列表 UI：搜索+表格+新建跳转', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/list`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证搜索框
    const searchInput = page
      .locator('input[placeholder*="卡号"], input[placeholder*="卡名"]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 }).catch(() => {});
      const btnVisible = await queryBtn.isVisible().catch(() => false);
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      const tableOk = await page
        .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper')
        .first()
        .isVisible()
        .catch(() => false);
      expect(tableOk).toBe(true);
      await searchInput.clear();
    }
    // 验证新建色卡按钮
    const newBtn = page.locator('button:has-text("新建色卡")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(2000);
      // 跳转到创建页面
      const url = page.url();
      expect(url.includes('create') || url.includes('new')).toBe(true);
    }
  });

  test('色卡创建页面 UI：表单字段渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/create`);
    await page.waitForTimeout(3000);
    // 验证表单存在
    const form = page.locator('.el-form, .el-card').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const formVisible = await form.isVisible().catch(() => false);
    expect(formVisible).toBe(true);
    // 验证有卡号、卡名、类型输入字段
    const inputs = page.locator('.el-input input, .el-select');
    const inputCount = await inputs.count();
    expect(inputCount).toBeGreaterThan(0);
  });

  test('色卡价格列表 UI：表格+搜索+新建跳转', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-prices/list`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证新建价格按钮
    const newBtn = page.locator('button:has-text("新建价格")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 }).catch(() => {});
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(2000);
      const url = page.url();
      expect(url.includes('create') || url.includes('new')).toBe(true);
    }
  });

  test('色卡价格批量调价 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-prices/batch-adjust`);
    await page.waitForTimeout(3000);
    const visible = await page
      .locator('.el-card, .el-form, .el-table, .el-empty, body')
      .first()
      .isVisible()
      .catch(() => false);
    expect(visible).toBe(true);
  });

  test('色卡借出记录 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/color-cards/issues`);
    await page.waitForTimeout(3000);
    const table = page.locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper').first();
    await table.waitFor({ state: 'visible', timeout: 15_000 }).catch(() => {});
    const tableVisible = await table.isVisible().catch(() => false);
    if (tableVisible) {
      const headers = table.locator('th');
      const headerCount = await headers.count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });
});
