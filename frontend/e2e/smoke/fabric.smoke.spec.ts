import { test, expect } from '../diagnose-fixture';
import { gotoWithRetry } from './_goto';

test.describe('greige-fabrics 页面冒烟测试', () => {
  test('页面加载', async ({ page }) => {
    // 面料域拆为三个独立页，坯布页为原 /fabric 的权威替代入口（/fabric 仅保留重定向）
    await gotoWithRetry(page, '/greige-fabrics');
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
