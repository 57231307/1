// 财务管理 E2E 套件 — 08 会计期间锁定（关账后录入被拒 + 真实文案逐字符 + 反结账对照）
//
// 核心真相（判据链，全部以 backend 源码写入方为准）
//   - 凭证创建：voucher_ops/crud.rs 的 validate_voucher_create_req 里期间锁仅在 is_production()
//     下执行，CI e2e 以 APP_ENV=development 运行（见 ci-cd.yml 的 E2E job），故创建不检锁。
//   - 凭证过账与反过账：voucher_ops/workflow.rs 的 post / unpost 调
//     AccountingPeriodService::check_date_locked；AR 收款走 ar_ops/collection.rs 的
//     check_date_locked_txn —— 两者均无环境开关，任何模式都生效，且共用同一文案模板。
//   - 拒绝装配：check_date_locked(_txn) 用 AppError::business_displayable 构造 →
//     utils/error.rs 把 BusinessErrorDisplayable 映射为 StatusCode::BAD_REQUEST 与机器码
//     BUSINESS_ERROR，并把真实文案外显到出参 message。因此本套件断恰 400（不是 403、
//     更不是小于 500 即绿），并把整条 message 逐字符钉死。
//   - 文案模板的取值来源：日期取凭证/收款日期本身，期间名取 accounting_periods.period_name，
//     由 missing_handlers.rs 的 create_accounting_period 写为 "YYYY 年 MM 月"（补零两位）。
//     本套件不照抄模板常量，期间名一律用 GET /finance/accounting-periods 读回的真实行值组装。
// 前提构造：关账前先用读端点确认期间行存在，缺失即 POST 创建，再 POST close，
//   最后回读该行断 status 为 CLOSED；任一环节不达标直接判红，不用条件守卫跳过断言。
// 负例回查：被拒的过账不得留下半截写入 —— 回读凭证状态仍为 reviewed、posted_by 仍为 NULL、
//   分录借贷金额与摘要原值未变；被拒的收款不得改动期间状态。
// 对照：反结账把期间恢复 OPEN 后，同日期同形态的凭证可过账成功并落 posted 与过账人，
//   证明前文的 400 确由期间锁触发，而不是余额校验或状态门。
// 隔离：使用远未来专用月份（2031-06 / 2031-10 / 2035-03），不触碰其他分片共用的当期。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  failureCode,
  APP_ERROR_CODES,
  genCode,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 会计期间行（GET /finance/accounting-periods 的 data 为裸数组，
 * 键名取自 missing_handlers.rs 的 AccountingPeriodDto，snake_case）。
 * status 写入方权威词表＝backend models/status/finance.rs 的 accounting_period 常量（大写 OPEN/CLOSED）。
 */
interface PeriodRow {
  id: number;
  year: number;
  period: number;
  period_name: string;
  status: string;
}

/** 凭证详情回读的精确形状（voucher 字段 flatten 至顶层，分录在 entries；分录列 DECIMAL(15,2) 常以字符串回喂） */
interface VoucherDetailLite {
  id: number;
  status: string | null;
  reviewed_by: number | null;
  posted_by: number | null;
  entries: Array<{
    line_no: number;
    subject_id: number | null;
    debit: unknown;
    credit: unknown;
    summary: string | null;
  }>;
}

/**
 * 期间锁拒绝文案模板（写入方＝accounting_period_service.rs 的 check_date_locked
 * 与 check_date_locked_txn，两处逐字符相同）。期间名由调用方用读回的真实行值传入。
 */
function lockedPeriodMessage(date: string, periodName: string): string {
  return `日期 ${date} 属于已结账的财务期间 (${periodName})，该期间的数据已被锁定，不可修改或新增。`;
}

/** 无期间录入拒绝文案模板（同一写入方的 else 分支，period 行缺失时命中） */
const NO_PERIOD_MESSAGE = '日期 2035-03-10 不在任何已设置的会计期间内，请先创建对应的会计期间。';

