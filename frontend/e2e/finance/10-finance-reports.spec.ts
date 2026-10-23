// 财务管理 E2E 套件 — 10 财务报表：底层数据一致性 + 恒等式 + 真 xlsx 导出字节 + 导出令牌 fail-closed
//
// 断真实数值/字节，禁 toBeTruthy / verifyEndpointHealthy / 小于 500 即绿 / 仅 toast。
// 三条数据口径（决定断言能写到什么强度，均以 backend 源码为准）：
//   - 试算平衡 get_trial_balance 读 account_subjects 的六列余额，且只取 status 为小写 active
//     的科目（master_data 词表）；余额列由 refresh-balance 从 status 为 posted 的凭证分录重算，
//     故用例必须先过账再刷新，否则报表读到的是主数据旧值。
//   - 总账 get_general_ledger 按 subject_code 与日期区间取 voucher_items 分录，不筛凭证状态，
//     并给出 direction（大写 DEBIT/CREDIT，写入方＝finance_report_service.rs 的 build_general_ledger_entries）
//     与逐笔累计 balance。用例对被 seed 的科目同时比对两条路径，即为报表数字与底层数据的一致性检查。
//   - 资产负债表 get_balance_sheet 按科目编码前缀取已过账分录的借贷差额聚合为固定 4 资产行 +
//     2 负债行（货币资金＝前缀 1001+1002，应收账款 1122，存货前缀 14，固定资产前缀 16，
//     应付账款 2202，预收账款 2203），所有者权益按 total_assets - total_liabilities 定义式装配。
// 空洞通过防线：试算平衡与资产负债表都先构造非零数据（seed 唯一科目 + 过账 + 刷新余额）再断言，
//   空库下 0 比 0 的恒等式不构成证据；被 seed 的科目编码唯一（genCode 前缀），
//   逐笔与余额可按科目隔离，因此精确金额断言不受同库并行分片干扰。
// 残留限制（如实登记，不靠放宽断言掩盖）：资产负债表的行为前缀聚合值，同库并行的其他套件会向
//   同一前缀（如 1001 库存现金）过账，聚合行无法钉死数值下界，故本套件对聚合行只断恒等式与装配
//   一致性（total 等于分项之和、资产等于负债加权益、行名与行数、报表日期），把可钉死的精确金额
//   放在唯一科目的总账与试算平衡两条路径上比对。
// 前置构造：会计期间不依赖 POST /finance/accounting-periods/init —— init_first_period 命中
//   任一 OPEN 期间即提前返回（get_current_period 取 year+period 最大的 OPEN 行，不保证是当期），
//   因此按 (year, month) 显式读回并必要时创建当期，状态非 OPEN 直接判红，不再 console.warn 后带病继续。
// 日期口径：凭证日期用 UTC 派生（后端 init 与余额区间均按 UTC），期间串与报表日期同样 UTC 派生，
//   避免本机时区与 UTC 跨月造成的期间错配。
import { test, expect } from '../diagnose-fixture';
import {
  API_BASE,
  API_PREFIX,
  loginViaUI,
  apiCall,
  apiCallRaw,
  tryCleanup,
  genCode,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 期间行（GET /finance/accounting-periods 的裸数组元素，键名取自 AccountingPeriodDto）。 */
interface PeriodRow {
  id: number;
  year: number;
  period: number;
  status: string;
}

/** 科目余额与报表金额列均为 DECIMAL(15,2)，Decimal 常以字符串回喂；非数值形态立即抛错不放宽。 */
function moneyAtScale(value: unknown, label: string, scale: number = 2): string {
  const n = typeof value === 'string' || typeof value === 'number' ? Number(value) : NaN;
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 金额形态非数值（契约漂移？）：${JSON.stringify(value)}`);
  }
  return n.toFixed(scale);
}

function num(obj: Record<string, unknown>, key: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) throw new Error(`键 "${key}" 非数字，raw=${JSON.stringify(obj[key])}`);
  return n;
}

/** 由同一个 UTC 时刻派生凭证日期与期间串，杜绝本机时区与 UTC 跨月错配。 */
function utcTodayAndPeriod(): { date: string; period: string; monthStart: string } {
  const d = new Date();
  const y = d.getUTCFullYear();
  const m = d.getUTCMonth() + 1;
  const pad = (n: number) => String(n).padStart(2, '0');
  return {
    date: `${y}-${pad(m)}-${pad(d.getUTCDate())}`,
    period: `${y}-${pad(m)}`,
    monthStart: `${y}-${pad(m)}-01`,
  };
}

interface ReportItem {
  name: string;
  amount: unknown;
  description?: string | null;
}

/** 分项行金额读取：缺 amount 键或非数值即抛错，不用 ?? 0 兜底掩盖缺键。 */
function itemAmount(item: ReportItem, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(item, 'amount')) {
    throw new Error(`${label} 缺少后端真实键 amount，实际键=${Object.keys(item).join(',')}`);
  }
  const n = Number(item.amount);
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 的 amount 非数字，raw=${JSON.stringify(item.amount)}`);
  }
  return n;
}

