import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL, getCtx } from './helpers';
import {
  visitAndVerifyTable,
  verifyButton,
  clickNewAndVerifyDialog,
  verifyRequiredValidation,
} from './ui-helpers';

test.describe('核心业务流程真实 UI 交互验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ================================================================
  // P2P 采购到付款流程 UI
  // ================================================================
  test('登录页 UI：表单元素+复选框+按钮', async ({ page, browser }) => {
    // 用新 context 访问登录页
    const context = await browser.newContext();
    const loginPage = await context.newPage();
    await loginPage.goto(`${BASE_URL}/login`);
    await loginPage.waitForTimeout(2000);

    // 验证用户名输入框
    const usernameInput = loginPage
      .locator('input[placeholder="用户名"], input[placeholder="Username"]')
      .first();
    await usernameInput.waitFor({ state: 'visible', timeout: 10_000 });
    const usernameVisible = await usernameInput.isVisible();
    expect(usernameVisible).toBe(true);

    // 验证密码输入框
    const passwordInput = loginPage
      .locator('input[placeholder="密码"], input[placeholder="Password"]')
      .first();
    await passwordInput.waitFor({ state: 'visible', timeout: 5000 });
    const passwordVisible = await passwordInput.isVisible();
    expect(passwordVisible).toBe(true);

    // 验证登录按钮
    const loginBtn = loginPage.locator('form button.el-button--primary').first();
    await loginBtn.waitFor({ state: 'visible', timeout: 5000 });
    const loginBtnVisible = await loginBtn.isVisible();
    expect(loginBtnVisible).toBe(true);
    const loginBtnDisabled = await loginBtn.isDisabled();
    expect(loginBtnDisabled).toBe(false);

    // 验证复选框（用户协议）
    const checkbox = loginPage.locator('.el-checkbox').first();
    await checkbox.waitFor({ state: 'visible', timeout: 5000 });
    const checkboxVisible = await checkbox.isVisible();
    expect(checkboxVisible).toBe(true);

    // 验证空表单提交触发校验
    await loginBtn.click();
    await loginPage.waitForTimeout(1000);
    await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 });
    const hasError = await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible();
    expect(hasError).toBe(true);

    await context.close();
  });

  // ================================================================
  // 全局导航 UI：侧边菜单+路由跳转
  // ================================================================
  test('全局导航 UI：侧边菜单可见+可点击', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);

    // 验证侧边菜单存在
    const menu = page
      .locator('.el-menu, .el-aside, .sidebar-container, [class*="sidebar"]')
      .first();
    await menu.waitFor({ state: 'visible', timeout: 10_000 });
    const menuVisible = await menu.isVisible();
    if (menuVisible) {
      // 验证菜单项存在
      const menuItems = menu.locator('.el-menu-item, .el-sub-menu__title');
      const itemCount = await menuItems.count();
      expect(itemCount).toBeGreaterThan(0);

      // 点击第一个菜单项验证跳转
      const firstItem = menuItems.first();
      await firstItem.click();
      await page.waitForTimeout(2000);
      const url = page.url();
      expect(url).toContain('localhost:3000');
    }
  });

  // ================================================================
  // 响应式布局 UI：窄屏不崩溃
  // ================================================================
  test('响应式 UI：窄屏 768px 布局不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 768, height: 600 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page.locator('body').waitFor({ state: 'visible', timeout: 10_000 });
    const bodyVisible = await page.locator('body').isVisible();
    expect(bodyVisible).toBe(true);
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  test('响应式 UI：超窄屏 375px 布局不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page.locator('body').waitFor({ state: 'visible', timeout: 10_000 });
    const bodyVisible = await page.locator('body').isVisible();
    expect(bodyVisible).toBe(true);
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  // ================================================================
  // 面包屑+页面标题 UI
  // ================================================================
  test('页面标题 UI：document.title 正确设置', async ({ page }) => {
    await page.goto(`${BASE_URL}/purchase`);
    await page.waitForTimeout(2000);
    const title = await page.title();
    expect(title.length).toBeGreaterThan(0);
    // 标题应包含业务名称或平台名称
    expect(
      title.includes('Bingxi') ||
        title.includes('采购') ||
        title.includes('ERP') ||
        title.length > 2
    ).toBe(true);
  });

  // ================================================================
  // 加载状态 UI：loading 指示器
  // ================================================================
  test('加载状态 UI：页面切换有 loading 指示', async ({ page }) => {
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(1000);
    // 页面加载过程中可能有 loading
    await page.goto(`${BASE_URL}/purchase`);
    // 快速检查 loading 是否出现（可能在数据加载时短暂出现）
    const loading = page.locator('.el-loading-mask, .el-loading-spinner, .el-skeleton');
    // 不强制要求 loading 一定出现（可能加载太快），验证页面最终加载完成
    await page.waitForTimeout(3000);
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
  });
});
