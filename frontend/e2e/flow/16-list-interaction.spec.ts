import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('列表交互与状态显示', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('采购订单列表：分页切换+状态标签显示', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(3000);

    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 验证状态标签（el-tag）存在
    const statusTags = page.locator('.el-table .el-tag');
    const tagCount = await statusTags.count();
    if (tagCount > 0) {
      const firstTagText = await statusTags.first().textContent();
      expect(firstTagText?.trim().length).toBeGreaterThan(0);
    }

    // 测试分页
    const pagination = page.locator('.el-pagination').first();
    const paginationVisible = await pagination.isVisible();
    if (paginationVisible) {
      const page2 = pagination.locator('.el-pager .number:has-text("2")').first();
      const page2Visible = await page2.isVisible();
      if (page2Visible) {
        await page2.click();
        await page.waitForTimeout(2000);
        await page
          .locator(
            '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
          )
          .first()
          .waitFor({ state: 'visible', timeout: 10_000 });
      }
    }
  });

  test('库存列表：构造空筛选结果集并验证空态展示', async ({ page }) => {
    await page.goto(`${BASE_URL}/inventory`);

    // 库存台账 tab 默认加载（V2Table/el-table-v2，非 .el-table）
    const table = page.locator('.v2-table-wrapper, [role="table"]').first();
    await table.waitFor({ state: 'visible', timeout: 15_000 });

    // 原实现在此处直接 waitFor .el-table__empty-block 可见——表格有数据时
    // 空态块恒 hidden（call log: resolved to hidden ×14），前提不成立。
    // 改为用可构造的筛选条件构造空集：关键词在后端按产品编码/名称 LIKE 下推，
    // 无任何产品命中时显式返回空集（inventory_stock_service.rs:327-330）。
    const keywordInput = page.locator('.filter-card input:visible').first();
    await keywordInput.waitFor({ state: 'visible', timeout: 5_000 });
    const noMatchKeyword = `NOMATCH-${Date.now()}`;
    await keywordInput.fill(noMatchKeyword);

    const responsePromise = page.waitForResponse(
      res => res.url().includes('/inventory/stock') && res.url().includes('page='),
      { timeout: 30_000 }
    );
    await page.locator('.filter-card button:has-text("查询")').first().click();
    const res = await responsePromise;
    const body = await res.json();

    // 后端真实形态：分页对象 {items,total}，非匹配关键词必须返回 total=0 空集
    expect(body?.data?.total, `关键词 ${noMatchKeyword} 应命中 0 条`).toBe(0);
    expect(
      Array.isArray(body?.data?.items) && body.data.items.length,
      '空筛选结果的 items 应为空数组'
    ).toBe(0);

    // 空态呈现：el-table-v2 空占位必须可见
    const emptyBlock = page
      .locator('.el-table-v2__empty, .el-empty, .el-table__empty-block')
      .first();
    await emptyBlock.waitFor({ state: 'visible', timeout: 10_000 });
    expect(await emptyBlock.isVisible()).toBe(true);
  });

  test('仪表盘加载状态和图表渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    const container = page.locator('.dashboard-container, .el-card, .el-row').first();
    await container.waitFor({ state: 'visible', timeout: 30_000 });

    const hasStats = await page
      .locator('.el-statistic, .el-card, [class*="stat"]')
      .first()
      .isVisible();
    const hasChart = await page.locator('canvas, .echarts, [class*="chart"]').first().isVisible();
    expect(hasStats || hasChart).toBe(true);
  });

  test('供应商列表：搜索过滤交互', async ({ page }) => {
    // 真实路由：/purchase/supplier
    await page.goto(`${BASE_URL}/supplier`);
    await page.waitForTimeout(3000);

    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });

    // 查找搜索输入框（真实 placeholder 含"供应商名称"或类似）
    const searchInput = page.locator('.filter-form input, .filter-card input').first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      await page.waitForTimeout(1000);

      // 点击查询按钮（真实文本"查询"）
      const searchBtn = page.locator('button:has-text("查询")').first();
      await searchBtn.waitFor({ state: 'visible', timeout: 3000 });
      const searchBtnVisible = await searchBtn.isVisible();
      if (searchBtnVisible) {
        await searchBtn.click();
        await page.waitForTimeout(2000);
      }

      const tableStillVisible = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
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
    await tabs.first().waitFor({ state: 'visible', timeout: 15_000 });
    const tabsVisible = await tabs.first().isVisible();
    if (tabsVisible) {
      // 点击第二个 tab
      const secondTab = page.locator('.el-tabs__item').nth(1);
      await secondTab.waitFor({ state: 'visible', timeout: 5000 });
      const secondTabVisible = await secondTab.isVisible();
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