/** 分项金额求和。 */
function sumItemAmounts(items: ReportItem[], label: string): number {
  return items.reduce((acc, it, i) => acc + itemAmount(it, `${label} 第 ${i + 1} 行`), 0);
}

/** 按 (year,period) 定位期间行（读端点＝GET /finance/accounting-periods，裸数组）。 */
async function findPeriod(
  page: import('@playwright/test').Page,
  year: number,
  period: number
): Promise<PeriodRow | undefined> {
  const arr = await apiCallRaw<unknown>(page, 'GET', '/finance/accounting-periods');
  if (!Array.isArray(arr)) throw new Error(`期间列表非数组：${JSON.stringify(arr).slice(0, 200)}`);
  return (arr as PeriodRow[]).find(p => p.year === year && p.period === period);
}

/**
 * 构造并回读"覆盖今天的当期处于 OPEN"这一前置：缺失即按 (year, month) 创建，
 * 状态非 OPEN 判红并点名实际状态（close_period 会先做未过账凭证与试算平衡前置，
 * 若被其他套件关掉当期，这里必须失败而不是带着 400 继续跑）。
 */
async function ensureCurrentPeriodOpen(
  page: import('@playwright/test').Page,
  year: number,
  month: number
): Promise<PeriodRow> {
  let found = await findPeriod(page, year, month);
  if (!found) {
    await apiCall(page, 'POST', '/finance/accounting-periods', { year, period: month });
    found = await findPeriod(page, year, month);
  }
  if (!found) throw new Error(`前置构造失败：无法定位或创建期间 ${year}-${month}`);
  expect(
    found.status,
    `前置构造失败：当期 ${year}-${month}（id=${found.id}）应为 OPEN，实际=${found.status}`
  ).toBe('OPEN');
  return found;
}

/**
 * 建一枚指定编码的叶子科目。
 * balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量
 * （debit/credit，小写英文），禁灌中文。
 */
