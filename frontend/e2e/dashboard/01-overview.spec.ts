// 仪表盘 E2E 测试
// 创建时间: 2026-08-19
// 覆盖范围：KPI 卡片加载 → 趋势图 → 库存分布饼图 → 动态活动表
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

test.describe('仪表盘', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('仪表盘 KPI 统计卡片正常加载', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page.locator('.el-card, .kpi-card, [class*="stat"]').first()).toBeVisible({
      timeout: 30000,
    });
  });

  test('仪表盘销售趋势图正常加载', async ({ page }) => {
    await page.goto('/dashboard');
    const chartContainer = page
      .locator('div')
      .filter({ has: page.locator('.echarts, .chart, svg') });
    await expect(chartContainer.first()).toBeVisible({ timeout: 30000 });
  });

  test('仪表盘最近活动表正常加载', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page.getByText(/最近活动|活动记录/)).toBeVisible({
      timeout: 30000,
    });
    const activityTable = page.locator('table, .el-table');
    await expect(activityTable).toBeVisible({ timeout: 30000 });
  });

  test('仪表盘日期筛选功能可用', async ({ page }) => {
    // 假绿清零（Tier A）：原 `if (await dateRange.isVisible()) { click; Escape }` 用
    // getByLabel(/日期/) 定位——Dashboard.vue 的 el-date-picker 没有 <label>「日期」，
    // 只有 startPlaceholder/endPlaceholder（且随 locale 变化），恒定位不到 → 零断言通过。
    // 日期筛选是仪表盘头部无条件渲染控件（Dashboard.vue:12 无 v-if），缺失即产品缺陷，必须红。
    // 改为硬断言：控件存在且可交互（点击后日期区间面板真实弹出）。纯 UI 控件，无需造数据。
    await page.goto('/dashboard');
    const dateFilter = page.locator('.dashboard-header .el-date-editor').first();
    await expect(dateFilter, '仪表盘头部日期筛选控件应存在且可见').toBeVisible({ timeout: 30000 });
    await dateFilter.click();
    await expect(
      page.locator('.el-date-range-picker, .el-picker-panel').first(),
      '点击日期筛选后日期区间选择面板应弹出'
    ).toBeVisible({ timeout: 30000 });
    await page.keyboard.press('Escape');
  });
});
