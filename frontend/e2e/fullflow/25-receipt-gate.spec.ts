// 交易域全流程契约级 E2E — 25 收货入库质检门控（建收货→质检不合格→确认入库被拒并回读状态）
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按既有契约事实编写，待 CI/联调验证。**
//
// 门控契约真值源（wave6 后端落地）：
// - 门控唯一判定：backend/src/services/purchase_receipt_service.rs
//   ::ensure_receipt_inspection_allows_flow —— purchase_receipt.inspection_status
//   （大写三态词表 PENDING/PASSED/REJECTED，models/status/purchase_inventory.rs
//   purchase_receipt_inspection，DDL NOT NULL DEFAULT 'PENDING'）仅 PASSED 放行；
//   REJECTED/PENDING 均拒绝（每单必经质检裁定，无配置旁路、无"未开即放行"分支）。
// - 拒绝错误形态：400 + BUSINESS_ERROR + business_displayable 真实文案：
//   REJECTED = "质检不合格的收货单不能确认入库，请先处理不合格品"（逐字符）；
//   PENDING  = 含"质检尚未完成"与"请先完成质检并录入结论"。
// - 回写链（wave5 已落地）：采购质检 complete（inspection_result=pass/fail/partial，
//   backend/tests/contract_wave5_inspection_result_authority_test.rs 词表同源锁）→
//   同事务回写收货单 inspection_status（pass→PASSED、fail/partial→REJECTED）。
// - 门控时序：确认入口 services/purchase_receipt_ops/state.rs::confirm_receipt，
//   判定先于库存写入与 PO 进度推进（零漂移由
//   backend/tests/contract_wave6_receipt_gate_test.rs 钉住，本 e2e 做契约级回读复证）。
//
// 链路（全 API 驱动，契约级；UI 门控提示另见"前端配套"报告）：
// 1) 建 PO→提交→审批→建收货单（四维齐套明细，同 22 的 API 种子形态）；
// 2) 收货单未质检（PENDING）直接确认 → 400 BUSINESS_ERROR + PENDING 文案（回读仍 DRAFT）；
// 3) 质检建单+完成（fail）→ 回读收货单 inspection_status=REJECTED；
//    再确认 → 400 + REJECTED 逐字符文案；回读状态/确认时间零漂移；
// 4) 对照组：另一张收货单质检 pass → 确认成功 200 → 回读 receipt_status=COMPLETED；
// 5) 结算入口 POST /ap/invoices/auto-generate 同受门控（PENDING/REJECTED 拒且零应付行、
//    PASSED 真能生成）——该入口可绕过 confirm 直连，与 confirm 共用同一判定；
// 6) 正向镜像：质检合格 → 确认成功 → 应付单确由本收货单派生落库（source_id 回读）。
// afterEach 尽力清理（被拒收货单仍 DRAFT 可删；已确认单删除会失败，属既有 DRAFT-only 门，忽略）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  getCtx,
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

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/** 建 PO（提交+审批）并按四维口径挂一张 DRAFT 收货单，返回锚点 id */
async function seedReceipt(
  page: Page,
  tag: string
): Promise<{
  rcvId: number;
  poId: number;
  productId: number;
  warehouseId: number;
  supplierId: number;
  inspPayloadBase: { receipt_id: number; supplier_id: number };
}> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪');
  const productId = ctx.productIds[0];
  const prod = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
  if (!prod.code || !prod.name)
    throw new Error(`产品 ${productId} 缺 code/name（收货明细非 Option 必填）`);

  const ts = genCode(tag);
  const po = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/orders', {
    supplier_id: ctx.supplierId,
    order_date: todayStr(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-F25-PO-${ts}`,
    items: [{ material_id: productId, quantity_ordered: '200', unit_price: '10.00' }],
  });
  const poId = requireNum(po.id, '建 PO');
  CLEANUP.push({ path: `/purchase/orders/${poId}`, label: `purchase_order#${poId}` });
  await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
  await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

  const rcv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
    supplier_id: ctx.supplierId,
    order_id: poId,
    receipt_date: todayStr(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-F25-RCV-${ts}`,
    items: [
      {
        line_no: 1,
        material_id: productId,
        material_code: prod.code,
        material_name: prod.name,
        batch_no: `F25B${ts}`,
        color_code: `F25C${ts}`,
        lot_no: `F25L${ts}`,
        grade: '一等品',
        quantity: '200',
        quantity_alt: '50',
        unit_master: prod.unit ?? '米',
        unit_price: '10.00',
      },
    ],
  });
  const rcvId = requireNum(rcv.id, '建收货单');
  CLEANUP.push({ path: `/purchase/receipts/${rcvId}`, label: `purchase_receipt#${rcvId}` });
  return {
    rcvId,
    poId,
    productId,
    warehouseId: ctx.warehouseIds[0],
    supplierId: ctx.supplierId,
    inspPayloadBase: { receipt_id: rcvId, supplier_id: ctx.supplierId },
  };
}

/** API 建质检单（关联收货单）并完成（结论 token 走权威词表 pass/fail/partial） */
async function inspectAndComplete(
  page: Page,
  base: { receipt_id: number; supplier_id: number },
  result: 'pass' | 'fail' | 'partial'
): Promise<number> {
  const insp = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/inspections', {
    receipt_id: base.receipt_id,
    supplier_id: base.supplier_id,
    inspection_date: todayStr(),
    notes: `E2E-F25-INSPECT-${result}`,
  });
  const inspId = requireNum(insp.id, '建质检单');
  CLEANUP.push({ path: `/purchase/inspections/${inspId}`, label: `purchase_inspection#${inspId}` });
  const rejectQuantity = result === 'pass' ? 0 : 50;
  await apiCall(page, 'POST', `/purchase/inspections/${inspId}/complete`, {
    pass_quantity: result === 'pass' ? 200 : 150,
    reject_quantity: rejectQuantity,
    inspection_result: result,
  });
  return inspId;
}

