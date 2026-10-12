// 财务管理 E2E 套件 — 12 发票管理（/finance/invoices，页面 /invoice-details）
//
// 覆盖审计确证发票管理域零覆盖。契约真值来源：
//   路由 backend/src/routes/finance.rs:43-61（/invoices CRUD + /{id}/approve + /{id}/verify，
//   经 mod.rs nest 至 /api/v1/erp/finance → 调用路径 /finance/invoices*）；
//   状态词表 backend/src/models/status/finance.rs:87-93：pending → approved → verified（全小写），
//   approve 状态门仅 pending（finance_invoice_service.rs:158-167），
//   verify 写入 status="verified" 并落 paid_date（finance_invoice_service.rs:189-226）。
// 全链每步「写后必 GET 回读」，含一个重复审批门控负例（错误族归一 763c7ab9 后该状态门走
//   脱敏 AppError::business → 断真实 400+BUSINESS_ERROR 机器码与状态不漂移；
//   文案回显内部状态 token 不外显，不断 message 原文），
// 最后到 /invoice-details 页面做 UI 级真值回读（表格行状态文案来自 GET 列表实数据，非 mock）。
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
  BASE_URL,
} from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 金额键读取：缺失/非数字直接抛错，不 `?? 0` 兜底掩盖缺键。Decimal 序列化为字符串，Number() 解析。 */
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

interface FinanceInvoiceListData {
  invoices: Array<Record<string, unknown>>;
  total: number;
}

test.describe('12 发票管理建→审批→核销全链回读', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('12-01 建→approve→verify 全链 GET 回读状态词表迁移 + 重复审批负例 + UI 行回读', async ({
    page,
  }) => {
    const invoiceNo = genCode('E2E12-INV');

    // ① 创建：初始态 pending，金额三件套按请求值落库（后端 round_dp(2) 归一，整数输入原值回读）。
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/finance/invoices', {
      invoice_no: invoiceNo,
      amount: 1000,
      tax_amount: 60,
      total_amount: 1060,
    });
    const id = Number(created.id);
    if (!Number.isFinite(id) || id <= 0)
      throw new Error(`建发票无有效 id：${JSON.stringify(created)}`);
    CLEANUP.push({ path: `/finance/invoices/${id}`, label: 'finance_invoice' });
    expect(String(created.status), '创建后初始态应为 pending').toBe('pending');
    expect(String(created.invoice_no), '发票号应原样落库回显').toBe(invoiceNo);

    // ② GET 回读创建真值（不信任 POST 响应体）。
    const g0 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/finance/invoices/${id}`);
    expect(String(g0.status), 'GET 回读创建态应为 pending').toBe('pending');
    expectMoneyEq(g0, 'amount', 1000, '创建金额');
    expectMoneyEq(g0, 'tax_amount', 60, '创建税额');
    expectMoneyEq(g0, 'total_amount', 1060, '创建价税合计');
    expect(g0.paid_date, '未核销时 paid_date 应为空').toBeNull();

    // ③ approve：pending→approved，回读迁移。
    await apiCall(page, 'POST', `/finance/invoices/${id}/approve`);
    const g1 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/finance/invoices/${id}`);
    expect(String(g1.status), '审批后应为 approved').toBe('approved');

    // ④ verify：approved→verified，paid_date 由空落为真实时间（handler 无状态门，链内合法前置）。
    await apiCall(page, 'POST', `/finance/invoices/${id}/verify`);
    const g2 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/finance/invoices/${id}`);
    expect(String(g2.status), '核销后应为 verified').toBe('verified');
    expect(g2.paid_date, '核销后 paid_date 应已写入（非空字符串）').not.toBeNull();
    expect(String(g2.paid_date).length, 'paid_date 应为真实时间文本').toBeGreaterThan(0);

    // ⑤ 门控负例：verified 再 approve 被拒（状态门仅 pending，service:159-167）。
    // 装配点 finance_invoice_service.rs:163 错误族归一（763c7ab9）由 bad_request 改
    // AppError::business（脱敏，文案含内部状态 token）→ 400 + BUSINESS_ERROR；
    // 断真实 400 + 机器码，且状态不漂移；不断脱敏后的 message 原文。
    const fail = await apiCallExpectFail(page, 'POST', `/finance/invoices/${id}/approve`);
    expect(
      fail.status,
      `verified 后重复 approve 应 400，实际=${fail.status} code=${fail.code}`
    ).toBe(400);
    expect(failureCode(fail), '重复 approve 机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const g3 = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/finance/invoices/${id}`);
    expect(String(g3.status), '被拒后状态不得漂移').toBe('verified');

    // ⑥ 列表端点回读：契约键为 {invoices,total}（finance_invoice_handler.rs:33-36，
    // 注意该端点无分页、全量返回，不属于 PaginatedResponse 一族）。
    const list = await apiCallRaw<FinanceInvoiceListData>(page, 'GET', '/finance/invoices');
    expect(Array.isArray(list.invoices), '列表响应应含数组键 invoices').toBe(true);
    expect(typeof list.total, '列表响应应含数值键 total').toBe('number');
    const row = list.invoices.find(o => String(o.invoice_no) === invoiceNo);
    if (!row) throw new Error(`列表 GET 未回读到发票 ${invoiceNo}（total=${list.total}）`);
    expect(String(row.status), '列表中该发票状态应为 verified').toBe('verified');

    // ⑦ UI 级回读：/invoice-details 表格行由真实 GET 数据渲染，
    // verified 对应状态文案「已核销」（views/invoice-details/index.vue:143-148）。
    await page.goto(`${BASE_URL}/invoice-details`, {
      waitUntil: 'domcontentloaded',
      timeout: 30000,
    });
    const invoiceRow = page
      .getByRole('row')
      .filter({ has: page.getByText(invoiceNo, { exact: true }) });
    await expect(invoiceRow, '页面应渲染出该发票行').toHaveCount(1);
    await expect(invoiceRow, '页面行内状态应显示「已核销」').toContainText('已核销');
  });
});

/** 金额相等断言（容差 0.01），键缺失即抛错。 */
function expectMoneyEq(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  const actual = readAmount(obj, key, label);
  expect(Math.abs(actual - expected), `${label}：期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}