/** 列出全部会计期间（GET /finance/accounting-periods，data 为裸数组；非数组即契约漂移，判红不兜底）。 */
async function listPeriods(page: import('@playwright/test').Page): Promise<PeriodRow[]> {
  const arr = await apiCallRaw<unknown>(page, 'GET', '/finance/accounting-periods');
  if (!Array.isArray(arr)) throw new Error(`期间列表非数组：${JSON.stringify(arr).slice(0, 200)}`);
  return arr as PeriodRow[];
}

/** 按 (year, period) 定位期间行，不存在返回 undefined（由调用方决定构造还是判红）。 */
async function findPeriod(
  page: import('@playwright/test').Page,
  year: number,
  period: number
): Promise<PeriodRow | undefined> {
  const rows = await listPeriods(page);
  return rows.find(p => p.year === year && p.period === period);
}

/**
 * 显式构造 (year,period) 期间处于 CLOSED 的前提，并返回回读到的期间行（含 period_name）。
 * - 不存在 → POST /finance/accounting-periods 创建（create_accounting_period 落 status OPEN）→ close。
 * - 存在且 OPEN → close。
 * - 存在且 CLOSED → 直接复用（重跑幂等：不重复关账，避免残留未过账凭证阻塞二次 close）。
 * 关账针对空月（该区间内无 status 非 posted 的凭证）必然成功；close_period 会先查
 * check_unposted_vouchers_txn 与本期试算平衡，任一不满足即业务拒绝，由下面的回读判红外显。
 */
async function ensurePeriodClosed(
  page: import('@playwright/test').Page,
  year: number,
  period: number
): Promise<PeriodRow> {
  let found = await findPeriod(page, year, period);
  if (!found) {
    await apiCall(page, 'POST', '/finance/accounting-periods', { year, period });
    found = await findPeriod(page, year, period);
  }
  if (!found) throw new Error(`前置构造失败：无法定位或创建期间 ${year}-${period}`);
  if (found.status === 'OPEN') {
    await apiCall(page, 'POST', `/finance/accounting-periods/${found.id}/close`);
    const closed = await findPeriod(page, year, period);
    if (!closed) throw new Error(`前置构造失败：关账后期间 ${year}-${period} 读回为空`);
    found = closed;
  }
  expect(
    found.status,
    `前置构造失败：期间 ${year}-${period}（id=${found.id}）应为 CLOSED，实际=${found.status}，` +
      'close_period 的未过账凭证或试算平衡前置未满足'
  ).toBe('CLOSED');
  expect(found.period_name, `期间 ${year}-${period} 的 period_name 应非空`).not.toBe('');
  return found;
}

/** 当前已认证用户 id（post 由 workflow.rs 写入 posted_by=auth 上下文的 user_id）。 */
async function fetchMyUserId(page: import('@playwright/test').Page): Promise<number> {
  const me = await apiCallRaw<{ id?: number }>(page, 'GET', '/auth/me');
  const myId = Number(me.id);
  if (!Number.isInteger(myId) || myId <= 0) {
    throw new Error(`前置失败：/auth/me 未返回有效用户 id：${JSON.stringify(me).slice(0, 200)}`);
  }
  return myId;
}

/** 金额按两位小数同一标度比较（voucher_items 的 debit/credit 列为 DECIMAL(15,2)），非数值形态立即抛错不放宽。 */
function amountAtScale(value: unknown, label: string): string {
  const n = typeof value === 'string' || typeof value === 'number' ? Number(value) : NaN;
  if (!Number.isFinite(n)) {
    throw new Error(`${label} 金额形态非数值（契约漂移？）：${JSON.stringify(value)}`);
  }
  return n.toFixed(2);
}

/**
 * 建一枚推进到目标态的平衡凭证，返回凭证 id 与详情回读。
 * 状态词表比较点与写入方（backend models/status/finance.rs 的 voucher 模块）逐字符相同（小写）。
 * balance_direction 写入方权威词表＝backend models/status/finance.rs 的 account_subject 常量（debit/credit），禁灌中文。
 */
