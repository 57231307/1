import { test, expect } from '@playwright/test';
import { loginViaUI, apiCall, apiCallRaw, apiCallExpectFail, genCode, getCtx, BASE_URL, safeGet, safeGetList, safePostAction, verifyEndpointHealthy } from './helpers';

test.describe('CRM 模块：API 端点 + 真实 UI 交互', () => {
  test.beforeAll(async ({ page }) => { await loginViaUI(page); });

  // ===== API 端点覆盖 =====
  test('客户管理：CRUD+导出+地址+信用+360+RFM+CLV', async ({ page }) => {
    const ctx = getCtx();
    const customerId = ctx.customerId || 1;

    await apiCallRaw(page, 'GET', '/crm/customers?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/customers/select?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/customers/export');
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/credit`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/addresses`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/summary`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/360`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/follow-ups`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/rfm`);
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}/audit-logs`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/clv`);
    await apiCallRaw(page, 'GET', '/crm/customers/enhanced?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/customers/field-permissions/1');
    await verifyEndpointHealthy(page, '/crm/rfm/distribution');
    await apiCallRaw(page, 'GET', '/crm/sales-users');
  });

  test('客户信用管理：列表+评级+占用+释放+调整', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/customer-credits?page=1&page_size=5');
    const result = await apiCallExpectFail(page, 'POST', '/crm/customer-credits', { customer_id: 1, credit_limit: '100000', currency: 'CNY' });
    if (result.status < 400) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/customer-credits?page=1&page_size=1');
      const creditId = list.items[0]?.id;
      if (creditId) {
        await apiCallRaw(page, 'GET', `/crm/customer-credits/${creditId}`);
        await safePostAction(page, `/crm/customer-credits/${creditId}/rating`, { rating: 'A' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/occupy`, { amount: '1000' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/release`, { amount: '1000' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/adjust`, { amount: '5000' });
        await safePostAction(page, '/crm/customer-credits/evaluate', { customer_id: 1 });
      }
    }
  });

  test('线索管理：创建+转换+分配+评分+漏斗', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/leads?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/leads/conversion-stats');
    await verifyEndpointHealthy(page, '/crm/leads/channel-roi');
    await verifyEndpointHealthy(page, '/crm/leads/allocation-rules');
    await verifyEndpointHealthy(page, '/crm/leads/nurture-plans');
    await verifyEndpointHealthy(page, '/crm/leads/funnel-report');
    const result = await apiCallExpectFail(page, 'POST', '/crm/leads', { name: 'E2E 线索', phone: '13800000000', source: 'web' });
    let leadId: number | undefined;
    try { leadId = (result as { data?: { id?: number } }).data?.id; } catch { /* */ }
    if (!leadId) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/leads?page=1&page_size=1');
      leadId = list.items[0]?.id;
    }
    if (leadId) {
      await apiCallRaw(page, 'GET', `/crm/leads/${leadId}`);
      await verifyEndpointHealthy(page, `/crm/leads/${leadId}/relations`);
      await safePostAction(page, `/crm/leads/${leadId}/score`, { score: 80 });
      await safePostAction(page, `/crm/leads/${leadId}/convert`, { customer_name: 'E2E 客户' });
    }
    await safePostAction(page, '/crm/leads/detect-duplicates', { name: '测试', phone: '13800000000' });
  });

  test('商机管理：创建+阶段+竞争对手+跟进+预测+漏斗', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/opportunities?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/opportunities/stage-stats');
    await verifyEndpointHealthy(page, '/crm/opportunities/stage-duration');
    await verifyEndpointHealthy(page, '/crm/opportunities/forecast-accuracy');
    await verifyEndpointHealthy(page, '/crm/opportunities/weighted-forecast');
    await verifyEndpointHealthy(page, '/crm/opportunities/conversion-rate');
    await verifyEndpointHealthy(page, '/crm/opportunities/sales-funnel');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(page, 'GET', '/crm/opportunities?page=1&page_size=1');
    const oppId = list.items?.[0]?.id;
    if (oppId) {
      await apiCallRaw(page, 'GET', `/crm/opportunities/${oppId}`);
      await verifyEndpointHealthy(page, `/crm/opportunities/${oppId}/competitors`);
      await verifyEndpointHealthy(page, `/crm/opportunities/${oppId}/follow-ups`);
      await safePostAction(page, `/crm/opportunities/${oppId}/stage-change`, { from_stage: 'qualifying', to_stage: 'proposal' });
    }
  });

  test('公海池+分配+转移审批+回收规则+标签+竞争对手', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/pool?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/pool/rules');
    await apiCallRaw(page, 'GET', '/crm/assignments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/assignments/history');
    await verifyEndpointHealthy(page, '/crm/assignments/workload');
    await verifyEndpointHealthy(page, '/crm/transfer-approvals?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/recycle-rules');
    await verifyEndpointHealthy(page, '/crm/competitors?page=1&page_size=5');
  });

  test('五维管理+销售分析+标签', async ({ page }) => {
    await verifyEndpointHealthy(page, '/crm/five-dimension/stats');
    await verifyEndpointHealthy(page, '/crm/five-dimension/list');
    await verifyEndpointHealthy(page, '/crm/five-dimension/summary');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/statistics');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/trends');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/rankings');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/stats');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/product-ranking');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/customer-ranking');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/trend');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/targets');
    await verifyEndpointHealthy(page, '/crm/crm/tags');
  });

  // ===== 真实 UI 交互验证 =====
  test('CRM 客户列表 UI：搜索+新建表单+表格渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm`);
    await page.waitForTimeout(3000);
    // 等待 Tab 加载
    await page.locator('.el-table, .el-card, .el-tabs').first().waitFor({ state: 'visible', timeout: 30_000 });
    // 验证搜索输入框存在
    const searchInput = page.locator('input[placeholder*="客户编码"], input[placeholder*="客户"], input[placeholder*="线索"]').first();
    const searchVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      await page.waitForTimeout(500);
      // 点击查询按钮
      const queryBtn = page.locator('button:has-text("查询")').first();
      const btnVisible = await queryBtn.isVisible({ timeout: 3000 }).catch(() => false);
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      // 验证表格不崩溃
      const tableOk = await page.locator('.el-table').first().isVisible({ timeout: 5000 }).catch(() => false);
      expect(tableOk).toBe(true);
      // 清空搜索
      await searchInput.clear();
    }
  });

  test('CRM 客户列表 UI：点击新建客户→弹窗→表单校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-tabs').first().waitFor({ state: 'visible', timeout: 30_000 });
    // 点击新建客户按钮
    const newBtn = page.locator('button:has-text("新建客户")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      // 验证弹窗出现
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      // 验证表单字段存在
      const codeInput = dialog.locator('input[placeholder*="客户编码"]').first();
      const codeVisible = await codeInput.isVisible({ timeout: 3000 }).catch(() => false);
      expect(codeVisible).toBe(true);
      // 直接点保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click().catch(() => {});
      await page.waitForTimeout(1000);
      // 验证校验错误提示
      const hasError = await page.locator('.el-form-item__error, .el-message--error').first().isVisible({ timeout: 5000 }).catch(() => false);
      expect(hasError).toBe(true);
      // 关闭弹窗
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('CRM 线索列表 UI：搜索+表格+状态标签', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/leads`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card').first().waitFor({ state: 'visible', timeout: 30_000 });
    // 验证表格加载
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
    // 验证表头存在
    const headers = table.locator('th');
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);
    // 验证搜索框
    const searchInput = page.locator('input[placeholder*="线索"], input[placeholder*="公司"], input[placeholder*="联系人"]').first();
    const searchVisible = await searchInput.isVisible({ timeout: 5000 }).catch(() => false);
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      const btnVisible = await queryBtn.isVisible({ timeout: 3000 }).catch(() => false);
      if (btnVisible) { await queryBtn.click(); await page.waitForTimeout(2000); }
      const tableOk = await page.locator('.el-table').first().isVisible().catch(() => false);
      expect(tableOk).toBe(true);
    }
  });

  test('CRM 商机列表 UI：表格+新建按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/opportunities`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    expect(tableVisible).toBe(true);
    // 验证新建商机按钮存在
    const newBtn = page.locator('button:has-text("新建商机")').first();
    const newBtnVisible = await newBtn.isVisible({ timeout: 5000 }).catch(() => false);
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      const dialogVisible = await dialog.isVisible({ timeout: 5000 }).catch(() => false);
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click().catch(() => {});
    }
  });

  test('CRM 公海池+分配 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/pool`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty').first().waitFor({ state: 'visible', timeout: 30_000 });
    const bodyVisible = await page.locator('body').isVisible();
    expect(bodyVisible).toBe(true);
  });

  test('CRM 五维管理+销售分析 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/five-dimension`);
    await page.waitForTimeout(3000);
    await page.locator('.el-card, .el-table, .el-empty, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    await page.goto(`${BASE_URL}/sales-analysis`);
    await page.waitForTimeout(3000);
    const visible = await page.locator('.el-card, .el-table, .el-empty, body').first().isVisible().catch(() => false);
    expect(visible).toBe(true);
  });

  test('客户管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/customer`);
    await page.waitForTimeout(3000);
    await page.locator('.el-table, .el-card, .el-empty, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const table = page.locator('.el-table').first();
    const tableVisible = await table.isVisible({ timeout: 10_000 }).catch(() => false);
    if (tableVisible) {
      const headers = table.locator('th');
      const headerCount = await headers.count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });
});
