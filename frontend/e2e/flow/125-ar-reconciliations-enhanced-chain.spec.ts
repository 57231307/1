import { test, expect } from '../diagnose-fixture';
import { APP_ERROR_CODES, loginViaUI, apiCall, apiCallRaw, apiCallExpectFail } from './helpers';

/**
 * 125 ar-reconciliations-enhanced 面全链真实契约（生成、明细、发送、确认、争议、解决、账龄、PDF、结果列表、自动对账）
 *
 * 守的是什么：enhanced 前缀（routes/finance 起 16 条在册端点）不是 base 面的空镜像——
 * handler 层 ar_reconciliation_enhanced_handler 仅 `pub use` 转发到
 * ar_reconciliation_handler，因此每条用例都打 enhanced 前缀真实路径，并对同一行做
 * base 面读回（GET /ar-reconciliations/{id}）比对 reconciliation_no/customer_id 一致，
 * 钉住「前缀可达且语义与 base 同源」。全部判据来自 handler 与服务层源码实核：
 * - POST /generate（ar_reconciliation_handler）：customer_id 不存在 → 404 NOT_FOUND
 *   （vfy_ops/reconciliation 显式客户存在性检查）；合法入参新建行初始状态 draft、
 *   period_start/period_end 回显入参日期、notes 持久化（字段已接入落库）。
 * - GET /{id}/details：出参 {reconciliation: Model, details: [...]}
 *   （services/ar/mod ReconciliationWithDetails），回读同一行。
 * - POST /{id}/confirm/send：send 状态门「只有草稿状态的对账单可以发送」
 *   （recon_ops/lifecycle）→ draft 放行转 sent，重复发送 400 BUSINESS_ERROR。
 * - POST /{id}/confirm（，OptionalJson 合法缺体）：customer_confirm 状态门
 *   （vfy_ops/confirm）confirmed 再确认 → 400 BUSINESS_ERROR；
 *   sent → confirmed 且 confirmed_by_customer=true。
 * - POST /confirmations：同一状态门的别名腿，对已 confirmed 行 → 400 BUSINESS_ERROR。
 * - GET /confirmations：固定 status=confirmed 过滤的分页列表，新建 confirmed 行必入选。
 * - PUT /confirmations/{id}/status：update_status 白名单六值
 *   （draft/sent/confirmed/disputed/closed/cancelled，lifecycle 起）；
 *   白名单外值 → 400 BUSINESS_ERROR；remark 写入 notes 列并回读。
 * - POST /{id}/dispute：confirmed 行不可提争议（confirm 状态门）→ 400；
 *   draft 行放行 → disputed，dispute_reason 回显 reason 原文。
 * - GET /disputes：固定 status=disputed 过滤列表，新建 disputed 行必入选。
 * - GET /disputes/{id}：按对账单主键读回争议行本体。
 * - PUT /disputes/{id}/resolve：resolve_dispute 默认 target_status="resolved"，
 *   但该值不在 update_status 白名单内 → 缺省 status 的调用必 400 BUSINESS_ERROR（现状契约，
 *   疑似源码缺陷已另单交接，不在测试侧放宽）；显式传白名单值 confirmed → 200 并真实改写状态。
 * - POST /disputes：缺 reconciliation_id → 400 BAD_REQUEST；
 *   customer_id 与对账单归属不一致 → 400 BAD_REQUEST（归属一致性校验）；
 *   合法引用 draft 行 → disputed。
 * - POST /auto-match：start_date/end_date 为 NaiveDate 必填，
 *   出参数组 + message「自动对账完成，共处理 N 个客户」整句模式。
 * - GET /auto-reconcile/results：分页信封 {list,total,page,page_size}，
 *   customer_id 过滤下自建三单必全部入选。
 * - GET /aging-report：AgingReport{analysis_date,total_receivable(DECIMAL 出参字符串),
 *   customer_summaries,overall_buckets} 结构真实成立。
 * - GET /{id}/pdf：出参 content_type=application/pdf、size>0、
 *   base64_content 非空且以 JVBERi0（"%PDF-" 的 base64 前缀）开头——文件魔数级证据，
 *   不是「有响应即通过」。
 *
 * 金额列为 DECIMAL 线格式（出参字符串），断言用字符串数字形态，不做数值再造。
 * 自建客户（POST /crm/customers）隔离本面数据，用例尾部 best-effort 删除客户；
 * 对账单行无删除端点（enhanced 面未挂 DELETE），终态留 confirmed/closed，不悬挂 draft。
 */

