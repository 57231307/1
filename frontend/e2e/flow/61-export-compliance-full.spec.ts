import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
  failureCode,
  verifyEndpointHealthy,
  verifyDownloadEndpointHealthy,
  APP_ERROR_CODES,
  BASE_URL,
  type ApiFailureResult,
} from './helpers';
import { fillFieldByLabel, formItemByExactLabel } from './ui-helpers';

/**
 * 61 出口合规：退税要素（免抵退）+ Incoterms 贸易术语 + 环保税（e2e 补齐 A 路）
 *
 * 端点真实性（逐条核实注册路径）：
 * - POST /export-refunds/customs-declarations                       routes/export_refund.rs:13-16（mod.rs:491 nest /api/v1/erp）
 * - GET  /export-refunds/sales-orders/{id}/documents-verification   routes/export_refund.rs:17-21
 * - POST /export-refunds/refund-calculation                         routes/export_refund.rs:22-25（纯计算，免抵退公式真相 export_refund_service.rs:166-194）
 * - POST/GET /export-refunds/refund-declarations                    routes/export_refund.rs:26-33
 * - GET  /export-refunds/{id}/print                                 routes/export_refund.rs:35-38（docx → download helper）
 * - GET  /incoterms/quotations/{id}/price-composition               routes/incoterms.rs:13-16
 * - POST /incoterms/cost-calculation                                routes/incoterms.rs:17-20（词表 utils/incoterms.rs:14-38，serde UPPERCASE）
 * - GET  /incoterms/usage-report?year=&month=                       routes/incoterms.rs:21-24（year/month 为必填 Query）
 * - POST/GET /environmental-tax/discharge-records                   routes/environmental_tax.rs:12-20（GET 必填 period_year/period_month）
 * - GET  /environmental-tax/tax-declarations                        routes/environmental_tax.rs:21-24
 *
 * 隔离策略（防串扰、用例自建自流转）：
 * - 环保税 GET/申报按 period_year+period_month **精确等值**过滤（service.rs list_by_period），
 *   选用远未来专属期间 2098-07（全仓 e2e grep 无其它用例在该期间造数），税额链可钉精确真值；
 * - 退税申报表聚合口径为真实所属期间半开区间 [期间月初, 次月月初)，且金额基数
 *   **只计入 status=verified 的报关单**（export_refund_service.rs VERIFIED_STATUS /
 *   verified_export_sales_amount，与核验端点、documents_complete 共用同一判定源）；
 *   新建报关单恒为 pending（后端暂无置 verified 的端点）→ pending 数据不进基数。
 *   故报关单聚合仍选最远专属期间 2099-01 并依赖「全仓无任何其它用例/seed 在
 *   export_date>=2099-01-01 造报关数据」这一 grep 事实，把聚合链精确真值钉死。
 *
 * 假绿防线：创建退税申报后必 GET /export-refunds/refund-declarations 回读落库金额链；
 * 排放记录创建后必 GET discharge-records 回读污染当量数与税额（数组信封显式断言，不用 ?? []）；
 * docx 走 download helper；月报 items 缺键即判红。
 *
 * 文案口径：export-compliance/index.vue 为非 i18n 页面（组件内裸中文源文本），
 * 断言词与该组件源文件逐字符一致；locales 无对应 key，不适用 i18n 口径亦无漂移风险。
 */

/**
 * 断言精确拒绝契约（收紧原「任意 4xx + 三族机器码来者不拒」的假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373 客户端族全映射 BAD_REQUEST 状态；
 *   middleware/trace_context.rs::normalize_extractor_rejection 把提取器拒绝同样归 400）；
 * - 机器码必须逐条等于调用点期望族：唯一性/业务规则 = BUSINESS_ERROR
 *   （export_refund_service.rs::create_customs_declaration → AppError::business），
 *   参数范围门 = BAD_REQUEST（同函数 bad_request），
 *   提取器缺参/枚举反序列化失败/年月非法 = VALIDATION_ERROR
 *   （trace_context.rs::normalize_extractor_rejection + incoterms_service.rs::monthly_usage_report
 *   → validation_displayable）。门别串号即判红交后端，永不读取文案（脱敏出参不可判）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${r.status} ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

/** 报关单号唯一化（服务端按 declaration_no 做唯一性校验，export_refund_service.rs:106-116） */
function declNo(): string {
  return genCode('E2E-CD');
}

