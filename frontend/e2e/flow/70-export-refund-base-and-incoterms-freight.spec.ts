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
  expectBusinessRejection,
  APP_ERROR_CODES,
  type ApiFailureResult,
} from './helpers';

/**
 * 70 出口合规域深断言（续 61 号）：退税申报基数"只认已核验"收紧口径 + Incoterms 主运费口径全量词表
 *
 * 端点与 DTO 真实性（逐条对过当前源码，非臆造）：
 * - POST /export-refunds/customs-declarations                     routes/export_refund.rs:14-17（mod.rs:493 nest /api/v1/erp）
 *   入参 CreateCustomsDeclarationRequest（export_refund_service.rs:28-42：declaration_no/export_date/
 *   total_amount/exchange_rate 必填，sales_order_id 等 Option）；出参 export_customs_declaration::Model
 *   （models/export_customs_declaration.rs:14-46，rust_decimal 出参字符串：total_amount/exchange_rate）；
 *   创建恒 status='pending'（service:146 Set("pending")）。
 * - POST /export-refunds/refund-declarations                      routes/export_refund.rs:26-29；
 *   入参 GenerateRefundDeclarationRequest（export_refund_handler.rs:27-34：period_year/period_month/
 *   refund_rate/input_vat_amount/carryforward_from_prev 必填）；出参 export_refund_declaration::Model
 *   （models/export_refund_declaration.rs:16-53：export_sales_amount/refundable_vat_amount/
 *   exempt_vat_amount/credit_vat_amount/actual_refund_amount/carryforward_amount/refund_rate/
 *   documents_complete/status，decimal 列全部字符串序列化）。
 * - GET  /export-refunds/refund-declarations?period_year=&period_month=
 *   routes/export_refund.rs:30-33；出参 Vec<RefundModel> **裸数组**（handler:94-104 to_value(Vec)），
 *   非 PaginatedResponse——本域列表没有分页信封，禁止臆造 items/total。
 * - POST /incoterms/cost-calculation                              routes/incoterms.rs:17-20（mod.rs:494 nest）；
 *   入参 CalculateCostsRequest（incoterms_handler.rs:26-33：incoterm serde UPPERCASE 枚举 +
 *   product_cost Decimal 必填 + freight/insurance/duty Option<Decimal>）；出参固定四键
 *   {product_cost, freight_cost, insurance_cost, duty_cost}（handler:59-64），Option::None 序列化为 null。
 *   判定源 utils/incoterms.rs::includes_freight（:111-116，EXW/FCA/FAS/FOB=false）、
 *   includes_insurance（:99-104，仅 CIF/CIP/DDP）、requires_duty_paid（:119-121，仅 DDP）；
 *   services/incoterms_service.rs:114-140：不含主运费的术语把提交的 freight_cost 置 None 返回。
 *
 * 本文件钉住的修复判据（对应后端修复点，而非"页面能打开"）：
 * 1) 退税基数只认已核验（export_refund_service.rs:96-107 VERIFIED_STATUS +
 *    verified_export_sales_amount，生成申报 :285-294 聚合口径）：
 *    修复前实现把期间内**全部**报关行逐单 total_amount×exchange_rate 求和进基数；收紧后
 *    非 verified 行一律不进基数。API 可建的全部报关行恒 pending（service:146），故同月
 *    "多张含金额报关单 + 生成当期申报" 的可断真值 = **基数 0**——旧实现此处必得非 0 和值，
 *    本断言对新旧实现有真实判别力，不是 200 空断言。
 * 2) 统计区间半开区间 [月初, 次月初)（service:267-289 gte(period_start) + lt(period_end)）：
 *    基数=0 的输出形态下区间修复与收紧前实现同样"看起来 0"，判别力依赖正样本（verified 行）；
 *    见下方"CI 不可测声明"。断言按可判部分钉死，不夸大。
 * 3) Incoterms 运费口径（判据 2）：同一批货（同一入参组）逐 11 术语走真实端点，
 *    FOB/FCA/FAS/EXW 的主运费必须为 null（未计入），CFR/CIF/CPT/CIP/DAP/DPU/DDP 必须原值计入；
 *    修复前 includes_freight 对 FOB 返 true → FOB 响应 freight_cost=提交值，本断言即判别新旧实现。
 *
 * ⚠️ CI 不可测声明（判据 1 的"一张已核验"正样本）——本文件不造假、不用 optional 检查吞绿：
 * 当前源码全库**不存在**任何把报关单 status 置 verified 的端点（routes/export_refund.rs 仅
 * create/verification(只读)/calculation/declarations 生成与列表；grep 全 handlers 无
 * export_customs_declaration 状态更新路径），且外汇核销单**没有创建端点**
 * （CreateFxVerificationRequest 标 dead_code "预留：待接入"，export_refund_service.rs:44-47），
 * documents_complete 的 verified+verified 前提（service:176-215）在 HTTP 面上不可达。
 * 因此"同月一张未核验+一张已核验、基数只含已核验金额"的正向样本无法经 API seed，
 * 属"出口退税域无核验写端点"的域闭环缺口——已列入交付报告"待主编排立案"，
 * 待补端点后再追加正向用例；在此之前仅凭本文件断言不足以证明 verified 行会被计入。
 *
 * 隔离与残留：报关单与退税申报表在本域**均未注册 DELETE 端点**（routes/export_refund.rs 全量
 * 仅上列 6 条），无法自清；故全部断言取"残留安全"形态——基数 0 / 齐全 false 对同期间历史残留
 * 行同样成立（残留行同为 pending），申报表按本次创建返回的唯一 id 定位回读。
 * 专属期间 2097-05/2097-12（全仓 e2e grep 无其它用例占用 2097，61 号占用 2099-01/2098-07/2027）。
 */