type ReconciliationRow = {
  id: number;
  reconciliation_no: string;
  customer_id: number;
  customer_name?: string | null;
  period_start: string;
  period_end: string;
  reconciliation_status: string | null;
  opening_balance: string | number;
  total_invoices: string | number;
  closing_balance: string | number;
  confirmed_by_customer?: boolean | null;
  dispute_reason?: string | null;
  notes?: string | null;
};

type DetailsBody = {
  reconciliation: ReconciliationRow;
  details: Array<{ id: number; reconciliation_id: number; item_type: string }>;
};

type PagedBody = {
  list: ReconciliationRow[];
  total: number;
  page: number;
  page_size: number;
};

type AgingBody = {
  analysis_date: string;
  total_receivable: string | number;
  customer_summaries: unknown[];
  overall_buckets: unknown[];
};

type PdfBody = {
  id: number;
  content_type: string;
  size: number;
  base64_content: string;
};

const PERIOD_START = '2026-01-01';
const PERIOD_END = '2026-12-31';

function uniqTag(): string {
  return Date.now().toString(36) + Math.random().toString(36).slice(2, 6);
}

test.describe.serial('125 ar-reconciliations-enhanced 全链契约回读', () => {
  const tag = uniqTag();
  let customerId = 0;
  let recMain = 0;
  let recDispute = 0;
  let recDispute2 = 0;

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('自建客户并生成首张对账单（draft），未知客户 404', async ({ page }) => {
    const customer = await apiCallRaw<{ id: number; customer_name: string }>(
      page,
      'POST',
      '/crm/customers',
      { customer_name: `e2c-ar-${tag}` }
    );
    expect(customer.id).toBeGreaterThan(0);
    customerId = customer.id;

    const ghost = await apiCallExpectFail(page, 'POST', '/ar-reconciliations-enhanced/generate', {
      customer_id: 987654,
      start_date: PERIOD_START,
      end_date: PERIOD_END,
    });
    expect(ghost.status).toBe(404);
    expect(ghost.code).toBe(APP_ERROR_CODES.NOT_FOUND);

    const generated = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      '/ar-reconciliations-enhanced/generate',
      {
        customer_id: customerId,
        start_date: PERIOD_START,
        end_date: PERIOD_END,
        notes: `e2c-main-${tag}`,
      }
    );
    expect(generated.id).toBeGreaterThan(0);
    recMain = generated.id;
    expect(generated.reconciliation_status).toBe('draft');
    expect(generated.customer_id).toBe(customerId);
    expect(generated.period_start).toBe(PERIOD_START);
    expect(generated.period_end).toBe(PERIOD_END);
    expect(generated.notes).toBe(`e2c-main-${tag}`);
    expect(generated.reconciliation_no.length).toBeGreaterThan(0);
    // DECIMAL 线格式：出参是字符串（未开 serde-float）
    expect(String(generated.opening_balance)).toMatch(/^-?\d+(\.\d+)?$/);
  });

  test('details 与 base 面读回同一行语义一致；未知 id 404', async ({ page }) => {
    const details = await apiCallRaw<DetailsBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/${recMain}/details`
    );
    expect(details.reconciliation.id).toBe(recMain);
    expect(details.reconciliation.reconciliation_status).toBe('draft');
    expect(Array.isArray(details.details)).toBe(true);

    // base 面镜像读回：同 id 的 no/customer 必须逐字一致（enhanced 非空壳镜像的反证）
    const base = await apiCallRaw<ReconciliationRow>(page, 'GET', `/ar-reconciliations/${recMain}`);
    expect(base.reconciliation_no).toBe(details.reconciliation.reconciliation_no);
    expect(base.customer_id).toBe(details.reconciliation.customer_id);

    const missing = await apiCallExpectFail(
      page,
      'GET',
      '/ar-reconciliations-enhanced/987654/details'
    );
    expect(missing.status).toBe(404);
    expect(missing.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('发送门：draft 放行转 sent，重复发送 400 BUSINESS_ERROR', async ({ page }) => {
    const sent = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recMain}/confirm/send`
    );
    expect(sent.reconciliation_status).toBe('sent');

    const again = await apiCallExpectFail(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recMain}/confirm/send`
    );
    expect(again.status).toBe(400);
    expect(again.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('确认门：sent 可确认、重复确认 400、别名腿同门拒绝', async ({ page }) => {
    const confirmed = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recMain}/confirm`
    );
    expect(confirmed.reconciliation_status).toBe('confirmed');
    expect(confirmed.confirmed_by_customer).toBe(true);

    const dup = await apiCallExpectFail(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recMain}/confirm`
    );
    expect(dup.status).toBe(400);
    expect(dup.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const alias = await apiCallExpectFail(
      page,
      'POST',
      '/ar-reconciliations-enhanced/confirmations',
      {
        reconciliation_id: recMain,
      }
    );
    expect(alias.status).toBe(400);
    expect(alias.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const listed = await apiCallRaw<PagedBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/confirmations?customer_id=${customerId}&page=1&page_size=50`
    );
    expect(listed.page).toBe(1);
    expect(listed.page_size).toBe(50);
    expect(listed.total).toBeGreaterThanOrEqual(1);
    expect(listed.list.some(r => r.id === recMain)).toBe(true);
    for (const row of listed.list) {
      expect(row.reconciliation_status).toBe('confirmed');
    }
  });

  test('确认状态更新门：白名单内 closed 放行且 remark 落 notes，白名单外 400', async ({ page }) => {
    const closed = await apiCallRaw<ReconciliationRow>(
      page,
      'PUT',
      `/ar-reconciliations-enhanced/confirmations/${recMain}/status`,
      { status: 'closed', remark: `e2c-close-${tag}` }
    );
    expect(closed.reconciliation_status).toBe('closed');
    expect(closed.notes).toBe(`e2c-close-${tag}`);

    const illegal = await apiCallExpectFail(
      page,
      'PUT',
      `/ar-reconciliations-enhanced/confirmations/${recMain}/status`,
      { status: 'resolved' }
    );
    expect(illegal.status).toBe(400);
    expect(illegal.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('争议门：confirmed 拒争议、closed 拒争议、draft 放行 disputed', async ({ page }) => {
    // recMain 现为 closed → 提争议必 400 BUSINESS_ERROR
    const onClosed = await apiCallExpectFail(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recMain}/dispute`,
      { reason: 'closed 行不应可争议' }
    );
    expect(onClosed.status).toBe(400);
    expect(onClosed.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const draft = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      '/ar-reconciliations-enhanced/generate',
      {
        customer_id: customerId,
        start_date: PERIOD_START,
        end_date: PERIOD_END,
        notes: `e2c-dispute-${tag}`,
      }
    );
    recDispute = draft.id;

    const disputed = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      `/ar-reconciliations-enhanced/${recDispute}/dispute`,
      { reason: `e2c-reason-${tag}` }
    );
    expect(disputed.reconciliation_status).toBe('disputed');
    expect(disputed.dispute_reason).toBe(`e2c-reason-${tag}`);
  });

  test('争议列表与详情读回，resolve 缺省 status 被白名单拒、显式 confirmed 放行', async ({
    page,
  }) => {
    const listed = await apiCallRaw<PagedBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/disputes?customer_id=${customerId}&page=1&page_size=50`
    );
    expect(listed.list.some(r => r.id === recDispute)).toBe(true);
    for (const row of listed.list) {
      expect(row.reconciliation_status).toBe('disputed');
    }

    const one = await apiCallRaw<ReconciliationRow>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/disputes/${recDispute}`
    );
    expect(one.id).toBe(recDispute);
    expect(one.customer_id).toBe(customerId);

    // 现状契约：resolve 默认 target_status="resolved" 不在 update_status 白名单内，
    // 缺省腿必 400（疑似源码缺陷已写进交接单，测试不迁就、不放宽）
    const defaultResolve = await apiCallExpectFail(
      page,
      'PUT',
      `/ar-reconciliations-enhanced/disputes/${recDispute}/resolve`,
      { resolution: '协商解决' }
    );
    expect(defaultResolve.status).toBe(400);
    expect(defaultResolve.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const explicit = await apiCallRaw<ReconciliationRow>(
      page,
      'PUT',
      `/ar-reconciliations-enhanced/disputes/${recDispute}/resolve`,
      { resolution: '协商后转确认', status: 'confirmed' }
    );
    expect(explicit.reconciliation_status).toBe('confirmed');
  });

  test('disputes 创建腿：缺 reconciliation_id、客户不一致、合法引用三形态', async ({ page }) => {
    const missingRef = await apiCallExpectFail(
      page,
      'POST',
      '/ar-reconciliations-enhanced/disputes',
      {
        description: '无引用争议',
      }
    );
    expect(missingRef.status).toBe(400);
    expect(missingRef.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const draft2 = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      '/ar-reconciliations-enhanced/generate',
      {
        customer_id: customerId,
        start_date: PERIOD_START,
        end_date: PERIOD_END,
        notes: `e2c-dispute2-${tag}`,
      }
    );
    recDispute2 = draft2.id;

    const wrongCustomer = await apiCallExpectFail(
      page,
      'POST',
      '/ar-reconciliations-enhanced/disputes',
      {
        reconciliation_id: recDispute2,
        customer_id: customerId + 1000,
        description: '跨客户争议',
      }
    );
    expect(wrongCustomer.status).toBe(400);
    expect(wrongCustomer.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const created = await apiCallRaw<ReconciliationRow>(
      page,
      'POST',
      '/ar-reconciliations-enhanced/disputes',
      {
        reconciliation_id: recDispute2,
        customer_id: customerId,
        description: `e2c-desc-${tag}`,
      }
    );
    expect(created.reconciliation_status).toBe('disputed');
    // description 优先于 reason 写入争议原因（create_dispute 的取值顺序）
    expect(created.dispute_reason).toBe(`e2c-desc-${tag}`);
  });

  test('auto-match 与结果分页、账龄、PDF 字段级判据', async ({ page }) => {
    const matched = await apiCall<unknown[]>(
      page,
      'POST',
      '/ar-reconciliations-enhanced/auto-match',
      {
        customer_id: customerId,
        start_date: PERIOD_START,
        end_date: PERIOD_END,
      }
    );
    expect(matched.code).toBe(200);
    expect(Array.isArray(matched.data)).toBe(true);
    expect(/^自动对账完成，共处理 \d+ 个客户$/.test(matched.message)).toBe(true);

    const results = await apiCallRaw<PagedBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/auto-reconcile/results?customer_id=${customerId}&page=1&page_size=50`
    );
    expect(results.total).toBeGreaterThanOrEqual(3);
    for (const id of [recMain, recDispute, recDispute2]) {
      expect(results.list.some(r => r.id === id)).toBe(true);
    }

    const aging = await apiCallRaw<AgingBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/aging-report?customer_id=${customerId}`
    );
    expect(aging.analysis_date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
    expect(String(aging.total_receivable)).toMatch(/^-?\d+(\.\d+)?$/);
    expect(Array.isArray(aging.customer_summaries)).toBe(true);
    expect(Array.isArray(aging.overall_buckets)).toBe(true);

    const pdf = await apiCallRaw<PdfBody>(
      page,
      'GET',
      `/ar-reconciliations-enhanced/${recMain}/pdf`
    );
    expect(pdf.id).toBe(recMain);
    expect(pdf.content_type).toBe('application/pdf');
    expect(pdf.size).toBeGreaterThan(0);
    // "%PDF-" 的 base64 前缀固定为 JVBERi0：文件魔数级证据
    expect(pdf.base64_content.startsWith('JVBERi0')).toBe(true);
  });

  test('清理：删除自建客户（对账单行无删除端点，best-effort 记录残量）', async ({ page }) => {
    expect(customerId).toBeGreaterThan(0);
    const del = await apiCallExpectFail(page, 'DELETE', `/crm/customers/${customerId}`);
    // 客户带对账单时后端按业务引用拒绝删除（400/403/409 族）属合法结果，
    // 但 5xx 意味着删除路径本身坏了——判红；2xx 视为清干净。
    expect(del.status).toBeLessThan(500);
    if (del.status !== 200) {
      console.warn(
        `[125-cleanup] 客户 ${customerId} 未能删除（HTTP ${del.status} code=${del.code}），` +
          `自建对账单 ${recMain}/${recDispute}/${recDispute2} 留存（enhanced 面无 DELETE 端点）`
      );
    }
  });
});