test.describe.serial('61 出口合规：退税要素 + Incoterms + 环保税', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('61-01 页面可达 + 已注册 GET 端点严格健康', async ({ page }) => {
    await page.goto(`${BASE_URL}/export-compliance`);
    await expect(page.getByRole('tab', { name: '出口退税', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '贸易术语', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '环保税', exact: true })).toBeVisible();

    await verifyEndpointHealthy(page, '/export-refunds/refund-declarations');
    await verifyEndpointHealthy(
      page,
      '/environmental-tax/discharge-records?period_year=2098&period_month=7'
    );
    await verifyEndpointHealthy(
      page,
      '/environmental-tax/tax-declarations?period_year=2098&period_month=7'
    );
    await verifyEndpointHealthy(page, '/incoterms/usage-report?year=2020&month=1');
  });

  test('61-02 UI 新建报关单 → POST 真实响应回读落库值；单证核验语义回读', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.salesOrderId) throw new Error('前置缺失：ctx.salesOrderId 未就绪');
    const no = declNo();

    await page.goto(`${BASE_URL}/export-compliance`);
    await page.getByRole('tab', { name: '出口退税', exact: true }).click();
    await page.getByRole('button', { name: '新建报关单' }).click();
    const dialog = page.getByRole('dialog', { name: '新建报关单' });
    await expect(dialog).toBeVisible();

    const [resp] = await Promise.all([
      // CSRF 竞败时前端会重放，必须等 status()===200 的最终响应（仓内既有范式）
      page.waitForResponse(
        r =>
          r.url().includes('/export-refunds/customs-declarations') &&
          r.request().method() === 'POST' &&
          r.status() === 200,
        { timeout: 30_000 }
      ),
      (async () => {
        await formItemByExactLabel(dialog, '销售订单ID')
          .locator('.el-input-number input')
          .fill(String(ctx.salesOrderId));
        await fillFieldByLabel(dialog, page, '报关单号', no);
        const dateInput = formItemByExactLabel(dialog, '出口日期').locator('input').first();
        await dateInput.click();
        await dateInput.fill('2027-03-01');
        await page.keyboard.press('Enter');
        await formItemByExactLabel(dialog, '总金额').locator('.el-input-number input').fill('1000');
        await formItemByExactLabel(dialog, '汇率').locator('.el-input-number input').fill('2');
        await dialog.getByRole('button', { name: '保存' }).click();
      })(),
    ]);
    const body = await resp.json();
    // 不是只看 toast：直接断 POST 200 真实响应里的落库模型逐字段
    expect(body?.data?.declaration_no, `POST 响应应回显报关单号 ${no}`).toBe(no);
    expect(Number(body?.data?.total_amount), '落库总金额应为 1000').toBe(1000);
    expect(Number(body?.data?.exchange_rate), '落库汇率应为 2').toBe(2);
    expect(body?.data?.export_date, '落库出口日期应为 2027-03-01').toBe('2027-03-01');
    expect(body?.data?.status, '新建报关单初始状态应为 pending').toBe('pending');
    expect(body?.data?.sales_order_id, '应绑定 ctx 销售订单').toBe(ctx.salesOrderId);
    await expect(page.getByText('报关单已创建')).toBeVisible({ timeout: 15_000 });

    // 单证核验真相（export_refund_service.rs:145-162）：报关单 status 必须是 'verified'
    // 才计入齐全，UI 新建的是 'pending' → 该订单核验结果必须为 false（精确语义断言）
    const verify = await apiCallRaw<{ documents_complete: boolean; sales_order_id: number }>(
      page,
      'GET',
      `/export-refunds/sales-orders/${ctx.salesOrderId}/documents-verification`
    );
    expect(verify.sales_order_id, '核验响应应回显请求的订单 id').toBe(ctx.salesOrderId);
    expect(verify.documents_complete, 'pending 报关单不应被判为单证齐全（核验只认 verified）').toBe(
      false
    );

    // 从未有过任何单证的订单 id → 同样 false（负向对照）
    const ghost = await apiCallRaw<{ documents_complete: boolean }>(
      page,
      'GET',
      '/export-refunds/sales-orders/999999999/documents-verification'
    );
    expect(ghost.documents_complete, '无单证订单应为 false').toBe(false);
  });

  test('61-03 报关单负例：重复单号 / 汇率≤0 / 金额为负', async ({ page }) => {
    const ctx = getCtx();
    if (!ctx.salesOrderId) throw new Error('前置缺失：ctx.salesOrderId 未就绪');
    const no = declNo();
    const base = {
      declaration_no: no,
      sales_order_id: ctx.salesOrderId,
      export_date: '2027-06-01',
      total_amount: '500',
      exchange_rate: '1',
    };
    await apiCall(page, 'POST', '/export-refunds/customs-declarations', base);
    const dup = await apiCallExpectFail(page, 'POST', '/export-refunds/customs-declarations', base);
    // 唯一性门（export_refund_service.rs::create_customs_declaration → AppError::business）
    expectRejected(dup, `重复报关单号 ${no}`, APP_ERROR_CODES.BUSINESS_ERROR);

    const badRate = await apiCallExpectFail(page, 'POST', '/export-refunds/customs-declarations', {
      declaration_no: declNo(),
      sales_order_id: ctx.salesOrderId,
      export_date: '2027-06-01',
      total_amount: '500',
      exchange_rate: '0',
    });
    // 汇率≤0 参数门（同函数 → AppError::bad_request）
    expectRejected(badRate, '汇率=0 应被拒绝', APP_ERROR_CODES.BAD_REQUEST);

    const negAmt = await apiCallExpectFail(page, 'POST', '/export-refunds/customs-declarations', {
      declaration_no: declNo(),
      sales_order_id: ctx.salesOrderId,
      export_date: '2027-06-01',
      total_amount: '-1',
      exchange_rate: '1',
    });
    // 金额为负参数门（同函数 → AppError::bad_request）
    expectRejected(negAmt, '负报关金额', APP_ERROR_CODES.BAD_REQUEST);
  });

  test('61-04 退税要素计算（免抵退公式精确真值，两组分支 + 缺参负例）', async ({ page }) => {
    // 分支 1：留抵不足 → 应退=min(13000, 6000)=6000，免抵=7000，结转=0
    const a = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/refund-calculation',
      {
        export_sales_amount: '100000',
        refund_rate: '0.13',
        input_vat_amount: '5000',
        carryforward_from_prev: '1000',
      }
    );
    expect(Number(a.refundable_vat_amount), '免抵退税额=100000×0.13').toBe(13000);
    expect(Number(a.actual_refund_amount), '应退税额=min(13000,1000+5000)').toBe(6000);
    expect(Number(a.exempt_vat_amount), '免抵税额=13000-6000').toBe(7000);
    expect(Number(a.carryforward_amount), '进项不足时结转应为 0').toBe(0);

    // 分支 2：留抵富余 → 应退=130 全退，结转=500-130=370
    const b = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/refund-calculation',
      {
        export_sales_amount: '1000',
        refund_rate: '0.13',
        input_vat_amount: '500',
        carryforward_from_prev: '0',
      }
    );
    expect(Number(b.refundable_vat_amount), '免抵退税额=1000×0.13').toBe(130);
    expect(Number(b.actual_refund_amount), '应退税额=min(130,500)').toBe(130);
    expect(Number(b.exempt_vat_amount), '免抵税额=130-130').toBe(0);
    expect(Number(b.carryforward_amount), '结转下期=500-130').toBe(370);

    // 缺必填字段 = Json 提取器拒绝（handler Json<RefundCalculationInput>，serde 缺字段即拒）
    // → middleware/trace_context.rs::normalize_extractor_rejection 归一为 400 + VALIDATION_ERROR
    const missing = await apiCallExpectFail(page, 'POST', '/export-refunds/refund-calculation', {
      export_sales_amount: '100',
    });
    expect(
      missing.status,
      `退税计算缺参应恰为 400（提取器拒绝归一族），实际 ${JSON.stringify(missing)}`
    ).toBe(400);
    expect(
      failureCode(missing),
      `退税计算缺参机器码应为 VALIDATION_ERROR，实际 ${failureCode(missing)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('61-05 退税申报表：2099-01 专属期间生成 → 列表回读金额链落库真值 → docx 打印', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.salesOrderId) throw new Error('前置缺失：ctx.salesOrderId 未就绪');
    // 专属远未来期间报关单：新建即 pending，未核验数据不进申报基数
    // （收紧口径：export_sales_amount 只计入 status=verified 报关单）→ 聚合应为 0，
    // 全仓无其它 2099 数据，聚合链真值可精确钉死。
    await apiCall(page, 'POST', '/export-refunds/customs-declarations', {
      declaration_no: declNo(),
      sales_order_id: ctx.salesOrderId,
      export_date: '2099-01-15',
      total_amount: '1000',
      exchange_rate: '2',
    });

    const gen = await apiCall<Record<string, unknown>>(
      page,
      'POST',
      '/export-refunds/refund-declarations',
      {
        period_year: 2099,
        period_month: 1,
        refund_rate: '0.13',
        input_vat_amount: '600',
        carryforward_from_prev: '0',
      }
    );
    const d = gen.data;
    const declId = Number(d?.id);
    expect(declId, `生成申报表应返回 id：${JSON.stringify(gen)}`).toBeGreaterThan(0);
    expect(Number(d?.export_sales_amount), 'pending 报关单不进申报基数，聚合销售额应为 0').toBe(0);
    expect(Number(d?.refundable_vat_amount), '免抵退税额=0×0.13=0').toBe(0);
    expect(Number(d?.actual_refund_amount), '应退=min(0,600)=0').toBe(0);
    expect(Number(d?.exempt_vat_amount), '免抵税额=0-0=0').toBe(0);
    expect(Number(d?.carryforward_amount), '结转=max(0,0+600-0)=600').toBe(600);
    expect(String(d?.declaration_no), '申报表号应带期间前缀 ERD-209901-').toMatch(
      /^ERD-209901-\d{5}$/
    );
    expect(
      d?.documents_complete,
      '当期仅有 pending 报关单 → 齐全标记必须为 false（核验只认 verified）'
    ).toBe(false);
    expect(d?.status, '新建申报表状态应为 draft').toBe('draft');

    // 列表回读（GET 支持期间过滤）：必须能按 id 找到这张表且金额链一致
    const list = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/export-refunds/refund-declarations?period_year=2099&period_month=1'
    );
    expect(
      Array.isArray(list),
      `refund-declarations GET 应返回数组（handler :94-104 to_value(Vec)），实际 ${JSON.stringify(list)}`
    ).toBe(true);
    const row = list.find(it => Number(it.id) === declId);
    expect(row, 'GET 列表按期间过滤应命中新建申报表').toBeTruthy();
    expect(Number(row?.export_sales_amount), '列表回读销售额应为 0（未核验不进基数）').toBe(0);
    expect(Number(row?.actual_refund_amount), '列表回读应退税额应为 0').toBe(0);
    expect(row?.status, '列表回读状态应为 draft').toBe('draft');

    await verifyDownloadEndpointHealthy(page, `/export-refunds/${declId}/print`);
  });

  test('61-06 Incoterms 价格构成：自建 FOB 报价单 → API 逐字段真值 + UI 查询回显', async ({
    page,
  }) => {
    const q = await seedFobQuotation(page);
    const pc = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/incoterms/quotations/${q}/price-composition`
    );
    // incoterms_service.rs:71-110：FOB 责任划分真相逐字段核对
    expect(pc.incoterm, 'price_terms=FOB 应解析为 FOB').toBe('FOB');
    expect(Number(pc.product_cost), 'product_cost=subtotal-tax_amount=500-0').toBe(500);
    expect(Number(pc.total_amount), 'total_amount 应为报价单总额 500').toBe(500);
    expect(String(pc.risk_transfer_point), 'FOB 风险转移点在装运港').toContain('装运港船上');
    expect(pc.cost_bearer, 'FOB 主费用双方共担').toBe('双方共担');
    expect(pc.export_clearance_party, 'FOB 出口清关卖方').toBe('卖方');
    expect(pc.import_clearance_party, 'FOB 进口清关买方').toBe('买方');
    expect(pc.transport_mode, 'FOB 仅适用海运/内河').toBe('海运/内河运输');

    // UI 贸易术语 tab：填报价单ID → 查询价格构成 → 结果框回显 FOB 真值
    await page.goto(`${BASE_URL}/export-compliance`);
    await page.getByRole('tab', { name: '贸易术语', exact: true }).click();
    const form = page.locator('.el-form--inline');
    await form.locator('.el-input-number input').first().fill(String(q));
    await page.getByRole('button', { name: '查询价格构成' }).click();
    const result = page.locator('.result-box').first();
    await expect(result, '结果框应渲染价格构成 JSON').toBeVisible({ timeout: 20_000 });
    await expect(result).toContainText('"incoterm": "FOB"');
    await expect(result).toContainText('装运港船上');

    // 不存在的报价单 → NOT_FOUND 判红（防「任何 id 都返回 200」的假健康）
    const ghost = await apiCallExpectFail(
      page,
      'GET',
      '/incoterms/quotations/999999999/price-composition'
    );
    expect(ghost.status, '不存在报价单应 404').toBe(404);
    expect(failureCode(ghost), '不存在报价单机器码应为 NOT_FOUND').toBe('NOT_FOUND');
  });

  test('61-07 Incoterms 费用判定：EXW/FOB/CIF/DDP 精确组合 + 非法术语拒绝', async ({ page }) => {
    const costs = {
      product_cost: '100',
      freight_cost: '30',
      insurance_cost: '5',
      duty_cost: '20',
    };
    const exw = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/incoterms/cost-calculation',
      {
        incoterm: 'EXW',
        ...costs,
      }
    );
    expect(Number(exw.product_cost), 'EXW 货款保留').toBe(100);
    expect(exw.freight_cost, 'EXW 不含运费（includes_freight=false）').toBeNull();
    expect(exw.insurance_cost, 'EXW 不含保险').toBeNull();
    expect(exw.duty_cost, 'EXW 不含关税').toBeNull();

    // 定稿口径（ICC Incoterms 2020）：FOB 与 FCA/FAS 同族，卖方仅承担装船前费用与风险，
    // 主运费由买方订立并承担；后端 utils/incoterms.rs includes_freight 已按此实现
    // （EXW/FCA/FAS/FOB 为 false），故 /incoterms/cost-calculation 对 FOB 的
    // freight_cost 必须返回 null，传入的运费不进 FOB 价格构成。
    const fob = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/incoterms/cost-calculation',
      {
        incoterm: 'FOB',
        ...costs,
      }
    );
    expect(fob.freight_cost, 'FOB 不含主运费（includes_freight=false，主运费归买方）').toBeNull();
    expect(fob.insurance_cost, 'FOB 不含保险（仅 CIF/CIP/DDP 含）').toBeNull();
    expect(fob.duty_cost, 'FOB 不含关税（仅 DDP 含）').toBeNull();

    const cif = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/incoterms/cost-calculation',
      {
        incoterm: 'CIF',
        ...costs,
      }
    );
    expect(Number(cif.freight_cost), 'CIF 含运费').toBe(30);
    expect(Number(cif.insurance_cost), 'CIF 含保险').toBe(5);
    expect(cif.duty_cost, 'CIF 不含关税（仅 DDP 含）').toBeNull();

    const ddp = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/incoterms/cost-calculation',
      {
        incoterm: 'DDP',
        ...costs,
      }
    );
    expect(Number(ddp.freight_cost), 'DDP 含运费').toBe(30);
    expect(Number(ddp.insurance_cost), 'DDP 含保险').toBe(5);
    expect(Number(ddp.duty_cost), 'DDP 含关税').toBe(20);

    // incoterm 为 serde 枚举（backend/src/utils/incoterms.rs Incoterms2020，UPPERCASE 词表）：
    // FOOBAR 反序列化失败 = Json 提取器拒绝 → trace_context.rs::normalize_extractor_rejection
    // 归一 400 + VALIDATION_ERROR。判状态+机器码，不判文案（serde 原文只进后端日志）。
    const bad = await apiCallExpectFail(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'FOOBAR',
      ...costs,
    });
    expect(bad.status, `非法术语应恰为 400（提取器拒绝归一族），实际 ${JSON.stringify(bad)}`).toBe(
      400
    );
    expect(failureCode(bad), `非法术语机器码应为 VALIDATION_ERROR，实际 ${failureCode(bad)}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });

  test('61-08 术语使用月报：缺参判红（契约）+ 自建 FOB 报价单进入当期统计', async ({ page }) => {
    // 契约真相：year/month 为必填 Query（incoterms_handler.rs::UsageReportQuery 非 Option），
    // 缺参 = Query 提取器拒绝 → trace_context.rs::normalize_extractor_rejection 归一
    // 400 + VALIDATION_ERROR。
    // 前端「术语使用报表」按钮不带参（api/export-compliance.ts:33-35）→ 该按钮恒失败，
    // 属源码缺陷（见交付报告），此处不放宽。
    const noParam = await apiCallExpectFail(page, 'GET', '/incoterms/usage-report');
    expect(
      noParam.status,
      `缺参月报应恰为 400（提取器拒绝归一族），实际 ${JSON.stringify(noParam)}`
    ).toBe(400);
    expect(
      failureCode(noParam),
      `缺参月报机器码应为 VALIDATION_ERROR，实际 ${failureCode(noParam)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    const q = await seedFobQuotation(page);
    await apiCall(page, 'POST', `/quotations/${q}/submit`);
    let status = await apiCallRaw<{ status: string }>(page, 'GET', `/quotations/${q}`).then(
      d => d.status
    );
    if (status !== 'approved') {
      await apiCall(page, 'POST', `/quotations/${q}/approve`);
      status = await apiCallRaw<{ status: string }>(page, 'GET', `/quotations/${q}`).then(
        d => d.status
      );
    }
    expect(status, '月报前置：报价单应为 approved').toBe('approved');

    const now = new Date();
    const year = now.getFullYear();
    const month = now.getMonth() + 1;
    const report = await apiCallRaw<{
      year: number;
      month: number;
      items: Array<Record<string, unknown>>;
    }>(page, 'GET', `/incoterms/usage-report?year=${year}&month=${month}`);
    expect(report.year, '月报应回显 year').toBe(year);
    expect(report.month, '月报应回显 month').toBe(month);
    // 信封真相（incoterms_service.rs IncotermsMonthlyReport{year,month,items}）：items 必为数组，
    // 禁止 `?? []` 吞缺键
    expect(
      Array.isArray(report.items),
      `月报 data.items 必须为数组，实际 ${JSON.stringify(report)}`
    ).toBe(true);
    const fobItem = (report.items as Array<Record<string, unknown>>).find(
      it => it.incoterm === 'FOB'
    );
    expect(
      fobItem,
      `当月月报应统计到自建 FOB 报价单，实际 items=${JSON.stringify(report.items)}`
    ).toBeTruthy();
    expect(Number(fobItem?.count) >= 1, 'FOB 使用次数应 ≥1').toBe(true);

    const badMonth = await apiCallExpectFail(
      page,
      'GET',
      '/incoterms/usage-report?year=2026&month=13'
    );
    // incoterms_service.rs::monthly_usage_report → validation_displayable（400 + VALIDATION_ERROR）
    expectRejected(badMonth, 'month=13 非法年月', APP_ERROR_CODES.VALIDATION_ERROR);
  });

  test('61-09 环保税：缺法定当量配置 fail-closed 拒绝计税且零落库 + 税额未配置 fail-visible + 负例族', async ({
    page,
  }) => {
    // 契约真相（CI #4675 E8 判责：测试前提过期，源码一侧已被证为正确）：
    // 提交 cfcd6bf3 按裁定 #6 拆掉「未知污染物按 1kg 兜底、默认税率 2.4」的旧行为。
    // environmental_tax_service.rs::calculate_tax 现为双门 fail-closed，且都发生在 insert 之前
    // （create_discharge_record 先算税再落库）：
    // ① 法定当量门（constants/environmental_tax.rs::statutory_pollution_equivalent）：
    //    税目表只登记 COD/cod、氨氮/NH3-N、VOCs/vocs、污泥/sludge；表外污染物一律
    //    400 + VALIDATION_ERROR（validation_displayable），禁止任何估算值兜底；
    // ② 适用税额门：税额是地方可变值，只来自部署配置 env_tax_rate_per_equivalent /
    //    环境变量 ENV_TAX_RATE_PER_EQUIVALENT（settings.rs OnceLock 启动期注入，无运行时改口端点）。
    // ⚠️ 本用例按裁定改走**显式负例**而非正向真值的原因（如实登记，不硬造正向）：
    //    CI e2e 后端启动环境全仓 grep 未设置 ENV_TAX_RATE_PER_EQUIVALENT、config.yaml 不入库
    //    （example 中该项注释掉），税率=未配置 → 即使改用税目表内真实污染物（COD），
    //    计税仍在第二门被 400 + BUSINESS_ERROR 显式拒绝，e2e 侧无法注入配置走通正向链。
    //    正向税额真值链（当量=排放量÷法定值、税额=当量×配置税率）由后端契约测
    //    tests/contract_wave5_env_tax_rates_from_config_test.rs 以注入配置税率的方式钉死；
    //    若将来 CI 部署配置了税率，本用例应恢复正向断言（已登记为残余风险交编排方）。
    const tag = Date.now().toString().slice(-6);
    const uniq = `E2EPF${tag}`;
    // 2098-07 专属期间（全仓 e2e grep 无其它用例在该期间造数），负例零落库可精确回读。
    const period = { period_year: 2098, period_month: 7 };

    // 负例1｜税目表外污染物（随机名保证必然未登记）→ 400 + VALIDATION_ERROR，
    // 只判 HTTP 状态 + 信封机器码，不断错误文案。
    const unregistered = await apiCallExpectFail(
      page,
      'POST',
      '/environmental-tax/discharge-records',
      {
        discharge_type: 'wastewater',
        pollutant_name: uniq,
        discharge_amount: '250',
        ...period,
      }
    );
    expect(unregistered.status, `表外污染物应 400 拒绝计税：${JSON.stringify(unregistered)}`).toBe(
      400
    );
    expect(
      failureCode(unregistered),
      `表外污染物机器码应为 VALIDATION_ERROR：${JSON.stringify(unregistered)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 负例2｜税目表内真实污染物（COD 法定当量 1kg）在当前部署「适用税额未配置」下
    // 同样必须 fail-visible 显式拒绝（400 + BUSINESS_ERROR），禁止静默算出"看似正常"的税额。
    const noRate = await apiCallExpectFail(page, 'POST', '/environmental-tax/discharge-records', {
      discharge_type: 'wastewater',
      pollutant_name: 'COD',
      discharge_amount: '100',
      ...period,
    });
    expect(noRate.status, `COD 在税额未配置部署下应 400：${JSON.stringify(noRate)}`).toBe(400);
    expect(failureCode(noRate), `COD 机器码应为 BUSINESS_ERROR：${JSON.stringify(noRate)}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // 回读证明零落库（fail-closed 拒绝不产生半行）：专属期间列表按数组信封显式断言，
    // 且不含本轮任何被拒探针（uniq 随机名 + COD/100kg 组合在负例下不可能入库）。
    const list = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/environmental-tax/discharge-records?period_year=2098&period_month=7'
    );
    expect(
      Array.isArray(list),
      `discharge-records GET 应返回数组，实际 ${JSON.stringify(list)}`
    ).toBe(true);
    expect(
      list.filter(it => it.pollutant_name === uniq).length,
      `表外污染物被拒后 ${uniq} 不得有任何落库行`
    ).toBe(0);
    expect(
      list.filter(it => it.pollutant_name === 'COD' && String(it.discharge_amount) === '100')
        .length,
      'COD 探针被税额门拒绝后不得落库（该专属期间无正向造数）'
    ).toBe(0);

    // 申报汇总同理为空聚合（generate_tax_declaration 读的是同一张零写入的期间表）
    const decl = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      '/environmental-tax/tax-declarations?period_year=2098&period_month=7'
    );
    expect(
      Array.isArray(decl),
      `tax-declarations GET 应返回数组，实际 ${JSON.stringify(decl)}`
    ).toBe(true);
    expect(decl.filter(it => it.pollutant_name === uniq).length, '申报表不应聚合出被拒污染物').toBe(
      0
    );

    // 负例3｜非法排放类型（validate_discharge_type 先于计税执行）→ 400 + BAD_REQUEST。
    const badType = await apiCallExpectFail(page, 'POST', '/environmental-tax/discharge-records', {
      discharge_type: 'radioactive',
      pollutant_name: 'COD',
      discharge_amount: '1',
      ...period,
    });
    expect(badType.status, `非法排放类型应 400：${JSON.stringify(badType)}`).toBe(400);
    expect(failureCode(badType), `非法排放类型机器码应为 BAD_REQUEST`).toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );

    // 负例4｜负排放量 → 400 + BAD_REQUEST（create_discharge_record 入参门）。
    const neg = await apiCallExpectFail(page, 'POST', '/environmental-tax/discharge-records', {
      discharge_type: 'wastewater',
      pollutant_name: 'COD',
      discharge_amount: '-5',
      ...period,
    });
    expect(neg.status, `负排放量应 400：${JSON.stringify(neg)}`).toBe(400);
    expect(failureCode(neg), `负排放量机器码应为 BAD_REQUEST`).toBe(APP_ERROR_CODES.BAD_REQUEST);

    // 负例5｜GET 缺必填 period_year/period_month（handler PeriodQuery 非 Option）
    // → 提取器拒绝统一信封 400 + VALIDATION_ERROR（error.rs::request_decoding_failed 族）。
    const noPeriod = await apiCallExpectFail(page, 'GET', '/environmental-tax/discharge-records');
    expect(noPeriod.status, `GET 缺 period 必填应 400：${JSON.stringify(noPeriod)}`).toBe(400);
    expect(failureCode(noPeriod), 'GET 缺必填参数机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });
});