type Row = Record<string, unknown>;

/** BiResponse 之外的本域出参就是裸 Model——列表为裸数组，缺键/变形即红，不兜底 */
function requireBareArray(data: unknown, endpoint: string): Row[] {
  expect(
    Array.isArray(data),
    `${endpoint}：出参 data 必须为裸数组（handler to_value(Vec<Model>)），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data as Row[];
}

function declNo(): string {
  return genCode('E2E70CD');
}

/** 创建一张报关单并断言 POST 真实响应即落库真值（status 必 pending、decimal 字符串归一） */
async function seedCustomsDeclaration(
  page: import('@playwright/test').Page,
  opts: { exportDate: string; totalAmount: string; exchangeRate: string }
): Promise<Row> {
  const no = declNo();
  const res = await apiCall<Row>(page, 'POST', '/export-refunds/customs-declarations', {
    declaration_no: no,
    sales_order_id: getCtx().salesOrderId,
    export_date: opts.exportDate,
    currency_code: 'USD',
    total_amount: opts.totalAmount,
    exchange_rate: opts.exchangeRate,
  });
  const d = res.data;
  expect(d, `报关单创建应返回模型：${JSON.stringify(res)}`).toBeTruthy();
  expect(Number(d?.id), '报关单创建应回 id').toBeGreaterThan(0);
  expect(d?.declaration_no, `回显报关单号=${no}`).toBe(no);
  expect(d?.export_date, `落库出口日期=${opts.exportDate}`).toBe(opts.exportDate);
  expect(Number(d?.total_amount), `落库原币金额=${opts.totalAmount}`).toBe(
    Number(opts.totalAmount)
  );
  expect(Number(d?.exchange_rate), `落库汇率=${opts.exchangeRate}`).toBe(Number(opts.exchangeRate));
  expect(d?.status, '新建报关单必须为 pending（service:146 唯一写入口径）').toBe('pending');
  return d as Row;
}

/**
 * 断言精确拒绝契约（收紧原「任意 4xx、只判区间」的假绿）：
 * - HTTP 必须恰为 400；机器码逐条等于调用点期望族。
 * 本文件两个调用点都是 **提取器层拒绝**（incoterm 为 serde UPPERCASE 枚举
 * backend/src/utils/incoterms.rs::Incoterms2020；CalculateCostsRequest.product_cost 必填非
 * Option，handlers/incoterms_handler.rs），serde 拒绝由
 * middleware/trace_context.rs::normalize_extractor_rejection 收进统一信封
 * 400 + code=VALIDATION_ERROR。永不判文案（serde 原文只进后端日志）。
 */
function expectRejected4xx(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${JSON.stringify(r)}`).toBe(400);
  expect(failureCode(r), `${what}：机器码应为 ${expectedCode}，实际 ${JSON.stringify(r)}`).toBe(
    expectedCode
  );
}

