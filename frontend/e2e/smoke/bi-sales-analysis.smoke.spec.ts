import { test, expect } from '../diagnose-fixture';
import { gotoWithRetry } from './_goto';

test.describe('sales-analysis BI 页签冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    // BI 多维分析并入 /sales-analysis 第二页签，旧 /bi/sales-analysis 重定向到 ?tab=bi
    await gotoWithRetry(page, '/sales-analysis?tab=bi');
    await expect(page.locator('body')).toBeVisible({ timeout: 30_000 });
    await expect(
      page
        .locator(
          '.el-table, .el-table-v2, .el-card, .el-form, .el-tabs, .dashboard-container, canvas, .echarts, .el-result, .error-page, .el-empty'
        )
        .first()
    ).toBeAttached({ timeout: 30_000 });
  });
});
