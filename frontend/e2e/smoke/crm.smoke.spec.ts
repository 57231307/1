import { test, expect } from '../diagnose-fixture';
import { gotoWithRetry } from './_goto';

test.describe('customer 页面冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    // 客户列表唯一入口 /customer（原 /crm 已合并为重定向）
    await gotoWithRetry(page, '/customer');
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