test.describe('25 收货入库质检门控契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('25-01 未质检（PENDING）直接确认 → 400 BUSINESS_ERROR+可行动文案，状态零漂移', async ({
    page,
  }) => {
    const { rcvId, poId } = await seedReceipt(page, 'F25P');

    const fail = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${rcvId}/confirm`);
    expect(
      fail.status,
      `PENDING 确认应 400，实际=${fail.status} body=${JSON.stringify(fail)}`
    ).toBe(400);
    expect(failureCode(fail), 'PENDING 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      String(fail.message ?? ''),
      'PENDING 拒绝必须外显原因与行动路径（displayable 真实文案）'
    ).toMatch(/质检尚未完成.*请先完成质检并录入结论/);

    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rcvId}`
    );
    expect(after.receipt_status, '被拒后收货单必须仍为 DRAFT').toBe('DRAFT');
    expect(after.confirmed_at ?? null, '被拒后确认时间必须为 NULL（零漂移）').toBeNull();
    expect(after.inspection_status, '未质检事实保持 PENDING（门控不改数据）').toBe('PENDING');

    // 门控时序复证（后端有源码扫描锁，e2e 侧要真实回证）：判定必须先于
    // update_order_received_quantity / update_inventory_txn，因此被拒后
    // 采购订单明细的已收数量不得被推进。
    const poAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/orders/${poId}`
    );
    const poItems = Array.isArray(poAfter.items)
      ? (poAfter.items as Record<string, unknown>[])
      : [];
    const item0 = poItems.find(it => Number(it.received_quantity ?? 0) > 0);
    expect(
      item0,
      `被拒确认后订单 ${poId} 明细仍不应有 received_quantity>0，实际 items=${JSON.stringify(poItems)}`
    ).toBeUndefined();
  });

  test('25-02 质检 fail 回写 REJECTED → 确认被拒逐字符文案 + 零漂移；pass 对照确认可成功', async ({
    page,
  }) => {
    // 反向主链：建收货→质检不合格→确认入库被拒并回读状态
    const rejected = await seedReceipt(page, 'F25R');
    await inspectAndComplete(page, rejected.inspPayloadBase, 'fail');

    const rcv = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rejected.rcvId}`
    );
    expect(rcv.inspection_status, '质检 fail 必须同事务回写收货单 REJECTED（wave5 链复证）').toBe(
      'REJECTED'
    );

    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${rejected.rcvId}/confirm`
    );
    expect(
      fail.status,
      `REJECTED 确认应 400，实际=${fail.status} body=${JSON.stringify(fail)}`
    ).toBe(400);
    expect(failureCode(fail), 'REJECTED 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(fail.message, 'REJECTED 文案必须与裁定原文逐字符一致（不含单号）').toBe(
      '质检不合格的收货单不能确认入库，请先处理不合格品'
    );

    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${rejected.rcvId}`
    );
    expect(after.receipt_status, '被拒后不得推进状态').toBe('DRAFT');
    expect(after.confirmed_at ?? null, '被拒后不得写确认时间').toBeNull();
    expect(after.inspection_status, '被拒后检验状态不得被改写').toBe('REJECTED');

    // 正向对照：质检 pass（回写 PASSED）→ 确认成功 → COMPLETED（门控不是"一律拒绝"）
    const passed = await seedReceipt(page, 'F25A');
    await inspectAndComplete(page, passed.inspPayloadBase, 'pass');
    const mid = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${passed.rcvId}`
    );
    expect(mid.inspection_status, '质检 pass 必须回写 PASSED').toBe('PASSED');
    await apiCall(page, 'POST', `/purchase/receipts/${passed.rcvId}/confirm`);
    const done = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${passed.rcvId}`
    );
    expect(done.receipt_status, 'PASSED 收货单确认后必须 COMPLETED').toBe('COMPLETED');
    expect(done.confirmed_at, '确认时间必须真实写入').toBeTruthy();
  });

  test('25-03 partial 收货单确认同样被拒（partial→REJECTED 映射，不开"部分合格"旁路）', async ({
    page,
  }) => {
    const seed = await seedReceipt(page, 'F25X');
    await inspectAndComplete(page, seed.inspPayloadBase, 'partial');

    const rcv = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/receipts/${seed.rcvId}`
    );
    expect(rcv.inspection_status, 'partial 必须映射 REJECTED（wave5 裁定复证）').toBe('REJECTED');

    const fail = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${seed.rcvId}/confirm`);
    expect(fail.status, 'partial(REJECTED) 确认必须 400').toBe(400);
    expect(failureCode(fail), '机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(fail.message).toBe('质检不合格的收货单不能确认入库，请先处理不合格品');
  });

  test('25-04 结算入口 auto-generate 同受门控（PENDING/REJECTED 拒且零应付，PASSED 放行）', async ({
    page,
  }) => {
    // POST /ap/invoices/auto-generate 可被 HTTP 直连调用、不经 confirm，
    // 因此它是门控的第二入口（ap_invoice_ops/receipt.rs:113 复用同一判定）。
    // 此前该入口在全部 e2e 中零覆盖——门控只测了 confirm 半边等于没测结算半边。
    const pending = await seedReceipt(page, 'F25G');
    const failPending = await apiCallExpectFail(page, 'POST', '/ap/invoices/auto-generate', {
      receipt_id: pending.rcvId,
    });
    expect(
      failPending.status,
      `PENDING 收货单生成应付必须 400，实际=${failPending.status} body=${JSON.stringify(failPending)}`
    ).toBe(400);
    expect(failureCode(failPending), 'auto-generate 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(failPending.message ?? ''), 'PENDING 结算拒绝文案必须指向质检未完成').toMatch(
      /质检尚未完成/
    );

    const rejected = await seedReceipt(page, 'F25H');
    await inspectAndComplete(page, rejected.inspPayloadBase, 'fail');
    const failRejected = await apiCallExpectFail(page, 'POST', '/ap/invoices/auto-generate', {
      receipt_id: rejected.rcvId,
    });
    expect(failRejected.status, 'REJECTED 收货单生成应付必须 400').toBe(400);
    expect(
      String(failRejected.message ?? ''),
      'REJECTED 结算拒绝必须逐字符对应"不合格"分支文案'
    ).toBe('质检不合格的收货单不能生成应付结算，请先处理不合格品');

    // 零应付复证：两张被拒的单都不能留下任何 source_type=PURCHASE_RECEIPT 的应付行
    for (const seed of [pending, rejected]) {
      const list = await apiCallRaw<unknown>(
        page,
        'GET',
        `/ap/invoices?supplier_id=${seed.supplierId}&page=1&page_size=100`
      );
      const itemsRaw = (list as { items?: unknown }).items ?? list;
      const items = Array.isArray(itemsRaw) ? (itemsRaw as Record<string, unknown>[]) : [];
      const polluted = items.find(
        it => it.source_type === 'PURCHASE_RECEIPT' && Number(it.source_id) === seed.rcvId
      );
      expect(
        polluted,
        `被拒收货单 ${seed.rcvId} 不应生成应付行，实际=${JSON.stringify(polluted)}`
      ).toBeUndefined();
    }

    // 正向：PASSED 后同一入口必须真能生成（证明门控不是"一律拒绝"）
    const passed = await seedReceipt(page, 'F25I');
    await inspectAndComplete(page, passed.inspPayloadBase, 'pass');
    await apiCall(page, 'POST', '/ap/invoices/auto-generate', { receipt_id: passed.rcvId });
    const afterList = await apiCallRaw<unknown>(
      page,
      'GET',
      `/ap/invoices?supplier_id=${passed.supplierId}&page=1&page_size=100`
    );
    const afterRaw = (afterList as { items?: unknown }).items ?? afterList;
    const generated = (Array.isArray(afterRaw) ? (afterRaw as Record<string, unknown>[]) : []).find(
      it => it.source_type === 'PURCHASE_RECEIPT' && Number(it.source_id) === passed.rcvId
    );
    expect(
      generated,
      `PASSED 收货单 ${passed.rcvId} 应经 auto-generate 生成应付单，实际列表=${JSON.stringify(afterRaw)?.slice(0, 300)}`
    ).toBeTruthy();
    CLEANUP.push({
      path: `/ap/invoices/${requireNum(generated!.id, '自动生成的应付单 id')}`,
      label: 'ap_invoice(auto-generate)',
    });
  });

  test('25-05 确认入库正向链必须自动生成应付（结算门与入库门同口径的镜像复证）', async ({
    page,
  }) => {
    // state.rs::confirm_receipt 在确认事务内调 auto_generate_from_receipt；
    // 应付侧同一门控只有 PASSED 才放行，因此"质检合格→确认成功→应付落库"整链必须成立。
    const seed = await seedReceipt(page, 'F25J');
    await inspectAndComplete(page, seed.inspPayloadBase, 'pass');
    await apiCall(page, 'POST', `/purchase/receipts/${seed.rcvId}/confirm`);

    const list = await apiCallRaw<unknown>(
      page,
      'GET',
      `/ap/invoices?supplier_id=${seed.supplierId}&page=1&page_size=100`
    );
    const itemsRaw = (list as { items?: unknown }).items ?? list;
    const items = Array.isArray(itemsRaw) ? (itemsRaw as Record<string, unknown>[]) : [];
    const generated = items.find(
      it => it.source_type === 'PURCHASE_RECEIPT' && Number(it.source_id) === seed.rcvId
    );
    expect(
      generated,
      `确认入库后应存在由本收货单派生的应付单（source_id=${seed.rcvId}），实际=${JSON.stringify(items).slice(0, 300)}`
    ).toBeTruthy();
    // 与 fullflow/20-01 的既有契约一致：自动应付初始为 DRAFT（不放宽成"任一状态"）
    expect(
      String(generated!.invoice_status),
      `自动生成的应付单初始应为 DRAFT，实际=${generated!.invoice_status}`
    ).toBe('DRAFT');
    CLEANUP.push({
      path: `/ap/invoices/${requireNum(generated!.id, '应付单 id')}`,
      label: 'ap_invoice(confirm 派生)',
    });
  });
});
