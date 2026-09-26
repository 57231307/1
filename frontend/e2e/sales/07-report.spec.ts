// P9-3 销售 E2E 套件 — 07 销售分析（真实 BI 视图）
// 覆盖范围：/sales-analysis 销售分析页统计/排名/目标渲染 + 导出报表

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 router 与 views/sales-analysis 核对）：
 * - 原用例 goto '/sales/report/order-summary'、'/sales/report/performance'、
 *   '/sales/report/collection-rate'、'/sales/report/gross-margin' —— 这些子路由与
 *   “统计维度/起始月份/截止月份/查询按钮 + 分组报表表格”在真实应用完全不存在（router 中无
 *   任何 /sales/report/* 路由）。
 * - 真实“销售统计/报表”视图为 /sales-analysis（router path:'sales-analysis' →
 *   views/sales-analysis/index.vue），页面为 BI 布局，真实可断言元素（locales salesAnalysis.*）：
 *   标题 '销售分析'（index.pageTitle）；统计卡 labelMonthOrders='本月订单数' / labelMonthAmount=
 *   '本月销售额' / labelGrossProfitRate='毛利率' / labelActiveCustomers='活跃客户数'；
 *   产品排名卡 cardTitle='产品销售排名'、客户排名卡 cardTitle='客户销售排名'；
 *   销售目标卡 cardTitle='销售目标'；导出按钮 index.buttonExport='导出报表'
 *   → useSaProc.handleExport 触发 blob 下载 '销售分析报表.xlsx' + msg.success('exportSuccess')。
 *   —— 本用例据真实视图重写，去除“按客户分组/回款率/毛利率列/导出行”等不存在的报表表格断言。
 */
test.describe('07 销售分析（真实 BI 视图）', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('07-01 销售分析页可访问并渲染统计卡', async ({ page }) => {
    await page.goto('/sales-analysis');
    await expect(page.getByText('销售分析').first()).toBeVisible();
    await expect(page.getByText('本月订单数')).toBeVisible();
    await expect(page.getByText('本月销售额')).toBeVisible();
    await expect(page.getByText('毛利率')).toBeVisible();
  });

  test('07-02 产品与客户排名卡片渲染', async ({ page }) => {
    await page.goto('/sales-analysis');
    await expect(page.getByText('产品销售排名')).toBeVisible();
    await expect(page.getByText('客户销售排名')).toBeVisible();
  });

  test('07-03 销售目标卡片渲染', async ({ page }) => {
    await page.goto('/sales-analysis');
    await expect(page.getByText('销售目标')).toBeVisible();
  });

  test('07-04 销售分析可导出报表 xlsx', async ({ page }) => {
    await page.goto('/sales-analysis');
    const downloadPromise = page.waitForEvent('download');
    await page.getByRole('button', { name: '导出报表' }).click();
    const download = await downloadPromise;
    expect(download.suggestedFilename()).toMatch(/\.xlsx$/);
  });
});
