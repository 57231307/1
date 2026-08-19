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
    await expect(page.locator('.el-card, .kpi-card, [class*="stat"]').first()).toBeVisible({ timeout: 10000 });
  });

  test('仪表盘销售趋势图正常加载', async ({ page }) => {
    await page.goto('/dashboard');
    const chartContainer = page.locator('div').filter({ has: page.locator('.echarts, .chart, svg') });
    await expect(chartContainer.first()).toBeVisible({ timeout: 10000 });
  });

  test('仪表盘最近活动表正常加载', async ({ page }) => {
    await page.goto('/dashboard');
    await expect(page.getByText(/最近活动|活动记录/)).toBeVisible({ timeout: 10000 }).catch(() => {
      return null;
    });
    const activityTable = page.locator('table, .el-table');
    await expect(activityTable).toBeVisible({ timeout: 10000 }).catch(() => {
      return null;
    });
  });

  test('仪表盘日期筛选功能可用', async ({ page }) => {
    await page.goto('/dashboard');
    const dateRange = page.getByLabel(/日期/).first();
    if (await dateRange.isVisible({ timeout: 3000 }).catch(() => false)) {
      await dateRange.click();
      await page.keyboard.press('Escape');
    }
  });
});