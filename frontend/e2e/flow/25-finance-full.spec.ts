import { test, expect } from '../diagnose-fixture';
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

test.describe('财务模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('资金管理：账户+存取+冻结+转账+审批+报表+预测', async ({ page }) => {
    await verifyEndpointHealthy(page, '/fund-management/accounts?page=1&page_size=5');
    // accounts/by-type 契约（据代码订正判责报告）：报告所称「reports/by-type / report_type /
    // handlers/report_handler.rs:16 / models/report.rs ReportType 枚举」经全树 grep 均不存在
    // （backend 无该路由、无 report_handler.rs、无 enum ReportType）。file 25 中本行才是真实 400：
    // handler fund_management_handler.rs:448-456 AccountsByTypeQuery.account_type:String 必填
    // （非 Option/无 serde 默认），缺 → serde 400 missing field `account_type`；
    // service list_accounts_by_type(fund_management_service.rs:706-712) 按 AccountType+ACTIVE 过滤，
    // 空命中仍 200。按报告本意「取值必须在后端权威词表内」：account_type 权威词表
    // fund_management_service.rs:23-32（bank/cash/alipay/wechat），取 bank 为合法枚举项
    // （此为过滤入参，非伪造实体 id；与报告对 report_type 的处理口径一致）。
    await verifyEndpointHealthy(page, '/fund-management/accounts/by-type?account_type=bank');
    await verifyEndpointHealthy(page, '/fund-management/transfers?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/fund-management/transfers/pending');
    // 资金日报/月报（finance.rs:599/603 已注册）：handler 的 Query 结构体 date / year+month
    // 为必填(非 Option、无 serde default)，缺参会 400；报表为按日期区间聚合、空数据仍返回 200，
    // 故用合法当前日期作入参迁回严格(非实体 id 查询、不会 404)。
    const today = new Date();
    const fundDailyDate = today.toISOString().slice(0, 10);
    const fundYear = today.getFullYear();
    const fundMonth = today.getMonth() + 1;
    await verifyEndpointHealthy(page, `/fund-management/reports/daily?date=${fundDailyDate}`);
    await verifyEndpointHealthy(
      page,
      `/fund-management/reports/monthly?year=${fundYear}&month=${fundMonth}`
    );
    await verifyEndpointHealthy(page, '/fund-management/cash-flow-forecast');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/fund-management/accounts?page=1&page_size=1'
    );
    const acctId = list.items?.[0]?.id;
    if (acctId) {
      await apiCallRaw(page, 'GET', `/fund-management/accounts/${acctId}`);
      await safePostAction(page, `/fund-management/accounts/${acctId}/deposit`, { amount: '1000' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/withdraw`, { amount: '500' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/freeze`, { amount: '200' });
      await safePostAction(page, `/fund-management/accounts/${acctId}/unfreeze`, { amount: '200' });
    }
  });

  test('多币种+汇率+应收对账+财务分析+报表+AP/AR+预算+固定资产+科目', async ({ page }) => {
    // 币种
    await verifyEndpointHealthy(page, '/currencies');
    await verifyEndpointHealthy(page, '/currencies/base');
    await verifyEndpointHealthy(page, '/exchange-rates?page=1&page_size=5');
    // 应收对账
    await verifyEndpointHealthy(page, '/ar-reconciliations?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar-reconciliations-enhanced/aging-report');
    // 自动对账结果（finance.rs:1027 已注册）：list_results 的 Query 字段全 Option、page 默认 1，
    // admin 上下文应 2xx → 迁回严格。
    await verifyEndpointHealthy(page, '/ar-reconciliation-alias/auto-reconcile/results');
    // 财务分析
    // financial-analysis/reports（finance.rs:497，Query<Value> 空参默认分页）与
    // indicators（finance.rs:515，Query 的 page/page_size 为必填非 Option，缺参 400，故补合法分页）：迁回严格。
    await verifyEndpointHealthy(page, '/financial-analysis/reports');
    await verifyEndpointHealthy(page, '/financial-analysis/indicators?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/financial-analysis/dupont');
    // 报表
    await verifyEndpointHealthy(page, '/finance/reports/balance-sheet');
    await verifyEndpointHealthy(page, '/finance/reports/income-statement');
    await verifyEndpointHealthy(page, '/finance/reports/cash-flow');
    await verifyEndpointHealthy(page, '/finance/reports/trial-balance');
    // AP/AR
    await verifyEndpointHealthy(page, '/ap/invoices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ap/payments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar/invoices?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/ar/payments?page=1&page_size=5');
    // 预算
    await verifyEndpointHealthy(page, '/budgets?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/budgets/plans?page=1&page_size=5');
    // 预算执行预警（finance.rs:407 已注册）：budget_year 缺省时后端取当前年度，admin 应 2xx → 迁回严格。
    await verifyEndpointHealthy(page, '/budgets/execution-warnings');
    // 固定资产
    await verifyEndpointHealthy(page, '/fixed-assets?page=1&page_size=5');
    const faList = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/fixed-assets?page=1&page_size=1'
    );
    if (faList.items?.[0]?.id) {
      await apiCallRaw(page, 'GET', `/fixed-assets/${faList.items?.[0].id}`);
      await verifyEndpointHealthy(
        page,
        `/fixed-assets/${faList.items?.[0].id}/depreciation-records`
      );
      await safePostAction(page, `/fixed-assets/${faList.items?.[0].id}/depreciate`);
    }
    // 科目
    await verifyEndpointHealthy(page, '/subjects?page=1&page_size=50');
    // assist-accounting 域为 nest 挂载（routes/analytics.rs:596 .nest("/assist-accounting", …)），
    // 注册面只有子路径 dimensions/records/records/business/records/five-dimension/{id}/
    // summary/drill-down/balance/check-balance（analytics.rs:44-79，
    // route-snapshot.txt:219-226 逐一在册）；裸 "/assist-accounting" 根路径【本就无路由】，
    // 404 是注册面事实而非缺口——该域功能齐全（前端 api/assist-accounting.ts 也只消费子路径），
    // 旧裸前缀探针属路径写错，改钉真实子路径，不进任何豁免清单。
    // /records 契约：AssistRecordQueryParams 全 Option（assist_accounting_handler.rs:83-90），
    // page 缺省 1、page_size 缺省 20 并 clamp；service query_assist_records 无过滤即全表分页，
    // 空表仍 200（assist_accounting_service.rs:151-171，fetch_page 已做 1→0 对齐）。
    await verifyEndpointHealthy(page, '/assist-accounting/records?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/period-adjustments?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('固定资产列表 UI：搜索+新建弹窗+折旧按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/fixed-assets`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchBtn = page.locator('button:has-text("查询")').first();
    await searchBtn.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchBtn.isVisible();
    if (searchVisible) {
      await searchBtn.click();
      await page.waitForTimeout(2000);
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
      expect(tableOk).toBe(true);
    }
    // 新建资产
    const newBtn = page.locator('button:has-text("新建资产")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      // 验证表单字段
      const inputs = dialog.locator('.el-input, .el-input-number, .el-select');
      const inputCount = await inputs.count();
      expect(inputCount).toBeGreaterThan(0);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('预算列表 UI：搜索+新建弹窗', async ({ page }) => {
    await page.goto(`${BASE_URL}/budget`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建方案")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('资金管理 UI：新建账户+转账按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/fund`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建账户")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
    // 验证转账按钮：按钮在“转账记录”Tab 内（默认激活 account），需先切 Tab
    // 真实按钮文本是“新建转账”（fund.transferTab.buttonNewTransfer），非“账户转账”
    const transferTabItem = page.locator('.el-tabs__item:has-text("转账记录")').first();
    await transferTabItem.waitFor({ state: 'visible', timeout: 10_000 });
    await transferTabItem.click();
    // 等 Tab 面板激活（aria-selected 或按钮可见）
    const transferBtn = page.locator('button:has-text("新建转账")').first();
    await transferBtn.waitFor({ state: 'visible', timeout: 10_000 });
    const transferVisible = await transferBtn.isVisible();
    expect(transferVisible).toBe(true);
  });

  test('币种管理 UI：新建币种+新增汇率', async ({ page }) => {
    await page.goto(`${BASE_URL}/currency`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建币种")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('会计期间 UI：新建期间+初始化年度', async ({ page }) => {
    await page.goto(`${BASE_URL}/accounting-period`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建期间")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
    // 验证初始化年度按钮（dialog 关闭动画可能遮挡，给足 10s）
    const initBtn = page.locator('button:has-text("初始化年度")').first();
    await initBtn.waitFor({ state: 'visible', timeout: 10_000 });
    const initVisible = await initBtn.isVisible();
    expect(initVisible).toBe(true);
  });

  test('应收对账 UI：新建对账+搜索', async ({ page }) => {
    await page.goto(`${BASE_URL}/ar-reconciliation`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新增对账"), button:has-text("新建对账")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 10_000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      const dialog = page.locator('.el-dialog:visible').first();
      await dialog.waitFor({ state: 'visible', timeout: 10_000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('应付管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/ap`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // “生成对账”按钮在“对账管理”Tab 内（默认激活 invoice），需先切 Tab
    const recTabItem = page.locator('.el-tabs__item:has-text("对账管理")').first();
    await recTabItem.waitFor({ state: 'visible', timeout: 10_000 });
    await page.keyboard.press('Escape');
    await page.waitForTimeout(300);
    await recTabItem.click({ force: true });
    const genBtn = page.locator('button:has-text("生成对账")').first();
    await genBtn.waitFor({ state: 'visible', timeout: 10_000 });
    const genVisible = await genBtn.isVisible();
    expect(genVisible).toBe(true);
  });

  test('财务报表 UI：生成+导出', async ({ page }) => {
    await page.goto(`${BASE_URL}/finance-report`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-form')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证生成报表按钮
    const genBtn = page.locator('button:has-text("生成报表")').first();
    await genBtn.waitFor({ state: 'visible', timeout: 5000 });
    const genVisible = await genBtn.isVisible();
    // 验证导出按钮
    const exportBtn = page.locator('button:has-text("导出")').first();
    await exportBtn.waitFor({ state: 'visible', timeout: 5000 });
    const exportVisible = await exportBtn.isVisible();
    expect(genVisible || exportVisible).toBe(true);
  });
});
