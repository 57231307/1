// 财务管理 E2E 套件 — 11 AP/AR 核销闭环 + AP 对账状态机 + AP 付款申请提交门控
//
// 覆盖审计确证的财务域零覆盖缺口，全部为「写后必 GET 回读真实金额/状态」的端到端真值断言：
//   1. AP 手工核销闭环（routes/finance.rs:749-777 /ap/verifications/*）：
//      建发票×2 + 方案B付款链 → POST /ap/verifications/manual → 回读目标发票 paid 增/unpaid 减
//      → POST /ap/verifications/{id}/cancel → 回读金额回退，且不影响非本次明细的发票。
//      说明：AP 应付单契约（models/ap_invoice.rs）没有 verified_amount 键，
//      核销对发票的真实作用面就是 paid_amount/unpaid_amount/invoice_status 三列，
//      核销单自身的金额以 GET /ap/verifications/{id}.total_amount 回读。
//   2. AR 核销链（routes/finance.rs:898-929 /ar/verifications/*）：
//      auto（POST /ar/verifications/auto，无 body，全局贪心匹配）→ manual → cancel，
//      每步回读发票 received_amount/unpaid_amount/status 与核销单 reconciliation_status。
//      AR 收款必须先 POST /ar/payments/{id}/confirm（status pending→confirmed）才可被核销，
//      且收款建单不带 invoice_ids（带则 link_invoices 即时分配，会污染核销增量基线）。
//   3. AP 对账（routes/finance.rs:782-810 /ap/reconciliations/*）：
//      generate（初始 PENDING，期初+本期应付-本期付款=期末 算术真值回读）
//      → confirm 分支（PENDING→CONFIRMED）与 dispute 分支（PENDING→DISPUTED，原因回显），
//      各含门控负例（已确认再确认被拒 / DISPUTED 确认被拒），断真实 400+机器码且状态不漂移。
//      前置：generate 对账口径排除 DRAFT/CANCELLED（ap_reconciliation_ops/crud.rs:41-44），
// 发票建单默认 DRAFT（ap_invoice_ops/crud.rs:75），须先 approve 到 AUDITED（判责，
// 推翻 §D「核销回写事务真缺陷」——缺的是用例审核步骤，后端口径不动）。
//      状态词表：backend/src/models/status/finance.rs:104-112（PENDING/CONFIRMED/DISPUTED 大写）。
//   4. AP 付款申请 submit 门控负例（ap_payment_request_service.rs:336-338）：
//      无 items 建单成功（DRAFT）→ submit 被拒 400 BUSINESS_ERROR → 回读仍 DRAFT（无副作用）
//      → 对照：带真实 items 的申请 submit 成功 → APPROVING。
//      诚实标注①：门控文案「付款申请没有明细，不可提交」由 AppError::business 构造，
//      HTTP 出参 message 统一脱敏为「业务处理失败」（utils/error.rs:490-504 白名单式外显），
//      断该原文必红且断的是脱敏契约而非门控行为，故改断真实可得的事实：
//      400 + BUSINESS_ERROR 机器码 + 状态回读未迁移。若需外显该文案，须后端将该构造点
//      迁为 business_displayable——交编排方判责，e2e 不掩盖。
//      诚实标注②：UpdateApPaymentRequest（ap_payment_request_service.rs:698-729）不含 items，
//      路由也无申请明细子资源——已建 DRAFT 单没有任何合法端点可为其「补明细」，
//      故「补真实 items→再 submit 成功」以同供应商同金额、唯一变量为 items 的第二张单对照证明。
//      诚实标注③：AP cancel 只置核销单 CANCELLED 并回退发票金额，不删 ap_verification_item，
//      故「未核销付款」列表在 cancel 后仍不含该付款（verified_map 把已取消明细也计入）——
//      属后端统计口径缺陷，本用例不为其放宽或反向断言，仅断发票金额回退这一资金真值。
// 红线：不用 verifyEndpointHealthy / >=400 / 仅 toast / toBeTruthy；金额键缺失即抛错不 ?? 0 兜底。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 读取响应的一个金额字段：键缺失/非数字直接抛错，不 `?? 0` 兜底掩盖缺键（假绿来源）。
 * rust_decimal 默认序列化为字符串（"600.00"），故 Number() 解析。
 */
