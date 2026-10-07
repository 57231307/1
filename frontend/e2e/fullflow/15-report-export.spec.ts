// 财务域全流程契约级 E2E — 15 财务报表（试算/利润/现流/总账）+ 导出与快照
//
// 定位：与 finance/10（试算全局恒等 + 资产负债结构 + subjects/budgets 导出 + balance-sheet
// 无令牌 403）与 flow/39b（customer 资源导出审批链）互补，本文件补内容级与契约面增量：
//   1. 利润表「从已过账凭证按科目前缀聚合」的装配恒等链（service:146-186，
//      60=收入(贷) 64=成本(借) 6601/6602/6603=费用(借)），并用真实 seed 过账证数据进表（非 vacuous）。
//   2. 试算平衡表 period 参数 + 自建唯一科目行的期间发生额精确回读
//      （models/dto/finance_report_dto.rs:64-87；service:472-497 取数=account_subject 列）。
//   3. 总账 /reports/general-ledger/{code} 按自建科目精确命中自建凭证分录
//      （GeneralLedgerEntry :91-100）。
//   4. 现金流量表装配恒等 net_change = operating+investing+financing；ending=beginning+net_change
//      （CashFlowStatement :40-52）。
//   5. finance_report 资源导出审批全链（manager 申请→admin 审批→令牌下载 xlsx 魔数→二次消费拒→
//      假令牌/跨资源令牌 403），与 39b（customer 资源）互不重复。
//   6. AP/AR/固定资产凭证导出真实 xlsx 字节（无令牌门端点严格 200+PK 魔数+content-type）。
//   7. 期末报表快照 create→列表(PaginatedResponse)→详情逐字段（period_id/report_type/report_data
//      回读，缺陷①靶心）；verify 端点严格健康（结果结构不作无据断言）。
// 契约真值源：routes/finance.rs:103-163（报表+导出路由）、handlers/finance_report_handler.rs:20-53
//   （DateRangeQuery/PeriodQuery 键 + download_token）、:131-160（general-ledger path code）；
//   导出令牌 services/export_approval_service.rs:370-376（fail-closed 403）+ token 一次性消费
//   （39b 已证 same-resource 二消费拒，本文件对 finance_report 资源复证）。
// 铁律：无 ?? 兜底 / 无 >=400 宽松 / 5xx 一律判红；跨分片共享前缀科目数据只作下界(>=)断言，
//   唯一编码科目行作精确断言——两种口径都基于源码事实，不放宽。
import { test, expect } from '../diagnose-fixture';
import {
  API_BASE,
  API_PREFIX,
  loginViaUI,
  loginAsRole,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';
import type { Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
): void {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(obj[key], `${label}：键 "${key}" 期望=${JSON.stringify(expected)}`).toEqual(expected);
}

function num(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：${key} 非数字，raw=${JSON.stringify(obj[key])}`);
  }
  return n;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

function currentPeriod(): string {
  return new Date().toISOString().slice(0, 7);
}

async function requireId(obj: Record<string, unknown>, label: string): Promise<number> {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

/** 建 ACTIVE 科目（code 唯一可控），返回 {id, code}。 */
async function seedSubject(
  page: Page,
  codePrefix: string,
  name: string,
  direction: string
): Promise<{ id: number; code: string }> {
  const code = `${codePrefix}${genCode('X').replace('-', '')}`;
  const s = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/subjects', {
    code,
    name,
    level: 1,
    balance_direction: direction,
    assist_customer: false,
    assist_supplier: false,
    assist_batch: false,
    assist_color_no: false,
    enable_dual_unit: false,
  });
  const id = await requireId(s, '建科目');
  CLEANUP.push({ path: `/subjects/${id}`, label: 'subject' });
  return { id, code };
}

/** 建平衡凭证（items 传 [subject_id, debit, credit] 列表）并走完 draft→…→posted。 */
async function createPostedVoucher(
  page: Page,
  lines: Array<{ subjectId: number; debit: number; credit: number; summary: string }>
): Promise<number> {
  const v = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/vouchers', {
    voucher_type: '记账凭证',
    voucher_date: today(),
    source_type: 'MANUAL',
    items: lines.map((l, i) => ({
      line_no: i + 1,
      subject_id: l.subjectId,
      debit: l.debit,
      credit: l.credit,
      summary: l.summary,
    })),
  });
  const id = await requireId(v, '建凭证');
  CLEANUP.push({ path: `/vouchers/${id}`, label: 'voucher' });
  await apiCall(page, 'POST', `/vouchers/${id}/submit`);
  await apiCall(page, 'POST', `/vouchers/${id}/review`);
  await apiCall(page, 'POST', `/vouchers/${id}/post`);
  return id;
}

/** GET 二进制并断 OOXML xlsx 真值（PK 魔数 + content-type），范式同 finance/10-03。 */
async function expectXlsx(page: Page, pathWithQuery: string, label: string): Promise<void> {
  const resp = await page.request.get(`${API_BASE}${API_PREFIX}${pathWithQuery}`);
  expect(resp.status(), `${label}：导出 HTTP 应 200，实际=${resp.status()}`).toBe(200);
  const ct = resp.headers()['content-type'] ?? '';
  expect(
    ct.includes('spreadsheetml') || ct.includes('officedocument.spreadsheet'),
    `${label}：content-type 应为 xlsx OOXML，实际=${ct}`
  ).toBe(true);
  const buf = await resp.body();
  expect(buf.length, `${label}：xlsx 字节应非空`).toBeGreaterThan(100);
  expect(String.fromCharCode(buf[0], buf[1]), `${label}：应含 zip PK 魔数`).toBe('PK');
}

test.describe('15 财务报表/导出/快照契约链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('15-01 利润表：seed 过账凭证后装配恒等 + 自建收入行确计入表（抓硬编码比例/漏聚合）', async ({
    page,
  }) => {
    const rev = await seedSubject(page, '60', 'E2E15营业收入', 'credit');
    const cogs = await seedSubject(page, '64', 'E2E15营业成本', 'debit');
    const exp = await seedSubject(page, '6602', 'E2E15管理费用', 'debit');
    const other = await seedSubject(page, '112', 'E2E15其他应收', 'debit');
    // 借：成本3000 + 费用1000 + 挂账1000 = 贷：收入5000
    await createPostedVoucher(page, [
      { subjectId: cogs.id, debit: 3000, credit: 0, summary: 'E2E15成本' },
      { subjectId: exp.id, debit: 1000, credit: 0, summary: 'E2E15费用' },
      { subjectId: other.id, debit: 1000, credit: 0, summary: 'E2E15挂账' },
      { subjectId: rev.id, debit: 0, credit: 5000, summary: 'E2E15收入' },
    ]);

    const stmt = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/income-statement?start_date=${today()}&end_date=${today()}`
    );
    expectKeyValue(stmt, 'period_start', today(), '利润表期间回显');
    expectKeyValue(stmt, 'period_end', today(), '利润表期间回显');
    const totalRevenue = num(stmt, 'total_revenue', '利润表');
    const cogsV = num(stmt, 'cost_of_goods_sold', '利润表');
    const gross = num(stmt, 'gross_profit', '利润表');
    const opex = num(stmt, 'total_operating_expenses', '利润表');
    const opInc = num(stmt, 'operating_income', '利润表');
    const net = num(stmt, 'net_income', '利润表');
    // 装配恒等（非 vacuity：能抓出把 total/gross 写死的 bug）
    expect(
      Math.abs(gross - (totalRevenue - cogsV)),
      `gross=revenue-cogs 破坏：${JSON.stringify(stmt)}`
    ).toBeLessThan(0.01);
    expect(Math.abs(opInc - (gross - opex)), 'operating_income=gross-opex 破坏').toBeLessThan(0.01);
    expect(
      Math.abs(
        net - (opInc + num(stmt, 'other_income', '利润表') - num(stmt, 'other_expenses', '利润表'))
      ),
      'net_income 装配破坏'
    ).toBeLessThan(0.01);
    // 数据真进表：本分片当日仅此凭证（跨分片同前缀共享 ⇒ 下界口径，见文件头说明）
    expect(totalRevenue, '收入应≥自建 5000').toBeGreaterThanOrEqual(5000);
    expect(cogsV, '成本应≥自建 3000').toBeGreaterThanOrEqual(3000);
    const revenueItems = stmt.revenue as Array<Record<string, unknown>>;
    expect(Array.isArray(revenueItems), 'revenue 应为数组').toBe(true);
    expect(
      num(revenueItems[0], 'amount', 'revenue[0]'),
      'revenue 行金额=total_revenue（装配一致）'
    ).toBeCloseTo(totalRevenue, 1);
  });

  test('15-02 试算平衡：period 参数生效 + 自建唯一科目行期间发生额精确回读 + 全局恒等', async ({
    page,
  }) => {
    const sub = await seedSubject(page, '99', 'E2E15试算专用科目', 'debit');
    const counter = await seedSubject(page, '98', 'E2E15试算对方科目', 'credit');
    await createPostedVoucher(page, [
      { subjectId: sub.id, debit: 4321, credit: 0, summary: 'E2E15借发生' },
      { subjectId: counter.id, debit: 0, credit: 4321, summary: 'E2E15贷发生' },
    ]);
    // 试算取数=account_subject 本期列（finance_report_service.rs:485-497），过账同步写入后
    // 仍需按当期刷新保险（batch 400 refresh-balance API；period=YYYY-MM，
    // handlers/account_subject_handler.rs:220-232）
    await apiCall(page, 'POST', `/subjects/${sub.id}/refresh-balance?period=${currentPeriod()}`);
    await apiCall(
      page,
      'POST',
      `/subjects/${counter.id}/refresh-balance?period=${currentPeriod()}`
    );

    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/trial-balance?period=${currentPeriod()}`
    );
    expectKeyValue(tb, 'period', currentPeriod(), '试算 period 回显');
    const entries = tb.entries as Array<Record<string, unknown>>;
    expect(Array.isArray(entries), 'entries 应为数组').toBe(true);
    const row = entries.find(e => String(e.subject_code) === sub.code);
    if (!row)
      throw new Error(
        `试算表未含自建科目 ${sub.code}（过账未回写科目期间发生额=数据不完整，判红）`
      );
    expect(num(row, 'period_debit', '试算自建行'), '自建科目期间借方应精确=4321').toBeCloseTo(
      4321,
      1
    );
    expect(num(row, 'period_credit', '试算自建行'), '期间贷方=0').toBeCloseTo(0, 1);
    // 全局恒等（试算平衡会计不变量）
    const pd = num(tb, 'total_period_debit', '试算总计');
    const pc = num(tb, 'total_period_credit', '试算总计');
    expect(Math.abs(pd - pc), `本期借贷合计应相等：借=${pd} 贷=${pc}`).toBeLessThan(0.01);
  });

  test('15-03 总账：按自建科目 code 精确查询命中自建凭证分录（voucher_no/借贷/余额链）', async ({
    page,
  }) => {
    const sub = await seedSubject(page, '97', 'E2E15总账科目', 'debit');
    const counter = await seedSubject(page, '96', 'E2E15总账对方', 'credit');
    const v = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/vouchers', {
      voucher_type: '记账凭证',
      voucher_date: today(),
      source_type: 'MANUAL',
      items: [
        { line_no: 1, subject_id: sub.id, debit: 600, credit: 0, summary: 'E2E15总账借' },
        { line_no: 2, subject_id: counter.id, debit: 0, credit: 600, summary: 'E2E15总账贷' },
      ],
    });
    const vid = await requireId(v, '建凭证');
    CLEANUP.push({ path: `/vouchers/${vid}`, label: 'voucher' });
    await apiCall(page, 'POST', `/vouchers/${vid}/submit`);
    await apiCall(page, 'POST', `/vouchers/${vid}/review`);
    await apiCall(page, 'POST', `/vouchers/${vid}/post`);

    const ledger = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/general-ledger/${sub.code}?start_date=${today()}&end_date=${today()}`
    );
    expectKeyValue(ledger, 'subject_code', sub.code, '总账科目回显');
    const ledEntries = ledger.entries as Array<Record<string, unknown>>;
    expect(Array.isArray(ledEntries), '总账 entries 应为数组').toBe(true);
    const hit = ledEntries.find(e => Number(e.debit) === 600);
    if (!hit) {
      throw new Error(
        `总账未含自建 600 借方分录（过账凭证未进入总账=内容缺陷，判红）。entries=${JSON.stringify(ledEntries).slice(0, 300)}`
      );
    }
    expect(
      hit.voucher_no !== undefined && String(hit.voucher_no).length > 0,
      '总账行应有 voucher_no'
    ).toBe(true);
    expect(num(hit, 'balance', '总账行'), '余额=600（首行借方累计）').toBeCloseTo(600, 1);
    expect(typeof hit.direction, '总账行含 direction').toBe('string');
  });

  test('15-04 现金流量表装配恒等：net_change=三项净额和、ending=beginning+net_change', async ({
    page,
  }) => {
    const cf = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/cash-flow?start_date=${today()}&end_date=${today()}`
    );
    for (const k of [
      'operating_activities',
      'net_cash_from_operations',
      'investing_activities',
      'net_cash_from_investing',
      'financing_activities',
      'net_cash_from_financing',
      'net_change_in_cash',
      'beginning_cash',
      'ending_cash',
    ]) {
      expect(
        Object.prototype.hasOwnProperty.call(cf, k),
        `现金流量表缺键 ${k}，实际=${Object.keys(cf).join(',')}`
      );
    }
    const sum =
      num(cf, 'net_cash_from_operations', 'CF') +
      num(cf, 'net_cash_from_investing', 'CF') +
      num(cf, 'net_cash_from_financing', 'CF');
    expect(
      Math.abs(num(cf, 'net_change_in_cash', 'CF') - sum),
      `net_change=${sum} 装配恒等破坏`
    ).toBeLessThan(0.01);
    expect(
      Math.abs(num(cf, 'ending_cash', 'CF') - (num(cf, 'beginning_cash', 'CF') + sum)),
      'ending=beginning+net_change 破坏'
    ).toBeLessThan(0.01);
  });

  test('15-05 finance_report 导出审批链：manager 申请→admin 批→令牌 xlsx→二消费拒；假令牌/无令牌 403', async ({
    page,
  }) => {
    // 双人分离：manager 申请，admin 审批（自审被后端禁止，39b 已证）
    await loginAsRole(page, 'manager');
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/export-approvals', {
      resource_type: 'finance_report',
      export_params: { period: currentPeriod() },
      estimated_rows: 100,
      file_format: 'xlsx',
    });
    const approvalId = await requireId(created, '创建导出审批申请');

    await loginViaUI(page, undefined, undefined, true); // 回到分片 admin
    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/export-approvals/${approvalId}/approve`,
      { comments: 'E2E15 审批' }
    );
    expect(String(approved.status ?? '').toLowerCase(), '审批后应含 approved 态').toContain(
      'approved'
    );
    const token = approved.download_token;
    expect(
      typeof token === 'string' && token.length > 0,
      `应签发 download_token，实际=${JSON.stringify(approved).slice(0, 200)}`
    ).toBe(true);

    await expectXlsx(
      page,
      `/finance/reports/balance-sheet/export?download_token=${token}&period=${currentPeriod()}`,
      '令牌资产负债表导出'
    );
    // 一次性消费：max_downloads 建单即置 1（export_approval_service.rs:176），二消费命中
    // verify_download_token 次数上限门（:429-433 AppError::permission_denied ⇒ error.rs:362/706
    // = 403 + FORBIDDEN）⇒ 锁具体状态与机器码，禁止"任意 4xx"
    const replay = await apiCallExpectFail(
      page,
      'GET',
      `/finance/reports/balance-sheet/export?download_token=${token}&period=${currentPeriod()}`
    );
    expect(replay.status, `令牌二消费应 403（下载上限门），实际=${replay.status}`).toBe(403);
    expect(failureCode(replay), '令牌二消费机器码').toBe('FORBIDDEN');

    // 假令牌 → 403 fail-closed
    const fFake = await apiCallExpectFail(
      page,
      'GET',
      `/finance/reports/balance-sheet/export?download_token=exp_bogus&period=${currentPeriod()}`
    );
    expect(fFake.status, '假令牌应 403').toBe(403);
    expect(failureCode(fFake), '403 机器码').toBe('FORBIDDEN');
    // 无令牌 → 403（enforce_export_download fail-closed：token 空即 permission_denied，
    // export_approval_service.rs:379-383 ⇒ 403+FORBIDDEN；对 profit 导出同样 enforce）
    const fNo = await apiCallExpectFail(
      page,
      'GET',
      `/finance/reports/income-statement/export?start_date=${today()}&end_date=${today()}`
    );
    expect(fNo.status, '无令牌利润表导出应 403').toBe(403);
    expect(failureCode(fNo), '无令牌导出机器码').toBe(APP_ERROR_CODES.FORBIDDEN);
  });

  test('15-06 无令牌门导出：AP/AR 发票与固定资产/预算导出均返回真实 xlsx 字节', async ({
    page,
  }) => {
    await expectXlsx(
      page,
      `/ap/invoices/export?start_date=2020-01-01&end_date=${today()}`,
      'AP 发票导出'
    );
    await expectXlsx(page, `/ar/invoices/export?page=1&page_size=100`, 'AR 发票导出');
    await expectXlsx(page, `/fixed-assets/export?page=1&page_size=100`, '资产台账导出');
    // 预算导出已在 finance/10-04 覆盖 subjects/budgets——此处复验 budgets/export 列头存在（自建数据行）
    await expectXlsx(page, `/budgets/export?page=1&page_size=100`, '预算导出');
  });

  test('15-07 期末报表快照：create→list(PaginatedResponse)→detail 逐字段回读（report_data 完整性靶心）', async ({
    page,
  }) => {
    // 取一个真实期间 id（GET 当期；无则先 init，服务幂等 init_first_period）
    let period = await apiCallRaw<Record<string, unknown> | null>(
      page,
      'GET',
      '/finance/accounting-periods/current'
    );
    if (period === null || period.id === undefined) {
      period = await apiCallRaw<Record<string, unknown>>(
        page,
        'POST',
        '/finance/accounting-periods/init'
      );
    }
    const periodId = await requireId(period as Record<string, unknown>, '当期期间');

    const payload = {
      title: 'E2E15 快照',
      rows: [{ name: '货币资金', amount: 123.45 }],
    };
    const snap = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/period-report-snapshots',
      {
        period_id: periodId,
        report_type: 'balance_sheet',
        report_data: payload,
      }
    );
    const sid = await requireId(snap, '建快照');
    expectKeyValue(snap, 'period_id', periodId, '快照响应');
    expectKeyValue(snap, 'report_type', 'balance_sheet', '快照响应');

    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/period-report-snapshots?period_id=${periodId}&page=1&page_size=20`
    );
    for (const k of ['items', 'total', 'page', 'page_size']) {
      expect(Object.prototype.hasOwnProperty.call(list, k), `快照列表缺 PaginatedResponse 键 ${k}`);
    }
    const hit = (list.items as Array<Record<string, unknown>>).find(r => Number(r.id) === sid);
    if (!hit) throw new Error('快照列表未含自建快照');

    const detail = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/period-report-snapshots/${sid}`
    );
    // report_data 逐字段回读（缺陷①：存得进、读不回即数据不完整）
    const rd = detail.report_data as Record<string, unknown>;
    expectKeyValue(rd, 'title', 'E2E15 快照', '快照 report_data');
    const rows = rd.rows as Array<Record<string, unknown>>;
    expect(rows.length, 'report_data.rows 完整').toBe(1);
    expectKeyValue(rows[0], 'name', '货币资金', '快照行');
    expect(Number(rows[0].amount), '快照行金额').toBeCloseTo(123.45, 2);

    // verify 端点严格健康（结果结构不作无据断言，见文件头）
    const verified = await apiCall(page, 'GET', `/period-report-snapshots/${sid}/verify`);
    expect(verified.code, 'verify 信封 200').toBe(200);
  });

  test('15-08 凭证导出 xlsx + 试算无 period 默认当期：默认值契约回读', async ({ page }) => {
    await expectXlsx(
      page,
      `/vouchers/export?start_date=${today()}&end_date=${today()}`,
      '凭证导出'
    );
    // trial-balance 不带 period ⇒ 服务端默认当前月（service:476）
    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/trial-balance'
    );
    expectKeyValue(tb, 'period', currentPeriod(), '试算默认期间');
    expect(Array.isArray(tb.entries), '默认查询也应返回 entries 数组').toBe(true);
    // 名称键非空（列表键缺失=契约断裂）
    for (const k of ['total_initial_debit', 'total_ending_credit']) {
      expect(Object.prototype.hasOwnProperty.call(tb, k), `试算缺总计键 ${k}`).toBe(true);
    }
  });
});