async function seedVoucherToReviewed(
  page: import('@playwright/test').Page,
  voucherDate: string,
  amount: number,
  subjectPrefix: string
): Promise<number> {
  const subjectIds: number[] = [];
  for (const direction of ['debit', 'credit'] as const) {
    const code = genCode(subjectPrefix + direction.toUpperCase());
    const created = await apiCall<{ id?: number }>(page, 'POST', '/subjects', {
      code,
      name: `E2E 期间锁科目${direction}${code}`,
      level: 1,
      balance_direction: direction,
    });
    const id = created.data?.id;
    if (!id) throw new Error(`建科目失败：${JSON.stringify(created)}`);
    CLEANUP.push({ path: `/subjects/${id}`, label: 'account_subject' });
    subjectIds.push(id);
  }
  const [debitSubject, creditSubject] = subjectIds;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
    voucher_type: '记',
    voucher_date: voucherDate,
    items: [
      { subject_id: debitSubject, debit: amount, credit: 0, summary: 'E2E 借' },
      { subject_id: creditSubject, debit: 0, credit: amount, summary: 'E2E 贷' },
    ],
  });
  const voucherId = created.data?.id;
  expect(typeof voucherId, `平衡凭证应先创建成功（日期 ${voucherDate}）`).toBe('number');
  if (!voucherId) throw new Error(`凭证创建未返回 id：${JSON.stringify(created).slice(0, 200)}`);
  CLEANUP.push({ path: `/vouchers/${voucherId}`, label: 'voucher' });
  await apiCall(page, 'POST', `/vouchers/${voucherId}/submit`);
  await apiCall(page, 'POST', `/vouchers/${voucherId}/review`);
  return voucherId;
}

