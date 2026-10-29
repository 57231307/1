/**
 * 报价到销售完成段的跨岗位真实账号接力链
 *
 * 功能：按后端既有建审分离授权，用岗位真账号接力走完"建报价、提交、审批通过、
 * 审批驳回、驳回后修订、已审批转销售订单"整段，并在每一步回读库里真值；同时把
 * "同一身份不得自批"与"审批岗不得建单"两条职责分离在运行时钉住。
 * 调用方：Playwright 分片（该组 targets 覆盖 e2e/quotations/ 整目录，新文件自动纳入，
 *   不需改流水线矩阵；分片覆盖自检要求每条用例都被某组真选中）。
 * 入参：role-credentials.json 或 env 提供的 salesperson 与 sales_manager 岗位账号；
 *      实体夹具 getCtx 的 customerId 与 quotationProductId 与 quotationProductUnit。
 * 传给谁：后端 /api/v1/erp/quotations 一族端点与 /sales-orders 读口，经真实登录会话发起。
 * 存什么：本轮自建的报价单与其明细与审批结果，以及转单生成的销售订单草稿行。
 * 存哪里：sales_quotations 与 sales_quotation_items 与 sales_orders 与 sales_order_items，
 *        全部带本轮唯一 notes 标记，单号由后端生成。
 *
 * 授权依据（不虚构、不越界，逐键对齐后端同名分组）：
 *   建单岗  后端 init_service_ops/permission.rs 的 sales_rep 分组持 quotations 读与建，
 *            不持任何 approve 或 reject 键；CI 侧该岗别名为 salesperson，其补授清单见
 *            e2e global-setup 的同名条目。
 *   审批岗  后端同文件 sales_manager 分组持 quotations 读与 approve 与 reject，
 *            其自述已把 create 显式移除，因此本文件断该岗建单必被拒。
 * 状态词表依据：models/status/sales.rs 的 quotation 分组（小写 draft 与 pending_approval
 *   与 approved 与 rejected 与 cancelled 与 converted 与 expired）。
 * 修订与转单门依据：services/quotation_ops/update.rs 允许草稿态与被驳回态改写；
 *   services/quotation_convert_service.rs 仅已审批可转单；提交与转单都先行级归属门
 *   handlers/quotation_handler.rs 的 get_by_id_scoped 判定。
 * 理由必填依据：handlers/quotation_handler.rs 的 ApproveQuotationRequest 与 RejectRequest
 *   在服务端收口为非空，空理由走 validation 通道。
 *
 * 隔离与清理口径：本轮单据一律带唯一 notes 标记，断言只按本轮自建 id 精确回读，
 *   不依赖库内既有数据条数。清理只用草稿态的已注册删除腿，并在清理后回读确认行已消失；
 *   已进入待审与已审批与已转换态的行留库，靠唯一标记可点名，不假装已清。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  loginAsRole,
  APP_ERROR_CODES,
} from '../flow/helpers';

const STATUS = {
  DRAFT: 'draft',
  PENDING_APPROVAL: 'pending_approval',
  APPROVED: 'approved',
  REJECTED: 'rejected',
  CONVERTED: 'converted',
} as const;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0) {
    throw new Error(`${label}：期望正整数，实际 raw=${JSON.stringify(v)}`);
  }
  return n;
}

function expectKeyValue(
  obj: Record<string, unknown>,
  key: string,
  expected: unknown,
  label: string
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(
    obj[key],
    `${label}：键 ${key} 期望=${JSON.stringify(expected)} 实际=${JSON.stringify(obj[key])}`
  ).toEqual(expected);
}

/**
 * 建单岗自建一份值可辨识的草稿报价单（数量 40、单价 12.5、不含税）。
 * 入参 page 需已切到建单岗会话；返回后端生成的 id 与单号与本轮唯一标记。
 */
async function seedQuotationAsSalesperson(
  page: Page
): Promise<{ id: number; no: string; marker: string }> {
  const ctx = getCtx();
  if (!ctx.customerId) {
    throw new Error('前置缺失：夹具 customerId 未就绪');
  }
  if (!ctx.quotationProductId) {
    throw new Error('前置缺失：夹具 quotationProductId 未就绪');
  }
  const marker = genCode('XR1');
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/quotations', {
    customer_id: ctx.customerId,
    quotation_date: new Date().toISOString().slice(0, 10),
    valid_until: new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10),
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
    notes: `E2E-XROLE-${marker}`,
  });
  const id = requireNum(created.id, `建报价单 ${marker}`);
  const no = String(created.quotation_no ?? '');
  if (no === '') {
    throw new Error(
      `建报价单 ${marker}：后端未回出 quotation_no，实际键=${Object.keys(created).join(',')}`
    );
  }
  return { id, no, marker };
}

