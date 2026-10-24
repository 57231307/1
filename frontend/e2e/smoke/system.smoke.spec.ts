// system 域功能页冒烟测试
// 覆盖系统管理域拆分后的六个功能页各自的入口渲染，以及旧聚合地址 /system 的重定向落点。
import { test, expect } from '../diagnose-fixture';
import { gotoWithRetry } from './_goto';

// 列表页以页头标题（各页自身的 h2）加表格作为内容判据
const LIST_PAGES: Array<{ path: string; title: string }> = [
  { path: '/users', title: '用户管理' },
  { path: '/roles', title: '角色管理' },
  { path: '/permissions', title: '权限管理' },
  { path: '/field-permissions', title: '字段权限' },
  { path: '/webhooks', title: 'Webhook 配置' },
];

test.describe('system 页面冒烟测试', () => {
  for (const pageCase of LIST_PAGES) {
    test(`${pageCase.path} 页面加载`, async ({ page }) => {
      await gotoWithRetry(page, pageCase.path);
      await expect(page.locator('body')).toBeVisible({ timeout: 30_000 });
      await expect(page.getByRole('heading', { name: pageCase.title })).toBeVisible({
        timeout: 30_000,
      });
      await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30_000 });
    });
  }

  test('/company 页面加载', async ({ page }) => {
    // 公司信息页是表单页（无列表），以表单自身作为内容判据
    await gotoWithRetry(page, '/company');
    await expect(page.locator('body')).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('heading', { name: '公司信息设置' })).toBeVisible({
      timeout: 30_000,
    });
    await expect(page.getByLabel('公司名称')).toBeVisible({ timeout: 30_000 });
  });

  test('/system 旧地址重定向到用户管理页', async ({ page }) => {
    // /system 只保留重定向，旧地址必须可达且不落 404，落点是 /users
    await gotoWithRetry(page, '/system');
    await expect(page.locator('body')).toBeVisible({ timeout: 30_000 });
    await expect(page, '/system 应重定向到 /users').toHaveURL(/\/users$/);
    expect(new URL(page.url()).pathname, '重定向落点 pathname').toBe('/users');
    await expect(page.getByRole('heading', { name: '用户管理' })).toBeVisible({ timeout: 30_000 });
  });
});