test.describe('08 会计期间锁定', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('08-01 关账后向该期过账凭证被拒 400 + BUSINESS_ERROR + 整条锁定文案逐字符 + 零变化回查', async ({
    page,
  }) => {
    // 远未来专用月，空月关账必成；前提由 ensurePeriodClosed 构造并回读，不达标即判红。
    const closed = await ensurePeriodClosed(page, 2031, 6);

    const voucherId = await seedVoucherToReviewed(page, '2031-06-15', 500, 'E2E-PLK');
    const myId = await fetchMyUserId(page);

    // 过账前状态门前置自证：review 已把状态推进到 reviewed 并落 reviewed_by，
    // 否则下面的拒绝可能来自状态门而不是期间锁，断言就失去指向。
    const before = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${voucherId}`);
    expect(before.status, `过账前凭证应为 reviewed，实际=${before.status}`).toBe('reviewed');
    expect(
      before.reviewed_by,
      `过账前 reviewed_by 应为执行审核的当前用户 ${myId}，实际=${before.reviewed_by}`
    ).toBe(myId);
    expect(before.posted_by, '过账前 posted_by 应为 NULL').toBeNull();
    expect(before.entries.length, '过账前分录应为提交的两行').toBe(2);

    // 过账触发无环境开关的 check_date_locked → 该期已 CLOSED → 400。
    const fail = await apiCallExpectFail(page, 'POST', `/vouchers/${voucherId}/post`);
    expect(fail.status, `过账应被期间锁拒且恰为 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message, '锁定文案应逐字符等于写入方模板（期间名用回读值）').toBe(
      lockedPeriodMessage('2031-06-15', closed.period_name)
    );

    // 零变化回查：被拒的过账绝不能把状态或金额半截写入。
    const after = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${voucherId}`);
    expect(after.status, '被拒后状态不得从 reviewed 漂移').toBe('reviewed');
    expect(after.posted_by, '被拒后不得写入 posted_by').toBeNull();
    expect(after.reviewed_by, '被拒后 reviewed_by 应保持审核人').toBe(myId);
    expect(after.entries.length, '被拒后分录行数不得变化').toBe(2);
    const debitEntry = after.entries.find(e => e.debit !== null && Number(e.debit) > 0);
    const creditEntry = after.entries.find(e => e.credit !== null && Number(e.credit) > 0);
    if (!debitEntry || !creditEntry) {
      throw new Error(
        `被拒后回读缺少借/贷金额行（debit=${JSON.stringify(debitEntry)} credit=${JSON.stringify(creditEntry)}）`
      );
    }
    expect(amountAtScale(debitEntry.debit, '借方行借额'), '借方行借额应仍为 500').toBe('500.00');
    expect(amountAtScale(creditEntry.credit, '贷方行贷额'), '贷方行贷额应仍为 500').toBe('500.00');
    expect(debitEntry.summary, '借方行摘要应落库为提交原文').toBe('E2E 借');
    expect(creditEntry.summary, '贷方行摘要应落库为提交原文').toBe('E2E 贷');

    // 期间状态同样未被写坏：锁拒绝不影响 CLOSED。
    const periodAfter = await findPeriod(page, 2031, 6);
    expect(periodAfter?.status, '被拒的过账不得改动期间状态').toBe('CLOSED');
  });

  test('08-02 关账后向该期收款被拒（文案逐字符）+ 反结账持久化回读 + 同日期可过账对照', async ({
    page,
  }) => {
    const closed = await ensurePeriodClosed(page, 2031, 10);

    // AR 收款走同一 check_date_locked_txn（无环境开关）。期间锁先于客户与应收校验执行，
    // 故 customer_id 用占位 1 也不影响锁拒绝断言的指向。
    const fail = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: 1,
      amount: 100,
      payment_method: '银行转账',
      payment_date: '2031-10-20',
    });
    expect(fail.status, `收款应被期间锁拒且恰为 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message, '收款锁定文案应逐字符等于写入方模板（期间名用回读值）').toBe(
      lockedPeriodMessage('2031-10-20', closed.period_name)
    );

    // 反结账：CLOSED → OPEN，并以读端点回证持久化状态（不是只看响应体）。
    await apiCall(page, 'POST', `/finance/accounting-periods/${closed.id}/reopen`, {
      reason: 'E2E 反结账持久化验证',
    });
    const reopened = await findPeriod(page, 2031, 10);
    expect(reopened?.status, '反结账后期间应为 OPEN').toBe('OPEN');
    expect(reopened?.period_name, '反结账不应改动期间名').toBe(closed.period_name);

    // 对照：期间恢复 OPEN 后，同日期同形态的凭证可过账成功 —— 证明上面的 400 确由期间锁触发。
    // post 在 workflow.rs 写入 posted 与 posted_by；被拒与放行的唯一差别就是该期 status。
    const contrastId = await seedVoucherToReviewed(page, '2031-10-20', 300, 'E2E-PLKC');
    await apiCall(page, 'POST', `/vouchers/${contrastId}/post`);
    const myId = await fetchMyUserId(page);
    const posted = await apiCallRaw<VoucherDetailLite>(page, 'GET', `/vouchers/${contrastId}`);
    expect(posted.status, `反结账后同日期凭证应可过账，实际=${posted.status}`).toBe('posted');
    expect(posted.posted_by, `过账人应为执行过账的当前用户 ${myId}，实际=${posted.posted_by}`).toBe(
      myId
    );
    expect(posted.entries.length, '对照凭证分录仍为两行').toBe(2);
    const debitTotal = posted.entries.reduce((acc, e) => acc + Number(e.debit), 0);
    expect(
      amountAtScale(debitTotal, '对照凭证借方合计'),
      '反结账后过账成功应把提交借方 300 原值入账'
    ).toBe('300.00');
  });

  test('08-03 未设置期间的日期录入被拒（无期间分支文案逐字符）', async ({ page }) => {
    // 2035-03 从不建期，命中 check_date_locked_txn 的无期间分支（else 分支文案与锁定分支不同）。
    const rows = await listPeriods(page);
    expect(
      rows.some(p => p.year === 2035 && p.period === 3),
      '前置：2035-03 不应已存在期间'
    ).toBe(false);

    const fail = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: 1,
      amount: 100,
      payment_method: '银行转账',
      payment_date: '2035-03-10',
    });
    expect(fail.status, `无期间录入应被拒且恰为 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), `机器码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message, '无期间文案应逐字符等于写入方模板，不得只断子串').toBe(NO_PERIOD_MESSAGE);

    // 零变化回查：被拒的收款不应留下一条 2035-03 的期间行或改动既有期间数。
    const rowsAfter = await listPeriods(page);
    expect(
      rowsAfter.some(p => p.year === 2035 && p.period === 3),
      '被拒的收款不得创建 2035-03 期间'
    ).toBe(false);
  });
});