async function readQuotation(page: Page, id: number): Promise<Record<string, unknown>> {
  return await apiCallRaw<Record<string, unknown>>(page, 'GET', `/quotations/${id}`);
}

test.describe('报价到销售完成 跨岗位真账号接力链', () => {
  test.beforeEach(async ({ page }) => {
    await ensureTestEntities(page);
  });

  test('XR1-01 建单岗提交后自批必被拒，且被拒不改状态', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);

    await apiCall(page, 'POST', `/quotations/${q.id}/submit`);
    const submitted = await readQuotation(page, q.id);
    expectKeyValue(submitted, 'status', STATUS.PENDING_APPROVAL, `${q.marker} 提交后状态`);

    // 建审分离：建单岗持 quotations 读与建，后端不授 approve 键，自批必须 403
    const denied = await apiCallExpectFail(page, 'POST', `/quotations/${q.id}/approve`, {
      approval_reason: '建单岗自批不应通过',
    });
    expect(denied.status, `建单岗自批应 HTTP 403，实际=${denied.status}`).toBe(403);
    expect(failureCode(denied), `建单岗自批机器码应为 FORBIDDEN，实际=${failureCode(denied)}`).toBe(
      APP_ERROR_CODES.FORBIDDEN
    );

    const afterDenied = await readQuotation(page, q.id);
    expectKeyValue(
      afterDenied,
      'status',
      STATUS.PENDING_APPROVAL,
      `${q.marker} 越权被拒后状态不得漂移`
    );
  });

  test('XR1-02 审批岗不得建单，只能读与审', async ({ page }) => {
    await loginAsRole(page, 'sales_manager');
    const ctx = getCtx();
    const denied = await apiCallExpectFail(page, 'POST', '/quotations', {
      customer_id: ctx.customerId,
      quotation_date: new Date().toISOString().slice(0, 10),
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
          quantity: '1',
          unit_price: '1',
          unit_price_with_tax: '1',
        },
      ],
      notes: `E2E-XROLE-${genCode('XR1B')}`,
    });
    expect(denied.status, `审批岗建单应 HTTP 403，实际=${denied.status}`).toBe(403);
    expect(failureCode(denied), `审批岗建单机器码应为 FORBIDDEN，实际=${failureCode(denied)}`).toBe(
      APP_ERROR_CODES.FORBIDDEN
    );
  });

  test('XR1-03 合理报价由审批岗通过并落库理由，转单回读订单草稿', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);
    await apiCall(page, 'POST', `/quotations/${q.id}/submit`);

    // 同一 page 切身份：loginAsRole 强制清旧会话，等价两个真人先后操作
    await loginAsRole(page, 'sales_manager');
    const reason = '价格与交期合理 同意';
    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/quotations/${q.id}/approve`,
      {
        approval_reason: reason,
      }
    );
    expectKeyValue(approved, 'status', STATUS.APPROVED, `${q.marker} 审批通过后状态`);
    expectKeyValue(approved, 'approval_reason', reason, `${q.marker} 审批理由须逐字落库回读`);

    await loginAsRole(page, 'salesperson');
    const order = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/quotations/${q.id}/convert`
    );
    const orderId = requireNum(order.id, `${q.marker} 转单应生成销售订单`);
    const orderNo = String(order.order_no ?? '');
    if (!orderNo.startsWith('SO')) {
      throw new Error(`${q.marker} 转单回读订单号形态异常，实际=${orderNo}`);
    }
    const asDraft = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/sales-orders/${orderId}`
    );
    expectKeyValue(asDraft, 'order_no', orderNo, `${q.marker} 订单详情单号应与转单出参一致`);
    expectKeyValue(asDraft, 'status', 'draft', `${q.marker} 转单产物应为订单草稿`);

    const finalQuote = await readQuotation(page, q.id);
    expectKeyValue(finalQuote, 'status', STATUS.CONVERTED, `${q.marker} 转单后报价应进已转换终态`);
  });

  test('XR1-04 不合理报价驳回需理由，空理由被拒且状态不漂移', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);
    await apiCall(page, 'POST', `/quotations/${q.id}/submit`);

    await loginAsRole(page, 'sales_manager');
    const empty = await apiCallExpectFail(page, 'POST', `/quotations/${q.id}/reject`, {
      reason: '   ',
    });
    expect(empty.status, `空白驳回理由应 HTTP 400，实际=${empty.status}`).toBe(400);
    expect(
      failureCode(empty),
      `空白驳回理由机器码应为 VALIDATION_ERROR，实际=${failureCode(empty)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const stillPending = await readQuotation(page, q.id);
    expectKeyValue(
      stillPending,
      'status',
      STATUS.PENDING_APPROVAL,
      `${q.marker} 空理由被拒后不得改状态`
    );

    const rejectReason = '单价高于当期色卡价 请重报';
    const rejected = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/quotations/${q.id}/reject`,
      {
        reason: rejectReason,
      }
    );
    expectKeyValue(rejected, 'status', STATUS.REJECTED, `${q.marker} 驳回后状态`);
    expectKeyValue(
      rejected,
      'rejection_reason',
      rejectReason,
      `${q.marker} 驳回理由须逐字落库回读`
    );
  });

  test('XR1-05 被驳回报价由建单岗修订后新价落库，未再审批仍不可转单', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);
    await apiCall(page, 'POST', `/quotations/${q.id}/submit`);

    await loginAsRole(page, 'sales_manager');
    await apiCall(page, 'POST', `/quotations/${q.id}/reject`, { reason: '毛利不达标 驳回重报' });

    await loginAsRole(page, 'salesperson');
    const detail = await readQuotation(page, q.id);
    const items = Array.isArray(detail.items) ? (detail.items as Record<string, unknown>[]) : [];
    if (items.length === 0) {
      throw new Error(`${q.marker} 详情未回出明细行，实际键=${Object.keys(detail).join(',')}`);
    }
    const itemId = requireNum(items[0].id, `${q.marker} 明细行主键`);
    await apiCall(page, 'PUT', `/quotations/${q.id}`, {
      items: [{ id: itemId, quantity: '40', unit_price: '9.8', unit_price_with_tax: '9.8' }],
    });
    const revised = await readQuotation(page, q.id);
    expectKeyValue(
      revised,
      'status',
      STATUS.REJECTED,
      `${q.marker} 被驳回态修订不自动改变审批结论`
    );
    const revisedItems = Array.isArray(revised.items)
      ? (revised.items as Record<string, unknown>[])
      : [];
    expect(
      Number(revisedItems[0]?.unit_price),
      `${q.marker} 修订后单价应落库为 9.8，实际=${JSON.stringify(revisedItems[0]?.unit_price)}`
    ).toBe(9.8);

    // 未再获审批的报价不得转单：转单门只认已审批态
    const denied = await apiCallExpectFail(page, 'POST', `/quotations/${q.id}/convert`);
    expect(denied.status, `被驳回报价转单应被拒，实际=${denied.status}`).toBeLessThan(500);
    expect(
      failureCode(denied),
      `转单被拒机器码应为 BUSINESS_ERROR，实际=${failureCode(denied)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const unchanged = await readQuotation(page, q.id);
    expectKeyValue(unchanged, 'status', STATUS.REJECTED, `${q.marker} 转单被拒后报价状态不得漂移`);
  });

  test('XR1-06 草稿报价由建单岗走已注册删除真腿清理并回读消失', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);

    await apiCall(page, 'DELETE', `/quotations/${q.id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/quotations/${q.id}`);
    expect(gone.status, `删除后详情应 404，实际=${gone.status}`).toBe(404);
  });

  test('XR1-07 审批岗可读建单岗待审报价（跨岗可见），但不可替其提交', async ({ page }) => {
    await loginAsRole(page, 'salesperson');
    const q = await seedQuotationAsSalesperson(page);

    // 草稿尚未提交：审批岗能读到（持 quotations 读），但提交属建单岗推进自身草稿的动作
    await loginAsRole(page, 'sales_manager');
    const seen = await readQuotation(page, q.id);
    expectKeyValue(seen, 'quotation_no', q.no, `${q.marker} 审批岗读到的单号应与建单岗一致`);
    const denied = await apiCallExpectFail(page, 'POST', `/quotations/${q.id}/submit`);
    expect(denied.status, `审批岗替建单岗提交应被拒，实际=${denied.status}`).toBeLessThan(500);
    const stillDraft = await readQuotation(page, q.id);
    expectKeyValue(stillDraft, 'status', STATUS.DRAFT, `${q.marker} 被拒提交后仍应为草稿`);

    // 清理走已注册删除真腿，且以归属岗身份发起，末尾回读确认行已消失
    await loginAsRole(page, 'salesperson');
    await apiCall(page, 'DELETE', `/quotations/${q.id}`);
    const gone = await apiCallExpectFail(page, 'GET', `/quotations/${q.id}`);
    expect(gone.status, `${q.marker} 清理后详情应 404，实际=${gone.status}`).toBe(404);
  });
});