test.describe.serial('70 退税申报基数收紧 + Incoterms 主运费口径全量判定', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('70-01 同月多张 pending 报关单+下界前/上界日各一张 → 2097-05 申报基数必须 0（只认 verified 收紧口径）', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.salesOrderId) throw new Error('前置缺失：ctx.salesOrderId 未就绪');

    // 同月两张（人民币口径 2000 与 750）：修复前实现会全部计入基数=2750，收紧后必须 0
    await seedCustomsDeclaration(page, {
      exportDate: '2097-05-10',
      totalAmount: '1000',
      exchangeRate: '2',
    });
    await seedCustomsDeclaration(page, {
      exportDate: '2097-05-20',
      totalAmount: '500',
      exchangeRate: '1.5',
    });
    // 区间边界哨兵：次月月初（半开区间上界，应 excluded）与当月前一天（下界前，应 excluded）。
    // 注意：基数 0 对"仅下界 gte"的旧区间实现同样成立（这些行还差 status 关），
    // 本组哨兵的价值在残留安全 + 与判据 1 叠加后期间+状态双闸门回归时能立刻显形。
    await seedCustomsDeclaration(page, {
      exportDate: '2097-06-01',
      totalAmount: '999',
      exchangeRate: '3',
    });
    await seedCustomsDeclaration(page, {
      exportDate: '2097-04-30',
      totalAmount: '888',
      exchangeRate: '4',
    });

    const gen = await apiCall<Row>(page, 'POST', '/export-refunds/refund-declarations', {
      period_year: 2097,
      period_month: 5,
      refund_rate: '0.13',
      input_vat_amount: '100',
      carryforward_from_prev: '50',
    });
    const d = gen.data;
    const declId = Number(d?.id);
    expect(declId, `生成申报表应回 id：${JSON.stringify(gen)}`).toBeGreaterThan(0);
    // —— 根因锚点：基数只认 verified，全部 pending → 0（修复前=1000×2+500×1.5=2750）——
    expect(
      Number(d?.export_sales_amount),
      '未核验报关单不进申报基数，export_sales_amount 必须=0'
    ).toBe(0);
    expect(Number(d?.refundable_vat_amount), '免抵退税额=0×0.13=0').toBe(0);
    expect(Number(d?.actual_refund_amount), '应退=min(0, 100+50)=0').toBe(0);
    expect(Number(d?.exempt_vat_amount), '免抵=0-0=0').toBe(0);
    expect(Number(d?.carryforward_amount), '结转=max(0,100+50-0)=150').toBe(150);
    expect(Number(d?.credit_vat_amount), '当期进项回显=100').toBe(100);
    expect(Number(d?.refund_rate), '退税率回显=0.13').toBe(0.13);
    expect(d?.documents_complete, '仅 pending 报关单 → documents_complete 必须 false').toBe(false);
    expect(d?.status, '新建申报表=draft（service:341）').toBe('draft');
    expect(String(d?.declaration_no), '申报表号期间前缀 ERD-209705-').toMatch(/^ERD-209705-\d{5}$/);

    // 写后 GET 回读（裸数组信封显式断言 + 按唯一 id 定位，禁 ?? [] 兜底）
    const listEp = '/export-refunds/refund-declarations?period_year=2097&period_month=5';
    const list = requireBareArray(await apiCallRaw(page, 'GET', listEp), listEp);
    const row = list.find(it => Number(it.id) === declId);
    expect(row, `GET 期间过滤应命中申报表 id=${declId}，实际 ${JSON.stringify(list)}`).toBeTruthy();
    expect(Number(row?.export_sales_amount), '列表回读基数同样必须=0').toBe(0);
    expect(Number(row?.carryforward_amount), '列表回读结转=150').toBe(150);
    expect(row?.documents_complete, '列表回读齐全=false').toBe(false);
  });

  test('70-02 申报期间非法值（month=13/0）业务拒绝+拒绝后无痕；month=12 跨年上界合法', async ({
    page,
  }) => {
    const genBody = (month: number) => ({
      period_year: 2097,
      period_month: month,
      refund_rate: '0.13',
      input_vat_amount: '0',
      carryforward_from_prev: '0',
    });

    // 非法月份 13：service:262-266 business_displayable（只回显用户自提输入）→ HTTP 400 + BUSINESS_ERROR + 可读原因
    const bad13 = await apiCallExpectFail(page, 'POST', '/export-refunds/refund-declarations', {
      ...genBody(13),
    });
    expectBusinessRejection(bad13, 'period_month=13 应被业务拒绝（400+BUSINESS_ERROR+原因外显）');
    expect(failureCode(bad13), '机器码=BUSINESS_ERROR').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      String(bad13.message),
      `拒绝原因应可外显定位到月份非法，实际 ${JSON.stringify(bad13)}`
    ).toContain('月份非法');

    // 非法月份 0（下界外）同样拒绝
    const bad0 = await apiCallExpectFail(page, 'POST', '/export-refunds/refund-declarations', {
      ...genBody(0),
    });
    expectBusinessRejection(bad0, 'period_month=0 应被业务拒绝');

    // 被拒的写回读断无痕：month=13 期间 GET 必须空数组（GET 过滤同样接 13，聚合不到任何表）
    const ghostList = requireBareArray(
      await apiCallRaw(
        page,
        'GET',
        '/export-refunds/refund-declarations?period_year=2097&period_month=13'
      ),
      'GET refund-declarations?period_month=13'
    );
    expect(ghostList.length, '被拒期间不得留下任何申报表（无痕）').toBe(0);

    // 合法上界 month=12：次月滚入下年 1 月（service:273-283 跨年分支），生成成功且全零链
    const dec = await apiCall<Row>(page, 'POST', '/export-refunds/refund-declarations', {
      ...genBody(12),
    });
    const decId = Number(dec.data?.id);
    expect(decId, `2097-12 申报表应生成：${JSON.stringify(dec)}`).toBeGreaterThan(0);
    expect(Number(dec.data?.export_sales_amount), '2097-12 无报关数据 → 基数=0').toBe(0);
    expect(Number(dec.data?.carryforward_amount), '进项/留抵皆 0 → 结转=0').toBe(0);
    expect(dec.data?.documents_complete, '无单证 → 齐全=false').toBe(false);
    expect(String(dec.data?.declaration_no), '申报表号前缀 ERD-209712-').toMatch(
      /^ERD-209712-\d{5}$/
    );
    const decList = requireBareArray(
      await apiCallRaw(
        page,
        'GET',
        '/export-refunds/refund-declarations?period_year=2097&period_month=12'
      ),
      'GET refund-declarations?period_month=12'
    );
    expect(
      decList.find(it => Number(it.id) === decId),
      `2097-12 列表应回读 id=${decId}`
    ).toBeTruthy();
  });

  test('70-03 同一批货 11 术语全量运费口径：EXW/FCA/FAS/FOB 主运费必须 null，其余原值计入（CIF 对照 FOB）', async ({
    page,
  }) => {
    // 一组真实"货"：货款 8000.25 / 主运费 1200.50 / 保险 300.25 / 关税 500.50
    // （全部二进制精确可表示的小数，聚合断言用 === 精确相等，不做容差放宽）
    const cargo = {
      product_cost: '8000.25',
      freight_cost: '1200.50',
      insurance_cost: '300.25',
      duty_cost: '500.50',
    };
    const NO_FREIGHT = ['EXW', 'FCA', 'FAS', 'FOB'];
    const WITH_INSURANCE = ['CIF', 'CIP', 'DDP'];
    const ALL_TERMS = ['EXW', 'FCA', 'CPT', 'CIP', 'DAP', 'DPU', 'DDP', 'FAS', 'FOB', 'CFR', 'CIF'];

    const results: Record<string, Row> = {};
    for (const term of ALL_TERMS) {
      const r = await apiCallRaw<Row>(page, 'POST', '/incoterms/cost-calculation', {
        incoterm: term,
        ...cargo,
      });
      expect(r, `cost-calculation(${term}) 应返回四键对象，实际 ${JSON.stringify(r)}`).toBeTruthy();
      expect(Number(r.product_cost), `${term}：货款恒定计入`).toBe(8000.25);
      const expectFreight = NO_FREIGHT.includes(term) ? null : 1200.5;
      if (expectFreight === null) {
        expect(
          r.freight_cost,
          `${term}：includes_freight=false（utils/incoterms.rs:111-116），主运费必须判为未计入（null），实际 ${JSON.stringify(r)}`
        ).toBeNull();
      } else {
        expect(
          Number(r.freight_cost),
          `${term}：含主运费术语应原值 1200.50 计入，实际 ${JSON.stringify(r)}`
        ).toBe(expectFreight);
      }
      const expectInsurance = WITH_INSURANCE.includes(term) ? 300.25 : null;
      if (expectInsurance === null) {
        expect(r.insurance_cost, `${term}：不含保险术语必须 null`).toBeNull();
      } else {
        expect(Number(r.insurance_cost), `${term}：保险应原值计入`).toBe(expectInsurance);
      }
      if (term === 'DDP') {
        expect(Number(r.duty_cost), 'DDP 关税计入').toBe(500.5);
      } else {
        expect(r.duty_cost, `${term}：仅 DDP 含关税`).toBeNull();
      }
      results[term] = r;
    }

    // 判据 2 的字面要求：同一批货 FOB vs CIF 成对对照（修复前 FOB 会错误带上 1200.50）
    expect(results.FOB.freight_cost, 'FOB 同货对照：主运费未计入（归买方订立）').toBeNull();
    expect(Number(results.CIF.freight_cost), 'CIF 同货对照：主运费原值计入').toBe(1200.5);
    expect(Number(results.FOB.product_cost), '两术语货款口径一致').toBe(
      Number(results.CIF.product_cost)
    );
  });

  test('70-04 成本计算非法值/缺省边界：小写术语拒绝、缺 product_cost 拒绝、CIF 未提交运费→null（与显式 0 区分）', async ({
    page,
  }) => {
    // serde rename_all=UPPERCASE（utils/incoterms.rs::Incoterms2020）：'fob' 不是合法 token，
    // Json 反序列化失败 → trace_context.rs 归一 400 + VALIDATION_ERROR
    const lower = await apiCallExpectFail(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'fob',
      product_cost: '100',
    });
    expectRejected4xx(
      lower,
      "小写 incoterm 'fob'（词表只认大写）",
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // 必填 product_cost 缺失 → 反序列化拒绝（不许静默按 0 计算），同一提取器归一族
    const missing = await apiCallExpectFail(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'CIF',
      freight_cost: '30',
    });
    expectRejected4xx(missing, '缺必填 product_cost', APP_ERROR_CODES.VALIDATION_ERROR);

    // Option 缺省语义：CIF 含运费但未提交 freight_cost → null（"没报运费"），
    // 与显式 '0'（"报了 0"）可区分；顺带钉住 0 值原样透传
    const cifBare = await apiCallRaw<Row>(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'CIF',
      product_cost: '100',
    });
    expect(cifBare.freight_cost, 'CIF 未提交运费 → null 而非 0/缺键').toBeNull();
    expect(Number(cifBare.product_cost), '货款 100 恒定计入').toBe(100);
    const cifZero = await apiCallRaw<Row>(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'CIF',
      product_cost: '100',
      freight_cost: '0',
    });
    expect(Number(cifZero.freight_cost), '显式 0 运费应原样透传 0（与 null 区分）').toBe(0);

    // FOB 提交运费被置 null 是"判据 2 的机制本体"：入参非空但出参 null（service:121-126）
    const fob = await apiCallRaw<Row>(page, 'POST', '/incoterms/cost-calculation', {
      incoterm: 'FOB',
      product_cost: '100',
      freight_cost: '1200.50',
    });
    expect(
      fob.freight_cost,
      `FOB 收到非空运费入参仍必须置 null（includes_freight 门控生效），实际 ${JSON.stringify(fob)}`
    ).toBeNull();
  });
});