function readAmount(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) {
    throw new Error(`${label}：键 "${key}" 不可解析为有限数字，raw=${JSON.stringify(obj[key])}`);
  }
  return n;
}

/** 断言两个金额相等（容差 0.01，货币精度），失败信息打印实际值。 */
function expectMoney(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}：期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

/** 从 {id:...} 形状的响应取数字主键，取不到即抛错（不静默继续）。 */
function requireId(obj: Record<string, unknown>, label: string): number {
  const id = Number(obj.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(`${label}：响应无有效 id，实际键=${Object.keys(obj).join(',')}`);
  }
  return id;
}

function today(): string {
  return new Date().toISOString().slice(0, 10);
}

async function seedSupplier(page: import('@playwright/test').Page, tag: string): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E核销11${tag}${Date.now().toString().slice(-8)}`,
    supplier_short_name: 'E2E11',
    contact_phone: '13800003333',
  });
  if (!res.data?.id) throw new Error(`建供应商失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${res.data.id}`, label: 'supplier' });
  return res.data.id;
}

async function seedCustomer(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
    customer_name: `E2E核销11客${Date.now().toString().slice(-8)}`,
  });
  if (!res.data?.id) throw new Error(`建客户失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/crm/customers/${res.data.id}`, label: 'customer' });
  return res.data.id;
}

/** 建 AP 应付单并审核（DRAFT→AUDITED），返回 id。amount 为应付原额。 */
async function seedApprovedApInvoice(
  page: import('@playwright/test').Page,
  supplierId: number,
  amount: number
): Promise<number> {
  const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/invoices', {
    supplier_id: supplierId,
    invoice_type: 'PURCHASE',
    amount,
    invoice_date: today(),
    due_date: today(),
  });
  const id = requireId(inv, '建应付单');
  CLEANUP.push({ path: `/ap/invoices/${id}`, label: 'ap_invoice' });
  await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);
  return id;
}

/** 方案B付款链：申请(引用 invoiceId，金额 applyAmount) → 提交 → 审批 → 建付款 → 流水号 → 确认。 */
async function seedConfirmedApPayment(
  page: import('@playwright/test').Page,
  supplierId: number,
  invoiceId: number,
  applyAmount: number
): Promise<number> {
  const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
    supplier_id: supplierId,
    request_date: today(),
    payment_type: 'PROGRESS',
    payment_method: 'TT',
    request_amount: applyAmount,
    currency: 'CNY',
    items: [{ invoice_id: invoiceId, apply_amount: applyAmount, notes: 'E2E11 核销闭环前置付款' }],
  });
  const requestId = requireId(req, '建付款申请');
  await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
  await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);

  const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
    request_id: requestId,
    payment_date: today(),
  });
  const paymentId = requireId(pay, '建付款单');
  await apiCall(page, 'PUT', `/ap/payments/${paymentId}`, {
    transaction_no: genCode('E2E11-TXN'),
  });
  await apiCall(page, 'POST', `/ap/payments/${paymentId}/confirm`);
  return paymentId;
}

/** 取数组条目的数字 id 集合（缺 id 抛错，不假定字段名拼写之外的形状）。 */
function idsOf(list: Array<Record<string, unknown>>, label: string): number[] {
  return list.map(o => {
    const id = Number(o.id);
    if (!Number.isFinite(id)) throw new Error(`${label}：条目缺少数字 id：${JSON.stringify(o)}`);
    return id;
  });
}

interface ArVerificationListItem extends Record<string, unknown> {
  id: number;
}
interface ArVerificationList {
  list: ArVerificationListItem[];
  total: number;
}
interface ArVerificationDetail extends Record<string, unknown> {
  items: Array<Record<string, unknown>>;
}

test.describe('11 AP/AR 核销闭环 + AP 对账 + 付款申请提交门控', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    // 会计期间初始化：AP 付款确认生成付款凭证、AR 收款确认都要过 check_date_locked。
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[11] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('11-01 AP 手工核销闭环：manual 后回读发票 paid 增 unpaid 减，cancel 后精确回退', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, 'A');
    // 发票 A(1000)：被付款申请引用，confirm 时已分配 600（paid=600，unpaid=400）。
    const invoiceA = await seedApprovedApInvoice(page, supplierId, 1000);
    // 发票 B(600)：付款 confirm 未引用它（申请明细只挂 A），手工核销把它核到结清，
    // 避免同一笔钱在 confirm 分配与核销累加里落到同一张发票上（那将断不清真实增量来源）。
    const invoiceB = await seedApprovedApInvoice(page, supplierId, 600);
    const paymentId = await seedConfirmedApPayment(page, supplierId, invoiceA, 600);

    // confirm 已把 A 推到 paid=600（与 04-02 同口径），作为核销前基线复核。
    const aBefore = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceA}`
    );
    expectMoney(readAmount(aBefore, 'paid_amount', 'confirm 后 A'), 600, 'A confirm 后 paid');

    // B 核销前金额真值：paid=0、unpaid=600、状态 AUDITED（approve 写入值）。
    const bBefore = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceB}`
    );
    expectMoney(readAmount(bBefore, 'paid_amount', '核销前 B'), 0, 'B 核销前 paid');
    expectMoney(readAmount(bBefore, 'unpaid_amount', '核销前 B'), 600, 'B 核销前 unpaid');
    expect(String(bBefore.invoice_status), 'B approve 后应为 AUDITED').toBe('AUDITED');

    // 未核销清单：付款与发票 B 都应可见（核销关系尚未建立）。
    const unverifiedPaymentsBefore = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/ap/verifications/unverified/payments?supplier_id=${supplierId}`
    );
    expect(
      idsOf(unverifiedPaymentsBefore, '未核销付款'),
      `核销前未核销付款列表应包含付款单 ${paymentId}`
    ).toContain(paymentId);
    const unverifiedInvoicesBefore = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/ap/verifications/unverified/invoices?supplier_id=${supplierId}`
    );
    expect(
      idsOf(unverifiedInvoicesBefore, '未核销应付单'),
      `核销前未核销应付单列表应包含发票 B=${invoiceB}`
    ).toContain(invoiceB);

    // 手工核销：付款 600 ↔ 发票 B 600。
    const verify = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/ap/verifications/manual',
      {
        supplier_id: supplierId,
        items: [
          {
            invoice_id: invoiceB,
            payment_id: paymentId,
            verify_amount: 600,
            notes: 'E2E11 手工核销',
          },
        ],
        notes: 'E2E11 手工核销闭环',
      }
    );
    const vid = requireId(verify, '手工核销单');
    expect(String(verify.verification_type), '手工核销单类型应为 MANUAL').toBe('MANUAL');
    expect(String(verify.verification_status), '核销即时状态应为 COMPLETED').toBe('COMPLETED');
    expectMoney(readAmount(verify, 'total_amount', '核销单'), 600, '核销单 total_amount');

    // 回读发票 B：paid=600、unpaid=0、状态 PAID；恒等 amount-paid=unpaid。
    const bAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceB}`
    );
    expectMoney(readAmount(bAfter, 'paid_amount', '核销后 B'), 600, 'B 核销后 paid 增');
    expectMoney(readAmount(bAfter, 'unpaid_amount', '核销后 B'), 0, 'B 核销后 unpaid 减');
    expectMoney(
      readAmount(bAfter, 'amount', 'B 原额') - readAmount(bAfter, 'paid_amount', 'B 已付'),
      readAmount(bAfter, 'unpaid_amount', 'B 未付'),
      'AP 恒等：amount-paid=unpaid（核销后）'
    );
    expect(String(bAfter.invoice_status), 'B 全额核销后状态应为 PAID').toBe('PAID');

    // 回读核销单详情端点（金额回读走真实 GET，不信任 POST 响应体）。
    const vAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/verifications/${vid}`
    );
    expect(String(vAfter.verification_status), 'GET 核销单应为 COMPLETED').toBe('COMPLETED');
    expectMoney(readAmount(vAfter, 'total_amount', 'GET 核销单'), 600, 'GET 核销单金额');

    // 付款已全额核销 → 从未核销付款列表消失。
    const unverifiedPaymentsAfter = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/ap/verifications/unverified/payments?supplier_id=${supplierId}`
    );
    expect(
      idsOf(unverifiedPaymentsAfter, '未核销付款(核销后)'),
      `核销后付款 ${paymentId} 应离开未核销列表`
    ).not.toContain(paymentId);
    // 发票 B unpaid=0 → 离开未核销应付单列表；发票 A 仍 unpaid=400 在列。
    const unverifiedInvoicesAfter = await apiCallRaw<Array<Record<string, unknown>>>(
      page,
      'GET',
      `/ap/verifications/unverified/invoices?supplier_id=${supplierId}`
    );
    const invIdsAfter = idsOf(unverifiedInvoicesAfter, '未核销应付单(核销后)');
    expect(invIdsAfter, `核销后发票 B=${invoiceB} 应离开未核销列表`).not.toContain(invoiceB);
    expect(invIdsAfter, `发票 A=${invoiceA} unpaid=400 应仍在列表`).toContain(invoiceA);

    // 取消核销（见文件头诚实标注③：cancel 后不改判「未核销列表」口径）。
    const cancelReason = 'E2E11 取消核销回退';
    await apiCall(page, 'POST', `/ap/verifications/${vid}/cancel`, { reason: cancelReason });

    // 回读核销单：CANCELLED + 原因回显 + 取消人非空。
    const vCancelled = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/verifications/${vid}`
    );
    expect(String(vCancelled.verification_status), '取消后核销单应为 CANCELLED').toBe('CANCELLED');
    expect(String(vCancelled.cancelled_reason), '取消原因应回显').toBe(cancelReason);
    expect(Number(vCancelled.cancelled_by), '取消人应为真实操作者 id').toBeGreaterThan(0);

    // 回读发票 B：金额精确回退（paid=0、unpaid=600），状态按门控回 AUDITED（paid==0 分支，
    // restore_invoices_on_cancel，ap_verification_service.rs:548-556）。
    const bRolled = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceB}`
    );
    expectMoney(readAmount(bRolled, 'paid_amount', '取消后 B'), 0, 'B 取消后 paid 回退');
    expectMoney(readAmount(bRolled, 'unpaid_amount', '取消后 B'), 600, 'B 取消后 unpaid 回退');
    expect(String(bRolled.invoice_status), 'B 取消后状态门控回 AUDITED').toBe('AUDITED');

    // 取消只作用于本次核销明细的发票：A 不受牵连。
    const aRolled = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/invoices/${invoiceA}`
    );
    expectMoney(readAmount(aRolled, 'paid_amount', '取消后 A'), 600, 'A 不受 cancel 影响');
    expectMoney(readAmount(aRolled, 'unpaid_amount', '取消后 A'), 400, 'A 未付保持 400');
  });

  test('11-02 AR 核销链：auto→manual→cancel 全链金额回读', async ({ page }) => {
    const customerId = await seedCustomer(page);
    const d = today();

    // 应收单 1000，received=0/unpaid=1000 基线。
    const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/invoices', {
      customer_id: customerId,
      invoice_amount: 1000,
      invoice_date: d,
      due_date: d,
    });
    const invoiceId = requireId(inv, '建应收单');
    CLEANUP.push({ path: `/ar/invoices/${invoiceId}`, label: 'ar_invoice' });
    expectMoney(readAmount(inv, 'received_amount', '建单后'), 0, '建单 received');
    expectMoney(readAmount(inv, 'unpaid_amount', '建单后'), 1000, '建单 unpaid');

    // 收款 300：不带 invoice_ids（见文件头说明），并回读确认建单未动发票金额。
    const c1 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: customerId,
      amount: 300,
      payment_method: '银行转账',
      payment_date: d,
    });
    const collectionId1 = requireId(c1, '建收款 C1');
    const invAfterCreate = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(
      readAmount(invAfterCreate, 'received_amount', '收款建单未确认未关联'),
      0,
      '无关联收款建单不应改发票'
    );

    // confirm：pending→confirmed（核销要求收款已确认，词表 models/status/finance.rs:16）。
    await apiCall(page, 'POST', `/ar/payments/${collectionId1}/confirm`);
    const c1Got = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/payments/${collectionId1}`
    );
    expect(String(c1Got.status), '收款确认后应为 confirmed').toBe('confirmed');

    // 自动核销（全局贪心；verified_count 是全库口径不作本用例真值断言，
    // 真值一律走发票与核销单的真实 GET 回读。专属新客+新单 → 本发票增量确定）。
    await apiCall(page, 'POST', '/ar/verifications/auto');
    const invAfterAuto = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(
      readAmount(invAfterAuto, 'received_amount', 'auto 核销后'),
      300,
      'auto 后 received 增'
    );
    expectMoney(readAmount(invAfterAuto, 'unpaid_amount', 'auto 核销后'), 700, 'auto 后 unpaid 减');
    expect(String(invAfterAuto.status), 'auto 后发票状态应为 PARTIAL_PAID').toBe('PARTIAL_PAID');

    // 定位 auto 生成的核销单（发票+收款双过滤；专属新客数据 → 恰 1 条）。
    const autoList = await apiCallRaw<ArVerificationList>(
      page,
      'GET',
      `/ar/verifications?invoice_id=${invoiceId}&payment_id=${collectionId1}`
    );
    expect(autoList.list.length, `auto 核销单应恰 1 条，实际=${autoList.list.length}`).toBe(1);
    const autoRecId = requireId(autoList.list[0], 'auto 核销单');
    expect(String(autoList.list[0].reconciliation_status), 'auto 核销单应为 closed').toBe('closed');

    // 手工核销：第二笔确认收款 700 ↔ 同一发票剩余 700 → 结清。
    const c2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ar/payments', {
      customer_id: customerId,
      amount: 700,
      payment_method: '银行转账',
      payment_date: d,
    });
    const collectionId2 = requireId(c2, '建收款 C2');
    await apiCall(page, 'POST', `/ar/payments/${collectionId2}/confirm`);

    const manual = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/ar/verifications/manual',
      { invoice_id: invoiceId, payment_id: collectionId2, amount: 700, remark: 'E2E11 手工核销' }
    );
    const manualRecId = requireId(manual, 'AR 手工核销单');
    expect(String(manual.status), 'AR 手工核销即时状态应为 closed').toBe('closed');

    const invAfterManual = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(
      readAmount(invAfterManual, 'received_amount', 'manual 后'),
      1000,
      'manual 后 received 全额'
    );
    expectMoney(
      readAmount(invAfterManual, 'unpaid_amount', 'manual 后'),
      0,
      'manual 后 unpaid 归零'
    );
    expect(String(invAfterManual.status), '结清后发票状态应为 PAID').toBe('PAID');
    // 会计恒等：invoice_amount - received = unpaid
    expectMoney(
      readAmount(invAfterManual, 'invoice_amount', '应收原额') -
        readAmount(invAfterManual, 'received_amount', '已收'),
      readAmount(invAfterManual, 'unpaid_amount', '未收'),
      'AR 恒等：invoice-received=unpaid（manual 后）'
    );

    // 核销详情含双向明细（INVOICE + RECEIPT）。
    const detail = await apiCallRaw<ArVerificationDetail>(
      page,
      'GET',
      `/ar/verifications/${manualRecId}`
    );
    expect(String(detail.reconciliation_status), 'manual 核销单详情应为 closed').toBe('closed');
    expect(detail.items.length, '核销明细应为 INVOICE+RECEIPT 两条').toBe(2);
    const types = detail.items.map(i => String(i.item_type)).sort();
    expect(types, '明细类型集合').toEqual(['INVOICE', 'RECEIPT']);

    // 取消手工核销 → 回退 manual 那笔 700，auto 的 300 不受牵连。
    await apiCall(page, 'POST', `/ar/verifications/${manualRecId}/cancel`);
    const cancelled = await apiCallRaw<ArVerificationDetail>(
      page,
      'GET',
      `/ar/verifications/${manualRecId}`
    );
    expect(String(cancelled.reconciliation_status), '取消后核销单应为 cancelled').toBe('cancelled');

    const invAfterCancel = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/invoices/${invoiceId}`
    );
    expectMoney(
      readAmount(invAfterCancel, 'received_amount', 'cancel 后'),
      300,
      'cancel 只退 manual 的 700'
    );
    expectMoney(
      readAmount(invAfterCancel, 'unpaid_amount', 'cancel 后'),
      700,
      'cancel 后 unpaid 回 700'
    );
    expect(String(invAfterCancel.status), 'cancel 后回到 PARTIAL_PAID（auto 核销仍在）').toBe(
      'PARTIAL_PAID'
    );

    // auto 核销单未被牵连取消。
    const autoStill = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ar/verifications/${autoRecId}`
    );
    expect(String(autoStill.reconciliation_status), 'auto 核销单应保持 closed').toBe('closed');
  });

  test('11-03 AP 对账 generate→confirm：状态词表迁移与门控负例', async ({ page }) => {
    const supplierId = await seedSupplier(page, 'R1');
    const d = today();
    // 专属新供应商 + 当日唯一发票 → 期初 0、本期应付 800、本期付款 0 全部可控。
    // 对账纳入口径（判责推翻 §D 的「核销回写事务真缺陷」）
    // generate 排除 DRAFT/CANCELLED（backend/src/services/ap_reconciliation_ops/crud.rs:41-44）是既定口径，
    // 而建单默认即 DRAFT（ap_invoice_ops/crud.rs:75 invoice_status=STATUS_DRAFT），
    // 必须走真实审核步 approve（DRAFT→AUDITED，ap_invoice_ops/crud.rs:204）发票才进对账集合。
    // 旧注释「无需审批」为误判，缺的是用例前置步骤，不是后端缺陷；不得反向把 DRAFT 放进 generate。
    await seedApprovedApInvoice(page, supplierId, 800);

    const rec = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/ap/reconciliations/generate',
      { supplier_id: supplierId, start_date: d, end_date: d, notes: 'E2E11 对账确认链' }
    );
    const recId = requireId(rec, '生成对账单');

    // 回读：初始态 PENDING + 期末余额算术真值（opening+invoice-payment=closing）。
    const r0 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r0.reconciliation_status), '生成后初始态应为 PENDING').toBe('PENDING');
    expectMoney(readAmount(r0, 'opening_balance', '对账期初'), 0, '新供应商期初应为 0');
    expectMoney(readAmount(r0, 'total_invoice', '本期应付'), 800, '本期应付合计');
    expectMoney(readAmount(r0, 'total_payment', '本期付款'), 0, '新供应商本期无付款');
    expectMoney(readAmount(r0, 'closing_balance', '期末'), 800, '期末=0+800-0');

    // confirm：PENDING→CONFIRMED，确认人落真实操作者。
    await apiCall(page, 'POST', `/ap/reconciliations/${recId}/confirm`);
    const r1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r1.reconciliation_status), '确认后应为 CONFIRMED').toBe('CONFIRMED');
    expect(Number(r1.confirmed_by), 'confirmed_by 应为真实操作者 id').toBeGreaterThan(0);

    // 门控负例：已确认再确认被拒（ap_reconciliation_ops/confirm.rs:113-118）。
    // 断真实 400+机器码且状态不漂移；文案被 AppError::business 脱敏，不断原文（诚实标注①同理）。
    const fail = await apiCallExpectFail(page, 'POST', `/ap/reconciliations/${recId}/confirm`);
    expect(fail.status, `重复 confirm 应 400，实际 status=${fail.status} code=${fail.code}`).toBe(
      400
    );
    expect(failureCode(fail), '重复 confirm 机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const r2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r2.reconciliation_status), '被拒后状态不得漂移').toBe('CONFIRMED');
  });

  test('11-04 AP 对账 generate→dispute：争议分支回读原因回显与门控负例', async ({ page }) => {
    const supplierId = await seedSupplier(page, 'R2');
    const d = today();
    // 同 11-03：发票必须 approve 到 AUDITED 才计入 generate 对账口径
    // （ap_reconciliation_ops/crud.rs:41-44 排除 DRAFT 是既定口径，见 11-03 注释）。
    await seedApprovedApInvoice(page, supplierId, 500);

    const rec = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/ap/reconciliations/generate',
      { supplier_id: supplierId, start_date: d, end_date: d, notes: 'E2E11 争议链' }
    );
    const recId = requireId(rec, '生成对账单');
    const r0 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r0.reconciliation_status), '生成后初始态应为 PENDING').toBe('PENDING');
    expectMoney(readAmount(r0, 'closing_balance', '期末'), 500, '期末=0+500-0');

    // dispute：PENDING→DISPUTED，原因回显（用户自己提交的文本，可安全断真值）。
    const reason = 'E2E11 争议：对账金额与到货不符';
    await apiCall(page, 'POST', `/ap/reconciliations/${recId}/dispute`, { reason });
    const r1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r1.reconciliation_status), '争议后应为 DISPUTED').toBe('DISPUTED');
    expect(String(r1.disputed_reason), '争议原因应原样回显').toBe(reason);
    expect(Number(r1.disputed_by), 'disputed_by 应为真实操作者 id').toBeGreaterThan(0);

    // 门控负例：DISPUTED 不可确认（confirm 状态门仅收 PENDING），状态不得被推成 CONFIRMED。
    const fail = await apiCallExpectFail(page, 'POST', `/ap/reconciliations/${recId}/confirm`);
    expect(fail.status, `DISPUTED 后 confirm 应 400，实际=${fail.status}`).toBe(400);
    expect(failureCode(fail), 'DISPUTED 后 confirm 机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const r2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/reconciliations/${recId}`
    );
    expect(String(r2.reconciliation_status), '被拒后应停留在 DISPUTED').toBe('DISPUTED');
  });

  test('11-05 AP 付款申请 submit 门控：无明细被拒且状态不漂移，带明细提交成功', async ({
    page,
  }) => {
    const supplierId = await seedSupplier(page, 'G');
    const invoiceId = await seedApprovedApInvoice(page, supplierId, 1000);
    const base = {
      supplier_id: supplierId,
      request_date: today(),
      payment_type: 'PROGRESS',
      payment_method: 'TT',
      request_amount: 1000,
      currency: 'CNY',
    };

    // ① 无 items 建单成功（items 为 Option + #[serde(default)]，ap_payment_request_service.rs:654）。
    const r1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/ap/payment-requests',
      base
    );
    const requestId1 = requireId(r1, '无明细建付款申请');
    CLEANUP.push({ path: `/ap/payment-requests/${requestId1}`, label: 'ap_payment_request' });
    expect(String(r1.approval_status), '创建后应为 DRAFT').toBe('DRAFT');

    // ② submit 被拒：items.is_empty() 门控抛 AppError::business（service:336-338）。
    // 文案被脱敏，不断原文（文件头诚实标注①）。
    const fail = await apiCallExpectFail(page, 'POST', `/ap/payment-requests/${requestId1}/submit`);
    expect(
      fail.status,
      `无明细 submit 应 400，实际 status=${fail.status} code=${fail.code} message=${fail.message}`
    ).toBe(400);
    expect(failureCode(fail), '无明细 submit 机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // ③ 回读：被拒后状态未迁移（门控无副作用，不是「先迁移再报错」）。
    const g1 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${requestId1}`
    );
    expect(String(g1.approval_status), 'submit 被拒后应仍为 DRAFT').toBe('DRAFT');

    // ④ DRAFT 也不可审批（approve 状态门仅 APPROVING，service:379-385）。
    const failApprove = await apiCallExpectFail(
      page,
      'POST',
      `/ap/payment-requests/${requestId1}/approve`
    );
    expect(failApprove.status, `DRAFT 直接 approve 应 400，实际=${failApprove.status}`).toBe(400);
    expect(failureCode(failApprove), 'DRAFT approve 机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // ⑤ 对照：带真实 items 的申请 submit 成功 → APPROVING（证明 ② 的拒绝确由缺明细触发）。
    // 唯一变量是 items（同一单据无法补明细，见文件头诚实标注②）。
    const r2 = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
      ...base,
      items: [{ invoice_id: invoiceId, apply_amount: 1000, notes: 'E2E11 明细门控对照' }],
    });
    const requestId2 = requireId(r2, '带明细建付款申请');
    // APPROVING 后不可删除（delete 仅 DRAFT/REJECTED），tryCleanup 记 warn，留日志不留断言。
    CLEANUP.push({
      path: `/ap/payment-requests/${requestId2}`,
      label: 'ap_payment_request(提交后不可删属预期)',
    });
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId2}/submit`);
    const g2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/ap/payment-requests/${requestId2}`
    );
    expect(String(g2.approval_status), '带明细 submit 后应为 APPROVING').toBe('APPROVING');
  });
});
