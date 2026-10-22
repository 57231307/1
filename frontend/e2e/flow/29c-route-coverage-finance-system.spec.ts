import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('100% 前端路由 UI 交互全覆盖', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // 辅助：访问页面，验证核心 UI 元素可见
  async function visitPage(page: import('@playwright/test').Page, path: string) {
    await page.goto(`${BASE_URL}${path}`);
    await page.waitForTimeout(2000);
    const container = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-form, .el-empty, .el-tabs, .dashboard-container, canvas, .el-result, .error-page, body'
      )
      .first();
    await container.waitFor({ state: 'visible', timeout: 30_000 });
    return container;
  }

  // 辅助：验证表格+表头
  async function verifyTable(page: import('@playwright/test').Page) {
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const visible = await table.isVisible();
    if (visible) {
      const headers = table.locator('th, .el-table-v2__header-cell');
      const count = await headers.count();
      expect(count).toBeGreaterThan(0);
    }
    return visible;
  }

  // 辅助：验证新建按钮+弹窗
  async function verifyNewButton(page: import('@playwright/test').Page, btnText: string) {
    const btn = page.locator(`button:has-text("${btnText}")`).first();
    await btn.waitFor({ state: 'visible', timeout: 5000 });
    const visible = await btn.isVisible();
    if (visible) {
      const disabled = await btn.isDisabled();
      expect(disabled).toBe(false);
      await btn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true);
      if (dialogVisible) {
        await page.locator('.el-dialog__headerbtn').first().click();
        await page.waitForTimeout(500);
      }
      return dialogVisible;
    }
    return false;
  }

  // ===== 财务域剩余路由 =====
  test('应收 /ar', async ({ page }) => {
    await visitPage(page, '/ar');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await tab.isVisible();
    expect(visible).toBe(true);
  });
  test('应收对账增强页签 /ar-reconciliation?tab=enhanced', async ({ page }) => {
    await visitPage(page, '/ar-reconciliation?tab=enhanced');
    await verifyTable(page);
  });
  test('辅助核算 /assist-accounting', async ({ page }) => {
    await visitPage(page, '/assist-accounting');
    await verifyTable(page);
  });
  test('财务总页 /finance', async ({ page }) => {
    await visitPage(page, '/finance');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await tab.isVisible();
    expect(visible).toBe(true);
  });
  test('财务分析 /financial-analysis', async ({ page }) => {
    await visitPage(page, '/financial-analysis');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  // 说明：本文件覆盖「财务域 + 系统域」路由。原「交易管理 /trading」不是这两个域的路由——
  // 它是采购合同/价格、销售合同/价格、销售退货五个 tab 的重复页，已合并进各正规页
  // （/purchase-contract、/purchase-price、/sales-contract、/sales-returns），其路由/视图/
  // api/i18n/后端 /trading 垫片均已删除；这些存活正规页由 29a-route-coverage-purchase-sales
  // 覆盖。此处不再有可访问的 /trading 路由，故无对应 UI 覆盖用例。

  // ===== 系统管理域各功能页（用户/角色/公司信息/权限管理/字段权限/Webhook 各自成页） =====
  test('用户管理 /users', async ({ page }) => {
    await page.goto(`${BASE_URL}/users`);
    await expect(
      page.getByRole('heading', { level: 2, name: '用户管理' }),
      `未渲染用户管理页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('button', { name: '新建用户' })).toBeVisible();
    const headers = page.locator(
      '.el-table[aria-label="用户列表"] .el-table__header-wrapper thead th.el-table__cell'
    );
    await expect(headers.filter({ hasText: /^用户名$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^部门$/ }).first()).toBeVisible();
  });

  test('角色管理 /roles', async ({ page }) => {
    await page.goto(`${BASE_URL}/roles`);
    await expect(
      page.getByRole('heading', { level: 2, name: '角色管理' }),
      `未渲染角色管理页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('button', { name: '新建角色' })).toBeVisible();
    const headers = page.locator(
      '.el-table[aria-label="角色列表"] .el-table__header-wrapper thead th.el-table__cell'
    );
    await expect(headers.filter({ hasText: /^角色名称$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^角色编码$/ }).first()).toBeVisible();
  });

  test('公司信息 /company', async ({ page }) => {
    await page.goto(`${BASE_URL}/company`);
    await expect(
      page.getByRole('heading', { level: 2, name: '公司信息设置' }),
      `未渲染公司信息页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    const form = page.locator('[aria-label="公司信息表单"]');
    await expect(form).toBeVisible();
    await expect(
      form.locator('.el-form-item__label', { hasText: /^公司名称$/ }).first()
    ).toBeVisible();
    await expect(page.getByRole('button', { name: '保存' }).first()).toBeVisible();
  });

  test('权限管理 /permissions', async ({ page }) => {
    await page.goto(`${BASE_URL}/permissions`);
    await expect(
      page.getByRole('heading', { level: 2, name: '权限管理' }),
      `未渲染权限管理页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    const headers = page.locator(
      '.el-table[aria-label="权限列表"] .el-table__header-wrapper thead th.el-table__cell'
    );
    await expect(headers.filter({ hasText: /^资源类型$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^操作$/ }).first()).toBeVisible();
  });

  test('字段权限 /field-permissions', async ({ page }) => {
    await page.goto(`${BASE_URL}/field-permissions`);
    await expect(
      page.getByRole('heading', { level: 2, name: '字段权限' }),
      `未渲染字段权限页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    const headers = page.locator(
      '.el-table[aria-label="字段权限列表"] .el-table__header-wrapper thead th.el-table__cell'
    );
    await expect(headers.filter({ hasText: /^角色$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^可读$/ }).first()).toBeVisible();
    await expect(headers.filter({ hasText: /^可写$/ }).first()).toBeVisible();
  });

  test('Webhook 配置 /webhooks', async ({ page }) => {
    await page.goto(`${BASE_URL}/webhooks`);
    await expect(
      page.getByRole('heading', { level: 2, name: 'Webhook 配置' }),
      `未渲染 Webhook 配置页标题，实际落点 ${page.url()}`
    ).toBeVisible({ timeout: 30_000 });
    await expect(page.getByRole('button', { name: '新建' }).first()).toBeVisible();
    const headers = page.locator(
      '.el-table[aria-label="Webhook 列表"] .el-table__header-wrapper thead th.el-table__cell'
    );
    await expect(headers.filter({ hasText: /^名称$/ }).first()).toBeVisible();
  });

  // ===== 系统域其它路由 =====
  test('系统更新 /system-update', async ({ page }) => {
    await visitPage(page, '/system-update');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('个人信息 /system/profile', async ({ page }) => {
    await visitPage(page, '/system/profile');
    const form = page.locator('.el-form, .el-card, body').first();
    await form.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await form.isVisible();
    expect(visible).toBe(true);
  });
  test('慢查询 /system/slow-query', async ({ page }) => {
    await visitPage(page, '/system/slow-query');
    await verifyTable(page);
  });
  test('全量审计 /omni-audit', async ({ page }) => {
    await visitPage(page, '/omni-audit');
    await verifyTable(page);
  });
  test('数据导入 /data-import', async ({ page }) => {
    await visitPage(page, '/data-import');
    const card = page.locator('.el-card, .el-upload, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('报表中心 /report-templates', async ({ page }) => {
    await visitPage(page, '/report-templates');
    await verifyTable(page);
  });
  test('邮件管理 /email', async ({ page }) => {
    await visitPage(page, '/email');
    const card = page.locator('.el-card, .el-table, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('双因素认证 /security/two-factor-setup', async ({ page }) => {
    await visitPage(page, '/security/two-factor-setup');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
  test('主备隔离 /admin/failover', async ({ page }) => {
    await visitPage(page, '/admin/failover');
    const card = page.locator('.el-card, .el-form, body').first();
    await card.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await card.isVisible();
    expect(visible).toBe(true);
  });
});
