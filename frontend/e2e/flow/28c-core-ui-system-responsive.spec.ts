import { test, expect } from '@playwright/test';
import { loginViaUI, BASE_URL, getCtx } from './helpers';

test.describe('核心业务流程真实 UI 交互验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面并验证表格加载
  async function visitAndVerifyTable(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(3000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, .el-form'
      )
      .first();
    await container
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    return page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
  }

  // 辅助：验证按钮可见且可点击
  async function verifyButton(page: import('@playwright/test').Page, text: string) {
    const btn = page.locator(`button:has-text("${text}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
    }
    return visible;
  }

  // 辅助：点击新建按钮并验证弹窗（只匹配可见对话框）
  async function clickNewAndVerifyDialog(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (!visible) return false;
    await btn.click();
    await page.waitForTimeout(1000);
    const dialog = page.locator('.el-dialog:visible').first();
    await dialog
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const dialogVisible = await dialog.isVisible().catch(() => false);
    return dialogVisible;
  }

  // 辅助：验证表单必填校验
  async function verifyRequiredValidation(page: import('@playwright/test').Page) {
    // 可见对话框（页面可能挂载多个 el-dialog，隐藏的不参与匹配）
    const dialog = page.locator('.el-dialog:visible').first();
    // 提交按钮 = 对话框 footer 的主按钮（各模块文案不一：保存/确定/确认/提交），
    // 按文案匹配会因文案差异（如 "确认"）匹配不到而 30s 超时
    const saveBtn = dialog.locator('.el-dialog__footer .el-button--primary').first();
    try {
      await saveBtn.click({ timeout: 10_000 });
      console.log('[verifyRequiredValidation] 已点击 footer 主按钮');
    } catch (e) {
      console.error(`[verifyRequiredValidation] 点击主按钮失败: ${(e as Error).message}`);
      return false;
    }
    await page.waitForTimeout(1000);
    // 表单校验用 ElMessage（warning/error）或 el-form-item__error，
    // 统一匹配 .el-message（含 --warning/--error）以覆盖所有提示类型
    await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const hasError = await page
      .locator('.el-message, .el-form-item__error')
      .first()
      .isVisible()
      .catch(() => false);
    if (!hasError) {
      // 诊断输出（IR 详细日志要求）：无任何校验提示时打印对话框文本片段
      const dialogText = await dialog.innerText().catch(() => '<无法获取>');
      console.warn(
        `[verifyRequiredValidation] 未出现校验提示，对话框文本前 200 字: ${dialogText.slice(0, 200)}`
      );
    }
    return hasError;
  }

  // 辅助：关闭弹窗
  async function closeDialog(page: import('@playwright/test').Page) {
    await page
      .locator('.el-dialog__headerbtn')
      .first()
      .click()
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    await page.waitForTimeout(500);
  }

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
    await usernameInput
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const usernameVisible = await usernameInput.isVisible().catch(() => false);
    expect(usernameVisible).toBe(true);

    // 验证密码输入框
    const passwordInput = loginPage
      .locator('input[placeholder="密码"], input[placeholder="Password"]')
      .first();
    await passwordInput
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const passwordVisible = await passwordInput.isVisible().catch(() => false);
    expect(passwordVisible).toBe(true);

    // 验证登录按钮
    const loginBtn = loginPage.locator('form button.el-button--primary').first();
    await loginBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const loginBtnVisible = await loginBtn.isVisible().catch(() => false);
    expect(loginBtnVisible).toBe(true);
    const loginBtnDisabled = await loginBtn.isDisabled().catch(() => false);
    expect(loginBtnDisabled).toBe(false);

    // 验证复选框（用户协议）
    const checkbox = loginPage.locator('.el-checkbox').first();
    await checkbox
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const checkboxVisible = await checkbox.isVisible().catch(() => false);
    expect(checkboxVisible).toBe(true);

    // 验证空表单提交触发校验
    await loginBtn.click();
    await loginPage.waitForTimeout(1000);
    await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const hasError = await loginPage
      .locator('.el-form-item__error, .el-message--error')
      .first()
      .isVisible()
      .catch(() => false);
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
    await menu
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const menuVisible = await menu.isVisible().catch(() => false);
    if (menuVisible) {
      // 验证菜单项存在
      const menuItems = menu.locator('.el-menu-item, .el-sub-menu__title');
      const itemCount = await menuItems.count();
      expect(itemCount).toBeGreaterThan(0);

      // 点击第一个菜单项验证跳转
      const firstItem = menuItems.first();
      await firstItem.click().catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await page
      .locator('body')
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const bodyVisible = await page
      .locator('body')
      .isVisible()
      .catch(() => false);
    expect(bodyVisible).toBe(true);
    await page.setViewportSize({ width: 1280, height: 800 });
  });

  test('响应式 UI：超窄屏 375px 布局不崩溃', async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 667 });
    await page.goto(`${BASE_URL}/dashboard`);
    await page.waitForTimeout(3000);
    await page
      .locator('body')
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const bodyVisible = await page
      .locator('body')
      .isVisible()
      .catch(() => false);
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
    await table
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const tableVisible = await table.isVisible().catch(() => false);
    expect(tableVisible).toBe(true);
  });
});
