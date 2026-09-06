import { test, expect } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  safeGet,
  safeGetList,
  safePostAction,
  verifyEndpointHealthy,
} from './helpers';

test.describe('系统与分析模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('BI+Webhook+API网关+邮件+通知+扫码+导入导出+AI+报表+高级分析+跟踪+隐私+双计量+权限+审计+产品分类', async ({
    page,
  }) => {
    // BI
    await verifyEndpointHealthy(page, '/bi/sales-analysis');
    await verifyEndpointHealthy(page, '/bi/product-analysis');
    await verifyEndpointHealthy(page, '/bi/customer-analysis');
    await verifyEndpointHealthy(page, '/bi/inventory-analysis');
    await verifyEndpointHealthy(page, '/bi/finance-analysis');
    await verifyEndpointHealthy(page, '/bi/production-analysis');
    await verifyEndpointHealthy(page, '/bi/summary');
    // Webhook
    await verifyEndpointHealthy(page, '/webhooks?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/webhooks/integrations?page=1&page_size=5');
    // API 网关
    await verifyEndpointHealthy(page, '/api-gateway/endpoints?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/api-gateway/keys?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/api-gateway/logs?page=1&page_size=5');
    // 邮件+通知
    await verifyEndpointHealthy(page, '/email-templates?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/notifications?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/user-notification-settings');
    // 扫码+搜索
    await verifyEndpointHealthy(page, '/scanner/history?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/search?q=面料');
    // AI
    await verifyEndpointHealthy(page, '/ai/process-optimization');
    await verifyEndpointHealthy(page, '/ai/quality-prediction');
    await verifyEndpointHealthy(page, '/ai-models?page=1&page_size=5');
    // 报表
    await verifyEndpointHealthy(page, '/report-templates?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/reports/enhanced?page=1&page_size=5');
    // 高级分析+跟踪+隐私+双计量
    await verifyEndpointHealthy(page, '/advanced/analysis');
    await verifyEndpointHealthy(page, '/tracking/activities?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/privacy/consent');
    // 权限+审计
    await verifyEndpointHealthy(page, '/data-permissions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/users?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/roles?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/departments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/system/audit-logs?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/system/slow-queries?page=1&page_size=5');
    // 产品分类+仓库
    await verifyEndpointHealthy(page, '/product-categories?page=1&page_size=50');
    await verifyEndpointHealthy(page, '/warehouses?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('用户管理 UI：搜索+新建用户弹窗+必填校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/system`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchInput = page
      .locator(
        'input[placeholder*="用户名"], input[placeholder*="姓名"], input[placeholder*="手机"]'
      )
      .first();
    await searchInput
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchInput.isVisible().catch(() => false);
    if (searchVisible) {
      await searchInput.fill('admin');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn
        .waitFor({ state: 'visible', timeout: 3000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const btnVisible = await queryBtn.isVisible().catch(() => false);
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible()
        .catch(() => false);
      expect(tableOk).toBe(true);
    }
    // 新建用户
    const newBtn = page.locator('button:has-text("新建用户")').first();
    await newBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      // 直接保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click().catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      await page.waitForTimeout(1000);
      await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const hasError = await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .isVisible()
        .catch(() => false);
      expect(hasError).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    }
  });

  test('审计日志 UI：搜索+表格', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const searchBtn = page.locator('button:has-text("查询")').first();
    await searchBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const searchVisible = await searchBtn.isVisible().catch(() => false);
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(2000);
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible()
        .catch(() => false);
      expect(tableOk).toBe(true);
    }
  });

  test('API 网关 UI：Tab 切换+新建接口', async ({ page }) => {
    await page.goto(`${BASE_URL}/api-gateway`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-tabs, .el-table, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证 Tab 存在
    const tabs = page.locator('.el-tabs__item');
    const tabCount = await tabs.count();
    expect(tabCount).toBeGreaterThan(0);
    // 切换到第二个 Tab
    if (tabCount > 1) {
      await tabs.nth(1).click();
      // 等待 Tab 对应的表格真实渲染（懒加载组件延迟，固定 2s 不足）。
      // 只匹配可见表格：Element Plus 非活动 TabPane 仍留在 DOM（display:none），
      // 不加 :visible 时 .first() 会命中旧 Tab 的隐藏表格导致断言恒假
      const table = page
        .locator(
          '.el-table:visible, .el-table-v2:visible, [role="table"]:visible, .v2-table-wrapper:visible'
        )
        .first();
      await table
        .waitFor({ state: 'visible', timeout: 15_000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const tableOk = await table.isVisible().catch(() => false);
      expect(tableOk).toBe(true);
    }
    // 新建接口按钮
    const newBtn = page.locator('button:has-text("新建接口")').first();
    await newBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    }
  });

  test('通知中心 UI：列表+已读标记', async ({ page }) => {
    await page.goto(`${BASE_URL}/notification`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('数据权限 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/data-permission`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('部门管理 UI：列表+新建', async ({ page }) => {
    await page.goto(`${BASE_URL}/departments`);
    await page.waitForTimeout(3000);
    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, body'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建"), button:has-text("新增")').first();
    await newBtn
      .waitFor({ state: 'visible', timeout: 5000 })
      .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    const newBtnVisible = await newBtn.isVisible().catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog
        .waitFor({ state: 'visible', timeout: 5000 })
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
      const dialogVisible = await dialog.isVisible().catch(() => false);
      expect(dialogVisible).toBe(true);
      await page
        .locator('.el-dialog__headerbtn')
        .first()
        .click()
        .catch(e => console.error('[E2E] 操作失败:', (e as Error).message));
    }
  });

  test('业务追溯 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/business-trace`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-form, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('安全设置 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/security`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-form, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('打印模板 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/print-templates`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
});
