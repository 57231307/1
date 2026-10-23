// 本文件守的缺陷族：核销只动状态不动金额（或部分回写）、核销后应收余额与
// 「发票额-已核销额」对不上、超核销被放行、取消核销不回滚资金账本、
// 收款单双别名键（payment_no/collection_no、amount/collection_amount）不同源导致恒空列、
// 未确认收款被核销、终态回退。
// 金额断到分位（Decimal 字符串出参折 BigInt 分），状态比较值与写入方逐字符同源：
// ar_collection 小写 pending/confirmed/cancelled；ar_invoice 大写 APPROVED/PARTIAL_PAID/PAID/CANCELLED；
// 核销单 reconciliation_status 小写 closed/cancelled；核销明细 match_status 大写 MATCHED。
// 出参形状：/ar/payments 与 /ar/verifications 列表为 {list,total,page,page_size}；
// /ar/verifications/unverified/invoices 与 /unverified/payments 出参为裸数组；
// 核销详情含 items 明细数组（INVOICE 行金额为正、RECEIPT 行金额为负是账本记法）。
import { test, expect } from '../diagnose-fixture';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  failureCode,
  genCode,
  getCtx,
  loginViaUI,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

type Row = Record<string, unknown>;

function toScaled(v: unknown, scale: number, label: string): bigint {
  const s = typeof v === 'string' ? v.trim() : typeof v === 'number' ? String(v) : '';
  if (s === '') throw new Error(`${label}：Decimal 出参非 string|number，raw=${JSON.stringify(v)}`);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) throw new Error(`${label}：Decimal 出参形态非法，raw="${s}"`);
  const [, sign, intPart, frac = ''] = m;
  if (frac.length > scale && frac.slice(scale).replace(/0+$/, '') !== '') {
    throw new Error(`${label}：出参标度超出分位精度且非零，raw="${s}"`);
  }
  const padded = (intPart + frac).padEnd(intPart.length + scale, '0');
  const value = BigInt(padded);
  return sign === '-' ? -value : value;
}

/** 字面金额串 → 分 */
function cents(want: string): bigint {
  return toScaled(want, 2, `期望金额 ${want}`);
}

function eqAmount(actual: unknown, expectedCents: bigint, label: string): void {
  expect(
    toScaled(actual, 2, label),
    `${label}：期望(分)=${expectedCents} 实际=${JSON.stringify(actual)}`
  ).toBe(expectedCents);
}

/** 发票恒等式：invoice_amount = received_amount + unpaid_amount（分位） */
function expectInvoiceIdentity(inv: Row, label: string): void {
  // eqAmount 内部会对出参做一次折分位换算，这里必须传原始 Decimal 串，禁止预折导致重复换算
  eqAmount(
    inv.invoice_amount,
    toScaled(inv.received_amount, 2, `${label} 恒等式-已收`) +
      toScaled(inv.unpaid_amount, 2, `${label} 恒等式-未收`),
    `${label}：发票额=已收+未收（分位恒等）`
  );
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

async function createApprovedInvoice(
  page: import('@playwright/test').Page,
  customerId: number,
  amount: string
): Promise<{ id: number; invoice_no: string }> {
  const d = new Date().toISOString().slice(0, 10);
  const inv = await apiCallRaw<Row>(page, 'POST', '/ar/invoices', {
    customer_id: customerId,
    invoice_amount: amount,
    invoice_date: d,
    due_date: d,
  });
  const id = requireNum(inv.id, '建应收');
  await apiCall(page, 'POST', `/ar/invoices/${id}/approve`);
  const approved = await apiCallRaw<Row>(page, 'GET', `/ar/invoices/${id}`);
  expect(approved.status, '应收审批回读 APPROVED').toBe('APPROVED');
  return { id, invoice_no: String(approved.invoice_no) };
}

async function createPayment(
  page: import('@playwright/test').Page,
  customerId: number,
  amount: string,
  remark: string
): Promise<number> {
  const p = await apiCallRaw<Row>(page, 'POST', '/ar/payments', {
    customer_id: customerId,
    amount,
    payment_method: '银行转账',
    payment_date: new Date().toISOString().slice(0, 10),
    remark,
  });
  return requireNum(p.id, `建收款 ${amount}`);
}

async function readInvoice(page: import('@playwright/test').Page, id: number): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/ar/invoices/${id}`);
}

async function readPayment(page: import('@playwright/test').Page, id: number): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/ar/payments/${id}`);
}

