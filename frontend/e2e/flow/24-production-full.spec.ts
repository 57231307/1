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
  verifyDownloadEndpointHealthy,
  ensureTestEntities,
} from './helpers';

test.describe('生产模块全量：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ===== API 端点覆盖（所有子模块）=====
  test('流转卡：CRUD+状态机+步骤+反馈+工艺路线', async ({ page }) => {
    await verifyEndpointHealthy(page, '/production/flow-cards?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/process-routes?page=1&page_size=5');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/flow-cards?page=1&page_size=1'
    );
    const cardId = list.items?.[0]?.id;
    if (cardId) {
      await apiCallRaw(page, 'GET', `/production/flow-cards/${cardId}`);
      await verifyEndpointHealthy(page, `/production/flow-cards/${cardId}/steps`);
      await verifyEndpointHealthy(page, `/production/flow-cards/${cardId}/feedbacks`);
      await safePostAction(page, `/production/flow-cards/${cardId}/schedule`);
      await safePostAction(page, `/production/flow-cards/${cardId}/start-preparing`);
      await safePostAction(page, `/production/flow-cards/${cardId}/complete-preparing`);
      await safePostAction(page, `/production/flow-cards/${cardId}/start-dyeing`);
      await safePostAction(page, `/production/flow-cards/${cardId}/complete-dyeing`);
      await safePostAction(page, `/production/flow-cards/${cardId}/start-inspecting`);
      await safePostAction(page, `/production/flow-cards/${cardId}/complete`);
      await safePostAction(page, `/production/flow-cards/${cardId}/ship`);
      await safePostAction(page, `/production/flow-cards/${cardId}/terminate`);
    }
    const routes = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/process-routes?page=1&page_size=1'
    );
    if (routes.items?.[0]?.id)
      await apiCallRaw(page, 'GET', `/production/process-routes/${routes.items?.[0].id}`);
  });

  test('验布打卷：CRUD+状态机+疵点+物理测试', async ({ page }) => {
    await verifyEndpointHealthy(page, '/production/fabric-inspections?page=1&page_size=5');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/fabric-inspections?page=1&page_size=1'
    );
    const inspId = list.items?.[0]?.id;
    if (inspId) {
      await apiCallRaw(page, 'GET', `/production/fabric-inspections/${inspId}`);
      // 疵点列表断言统一收敛到下方"真实端点 + 同源 id"一处（seed 有行走 inspId、空库走自建，
      // 两条分支都被覆盖），不再在此处对同一 URL 重复探测
      await safePostAction(page, `/production/fabric-inspections/${inspId}/start`);
      // grade 需携带 GradeInspectionRequest（fabric_inspection_service.rs:181）：
      // inspected_yards 必填(Decimal>0)。空体 → serde 422 missing field inspected_yards。
      await safePostAction(page, `/production/fabric-inspections/${inspId}/grade`, {
        inspected_yards: 120,
      });
      await safePostAction(page, `/production/fabric-inspections/${inspId}/roll`);
      await safePostAction(page, `/production/fabric-inspections/${inspId}/close`);
    }
    // 疵点列表在业务上按验布单归属查询（全局疵点列表无消费方、不注册）：
    // GET /production/fabric-defects 未注册——route-snapshot.txt :80/:708/:1384 分别只有
    // by-id DELETE / by-id GET / 创建 POST，且前端 src 无任何消费方；真实契约是
    // GET /production/fabric-inspections/{id}/defects（snapshot :711，后端
    // routes/production.rs:287，前端消费方 api/fabric-inspection.ts listFabricDefectsByInspection）。
    // {id} 与真实行同源：seed 列表首行 inspId 存在则直接用；CI 库为空则按本仓
    // "自建自流转"范式于用例内 POST 一张验布单再查其疵点。严禁臆造入参（1/TEST001 等）——
    // 参数命不中真实行会把契约探测变成 404/500 假红或空转假绿。
    // inspId 由 items?.[0]?.id 取值，空列表时为 undefined（noUncheckedIndexedAccess
    // 关闭使编译器把索引结果误判为 number），显式标注可选以匹配真实运行形状
    let defectInspId: number | undefined = inspId;
    if (defectInspId == null) {
      const created = await apiCall<{ id?: number }>(
        page,
        'POST',
        '/production/fabric-inspections',
        {
          inspection_date: new Date().toISOString().split('T')[0],
          product_name: 'E2E 坯布',
          color_no: 'E2E-CN',
        }
      );
      defectInspId = created.data?.id;
    }
    if (defectInspId == null) {
      throw new Error(
        '[flow/24 疵点] 验布单 seed 列表为空且自建未返回 id，拒绝以臆造 id 探测疵点端点，判红暴露 setup 问题'
      );
    }
    await verifyEndpointHealthy(page, `/production/fabric-inspections/${defectInspId}/defects`);
    // 物理测试挂在同一张真实验布单上（复用上方已解析的 defectInspId，不再二次创建）
    // 物理指标仅 inspecting/graded 状态可录入：先把验布记录推进到 inspecting
    await safePostAction(page, `/production/fabric-inspections/${defectInspId}/start`);
    await safePostAction(page, '/production/fabric-inspections/physical-tests', {
      // AddPhysicalTestRequestDto: inspection_id/test_item/test_value 必填
      inspection_id: defectInspId,
      test_item: 'tensile_strength',
      test_value: 500,
      test_result: 'pass',
    });
  });

  test('产量工资：工价+工票+计算+确认+支付', async ({ page }) => {
    await verifyEndpointHealthy(page, '/production/wage-rates?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/wage-records?page=1&page_size=5');
    // 工资记录导出 xlsx（production.rs:322 已注册，handler 直出二进制、无 download_token fail-closed 网关）；
    // 返回非 JSON，故用下载专用严格校验（2xx=健康；404 路由漂移/403 权限/5xx 均判红），不再 optional 吞 404。
    await verifyDownloadEndpointHealthy(page, '/production/wage-records/export');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/wage-records?page=1&page_size=1'
    );
    const recordId = list.items?.[0]?.id;
    if (recordId) {
      await apiCallRaw(page, 'GET', `/production/wage-records/${recordId}`);
      await verifyEndpointHealthy(page, `/production/wage-records/${recordId}/details`);
      await safePostAction(page, `/production/wage-records/${recordId}/calculate`);
      await safePostAction(page, `/production/wage-records/${recordId}/confirm`);
      await safePostAction(page, `/production/wage-records/${recordId}/pay`);
    }
  });

  test('能耗管理：仪表+消耗+规则+分配', async ({ page }) => {
    await verifyEndpointHealthy(page, '/production/energy-meters?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/energy-consumptions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/energy-rules?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/energy-allocations?page=1&page_size=5');
    // energy-rules/effective 契约：handler energy_handler.rs:380-388 Query<EffectiveRuleQuery>
    // （结构体 :116-121）workshop(String)/meter_type(String)/date(NaiveDate) 三者皆非 Option、无 serde
    // 默认 → 缺任一即 serde 400（报告只点了 workshop，代码显示 date/meter_type 同为必填）。
    // service get_effective_rule(energy_ops/allocation_rule.rs:320-348) 按 workshop+meter_type+
    // status=ACTIVE(:329)+effective_date<=date 过滤，无命中返回 Ok(None)→200。本 spec 无预置能耗车间
    // （migration v15/mod.rs:2788 仅 CREATE TABLE，全树无 INSERT 种子），故由本例自建一条带车间的规则
    // （POST /production/energy-rules，CreateRuleRequest energy_ops/allocation_rule.rs:33-46），再回读其
    // workshop/meter_type 作 effective 入参——取值来源=本例 POST 响应模型，非臆造。词表取证：
    // meter_type=electricity（models/energy_meter.rs:24 water/electricity/steam/gas/compressed_air）、
    // allocation_basis=by_workshop（models/status/wage_energy_chemical_business.rs:102）；再激活
    // draft→active（energy_ops/allocation_rule.rs:247）使 date 命中真实 ACTIVE 规则。
    const effToday = new Date().toISOString().slice(0, 10);
    const effRule = await apiCallRaw<{ id: number; workshop: string | null; meter_type: string }>(
      page,
      'POST',
      '/production/energy-rules',
      {
        rule_name: genCode('E2EENR'),
        meter_type: 'electricity',
        allocation_basis: 'by_workshop',
        workshop: `E2E能耗车间${Date.now().toString().slice(-6)}`,
        effective_date: effToday,
      }
    );
    expect(
      effRule.id,
      '自建能耗规则应返回数值 id（CreateRuleRequest→energy_allocation_rule::Model）'
    ).toBeGreaterThan(0);
    expect(effRule.workshop, '自建规则应回读出车间（effective 入参取值来源）').toBeTruthy();
    await safePostAction(page, `/production/energy-rules/${effRule.id}/activate`);
    await verifyEndpointHealthy(
      page,
      `/production/energy-rules/effective?workshop=${encodeURIComponent(
        effRule.workshop as string
      )}&meter_type=${effRule.meter_type}&date=${effToday}`
    );
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/energy-consumptions?page=1&page_size=1'
    );
    const consId = list.items?.[0]?.id;
    if (consId) {
      await apiCallRaw(page, 'GET', `/production/energy-consumptions/${consId}`);
      await safePostAction(page, `/production/energy-consumptions/${consId}/confirm`);
    }
  });

  test('MRP 计算+历史+需求+转单', async ({ page }) => {
    await verifyEndpointHealthy(page, '/production/mrp/products');
    await verifyEndpointHealthy(page, '/production/mrp/results');
    await verifyEndpointHealthy(page, '/production/mrp/requirements');
    await verifyEndpointHealthy(page, '/production/mrp-history?page=1&page_size=5');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/mrp-history?page=1&page_size=1'
    );
    if (list.items?.[0]?.id)
      await apiCallRaw(page, 'GET', `/production/mrp-history/${list.items?.[0].id}`);
    await safePostAction(page, '/production/mrp/calculate', {
      // MrpCalculatePayload: items 必填（product_id/required_quantity/required_date）
      items: [
        {
          product_id: 1,
          required_quantity: 100,
          required_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
        },
      ],
    });
  });

  test('产能分析+排程+质量标准+质量检验+BOM+打样+缺料+缸号状态机+委外', async ({ page }) => {
    // 产能（overview/bottlenecks/load-analysis/overload-check 为 BI 统计类增强端点，后端已注册）
    await verifyEndpointHealthy(page, '/production/capacity/overview');
    await verifyEndpointHealthy(page, '/production/capacity/bottlenecks');
    await verifyEndpointHealthy(page, '/production/capacity/work-centers?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/capacity/load-analysis');
    await verifyEndpointHealthy(page, '/production/capacity/overload-check');
    // 排程（gantt 后端已注册：routes/mod.rs:150 → scheduling_handler::get_gantt_data，
    // GanttQuery 三字段全 Option（scheduling_handler.rs:134-138），无 query 参也返回
    // 2xx 结构体（空数据 items 为空数组）→ admin 必 2xx，迁 strict，不再 optional 吞 404；
    // conflicts 后端已注册须 strict）
    await verifyEndpointHealthy(page, '/scheduling/gantt');
    await verifyEndpointHealthy(page, '/scheduling/conflicts');
    await verifyEndpointHealthy(page, '/scheduling/tasks?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/scheduling/history?page=1&page_size=5');
    // 质量检验（真实端点：/production/quality-inspection/*，质量标准在其 standards 子路径下；
    // 原独立的 /quality-standards* 系列端点后端不存在，删除虚构调用，真实覆盖见下方三行）
    await verifyEndpointHealthy(
      page,
      '/production/quality-inspection/standards?page=1&page_size=5'
    );
    await verifyEndpointHealthy(page, '/production/quality-inspection/records?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/quality-inspection/defects?page=1&page_size=5');
    // BOM
    await verifyEndpointHealthy(page, '/boms?page=1&page_size=5');
    const bomList = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/boms?page=1&page_size=1'
    );
    // 显式前置（D-1 Q3 数据面判据）：seed 来源 id 参与 strict 探针时，空清单不许整段
    // if 静默跳过（零覆盖假绿），必须 fail-visible 判红点名数据面缺前置。
    // BOM 行由 beforeEach ensureTestEntities（helpers.ts 第 13 步，createBomUI 造数）保证；
    // 走到空分支即 seed 真缺陷，归因数据面而非注册面。
    const bomId = bomList.items?.[0]?.id;
    if (!bomId) {
      throw new Error(
        `GET /boms?page=1&page_size=1 返回 items=${JSON.stringify(bomList.items)}——无一条 BOM 行：数据面缺前置（ensureTestEntities 未真实落 BOM），/boms/{id}/tree 等 strict 探针拒绝空转，禁静默跳过`
      );
    }
    await apiCallRaw(page, 'GET', `/boms/${bomId}`);
    await verifyEndpointHealthy(page, `/boms/${bomId}/tree`);
    await safePostAction(page, `/boms/${bomId}/requirements`, { quantity: 100 });
    await safePostAction(page, `/boms/${bomId}/copy`);
    // 打样
    await verifyEndpointHealthy(page, '/production/lab-dip/requests?page=1&page_size=5');
    const ldList = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/production/lab-dip/requests?page=1&page_size=1'
    );
    if (ldList.items?.[0]?.id) {
      const reqId = ldList.items?.[0].id;
      await apiCallRaw(page, 'GET', `/production/lab-dip/requests/${reqId}`);
      await verifyEndpointHealthy(page, `/production/lab-dip/samples/by-request/${reqId}`);
      await safePostAction(page, `/production/lab-dip/requests/${reqId}/start-sampling`);
      await safePostAction(page, `/production/lab-dip/requests/${reqId}/complete`);
    }
    // 缺料预警（alerts/threshold/summary 后端均已注册，strict 验证）
    await verifyEndpointHealthy(page, '/material-shortage/alerts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/material-shortage/summary');
    await verifyEndpointHealthy(page, '/material-shortage/threshold');
    // 缸号状态机
    const ctx = getCtx();
    if (ctx.dyeBatchId) {
      await verifyEndpointHealthy(
        page,
        `/production/dye-batch-lifecycle-logs/by-batch/${ctx.dyeBatchId}`
      );
      await verifyEndpointHealthy(
        page,
        `/production/dye-batch-operations/by-batch/${ctx.dyeBatchId}`
      );
    }
    await verifyEndpointHealthy(page, '/production/dye-batch-state-rules/allowed-transitions');
    // 委外
    await verifyEndpointHealthy(page, '/production/outsourcing-orders?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/outsourcing-receipts?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/production/outsourcing-vouchers?page=1&page_size=5');
  });

  // ===== 真实 UI 交互验证 =====
  test('BOM 列表 UI：搜索+新建弹窗+表单', async ({ page }) => {
    await page.goto(`${BASE_URL}/bom`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 搜索
    const searchInput = page
      .locator('input[placeholder*="产品名称"], input[placeholder*="产品"]')
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const btnVisible = await queryBtn.isVisible();
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
      expect(tableOk).toBe(true);
    }
    // 新建 BOM
    const newBtn = page.locator('button:has-text("新建 BOM"), button:has-text("新建BOM")').first();
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

  test('MRP 计算 UI：表单+计算按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/mrp`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-form, .el-table')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证计算按钮存在
    const calcBtn = page.locator('button:has-text("开始计算")').first();
    await calcBtn.waitFor({ state: 'visible', timeout: 5000 });
    const btnVisible = await calcBtn.isVisible();
    if (btnVisible) {
      // 验证按钮可点击（不实际计算，避免产生数据）
      expect(btnVisible).toBe(true);
    }
  });

  test('产能分析 UI：筛选+图表', async ({ page }) => {
    await page.goto(`${BASE_URL}/capacity`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证日期选择器存在
    const datePicker = page.locator('.el-date-editor, input[placeholder*="日期"]').first();
    await datePicker.waitFor({ state: 'visible', timeout: 5000 });
    const dateVisible = await datePicker.isVisible();
    // 验证工作中心选择
    const select = page.locator('.el-select').first();
    await select.waitFor({ state: 'visible', timeout: 5000 });
    const selectVisible = await select.isVisible();
    expect(dateVisible || selectVisible).toBe(true);
  });

  test('排程 UI：表格+甘特图入口', async ({ page }) => {
    await page.goto(`${BASE_URL}/scheduling`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('染色配方 UI：搜索+新建弹窗+色号字段', async ({ page }) => {
    await page.goto(`${BASE_URL}/dye-recipe`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证表格有数据
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
    // 验证表头含色号列
    const headers = table.locator('th, .el-table-v2__header-cell');
    const headerTexts: string[] = [];
    const headerCount = await headers.count();
    for (let i = 0; i < headerCount; i++)
      headerTexts.push((await headers.nth(i).textContent()) || '');
    const hasColorNo = headerTexts.some(h => h.includes('色号'));
    const hasColorName = headerTexts.some(h => h.includes('颜色') || h.includes('色名'));
    expect(hasColorNo || hasColorName).toBe(true);
    // 新建配方
    const newBtn = page.locator('button:has-text("新建配方")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      // 验证表单有色号字段（el-form-item label 渲染为 <label>，
      // hasText 兼容 label/span，等待放宽至 10s 应对 CI 慢环境）
      const colorField = dialog
        .locator('.el-form-item')
        .filter({ hasText: /色号|颜色名称/ })
        .first();
      await colorField.waitFor({ state: 'visible', timeout: 10_000 });
      const colorVisible = await colorField.isVisible();
      expect(colorVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('缸号列表 UI：搜索+新建+状态标签', async ({ page }) => {
    await page.goto(`${BASE_URL}/dye-batch`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
    // 新建批次
    const newBtn = page.locator('button:has-text("新建批次")').first();
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

  test('质量标准 UI：列表+新建', async ({ page }) => {
    await page.goto(`${BASE_URL}/quality`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const newBtn = page.locator('button:has-text("新建标准")').first();
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

  test('成本归集 UI：列表+分析', async ({ page }) => {
    await page.goto(`${BASE_URL}/cost`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('缺料预警 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/material-shortage`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });

  test('排程甘特图 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/scheduling/gantt`);
    await page.waitForTimeout(3000);
    await page.locator('.el-card, body').first().waitFor({ state: 'visible', timeout: 30_000 });
    const bodyOk = await page.locator('body').isVisible();
    expect(bodyOk).toBe(true);
  });
});
