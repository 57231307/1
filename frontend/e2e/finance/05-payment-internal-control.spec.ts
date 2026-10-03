// 财务管理 E2E 套件 — 05 AP 付款「方案B 禁手工绕过」内控负向
//
// 任务 #942 缺口 2：AP 付款必须经「已审批的付款申请」派生（方案B 内控），
// 后端 ap_payment_service.rs::create 强制：request_id 必填、对应申请必须 APPROVED、
// 付款金额取 request_amount（CreateApPaymentRequest 根本不含金额字段）——三者共同构成"不可手工绕过"。
// 本套件用真实后端断这三条内控，全部断真实状态码/业务码/回读金额，禁 verifyEndpointHealthy / >=400。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  tryCleanup,
  APP_ERROR_CODES,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function num(obj: Record<string, unknown>, key: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`响应缺少键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n)) throw new Error(`键 "${key}" 非数字，raw=${JSON.stringify(obj[key])}`);
  return n;
}

async function seedApprovedInvoice(
  page: import('@playwright/test').Page,
  supplierId: number
): Promise<number> {
  const today = new Date().toISOString().slice(0, 10);
  const inv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/invoices', {
    supplier_id: supplierId,
    invoice_type: 'PURCHASE',
    amount: 1000,
    invoice_date: today,
    due_date: today,
  });
  const id = Number(inv.id);
  if (!id) throw new Error(`建应付单失败：${JSON.stringify(inv)}`);
  CLEANUP.push({ path: `/ap/invoices/${id}`, label: 'ap_invoice' });
  await apiCall(page, 'POST', `/ap/invoices/${id}/approve`);
  return id;
}

async function seedSupplier(page: import('@playwright/test').Page): Promise<number> {
  const res = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
    supplier_name: `E2E内控供${Date.now().toString().slice(-8)}`,
    supplier_short_name: 'E2E内',
    contact_phone: '13800002222',
  });
  if (!res.data?.id) throw new Error(`建供应商失败：${JSON.stringify(res)}`);
  CLEANUP.push({ path: `/purchase/suppliers/${res.data.id}`, label: 'supplier' });
  return res.data.id;
}

async function createDraftRequest(
  page: import('@playwright/test').Page,
  supplierId: number,
  invoiceId: number,
  amount: number
): Promise<number> {
  const today = new Date().toISOString().slice(0, 10);
  const req = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payment-requests', {
    supplier_id: supplierId,
    request_date: today,
    payment_type: 'PROGRESS',
    payment_method: 'TT',
    request_amount: amount,
    currency: 'CNY',
    items: [{ invoice_id: invoiceId, apply_amount: amount, notes: 'E2E 内控' }],
  });
  const id = Number(req.id);
  if (!id) throw new Error(`建付款申请失败：${JSON.stringify(req)}`);
  return id;
}

test.describe('05 AP 付款方案B内控负向', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await apiCall(page, 'POST', '/finance/accounting-periods/init', {}).catch(e =>
      console.warn('[05] 会计期间初始化失败:', (e as Error).message)
    );
  });

  test('05-01 未经审批的付款申请直接建付款被拒（400 BUSINESS_ERROR，非 5xx）', async ({ page }) => {
    const supplierId = await seedSupplier(page);
    const invoiceId = await seedApprovedInvoice(page, supplierId);
    const today = new Date().toISOString().slice(0, 10);

    // 只建 DRAFT 付款申请，不提交/审批。
    const requestId = await createDraftRequest(page, supplierId, invoiceId, 500);

    const fail = await apiCallExpectFail(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: today,
    });
    // 内控：未审批不可付款 → 400 + BUSINESS_ERROR 机器码（绝非 5xx，也非成功）。
    expect(fail.status, `应被拒为 400，实际 status=${fail.status} code=${fail.code}`).toBe(400);
    expect(failureCode(fail), `业务码应为 BUSINESS_ERROR，实际=${fail.code}`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(fail.message ?? '', '错误文案应指明未审批不可付款').toContain('审批');
  });

  test('05-02 缺失 request_id 的手工付款请求被拒（4xx 客户端错误，非 2xx/5xx）', async ({
    page,
  }) => {
    const today = new Date().toISOString().slice(0, 10);
    // 不带 request_id（后端 CreateApPaymentRequest.request_id 为必填 i32）。
    const fail = await apiCallExpectFail(page, 'POST', '/ap/payments', { payment_date: today });
    // 必填字段缺失 → axum Json 抽取器结构化拒绝；只允许 4xx（缺字段为 422/400），
    // 明确排除被当成"成功付款"(2xx) 与"服务器崩溃"(5xx)。这是结构性"不可绕过"，非放宽断言。
    expect([400, 403, 422], `缺 request_id 应被拒为 4xx，实际 status=${fail.status}`).toContain(
      fail.status
    );
  });

  test('05-03 试图手工注入付款金额被结构忽略：实付金额恒等于已审批申请金额', async ({ page }) => {
    const supplierId = await seedSupplier(page);
    const invoiceId = await seedApprovedInvoice(page, supplierId);
    const today = new Date().toISOString().slice(0, 10);

    // 建并审批一个申请额 777 的付款申请（admin 审批 ≤10万）。
    const requestId = await createDraftRequest(page, supplierId, invoiceId, 777);
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/approve`);

    // 建付款时恶意附带一个不存在的 payment_amount 字段（后端 DTO 无此字段，应被忽略）。
    const pay = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/ap/payments', {
      request_id: requestId,
      payment_date: today,
      payment_amount: 999999,
    });
    const paymentId = Number(pay.id);
    if (!paymentId) throw new Error(`建付款单失败：${JSON.stringify(pay)}`);
    CLEANUP.push({ path: `/ap/payments/${paymentId}`, label: 'ap_payment' });

    // 回读付款单：金额取申请派生值 777，而非注入的 999999 → 证明"手填金额"被内控消灭。
    const got = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/ap/payments/${paymentId}`);
    expect(num(got, 'payment_amount'), '实付金额必须等于申请金额 777（注入值应被忽略）').toBe(777);
  });
});
