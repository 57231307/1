// P0-5 面料多色号定价扩展 E2E 测试
// 创建时间: 2026-06-18
//
// v8 复审 P0-2 修复（2026-06-30）：
// 对齐批次 28 P0-1 fail-secure 模式，凭据从环境变量注入，禁止硬编码 admin/admin123。
//
// extras 分片修复（族 D）：
// 原 BASE_URL = process.env.BASE_URL || 'http://localhost:8080' 手写 login() 导航到 :8080/login——
// CI 前端由 playwright.config.ts webServer 起在 :3000（后端 :8082），:8080 无监听 →
// net::ERR_CONNECTION_REFUSED 全灭。改为复用 globalSetup 注入的真实登录态 storageState
// （playwright.config.ts use.storageState）+ 相对导航（baseURL 已是 :3000），
// 与同目录其他 spec 一致；不再自行拼绝对端口，也不重复登录。

import { test, expect } from './diagnose-fixture';

test.describe('面料多色号定价扩展', () => {
  test('1. 登录并访问色号价格列表', async ({ page }) => {
    await page.goto('/color-prices/list');
    await expect(page.getByRole('heading', { name: /色号价格/ })).toBeVisible({
      timeout: 30000,
    });
  });

  test('2. 详情页查看历史图表', async ({ page }) => {
    await page.goto('/color-prices/list');
    await page.waitForLoadState('networkidle');
    // 点击第一个详情链接（列表页应渲染详情入口；无入口=列表为空或页面异常，必须失败暴露）
    const detailLink = page.locator('a:has-text("详情"), button:has-text("详情")').first();
    await expect(detailLink, '[color-prices] 详情链接应可见').toBeVisible({ timeout: 30000 });
    await detailLink.click();
    await page.waitForLoadState('networkidle');
  });

  test('3. 批量调价页面加载', async ({ page }) => {
    await page.goto('/color-prices/batch');
    await page.waitForLoadState('networkidle');
  });
});
