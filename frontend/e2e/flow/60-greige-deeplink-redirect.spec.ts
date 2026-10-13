// 坯布深链重定向 E2E 锁定
// 覆盖 BUG-007，历史误链 /fabric/greige 应重定向到权威入口 /greige-fabrics，不再 404
import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('坯布深链重定向', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('访问 fabric/greige 重定向到独立坯布页', async ({ page }) => {
    await page.goto(`${BASE_URL}/fabric/greige`);
    // 重定向生效，地址落到权威入口而非 404
    await expect(page, 'URL 应重定向到 greige-fabrics').toHaveURL(/\/greige-fabrics$/);
    // 权威坯布页真实渲染（页面标题 h2）
    await expect(page.getByRole('heading', { name: '坯布管理' })).toBeVisible({ timeout: 30000 });
  });
});
