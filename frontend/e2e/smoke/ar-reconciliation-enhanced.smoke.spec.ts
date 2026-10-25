import { test, expect } from '../diagnose-fixture';
import { gotoWithRetry } from './_goto';

test.describe('ar-reconciliation 增强页签冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    // 增强对账并入 /ar-reconciliation 第二页签，旧 /ar-reconciliation/enhanced 重定向到 ?tab=enhanced
    await gotoWithRetry(page, '/ar-reconciliation?tab=enhanced');
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
