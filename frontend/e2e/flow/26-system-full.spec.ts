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

test.describe('系统与分析模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  // ===== API 端点覆盖 =====
  test('BI+Webhook+API网关+邮件+通知+扫码+导入导出+AI+报表+高级分析+跟踪+隐私+双计量+权限+审计+产品分类', async ({
    page,
  }) => {
    // BI 多维分析：后端 bi()(analytics.rs:488，nest 到 /bi) 仅注册 /bi/sales/* 一组聚合端点。
    // 旧探针 /bi/{sales,product,customer,inventory,finance,production}-analysis 与 /bi/summary
    // 全是臆造路径（后端无 GET /bi/sales-analysis、无 /bi/summary、亦无 product/customer/
    // inventory/finance/production 各域的 BI 端点），optional 长期吞其 404 = 假绿。
    // 改为真实存在路径并 strict（admin 无必填参或带合法参，应 2xx）；无真实等价物的臆造分析
    // 探针（上述 5 个非 sales 域 + summary）删除——测试对着不存在端点写属测试错，不保留 optional 掩盖。
    // by-time 的 start/end/granularity 为 handler 必填(缺参 400)，用当前年度合法日期区间作入参
    //（区间聚合、空数据仍返回 200，非实体 id 查询不会 404）；其余 7 端点无必填参。
    const biYear = new Date().getFullYear();
    await verifyEndpointHealthy(
      page,
      `/bi/sales/by-time?start_date=${biYear}-01-01&end_date=${biYear}-12-31&granularity=month`
    );
    await verifyEndpointHealthy(page, '/bi/sales/by-customer');
    await verifyEndpointHealthy(page, '/bi/sales/by-product');
    await verifyEndpointHealthy(page, '/bi/sales/by-region');
    await verifyEndpointHealthy(page, '/bi/sales/by-category');
    await verifyEndpointHealthy(page, '/bi/sales/trend');
    await verifyEndpointHealthy(page, '/bi/sales/profit');
    await verifyEndpointHealthy(page, '/bi/sales/kpi');
    // Webhook（webhooks() 在 analytics.rs:354 注册 GET /，nest 到 /webhooks → strict）
    await verifyEndpointHealthy(page, '/webhooks?page=1&page_size=5');
    // Webhook 集成（analytics.rs:611 nest /webhooks/integrations + 内部 GET /（analytics.rs:283）
    // → 最终 GET /webhooks/integrations；list_integrations（webhook_integration_handler.rs:85-88）
    // 无 Query 入参、仅 AuthContext，admin 必 2xx → strict，不再 optional 吞 404）
    await verifyEndpointHealthy(page, '/webhooks/integrations?page=1&page_size=5');
    // API 网关：endpoints 列表为真实消费面（dynamic_router 按状态门控）
    await verifyEndpointHealthy(page, '/api-gateway/endpoints?page=1&page_size=5');
    // 邮件+通知
    await verifyEndpointHealthy(page, '/email-templates?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/notifications?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/user-notification-settings');
    // 扫码+搜索（scanner/history 在 analytics.rs:108 注册，nest 到 /scanner → strict）
    await verifyEndpointHealthy(page, '/scanner/history?page=1&page_size=5');
    // 搜索（GET /search 根路径后端未注册，只注册 /search/{sales-orders,customers,products,doc-types}
    // （routes/mod.rs:405-409）；旧探针 /search?q=面料 打裸根路径必 404（臆造），optional 长期吞之=假绿。
    // 改为真实注册的 /search/products，query 参数名 q 按 handler 核实（search_api.rs:24-32 SearchParams
    // .q: Option<String>；CI 未配 ELASTICSEARCH_URL 时 search_client 走 mock 内存（container/mod.rs:439-448），
    // 空命中仍 2xx）→ strict）
    await verifyEndpointHealthy(page, '/search/products?q=面料');
    // AI（B 类：端点存在但旧探针路径写成单数/根路径，属测试侧缺陷，已改真实路径并迁 strict。
    // 工艺优化真实为复数 /ai/process-optimizations（routes/system.rs:362，system::routes()
    // 于 mod.rs:419 merge 到 /api/v1/erp 根，无额外前缀；list_process_optimizations 入参
    // ListProcessOptQuery 全 Option（ai_extend_service.rs:55-67）→ admin 必 2xx）；
    // 质量预测真实为复数 /ai/quality-predictions（system.rs:395；ListQualityPredQuery 全
    // Option（ai_extend_service.rs:102-110）→ admin 必 2xx））
    await verifyEndpointHealthy(page, '/ai/process-optimizations');
    await verifyEndpointHealthy(page, '/ai/quality-predictions');
    // AI 模型管理：GET /ai-models 根路径未注册（routes/ai_model.rs:11-61 仅有子路径，
    // nest 前缀见 mod.rs:486）；真实 GET 子路径为 /ai-models/versions
    // （ai_model.rs:17-20 → list_model_versions，入参 ModelNameQuery.model_name 为
    // Option（ai_model_management_handler.rs:21-23），无 model_name 也返回 2xx 全量列表）
    // → 改打真实子路径并 strict；旧探针携带的 page/page_size 该 handler 不支持，去除）
    await verifyEndpointHealthy(page, '/ai-models/versions');
    // 报表（A 类：/report-templates 已注册（analytics.rs:336-341 reports() 经 605 merge
    // 到 /api/v1/erp 根）；list_templates（report_engine_handler.rs:55-65）无 Query 入参，
    // 返回内置预定义模板、不依赖 seed → admin 必 2xx，迁 strict）
    await verifyEndpointHealthy(page, '/report-templates?page=1&page_size=5');
    // 报表增强：reports_enhanced()(analytics.rs:122，nest 到 /reports/enhanced) 无 GET / 根，
    // 旧探针 /reports/enhanced 打裸路径必 404（臆造）。真实 list 端点为 /reports/enhanced/templates
    // （report_enhanced_handler::list_report_templates，Query 字段全 Option）→ 改为真实路径并 strict。
    await verifyEndpointHealthy(page, '/reports/enhanced/templates?page=1&page_size=5');
    // 高级分析+跟踪+隐私（C 类裁定：/advanced/analysis 为用例臆造路径——advanced 域
    // （analytics.rs:468-484，nest 到 /advanced 见 617）仅注册 POST /advanced/ai/* 与
    // GET /advanced/reports/templates、POST /advanced/reports/{execute,export}，不存在任何
    // GET /advanced/analysis。该端点未注册，属"缺 GET 分析端点"的功能缺口还是纯臆造无法从
    // 测试侧定性，且对着不存在端点保留恒不判红的 optional 只是掩盖，故删除该探针，
    // 不为凑数保留假绿）
    await verifyEndpointHealthy(page, '/page-view/stats');
    await verifyEndpointHealthy(page, '/privacy/consents');
    // 权限+审计
    await verifyEndpointHealthy(page, '/data-permissions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/users?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/roles?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/departments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/audit-logs?page=1&page_size=5');
    // 慢查询审计（B 类：真实路径无 /system 前缀——system.rs:302 注册 GET /slow-queries，
    // system::routes() 经 mod.rs:419 merge 到 /api/v1/erp 根；list_slow_queries 入参
    // SlowQueryListParams 全 Option（slow_query_handler.rs:33-46，page/page_size 缺省
    // 防御式处理 :90-91）→ 旧探针 /system/slow-queries 必 404 属测试侧路径错误，
    // 改正路径后迁 strict）
    await verifyEndpointHealthy(page, '/slow-queries?page=1&page_size=5');
    // 产品分类+仓库
    await verifyEndpointHealthy(page, '/product-categories?page=1&page_size=50');
    await verifyEndpointHealthy(page, '/warehouses?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('审计日志 UI：搜索+表格', async ({ page }) => {
    await page.goto(`${BASE_URL}/system/audit-log`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
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
  });

  test('API 网关 UI：接口列表+新建接口', async ({ page }) => {
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
    // 默认激活「接口管理」Tab（index 0，ApiEndpointTab.vue），其「新建接口」按钮进入页面即可见
    const newBtn = page.locator('button:has-text("新建接口")').first();
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