/**
 * 自建一张「值可辨识」FOB 草稿报价单（qty=40 × 单价 12.5，税 0 → total 500），
 * 引用 ensureTestEntities 自建的报价专用产品（unit 取后端真实落库值，不写死）。
 * 说明：后端 quotations 域**未注册 DELETE /quotations/{id}**（routes/quotations.rs
 * 仅 GET/PUT/{id} 与动作端点），无硬删清理通道；用例留下的草稿/approved 报价单
 * 是本用例专属编号的可辨识残留（notes 带 E2E-61 前缀），不臆造删除端点，
 * 残留风险已在交付报告登记。返回报价单 id。
 */
async function seedFobQuotation(page: import('@playwright/test').Page): Promise<number> {
  const ctx = getCtx();
  if (!ctx.quotationProductId) throw new Error('前置缺失：ctx.quotationProductId 未就绪');
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  if (!ctx.userIds[0]) throw new Error('前置缺失：ctx.userIds[0] 未就绪');
  const res = await apiCall<{ id?: number; quotation_no?: string }>(page, 'POST', '/quotations', {
    customer_id: ctx.customerId,
    sales_user_id: ctx.userIds[0],
    quotation_date: new Date().toISOString().slice(0, 10),
    valid_until: new Date(Date.now() + 30 * 86400_000).toISOString().slice(0, 10),
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: false,
    tax_rate: '0',
    items: [
      {
        product_id: ctx.quotationProductId,
        unit: ctx.quotationProductUnit,
        quantity: '40',
        unit_price: '12.5',
        unit_price_with_tax: '12.5',
      },
    ],
    notes: `E2E-61-${Date.now().toString().slice(-6)}`,
  });
  const id = res.data?.id;
  if (!id) throw new Error(`报价单创建未返回 id：${JSON.stringify(res)}`);
  return id;
}
