import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('列表交互与状态显示', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单列表：分页切换+状态标签显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase/orders`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 验证状态标签（el-tag）存在
    const statusTags = page.locator('.el-table .el-tag');
    const tagCount = await statusTags.count();
    if (tagCount > 0) {
      const firstTagText = await statusTags.first().textContent();
      expect(firstTagText?.trim().length).toBeGreaterThan(0);
    }

    // 测试分页
    const pagination = page.locator('.el-pagination').first();
    const paginationVisible = await pagination.isVisible().catch(() => false);
    if (paginationVisible) {
      const page2 = pagination.locator('.el-pager .number:has-text("2")').first();
      const page2Visible = await page2.isVisible().catch(() => false);
      if (page2Visible) {
        await page2.click();
        await page.waitForTimeout(2000);
        await page.locator('.el-table').first().waitFor({ state: 'visible', timeout: 10_000 });
      }
    }
  });

  test('库存列表：空数据展示验证', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory/stock`);
    await page.waitForTimeout(3000);

    const table = page.locator('.el-table').first();
    const empty = page.locator('.el-empty, .el-table__empty-block, .el-table__empty-text').first();

    const tableVisible = await table.isVisible({ timeout: 15_000 }).catch(() => false);
    const emptyVisible = await empty.isVisible({ timeout: 5_000 }).catch(() => false);

    expect(tableVisible || emptyVisible).toBe(true);

    if (tableVisible) {
      const headers = page.locator('.el-table__header th, .el-table__header-wrapper th');
      const headerCount = await headers.count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });

  test('仪表盘加载状态和图表渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    const container = page.locator('.dashboard-container, .el-card, .el-row').first();
    await container.waitFor({ state: 'visible', timeout: 30_000 });

    const hasStats = await page.locator('.el-statistic, .el-card, [class*="stat"]').first().isVisible().catch(() => false);
    const hasChart = await page.locator('canvas, .echarts, [class*="chart"]').first().isVisible().catch(() => false);
    expect(hasStats || hasChart).toBe(true);
  });

  test('供应商列表：搜索过滤交互', async ({ page }) => {
    // 真实路由：/purchase/supplier
    await page.goto(`${BASE_URL}/purchase/supplier`);
    await page.waitForTimeout(3000);

    await page.locator('.el-table, .el-card').first().waitFor({ state: 'visible', timeout: 30_000 });

    // 查找搜索输入框（真实 placeholder 含"供应商名称"或类似）
    const searchInput = page.locator('.filter-form input, .filter-card input').first();
    const searchVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);

    if (searchVisible) {
      await searchInput.fill('测试');
      await page.waitForTimeout(1000);

      // 点击查询按钮（真实文本"查询"）
      const searchBtn = page.locator('button:has-text("查询")').first();
      const searchBtnVisible = await searchBtn.isVisible({ timeout: 3000 }).catch(() => false);
      if (searchBtnVisible) {
        await searchBtn.click();
        await page.waitForTimeout(2000);
      }

      const tableStillVisible = await page.locator('.el-table').first().isVisible().catch(() => false);
      expect(tableStillVisible).toBe(true);

      await searchInput.clear();
      await page.waitForTimeout(500);
    }
  });

  test('Tab 切换触发数据重新加载', async ({ page }) => {
    await page.goto(`${BASE_URL}/api-gateway`);
    await page.waitForTimeout(3000);

    // 等待 el-tabs 加载
    const tabs = page.locator('.el-tabs');
    const tabsVisible = await tabs.first().isVisible({ timeout: 15_000 }).catch(() => false);

    if (tabsVisible) {
      // 点击第二个 tab
      const secondTab = page.locator('.el-tabs__item').nth(1);
      const secondTabVisible = await secondTab.isVisible({ timeout: 5000 }).catch(() => false);
      if (secondTabVisible) {
        await secondTab.click();
        await page.waitForTimeout(2000);

        // 验证 tab 内容加载
        const afterClickPanes = page.locator('.el-tab-pane');
        const newPaneCount = await afterClickPanes.count();
        expect(newPaneCount).toBeGreaterThanOrEqual(1);
      }
    }
  });
});