async function seedSubject(
  page: import('@playwright/test').Page,
  code: string,
  name: string,
  direction: 'debit' | 'credit'
): Promise<number> {
  const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
    code,
    name,
    level: 1,
    balance_direction: direction,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`建科目失败（code=${code}）：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
  return id;
}

/** 建一张推进到 posted 的平衡凭证（当期已 OPEN，过账走 workflow.rs 的 post）。 */
async function seedPostedVoucher(
  page: import('@playwright/test').Page,
  voucherDate: string,
  amount: number,
  debitSubject: number,
  creditSubject: number
): Promise<number> {
  const created = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
    voucher_type: '记',
    voucher_date: voucherDate,
    items: [
      { subject_id: debitSubject, debit: amount, credit: 0, summary: 'E2E 报表借' },
      { subject_id: creditSubject, debit: 0, credit: amount, summary: 'E2E 报表贷' },
    ],
  });
  const voucherId = created.data?.id;
  expect(typeof voucherId, `平衡凭证应先创建成功（日期 ${voucherDate}）`).toBe('number');
  if (!voucherId) throw new Error(`凭证创建未返回 id：${JSON.stringify(created).slice(0, 200)}`);
  CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });
  await apiCall(page, 'POST', `/vouchers/${voucherId}/submit`);
  await apiCall(page, 'POST', `/vouchers/${voucherId}/review`);
  await apiCall(page, 'POST', `/vouchers/${voucherId}/post`);
  return voucherId;
}

/** 过账后把发生额刷回科目主数据（试算平衡与科目余额列的写入路径）。 */
async function refreshSubjectBalance(
  page: import('@playwright/test').Page,
  subjectId: number,
  period: string
): Promise<void> {
  await apiCall(page, 'POST', `/subjects/${subjectId}/refresh-balance?period=${period}`);
}

/** 总账响应形状（键名取自 finance_report_dto.rs 的 GeneralLedger / GeneralLedgerEntry）。 */
interface GeneralLedgerLite {
  subject_code: string;
  subject_name: string;
  entries: Array<{
    voucher_date: string;
    voucher_no: string;
    line_no: number;
    debit: unknown;
    credit: unknown;
    direction: string;
    balance: unknown;
  }>;
  opening_balance: unknown;
  closing_balance: unknown;
  total_debit: unknown;
  total_credit: unknown;
  period_start: string;
  period_end: string;
}

/**
 * 取某科目区间总账：路由 /finance/reports/general-ledger/{code}，科目编码走路径段，
 * 查询串只取 start_date 与 end_date（DateRangeQuery 两键均可选，缺省为当月 1 日至今天；
 * 本用例显式传区间，不依赖后端缺省）。
 */
async function fetchGeneralLedger(
  page: import('@playwright/test').Page,
  subjectCode: string,
  startDate: string,
  endDate: string
): Promise<GeneralLedgerLite> {
  const qs = new URLSearchParams({ start_date: startDate, end_date: endDate });
  return apiCallRaw<GeneralLedgerLite>(
    page,
    'GET',
    `/finance/reports/general-ledger/${subjectCode}?${qs.toString()}`
  );
}

/** 取导出端点原始响应（不当 JSON 解析，保留二进制 body）。 */
async function fetchExportRaw(
  page: import('@playwright/test').Page,
  erpPath: string
): Promise<Awaited<ReturnType<typeof page.request.get>>> {
  return page.request.get(`${API_BASE}${API_PREFIX}${erpPath}`, {
    headers: { 'X-Requested-With': 'XMLHttpRequest' },
    timeout: 60_000,
  });
}

/**
 * 真 xlsx 断言：200 + spreadsheetml + attachment 文件名带 .xlsx + PK(zip) 魔数 +
 * 体内含 OOXML 部件名 xl/workbook.xml + 非 HTML/XML 起头。
 * 部件名扫描用于证明 zip 内确为工作簿（而非改名容器），文件名正则钉死下载命名。
 */
async function assertRealXlsxBlob(
  resp: Awaited<ReturnType<typeof fetchExportRaw>>,
  ctx: string,
  dispositionPattern: RegExp
): Promise<void> {
  expect(resp.status(), `${ctx}：导出应 HTTP 200`).toBe(200);
  const ct = resp.headers()['content-type'] ?? '';
  expect(ct, `${ctx}：Content-Type 应为 OOXML spreadsheetml，实际=${ct}`).toContain(
    'spreadsheetml'
  );
  const cd = resp.headers()['content-disposition'] ?? '';
  expect(
    cd,
    `${ctx}：Content-Disposition 应为 attachment 且文件名以 .xlsx 结束，实际=${cd}`
  ).toMatch(dispositionPattern);

  const buf = await resp.body();
  expect(buf.length, `${ctx}：字节流长度应 >= 4（可读魔数）`).toBeGreaterThanOrEqual(4);
  expect(
    [buf[0], buf[1], buf[2], buf[3]],
    `${ctx}：前 4 字节应为 zip 魔数 0x50 0x4B 0x03 0x04（真 xlsx=zip 容器），实际=` +
      [buf[0], buf[1], buf[2], buf[3]].map(b => '0x' + b.toString(16).padStart(2, '0')).join(' ')
  ).toEqual([0x50, 0x4b, 0x03, 0x04]);

  const ascii = buf.toString('latin1');
  expect(
    ascii.includes('xl/workbook.xml'),
    `${ctx}：zip 体内应含 OOXML 工作簿部件 xl/workbook.xml（否则不是真 xlsx）`
  ).toBe(true);

  const headAscii = buf.subarray(0, 16).toString('latin1').toLowerCase();
  expect(
    headAscii,
    `${ctx}：body 头部不应是 HTML/XML 假 xls，实际头部=${JSON.stringify(headAscii)}`
  ).not.toMatch(/<html|<!doctype|<\?xml|<table|<worksheet/);
}

test.describe('10 财务报表与导出', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    const { period } = utcTodayAndPeriod();
    const [y, m] = period.split('-').map(Number);
    // 前置条件失败判红：当期期间按 (year, month) 显式构造并回读 OPEN，
    // 不依赖 init（init_first_period 命中任一 OPEN 期间即提前返回，不保证当期）。
    await ensureCurrentPeriodOpen(page, y, m);
  });

  test('10-01 试算平衡：非零合计相等且等于分项之和，seed 科目在总账与余额两条路径同值', async ({
    page,
  }) => {
    const { date, period, monthStart } = utcTodayAndPeriod();
    const debitCode = genCode('E2E-RPTD');
    const creditCode = genCode('E2E-RPTC');
    const debitId = await seedSubject(page, debitCode, `E2E 报表借科目${debitCode}`, 'debit');
    const creditId = await seedSubject(page, creditCode, `E2E 报表贷科目${creditCode}`, 'credit');

    await seedPostedVoucher(page, date, 1000, debitId, creditId);
    await refreshSubjectBalance(page, debitId, period);
    await refreshSubjectBalance(page, creditId, period);

    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/trial-balance?period=${period}`
    );
    expect(
      String(tb.period),
      `试算平衡应回显请求期间（报表期键写入方为 Utc::now 的 YYYY-MM）`
    ).toBe(period);

    if (!Array.isArray(tb.entries)) {
      throw new Error(`试算平衡缺少 entries 数组，实际键=${Object.keys(tb).join(',')}`);
    }
    const entries = tb.entries as Array<Record<string, unknown>>;
    const de = entries.find(e => String(e.subject_code) === debitCode);
    const ce = entries.find(e => String(e.subject_code) === creditCode);
    if (!de || !ce) {
      throw new Error(
        `entries 应含被 seed 的两枚科目（debit=${debitCode} 命中=${!!de} credit=${creditCode} 命中=${!!ce}）`
      );
    }
    // ① 底层数据一致性：过账分录 1000 经 refresh-balance 落到科目余额列，期末方向按余额方向装配。
    expect(moneyAtScale(de.ending_debit, '借方科目期末借方'), '期末借方应为 1000.00').toBe(
      '1000.00'
    );
    expect(moneyAtScale(de.ending_credit, '借方科目期末贷方'), '期末贷方应为 0.00').toBe('0.00');
    expect(moneyAtScale(ce.ending_credit, '贷方科目期末贷方'), '期末贷方应为 1000.00').toBe(
      '1000.00'
    );
    expect(moneyAtScale(ce.ending_debit, '贷方科目期末借方'), '期末借方应为 0.00').toBe('0.00');
    expect(moneyAtScale(de.period_debit, '借方科目本期借方'), '本期借方应为 1000.00').toBe(
      '1000.00'
    );
    expect(moneyAtScale(ce.period_credit, '贷方科目本期贷方'), '本期贷方应为 1000.00').toBe(
      '1000.00'
    );

    // ② 借贷合计相等且金额非零（空库 0 比 0 属空洞通过，故非零判据必写）。
    const totalDr = num(tb, 'total_ending_debit');
    const totalCr = num(tb, 'total_ending_credit');
    expect(totalDr, `试算平衡期末借方合计应非零，实际=${totalDr}`).toBeGreaterThan(0);
    expect(totalCr, `试算平衡期末贷方合计应非零，实际=${totalCr}`).toBeGreaterThan(0);
    expect(
      Math.abs(totalDr - totalCr),
      `借贷合计应相等：借 ${totalDr} vs 贷 ${totalCr}`
    ).toBeLessThan(0.01);

    // ③ 装配一致性：合计必须等于分项之和（抓 total 写死或漏行）。
    const sumDr = entries.reduce((a, e) => a + num(e, 'ending_debit'), 0);
    const sumCr = entries.reduce((a, e) => a + num(e, 'ending_credit'), 0);
    expect(
      Math.abs(totalDr - sumDr),
      `期末借方合计 ${totalDr} 应等于分项之和 ${sumDr}`
    ).toBeLessThan(0.01);
    expect(
      Math.abs(totalCr - sumCr),
      `期末贷方合计 ${totalCr} 应等于分项之和 ${sumCr}`
    ).toBeLessThan(0.01);

    // ④ 同一笔过账在总账路径的逐笔真值（总账取 voucher_items，余额取 account_subjects）。
    const monthEnd = date;
    const glDebit = await fetchGeneralLedger(page, debitCode, monthStart, monthEnd);
    expect(glDebit.subject_code, '总账应回显请求科目编码').toBe(debitCode);
    expect(glDebit.entries.length, `借方科目总账应只含本例 1 笔分录`).toBe(1);
    expect(moneyAtScale(glDebit.entries[0].debit, '总账借方分录借额'), '分录借额应为 1000.00').toBe(
      '1000.00'
    );
    expect(glDebit.entries[0].direction, '分录方向应为大写 DEBIT（总账写入方词表）').toBe('DEBIT');
    expect(moneyAtScale(glDebit.total_debit, '总账借方合计'), '总账借方合计应为 1000.00').toBe(
      '1000.00'
    );
    expect(moneyAtScale(glDebit.total_credit, '总账贷方合计'), '总账贷方合计应为 0.00').toBe(
      '0.00'
    );
    expect(moneyAtScale(glDebit.opening_balance, '总账期初'), '新建科目期初应为 0.00').toBe('0.00');
    expect(moneyAtScale(glDebit.closing_balance, '总账期末'), '总账期末应与科目余额列同值').toBe(
      '1000.00'
    );

    const glCredit = await fetchGeneralLedger(page, creditCode, monthStart, monthEnd);
    expect(glCredit.entries.length, `贷方科目总账应只含本例 1 笔分录`).toBe(1);
    expect(glCredit.entries[0].direction, '分录方向应为大写 CREDIT（总账写入方词表）').toBe(
      'CREDIT'
    );
    expect(
      moneyAtScale(glCredit.closing_balance, '贷方科目总账期末'),
      '总账按借减贷累计，贷方科目期末应为 -1000.00'
    ).toBe('-1000.00');
  });

  test('10-02 资产负债表：先造非零数据，断 total 等于分项之和与资产恒等式，报表日期为当天', async ({
    page,
  }) => {
    const { date, period, monthStart } = utcTodayAndPeriod();
    // 编码取报表前缀 + 唯一后缀：货币资金聚合前缀 1001、应付账款聚合前缀 2202，
    // 唯一后缀保证本例这笔分录可被单独追认，聚合行本身不钉下界（见文件头残留限制）。
    const cashCode = genCode('1001');
    const apCode = genCode('2202');
    const cashId = await seedSubject(page, cashCode, `E2E 报表现金科目${cashCode}`, 'debit');
    const apId = await seedSubject(page, apCode, `E2E 报表应付科目${apCode}`, 'credit');

    await seedPostedVoucher(page, date, 888.88, cashId, apId);
    await refreshSubjectBalance(page, cashId, period);
    await refreshSubjectBalance(page, apId, period);

    // 底层数据侧：本例这笔 888.88 确已进入科目余额列与总账。
    const tb = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/finance/reports/trial-balance?period=${period}`
    );
    if (!Array.isArray(tb.entries)) {
      throw new Error(`试算平衡缺少 entries 数组，实际键=${Object.keys(tb).join(',')}`);
    }
    const entries = tb.entries as Array<Record<string, unknown>>;
    const cashRow = entries.find(e => String(e.subject_code) === cashCode);
    const apRow = entries.find(e => String(e.subject_code) === apCode);
    if (!cashRow || !apRow) {
      throw new Error(
        `试算平衡应含本例两枚前缀科目（cash=${!!cashRow} ap=${!!apRow}），缺失即报表未取到非零数据`
      );
    }
    expect(
      moneyAtScale(cashRow.ending_debit, '现金科目期末借方'),
      '现金科目期末借方应为 888.88'
    ).toBe('888.88');
    expect(
      moneyAtScale(apRow.ending_credit, '应付科目期末贷方'),
      '应付科目期末贷方应为 888.88'
    ).toBe('888.88');
    const glCash = await fetchGeneralLedger(page, cashCode, monthStart, date);
    expect(
      moneyAtScale(glCash.closing_balance, '现金科目总账期末'),
      '现金总账期末应为 888.88'
    ).toBe('888.88');
    const glAp = await fetchGeneralLedger(page, apCode, monthStart, date);
    expect(moneyAtScale(glAp.closing_balance, '应付科目总账期末'), '应付总账期末应为 -888.88').toBe(
      '-888.88'
    );

    // 报表侧：结构与恒等式。
    const bs = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/finance/reports/balance-sheet'
    );
    expect(Array.isArray(bs.assets), 'assets 应为数组').toBe(true);
    expect(Array.isArray(bs.liabilities), 'liabilities 应为数组').toBe(true);
    expect(Array.isArray(bs.equity), 'equity 应为数组').toBe(true);
    const assets = bs.assets as ReportItem[];
    const liabilities = bs.liabilities as ReportItem[];
    const equity = bs.equity as ReportItem[];

    // 行名与行数由 finance_report_service.rs 的 build_balance_sheet_response 装配，逐字符钉死中文行名。
    expect(
      assets.map(a => a.name),
      '资产行应与后端装配顺序逐字符一致'
    ).toEqual(['货币资金', '应收账款', '存货', '固定资产']);
    expect(
      liabilities.map(l => l.name),
      '负债行应与后端装配顺序逐字符一致'
    ).toEqual(['应付账款', '预收账款']);
    expect(
      equity.map(e => e.name),
      '权益行应与后端装配顺序逐字符一致'
    ).toEqual(['所有者权益']);

    const assetSum = sumItemAmounts(assets, '资产分项');
    const liabSum = sumItemAmounts(liabilities, '负债分项');
    const equitySum = sumItemAmounts(equity, '权益分项');
    const totalAssets = num(bs, 'total_assets');
    const totalLiabilities = num(bs, 'total_liabilities');
    const totalEquity = num(bs, 'total_equity');

    expect(
      Math.abs(totalAssets - assetSum),
      `total_assets ${totalAssets} 应等于资产分项之和 ${assetSum}`
    ).toBeLessThan(0.01);
    expect(
      Math.abs(totalLiabilities - liabSum),
      `total_liabilities ${totalLiabilities} 应等于负债分项之和 ${liabSum}`
    ).toBeLessThan(0.01);
    expect(
      Math.abs(totalEquity - equitySum),
      `total_equity ${totalEquity} 应等于权益分项之和 ${equitySum}`
    ).toBeLessThan(0.01);
    // 会计恒等式：资产 = 负债 + 权益。后端按定义式装配该式（对任意数据成立），
    // 此处断言的价值是防止三处字段被分别改写；可被数据证伪的部分由上面的唯一科目逐笔比对承担。
    expect(
      Math.abs(totalAssets - (totalLiabilities + totalEquity)),
      `资产负债恒等式应成立：资产 ${totalAssets} vs 负债 ${totalLiabilities} + 权益 ${totalEquity}`
    ).toBeLessThan(0.01);

    // 报表日期由 get_balance_sheet 取 Utc::now().date_naive()，钉死为与本用例凭证日期同一个
    // UTC 日值（抓写死日期或本机时区与 UTC 错配）。
    expect(String(bs.report_date), '报表日期应为凭证所在的那个 UTC 日期').toBe(date);
  });

  test('10-03 会计科目导出返回真 OOXML xlsx（PK 魔数 + 工作簿部件 + 文件名）', async ({ page }) => {
    const resp = await fetchExportRaw(page, '/subjects/export');
    await assertRealXlsxBlob(
      resp,
      '会计科目导出',
      /^attachment; filename="account_subjects_export\.xlsx"$/
    );
  });

  test('10-04 预算明细导出返回带水印真 xlsx（PK 魔数 + 工作簿部件 + 文件名带时间戳）', async ({
    page,
  }) => {
    const resp = await fetchExportRaw(page, '/budgets/export');
    await assertRealXlsxBlob(
      resp,
      '预算明细导出',
      /^attachment; filename="budget_items_export_[^"]+\.xlsx"$/
    );
  });

  test('10-05 敏感报表导出 fail-closed：无 download_token 访问资产负债表导出被拒 403 + FORBIDDEN + 脱敏常量', async ({
    page,
  }) => {
    const resp = await fetchExportRaw(page, '/finance/reports/balance-sheet/export');
    // enforce_export_download 缺令牌 → AppError::permission_denied → 403（真实权限拒绝码，非小于 500 泛断）。
    expect(resp.status(), '未带审批令牌的报表导出应恰为 403').toBe(403);
    // 403 必须是统一失败信封（{code,message,trace_id,timestamp}），非 JSON 或非对象一律判红，
    // 否则会把 HTML 错误页 / CSRF 直出体当成权限拒绝放过。
    const raw: unknown = await resp.json().catch((e: Error) => {
      throw new Error(`403 响应体应为统一失败信封 JSON，解析失败：${e.message}`);
    });
    if (raw === null || typeof raw !== 'object' || Array.isArray(raw)) {
      throw new Error(`403 响应体不是 JSON 对象，实际=${JSON.stringify(raw).slice(0, 200)}`);
    }
    const body = raw as Record<string, unknown>;
    expect(
      String(body.code),
      `机器码应为 FORBIDDEN（utils/error.rs 的 PermissionDenied 分支），实际=${JSON.stringify(body).slice(0, 200)}`
    ).toBe('FORBIDDEN');
    // PermissionDenied 走脱敏站点：出参 message 恒为权限常量，内部令牌文案不外显。
    expect(String(body.message), '权限拒绝文案应逐字符等于脱敏常量').toBe('无权限');
    expect(
      typeof body.trace_id === 'string' && body.trace_id.length > 0,
      `失败信封应携带非空 trace_id，实际=${JSON.stringify(body).slice(0, 200)}`
    ).toBe(true);
  });
});