async function readVerification(page: import('@playwright/test').Page, id: number): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/ar/verifications/${id}`);
}

test.describe('92 收款→核销→应收余额 资金链路', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('92-01 主链：未确认核销被拒→确认→部分核销余额精确→二次核销→结清→终态超核销拒→逐笔取消核销资金全额回滚', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    const myId = requireNum(me.id, '当前用户');
    const tag = genCode('V');

    // 发票 1234.56（已审批），收款 1000.00 不直连发票（核销金额只能经核销门进入）
    const inv = await createApprovedInvoice(page, ctx.customerId, '1234.56');
    const payId = await createPayment(page, ctx.customerId, '1000.00', `E2E-${tag}-P1`);

    const pay0 = await readPayment(page, payId);
    expect(pay0.status, '收款初始小写词值 pending').toBe('pending');
    // 收款单双别名键同源自洽（恒空列缺陷族：任一别名缺键或两键不同源即红）
    expect(String(pay0.payment_no), 'payment_no 与 collection_no 必须同源同值').toBe(
      String(pay0.collection_no)
    );
    expect(toScaled(pay0.amount, 2, 'amount'), 'amount 与 collection_amount 必须同源同值').toBe(
      toScaled(pay0.collection_amount, 2, 'collection_amount')
    );
    eqAmount(pay0.amount, cents('1000.00'), '收款头金额落库');
    // 收款建单未核销不动发票：假落库在这里现形
    const invAtCreate = await readInvoice(page, inv.id);
    eqAmount(invAtCreate.received_amount, 0n, '未核销时发票已收必须为 0');
    expect(invAtCreate.status, '发票未收前保持 APPROVED').toBe('APPROVED');

    // 未确认收款核销被拒：BUSINESS_ERROR 且文案点名 pending
    const fPending = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: payId,
      amount: '100.00',
    });
    expect(fPending.status, 'pending 收款核销应 400').toBe(400);
    expect(failureCode(fPending), 'pending 收款核销机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const pendingMsg = String(fPending.message ?? '');
    expect(pendingMsg, `拒绝文案必须点名未确认状态，实际=${pendingMsg}`).toContain('pending');
    expect(pendingMsg, '拒绝文案必须点名核销门').toContain('核销');
    const invNoDrift = await readInvoice(page, inv.id);
    eqAmount(invNoDrift.received_amount, 0n, '被拒核销后发票已收分毫未动');

    // 确认收款：pending→confirmed，操作人与时间真实落库
    await apiCall(page, 'POST', `/ar/payments/${payId}/confirm`);
    const payC = await readPayment(page, payId);
    expect(payC.status, '确认回读 confirmed').toBe('confirmed');
    expect(Number(payC.confirmed_by), 'confirmed_by 落真实操作人').toBe(myId);
    expect(payC.confirmed_at, 'confirmed_at 必须写入').toBeTruthy();
    // 终态不可重复确认（状态门 BAD_REQUEST）
    const fReconfirm = await apiCallExpectFail(page, 'POST', `/ar/payments/${payId}/confirm`);
    expect(fReconfirm.status, 'confirmed 再确认应 400').toBe(400);
    expect(failureCode(fReconfirm), '再确认机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);

    // ── 第一笔部分核销 300.15 ──
    const v1 = await apiCallRaw<Row>(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: payId,
      amount: '300.15',
      remark: `E2E-${tag}-V1`,
    });
    const v1Id = requireNum(v1.id, '核销单1');
    expect(String(v1.status), '核销即时状态小写词值 closed').toBe('closed');
    expect(String(v1.reconciliation_no), '核销单号由后端生成且带 VER 前缀').toMatch(/^VER/);
    expect(String(v1.invoice_status), '响应内发票推进值 PARTIAL_PAID').toBe('PARTIAL_PAID');
    eqAmount(v1.invoice_unpaid_amount, cents('934.41'), '响应内未收余额=1234.56-300.15');
    // 权威回读：核销后应收余额 == 发票额 − 已核销额（分位）
    const invV1 = await readInvoice(page, inv.id);
    eqAmount(invV1.received_amount, cents('300.15'), '部分核销后已收');
    eqAmount(invV1.unpaid_amount, cents('934.41'), '部分核销后剩余余额=未核销部分');
    expect(invV1.status, '部分核销发票词值 PARTIAL_PAID').toBe('PARTIAL_PAID');
    expectInvoiceIdentity(invV1, '部分核销后');
    // 核销账本明细双向可读：INVOICE 正额 / RECEIPT 负额，document_no 与源单逐字符一致
    const v1Detail = await readVerification(page, v1Id);
    expect(String(v1Detail.reconciliation_status), '核销单详情状态 closed').toBe('closed');
    const items = v1Detail.items as Row[];
    expect(items.length, '核销明细必须恰 INVOICE+RECEIPT 两条').toBe(2);
    const invItem = items.find(i => i.item_type === 'INVOICE');
    const recItem = items.find(i => i.item_type === 'RECEIPT');
    expect(invItem, '明细缺 INVOICE 行').toBeTruthy();
    expect(recItem, '明细缺 RECEIPT 行').toBeTruthy();
    eqAmount(invItem!.amount, cents('300.15'), 'INVOICE 明细额');
    expect(String(invItem!.document_no), 'INVOICE 明细单号=发票号').toBe(inv.invoice_no);
    expect(String(invItem!.match_status), '明细匹配态大写词值 MATCHED').toBe('MATCHED');
    eqAmount(recItem!.amount, -cents('300.15'), 'RECEIPT 明细按记账方向存负额');
    expect(Number(recItem!.document_id), 'RECEIPT 明细锚定本收款单').toBe(payId);
    expect(recItem!.document_type, '收款单级账本行类型 AR_COLLECTION').toBe('AR_COLLECTION');
    // 核销不动收款头
    const payAfterV1 = await readPayment(page, payId);
    eqAmount(payAfterV1.amount, cents('1000.00'), '核销后收款头金额不变');
    // 候选列表（裸数组契约）：余额未清的发票与有余量收款都在列
    const candInv = await apiCallRaw<unknown>(
      page,
      'GET',
      `/ar/verifications/unverified/invoices?customer_id=${ctx.customerId}`
    );
    expect(Array.isArray(candInv), '未核销发票候选必须为裸数组').toBe(true);
    expect(
      (candInv as Row[]).some(r => Number(r.id) === inv.id),
      '尚有未收余额的发票必须在候选列表'
    ).toBe(true);

    // ── 超核销双门（各自点名原因）：发票未收不足 / 收款可用余额不足 ──
    const fOverInv = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: payId,
      amount: '999.99',
    });
    expect(fOverInv.status, '超发票未收核销应 400').toBe(400);
    expect(failureCode(fOverInv), '超发票未收机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const overInvMsg = String(fOverInv.message ?? '');
    expect(overInvMsg, `拒绝必须点名「未收金额」不足，实际=${overInvMsg}`).toContain('未收金额');
    expect(overInvMsg, '拒绝文案应回显剩余余额 934.41 供操作者自纠').toContain('934.41');
    const fOverPay = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: payId,
      amount: '700.00',
    });
    expect(fOverPay.status, '超收款可用余额核销应 400').toBe(400);
    expect(failureCode(fOverPay), '超收款余额机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const overPayMsg = String(fOverPay.message ?? '');
    expect(overPayMsg, `拒绝必须点名「可用余额」不足，实际=${overPayMsg}`).toContain('可用余额');
    expect(overPayMsg, '拒绝文案应回显收款可用余额 699.85').toContain('699.85');
    // 两次被拒无痕
    const invAfterRejects = await readInvoice(page, inv.id);
    eqAmount(invAfterRejects.received_amount, cents('300.15'), '被拒核销不得写入已收');
    eqAmount(invAfterRejects.unpaid_amount, cents('934.41'), '被拒核销不得扣减未收');

    // ── 第二笔核销 699.85（收款余量恰好用尽）──
    const v2 = await apiCallRaw<Row>(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: payId,
      amount: '699.85',
    });
    const v2Id = requireNum(v2.id, '核销单2');
    const invV2 = await readInvoice(page, inv.id);
    eqAmount(invV2.received_amount, cents('1000.00'), '两笔核销合计入已收');
    eqAmount(invV2.unpaid_amount, cents('234.56'), '剩余余额=发票额-已核销合计');
    expect(invV2.status, '仍有余额时保持 PARTIAL_PAID').toBe('PARTIAL_PAID');
    expectInvoiceIdentity(invV2, '二次核销后');
    // 收款分配用尽 → 从候选收款列表消失；发票仍有余额 → 留在候选发票列表
    const candPay2 = await apiCallRaw<unknown>(
      page,
      'GET',
      `/ar/verifications/unverified/payments?customer_id=${ctx.customerId}`
    );
    expect(
      (candPay2 as Row[]).some(r => Number(r.id) === payId),
      '已足额分配的收款单不得再出现在可核销收款候选'
    ).toBe(false);

    // ── 第三笔收款结清 → PAID 终态 ──
    const pay2Id = await createPayment(page, ctx.customerId, '234.56', `E2E-${tag}-P2`);
    await apiCall(page, 'POST', `/ar/payments/${pay2Id}/confirm`);
    const v3 = await apiCallRaw<Row>(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: pay2Id,
      amount: '234.56',
    });
    const v3Id = requireNum(v3.id, '核销单3');
    const invPaid = await readInvoice(page, inv.id);
    expect(invPaid.status, '余额清零发票词值 PAID').toBe('PAID');
    eqAmount(invPaid.unpaid_amount, 0n, '结清后未收=0');
    eqAmount(invPaid.received_amount, cents('1234.56'), '结清后已收=发票全额');
    expectInvoiceIdentity(invPaid, '结清后');
    const candInv3 = await apiCallRaw<unknown>(
      page,
      'GET',
      `/ar/verifications/unverified/invoices?customer_id=${ctx.customerId}`
    );
    expect(
      (candInv3 as Row[]).some(r => Number(r.id) === inv.id),
      '已结清发票必须退出可核销发票候选'
    ).toBe(false);

    // 终态再核销被拒（未收金额为 0 不足），发票不回退
    const fTerminal = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: inv.id,
      payment_id: pay2Id,
      amount: '0.01',
    });
    expect(fTerminal.status, 'PAID 终态再核销应 400').toBe(400);
    expect(failureCode(fTerminal), '终态核销机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const invTerminal = await readInvoice(page, inv.id);
    expect(invTerminal.status, '被拒后终态 PAID 不回退').toBe('PAID');

    // ── 取消前两笔核销：资金账本逐笔回滚，第三笔不受牵连 ──
    await apiCall(page, 'POST', `/ar/verifications/${v1Id}/cancel`);
    const invC1 = await readInvoice(page, inv.id);
    eqAmount(invC1.received_amount, cents('934.41'), '取消核销1回滚 300.15');
    eqAmount(invC1.unpaid_amount, cents('300.15'), '取消核销1余额回补');
    expect(invC1.status, '回补后仍部分收款 PARTIAL_PAID').toBe('PARTIAL_PAID');
    expectInvoiceIdentity(invC1, '取消核销1后');
    const v1After = await readVerification(page, v1Id);
    expect(String(v1After.reconciliation_status), '核销单1取消后小写词值 cancelled').toBe(
      'cancelled'
    );

    // 重复取消被拒（状态门仅 closed 可取消）
    const fRecancel = await apiCallExpectFail(page, 'POST', `/ar/verifications/${v1Id}/cancel`);
    expect(fRecancel.status, 'cancelled 再取消应 400').toBe(400);
    expect(failureCode(fRecancel), '再取消机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);
    const invNoReeffect = await readInvoice(page, inv.id);
    eqAmount(invNoReeffect.received_amount, cents('934.41'), '重复取消不得二次回滚');

    await apiCall(page, 'POST', `/ar/verifications/${v2Id}/cancel`);
    const invC2 = await readInvoice(page, inv.id);
    eqAmount(invC2.received_amount, cents('234.56'), '取消核销2后再收只剩第三笔');
    eqAmount(invC2.unpaid_amount, cents('1000.00'), '取消核销2余额回补 699.85');
    expect(invC2.status, '仍有已收 PARTIAL_PAID').toBe('PARTIAL_PAID');

    // 取消第三笔：已收清零，发票恢复审批后未收款语义（回滚词表 APPROVED，不是 PAID 残留）
    await apiCall(page, 'POST', `/ar/verifications/${v3Id}/cancel`);
    const invC3 = await readInvoice(page, inv.id);
    eqAmount(invC3.received_amount, 0n, '全部取消后已收清零');
    eqAmount(invC3.unpaid_amount, cents('1234.56'), '全部取消后余额回全额');
    expect(invC3.status, '清零回滚后状态词值 APPROVED（不得停留 PAID/PARTIAL_PAID）').toBe(
      'APPROVED'
    );
    expectInvoiceIdentity(invC3, '全部取消后');

    // 列表侧账本可读：三笔核销单同现列表（取消不删行，留痕语义）
    const vList = await apiCallRaw<Row>(
      page,
      'GET',
      `/ar/verifications?invoice_id=${inv.id}&payment_id=${payId}&page=1&page_size=50`
    );
    const vRows = pickListArray<Row>(vList, 'list', '核销列表（发票+收款双过滤）');
    const ids = vRows.map(r => Number(r.id));
    expect(ids.includes(v1Id), '取消的核销单仍应可查（留痕）').toBe(true);
    expect(ids.includes(v2Id), '双过滤应含同收款第二笔').toBe(true);
  });

  test('92-02 收款单金额族负例与一致性门：非法金额点名原因、直连分摊即入账、下调穿破已核销分配被拒', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');

    // 金额 ≤0 / 精度越界 / 收款方式空串三类拒绝各自点名
    const fZero = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: '0',
      payment_method: '银行转账',
      payment_date: new Date().toISOString().slice(0, 10),
    });
    expect(fZero.status, '0 元收款应 400').toBe(400);
    expect(failureCode(fZero), '0 元收款机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    // 校验白名单如实外显：点名被拒字段 amount 与金额必须为正的语义，不锁整句措辞
    expect(String(fZero.message ?? ''), '0 元拒绝应点名校验字段 amount').toContain('amount');
    expect(String(fZero.message ?? ''), '0 元拒绝应给出金额须为正的语义').toContain('金额必须为正');
    const fPrec = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: '1.999',
      payment_method: '银行转账',
      payment_date: new Date().toISOString().slice(0, 10),
    });
    expect(fPrec.status, '超精度收款应 400').toBe(400);
    expect(failureCode(fPrec), '超精度收款机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(fPrec.message ?? ''), '超精度拒绝应点名「精度」').toContain('精度');
    const fMethod = await apiCallExpectFail(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: '10',
      payment_method: '',
      payment_date: new Date().toISOString().slice(0, 10),
    });
    expect(fMethod.status, '空收款方式应 400').toBe(400);
    expect(failureCode(fMethod), '空收款方式机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 直连发票路径：创建时按未收额分摊入账并同事务落核销账本（pending 收款即占分配额）
    const invA = await createApprovedInvoice(page, ctx.customerId, '500.00');
    const payLinked = await apiCallRaw<Row>(page, 'POST', '/ar/payments', {
      customer_id: ctx.customerId,
      amount: '300.00',
      payment_method: '银行转账',
      payment_date: new Date().toISOString().slice(0, 10),
      invoice_ids: [invA.id],
    });
    const linkedId = requireNum(payLinked.id, '直连收款');
    const invLinked = await readInvoice(page, invA.id);
    eqAmount(invLinked.received_amount, cents('300.00'), '直连分摊即入已收');
    eqAmount(invLinked.unpaid_amount, cents('200.00'), '直连分摊后余额');
    expect(invLinked.status, '直连部分分摊词值 PARTIAL_PAID').toBe('PARTIAL_PAID');
    const payL = await readPayment(page, linkedId);
    expect(payL.status, '直连收款创建后仍是 pending（分摊与确认两件事）').toBe('pending');

    // 下调穿破已核销分配被拒：300.00 → 200.00（已分配 300.00）
    const fUnder = await apiCallExpectFail(page, 'PUT', `/ar/payments/${linkedId}`, {
      amount: '200.00',
    });
    expect(fUnder.status, '金额下调低于已分配额应 400').toBe(400);
    expect(failureCode(fUnder), '下调穿破分配机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(fUnder.message ?? ''), '拒绝应点名已核销分配门').toContain('已核销分配');
    const payUnchanged = await readPayment(page, linkedId);
    eqAmount(payUnchanged.amount, cents('300.00'), '被拒下调后收款金额零漂移');
    const invUnchanged = await readInvoice(page, invA.id);
    eqAmount(invUnchanged.received_amount, cents('300.00'), '被拒下调不得半改发票侧');

    // 合法上调通过并回读
    await apiCall(page, 'PUT', `/ar/payments/${linkedId}`, { amount: '300.01' });
    const payUp = await readPayment(page, linkedId);
    eqAmount(payUp.amount, cents('300.01'), '上调落库');

    // NOT NULL 列显式 null 清空被拒（不落默认值的业务拒绝）
    const fNullAmt = await apiCallExpectFail(page, 'PUT', `/ar/payments/${linkedId}`, {
      amount: null,
    });
    expect(fNullAmt.status, '金额显式 null 应 400').toBe(400);
    expect(failureCode(fNullAmt), '显式 null 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const payStill = await readPayment(page, linkedId);
    eqAmount(payStill.amount, cents('300.01'), '显式 null 被拒后金额未被清空');

    // pending 收款可直接取消，且取消必须联动回收分配账本：发票侧已收回滚、词值恢复
    await apiCall(page, 'POST', `/ar/payments/${linkedId}/cancel`);
    const payLinkedFinal = await readPayment(page, linkedId);
    expect(payLinkedFinal.status, '取消回读小写 cancelled').toBe('cancelled');
    const invAfterCancel = await readInvoice(page, invA.id);
    eqAmount(invAfterCancel.received_amount, 0n, '取消已分配收款必须回滚发票侧已收');
    expect(invAfterCancel.status, '回滚清零后发票词值恢复 APPROVED').toBe('APPROVED');
    eqAmount(invAfterCancel.unpaid_amount, cents('500.00'), '回滚后余额回全额');
    expectInvoiceIdentity(invAfterCancel, '取消直连收款后');
    // 已取消再取消被拒（状态门仅 pending）
    const fReCancel = await apiCallExpectFail(page, 'POST', `/ar/payments/${linkedId}/cancel`);
    expect(fReCancel.status, 'cancelled 再取消应 400').toBe(400);
    expect(failureCode(fReCancel), '再取消机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);

    // confirmed 收款不可直接取消（须先回收核销），状态门拒绝
    const payConfirmed = await createPayment(page, ctx.customerId, '20.00', 'E2E-CONFIRMED-CANCEL');
    await apiCall(page, 'POST', `/ar/payments/${payConfirmed}/confirm`);
    const fConfirmedCancel = await apiCallExpectFail(
      page,
      'POST',
      `/ar/payments/${payConfirmed}/cancel`
    );
    expect(fConfirmedCancel.status, 'confirmed 直接取消应 400').toBe(400);
    expect(failureCode(fConfirmedCancel), 'confirmed 取消机器码 BAD_REQUEST').toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );
    const payCStill = await readPayment(page, payConfirmed);
    expect(payCStill.status, '被拒取消后仍 confirmed').toBe('confirmed');
  });

  test('92-03 跨客户核销与非法金额：客户不一致点名拒绝、负数与精度金额拒于门外', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');

    // 第二个客户自建自清
    const custB = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `E2E-92B-${genCode('CB')}`,
    });
    const custBId = requireNum(custB.id, '建客户B');

    const invA = await createApprovedInvoice(page, ctx.customerId, '100.00');
    const payB = await createPayment(page, custBId, '100.00', 'E2E-92-X');
    await apiCall(page, 'POST', `/ar/payments/${payB}/confirm`);

    const fCross = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: invA.id,
      payment_id: payB,
      amount: '100.00',
    });
    expect(fCross.status, '跨客户核销应 400').toBe(400);
    expect(failureCode(fCross), '跨客户机器码 BUSINESS_ERROR').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(fCross.message ?? ''), '拒绝必须点名客户不一致').toContain('客户不一致');
    const invNoDrift = await readInvoice(page, invA.id);
    eqAmount(invNoDrift.received_amount, 0n, '跨客户被拒无痕');

    // 负数与超精度核销金额
    const fNeg = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: invA.id,
      payment_id: payB,
      amount: '-1.00',
    });
    expect(fNeg.status, '负额核销应 400').toBe(400);
    expect(failureCode(fNeg), '负额核销机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(fNeg.message ?? ''), '负额拒绝应点名「大于零」').toContain('大于零');
    const fSubCent = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: invA.id,
      payment_id: payB,
      amount: '0.001',
    });
    expect(fSubCent.status, '亚分位核销应 400').toBe(400);
    expect(failureCode(fSubCent), '亚分位机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 不存在的核销对象 404（不得降级为空成功或 500）
    const fGhost = await apiCallExpectFail(page, 'GET', '/ar/verifications/2147483647');
    expect(fGhost.status, '幽灵核销单应 404').toBe(404);
    expect(failureCode(fGhost), '幽灵核销机器码').toBe(APP_ERROR_CODES.NOT_FOUND);

    // 已取消发票核销被拒且点名原因（终态门）
    await apiCall(page, 'POST', `/ar/invoices/${invA.id}/cancel`, { reason: 'E2E 终态门测试' });
    const fCancelledInv = await apiCallExpectFail(page, 'POST', '/ar/verifications/manual', {
      invoice_id: invA.id,
      payment_id: payB,
      amount: '10.00',
    });
    expect(fCancelledInv.status, '已取消发票核销应 400').toBe(400);
    expect(failureCode(fCancelledInv), '已取消发票机器码 BAD_REQUEST').toBe(
      APP_ERROR_CODES.BAD_REQUEST
    );
    const invFinal = await readInvoice(page, invA.id);
    expect(invFinal.status, '被拒后发票仍 CANCELLED').toBe('CANCELLED');
    eqAmount(invFinal.received_amount, 0n, '终态发票不被核销写入已收');
  });
});
