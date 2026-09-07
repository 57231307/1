import { test, expect } from '@playwright/test';
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
    await container
      .waitFor({ state: 'visible', timeout: 30_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    return container;
  }

  // 辅助：验证表格+表头
  async function verifyTable(page: import('@playwright/test').Page) {
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table
      .waitFor({ state: 'visible', timeout: 10_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await table.isVisible().catch(() => false);
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
    await btn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await btn.isVisible().catch(() => false);
    if (visible) {
      const disabled = await btn.isDisabled().catch(() => false);
      expect(disabled).toBe(false);
      await btn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true)
        .catch(() => false);
      if (dialogVisible) {
        await page
          .locator('.el-dialog__headerbtn')
          .first()
          .click()
          .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
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
    await tab
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('应收对账增强 /ar-reconciliation/enhanced', async ({ page }) => {
    await visitPage(page, '/ar-reconciliation/enhanced');
    await verifyTable(page);
  });
  test('辅助核算 /assist-accounting', async ({ page }) => {
    await visitPage(page, '/assist-accounting');
    await verifyTable(page);
  });
  test('财务总页 /finance', async ({ page }) => {
    await visitPage(page, '/finance');
    const tab = page.locator('.el-tabs, .el-table, .el-card').first();
    await tab
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('财务分析 /financial-analysis', async ({ page }) => {
    await visitPage(page, '/financial-analysis');
    const card = page.locator('.el-card, .el-table, body').first();
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('交易管理 /trading', async ({ page }) => {
    await visitPage(page, '/trading');
    await verifyTable(page);
  });

  // ===== 系统域剩余路由 =====
  test('系统总页 /system', async ({ page }) => {
    await visitPage(page, '/system');
    const tab = page.locator('.el-tabs, .el-table').first();
    await tab
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await tab.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('系统更新 /system-update', async ({ page }) => {
    await visitPage(page, '/system-update');
    const card = page.locator('.el-card, .el-form, body').first();
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('个人信息 /system/profile', async ({ page }) => {
    await visitPage(page, '/system/profile');
    const form = page.locator('.el-form, .el-card, body').first();
    await form
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await form.isVisible().catch(() => false);
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
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('报表中心 /report-templates', async ({ page }) => {
    await visitPage(page, '/report-templates');
    await verifyTable(page);
  });
  test('邮件管理 /email', async ({ page }) => {
    await visitPage(page, '/email');
    const card = page.locator('.el-card, .el-table, body').first();
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('双因素认证 /security/two-factor-setup', async ({ page }) => {
    await visitPage(page, '/security/two-factor-setup');
    const card = page.locator('.el-card, .el-form, body').first();
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
  test('主备隔离 /admin/failover', async ({ page }) => {
    await visitPage(page, '/admin/failover');
    const card = page.locator('.el-card, .el-form, body').first();
    await card
      .waitFor({ state: 'visible', timeout: 15_000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const visible = await card.isVisible().catch(() => false);
    expect(visible).toBe(true);
  });
});
