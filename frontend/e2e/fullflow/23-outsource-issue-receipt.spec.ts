// 交易域全流程契约级 E2E — 23 委外发料→收回→结算（契约级 API 链）
//
// 定位：委外链每步回读后端权威状态（词表逐字符），并钉两条契约要点：
//   A. 明细读取信封键名与后端出参**同源**：orders/receipts/vouchers 列表 = PaginatedResponse
//      {items,total,page,page_size}（utils/response.rs:34，handler 返回类型直书）；
//      而 GET /production/outsourcing-orders/items/by-order/{id} 的 data 是**裸数组**
//      （handlers/outsourcing_handler.rs:234-240 ApiResponse<Vec<Model>>）——前端若按
//      items 键读明细必然恒空，这是"保存后明细显示不出来"的契约级成因之一。
//   B. 收回后库存/成本联动：confirm 收回单 ⇒ 订单推进 received 词值、损耗分类/率、
//      total_cost/unit_cost 计算落列、入库凭证生成、匹记录与质检记录联动（inspection_id 回写）。
// 取数口径：本例自建唯一单——委外域 order_no/receipt_no 由**客户端提供**
//   （services/outsourcing_ops/types.rs:30/:185 非 Option），故用 genCode 唯一码直接做锚点，
//   经 by-no 端点精确回查；afterEach 尽力清理（非 draft 删除被拒仅告警，属预期）。
//
// 契约真值源：
//   订单状态词表（小写）  backend/src/models/status/wage_energy_chemical_business.rs:261-277
//     draft→issued→processing→received→settled→closed（settle 门 order.rs:489-495 仅 received；
//     issue 门 order.rs:371-376 仅 draft；record_processing 门 :457-462 仅 issued）
//   收回单词表（小写）    同上 :287-295（draft/confirmed/cancelled；confirm 门 receipt.rs:319-324）
//   收回质检结论词表      同上 :328 起 outsourcing_receipt_quality_status：
//     pending/qualified/concession/unqualified（validate_receipt_quality_status receipt.rs:61-70
//     入口拒越界写法——历史上 passed/不合格 混写点）
//   损耗类型词表          同上 :279-285（normal/abnormal）
//   凭证类型词表          同上 :342-348（issue/fee/receipt/loss 小写）
//   收回前置              services/outsourcing_ops/order.rs:42-62 validate_receipt_eligibility
//     （仅 issued/processing 可收回；收回>发出 直接拒）
//   确认落账公式          services/outsourcing_ops/receipt.rs:336-477 + outsourcing_service.rs:51-115
//     loss=issue-return；rate=loss/issue；classify_loss(actual<=standard→normal)；
//     abnormal=超额损耗×单位材料成本；total_cost=material+fee+freight+abnormal；unit_cost=total/return
//   发料匹号门            services/piece_domain_service.rs:152-195 validate_pieces_for_issue
//     （每条明细必须带真实存在且 AVAILABLE 的生产匹号——无明细则整单放行，见 23-02 负例）
//   更新三态拒绝 NOT NULL 显式 null 的入口：order.rs:276-289 + types.rs:53-94
// 诚实标注/回归钉（本波源码修复后应为绿）：
//   settle 语义要求订单携带 processing_fee/freight_fee/tax_amount（order.rs:484 注释：
//   "需在订单更新时填入"），Create/Update DTO（services/outsourcing_ops/types.rs）已补
//   这三个真实键（NOT NULL 列 v15/mod.rs:3247-3249，Update 显式 null 拒清）⇒ 费用经
//   API 可录入回读，FEE 凭证金额取真实值。23-03 由缺陷钉转为回归钉，再红即回归。
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
import { pickListArray } from '../flow/ui-helpers';

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
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：响应缺少后端真实键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  expect(
    obj[key],
    `${label}：键 "${key}" 期望=${JSON.stringify(expected)} 实际=${JSON.stringify(obj[key])}`
  ).toEqual(expected);
}

function expectDecimal(
  obj: Record<string, unknown>,
  key: string,
  expected: number,
  label: string,
  tol = 0.005
) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺数值键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n))
    throw new Error(`${label}：${key} 不可解析，raw=${JSON.stringify(obj[key])}`);
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(tol);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * 自建唯一委外订单（order_type=dyeing，发料 100kg、材料成本 1000、标准损耗率 10%），
 * 全键回读（缺陷①）。初始 draft：total_cost=material_cost、unit_cost=0、return_quantity=0
 * （services/outsourcing_ops/order.rs:185-220 build+create）。
 */
async function seedOutsourcingOrder(page: Page, tag: string): Promise<Record<string, unknown>> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const orderNo = `E23-${tag}-${genCode('OS')}`;
  const created = await apiCallRaw<Record<string, unknown>>(
    page,
    'POST',
    '/production/outsourcing-orders',
    {
      order_no: orderNo,
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      color_no: `E23C${orderNo.slice(-6)}`,
      dye_lot_no: `E23L${orderNo.slice(-6)}`,
      issue_date: todayStr(),
      issue_quantity: '100',
      issue_unit: 'kg',
      material_cost: '1000',
      standard_loss_rate: '0.10',
      remarks: `E2E-F23-${tag}`,
    }
  );
  const id = requireNum(created.id, `建委外订单 ${orderNo}`);
  CLEANUP.push({ path: `/production/outsourcing-orders/${id}`, label: `outsourcing_order#${id}` });

  // by-no 精确锚定回读（handlers/outsourcing_handler.rs:148-154）
  const byNo = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/production/outsourcing-orders/by-no/${orderNo}`
  );
  expectKeyValue(byNo, 'id', id, 'by-no 回读');
  expectKeyValue(byNo, 'order_no', orderNo, 'by-no 回读单号');
  expectKeyValue(
    byNo,
    'status',
    'draft',
    '初始词值（wage_energy_chemical_business.rs:264 小写 draft）'
  );
  expectDecimal(byNo, 'issue_quantity', 100, '发料数量回读');
  expectDecimal(byNo, 'material_cost', 1000, '材料成本回读');
  expectDecimal(byNo, 'total_cost', 1000, '建单时 total_cost=material_cost（order.rs:207）');
  expectDecimal(byNo, 'return_quantity', 0, '建单 return_quantity 归零起步（order.rs:197）');
  expectDecimal(byNo, 'unit_cost', 0, '建单 unit_cost=0（order.rs:208）');
  expectKeyValue(byNo, 'voucher_no_issue', null, '未发料无发料凭证号');
  expectDecimal(byNo, 'standard_loss_rate', 0.1, '标准损耗率回读');
  return byNo;
}

test.describe('23 委外发料→收回→结算契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('23-01 主链：建单→发料(issued+OVIS凭证)→收回→确认(received 词值+成本/损耗/凭证/质检联动)→结算→关闭；信封键同源', async ({
    page,
  }) => {
    const order = await seedOutsourcingOrder(page, 'MAIN');
    const orderId = requireNum(order.id, '委外订单');

    // 明细端点信封：data 为**裸数组**（outsourcing_handler.rs:234-240），前端读取键必须同源
    const itemsRaw = await apiCallRaw<unknown>(
      page,
      'GET',
      `/production/outsourcing-orders/items/by-order/${orderId}`
    );
    expect(
      Array.isArray(itemsRaw),
      `items/by-order 出参应为裸数组（后端 ApiResponse<Vec>），实际键=${
        itemsRaw && typeof itemsRaw === 'object' ? Object.keys(itemsRaw).join(',') : typeof itemsRaw
      }`
    ).toBe(true);

    // 列表信封 = PaginatedResponse 四键（utils/response.rs:34）
    const list = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/production/outsourcing-orders?page=1&page_size=5'
    );
    for (const k of ['items', 'total', 'page', 'page_size']) {
      expect(
        Object.prototype.hasOwnProperty.call(list, k),
        `订单列表应含 PaginatedResponse 键 "${k}"，实际键=${Object.keys(list).join(',')}`
      ).toBe(true);
    }
    pickListArray<Record<string, unknown>>(list, 'items', '订单列表');

    // ── 发料（无明细⇒匹号门整单放行，见 23-02 的另一面）──
    const issued = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/issue`
    );
    expectKeyValue(issued, 'status', 'issued', '发料响应词值');
    expect(
      typeof issued.voucher_no_issue === 'string' &&
        (issued.voucher_no_issue as string).startsWith('OVIS'),
      `发料应回写 OVIS 前缀凭证号（order.rs:391-396 generate_no_with_txn("OVIS")），实际=${JSON.stringify(issued.voucher_no_issue)}`
    ).toBe(true);
    const issuedReread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(issuedReread, 'status', 'issued', '发料回查词值');

    // 发料凭证：voucher_type='issue'、金额=材料成本、未过账
    const vouchers1 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '发料后凭证列表'
    );
    const issueVch = vouchers1.find(v => v.voucher_type === 'issue');
    expect(
      issueVch,
      `应存在 issue 凭证，实际=${JSON.stringify(vouchers1.map(v => v.voucher_type))}`
    ).toBeTruthy();
    expectDecimal(issueVch!, 'amount', 1000, '发料凭证金额=material_cost（order.rs:408-410）');
    expectKeyValue(issueVch!, 'is_posted', false, '凭证初始未过账');

    // ── 收回单（draft；quality_status 用词表合法值 qualified）──
    const ctx = getCtx();
    const receiptNo = `E23-R${genCode('RC')}`;
    const rcpt = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: receiptNo,
        outsourcing_order_id: orderId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        color_no: order.color_no,
        dye_lot_no: order.dye_lot_no,
        batch_no: `E23B${receiptNo.slice(-6)}`,
        warehouse_id: ctx.warehouseIds[0],
        return_quantity: '95',
        quality_status: 'qualified',
        grade: '一等品',
        remarks: `E2E-F23-RCV-${receiptNo}`,
      }
    );
    const rcptId = requireNum(rcpt.id, '建收回单');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcptId}`,
      label: `outsourcing_receipt#${rcptId}`,
    });
    expectKeyValue(rcpt, 'status', 'draft', '收回单初始词值（receipt.rs:192）');
    expectDecimal(rcpt, 'unit_cost', 0, '建单 unit_cost=0（receipt.rs:180）');
    expectKeyValue(rcpt, 'quality_status', 'qualified', '质检结论按词表原样回读');

    // ── 确认收回 ⇒ received 词值 + 成本/损耗/凭证/质检四联动 ──
    await apiCall(page, 'POST', `/production/outsourcing-receipts/${rcptId}/confirm`);
    const rcptAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/by-no/${receiptNo}`
    );
    expectKeyValue(
      rcptAfter,
      'status',
      'confirmed',
      '收回单确认词值（finance 域外，wage_energy_chemical_business.rs:291-295）'
    );
    expectDecimal(rcptAfter, 'loss_quantity', 5, '损耗=100-95（receipt.rs:343 calc）');
    expectDecimal(rcptAfter, 'loss_rate', 0.05, '损耗率=5/100');
    expectKeyValue(
      rcptAfter,
      'loss_type',
      'normal',
      '5%<=标准10% ⇒ normal（classify_loss outsourcing_service.rs:91-96）'
    );
    expectDecimal(rcptAfter, 'abnormal_loss_amount', 0, '无超额损耗');
    expectDecimal(
      rcptAfter,
      'total_cost',
      1000,
      'total_cost=material+0+0+0（compute_total_cost :59-67）'
    );
    expectDecimal(
      rcptAfter,
      'unit_cost',
      1000 / 95,
      'unit_cost=total/return=1000/95（compute_unit_cost :69-76）',
      0.01
    );
    expect(
      requireNum(
        rcptAfter.inspection_id,
        '收回单回写质检记录 id（receipt.rs:484-500 trigger_quality_inspection）'
      ),
      'confirm 应联动生成质检记录并回写 inspection_id'
    ).toBeGreaterThan(0);

    const orderAfter = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    expectKeyValue(
      orderAfter,
      'status',
      'received',
      '收回后订单词值 received（词表 :270，"收回"的权威写法）'
    );
    expectDecimal(orderAfter, 'return_quantity', 95, '订单回写收回量');
    expectDecimal(orderAfter, 'loss_quantity', 5, '订单回写损耗量');
    expectDecimal(orderAfter, 'unit_cost', 1000 / 95, '订单成本联动（receipt.rs:470-477）', 0.01);
    expect(
      typeof orderAfter.voucher_no_receipt === 'string' &&
        (orderAfter.voucher_no_receipt as string).startsWith('OVRC'),
      `入库凭证号回写（receipt.rs:358-361 generate_no_with_txn("OVRC")），实际=${JSON.stringify(orderAfter.voucher_no_receipt)}`
    ).toBe(true);

    const vouchers2 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '确认后凭证列表'
    );
    const receiptVch = vouchers2.find(v => v.voucher_type === 'receipt');
    expect(receiptVch, '应存在 receipt 入库凭证').toBeTruthy();
    expectDecimal(receiptVch!, 'amount', 1000, '入库凭证金额=total_cost（receipt.rs:374）');
    expect(
      !vouchers2.some(v => v.voucher_type === 'loss'),
      'normal 损耗不应生成 loss 凭证（receipt.rs:426 条件门）'
    ).toBe(true);

    // 凭证过账端点回读（POST /production/outsourcing-vouchers/{id}/post）
    await apiCall(
      page,
      'POST',
      `/production/outsourcing-vouchers/${requireNum(receiptVch!.id, '入库凭证 id')}/post`
    );
    const vouchers3 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '过账后凭证列表'
    );
    const postedRow = vouchers3.find(v => Number(v.id) === Number(receiptVch!.id));
    expect(postedRow, '过账后凭证列表应仍含该 receipt 凭证').toBeTruthy();
    expectKeyValue(postedRow!, 'is_posted', true, 'post 后过账标记回读');

    // ── 结算：received→settled；FEE 凭证金额=processing_fee+freight_fee ──
    const settled = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/settle`
    );
    expectKeyValue(settled, 'status', 'settled', '结算词值（门 order.rs:489-495）');
    expect(
      typeof settled.voucher_no_fee === 'string' &&
        (settled.voucher_no_fee as string).startsWith('OVFE'),
      `加工费凭证号回写（order.rs:506-515），实际=${JSON.stringify(settled.voucher_no_fee)}`
    ).toBe(true);
    const vouchers4 = pickListArray<Record<string, unknown>>(
      await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/production/outsourcing-vouchers?outsourcing_order_id=${orderId}&page=1&page_size=20`
      ),
      'items',
      '结算后凭证列表'
    );
    const feeVch = vouchers4.find(v => v.voucher_type === 'fee');
    expect(feeVch, '应存在 fee 结算凭证（order.rs:517-535）').toBeTruthy();
    expectDecimal(
      feeVch!,
      'amount',
      0,
      'fee 凭证=processing_fee+freight_fee；本单建单未带费用键⇒0 起步（录入回读见 23-03）'
    );

    // ── 关闭：settled→closed ──
    const closed = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      `/production/outsourcing-orders/${orderId}/close`
    );
    expectKeyValue(closed, 'status', 'closed', '关闭词值（门 order.rs:581-587）');
  });

  test('23-02 门控与词表负例：重复发料拒、非 issued 收回拒、超量收回拒、越界质检词入口拒、明细带假匹号发料拒，全部锁机器码且无痕', async ({
    page,
  }) => {
    const ctx = getCtx();

    // A) 建单后未发料即建收回并 confirm ⇒ 收回前置门拒（order.rs:42-52 仅 issued/processing）
    const early = await seedOutsourcingOrder(page, 'EARLY');
    const earlyId = requireNum(early.id, 'EARLY 订单');
    const rcv0 = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: `E23-R0${genCode('E')}`,
        outsourcing_order_id: earlyId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '10',
        quality_status: 'qualified',
      }
    );
    const rcv0Id = requireNum(rcv0.id, '建收回单(early)');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${rcv0Id}`,
      label: `receipt(early)#${rcv0Id}`,
    });
    const f0 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-receipts/${rcv0Id}/confirm`
    );
    expect(
      f0.status,
      `draft 订单收回 confirm 应 400，实际=${f0.status} body=${JSON.stringify(f0)}`
    ).toBe(400);
    expect(failureCode(f0), '收回前置机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const earlyStill = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${earlyId}`
    );
    expectKeyValue(earlyStill, 'status', 'draft', '被拒后订单仍 draft 无痕');
    const rcv0Still = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-receipts/${rcv0Id}`
    );
    expectKeyValue(rcv0Still, 'status', 'draft', '被拒后收回单仍 draft 无痕');

    // B) 质检结论越界词 'passed'（别域写法）⇒ 入口拒（receipt.rs:61-70/:153-154）
    const fQ = await apiCallExpectFail(page, 'POST', '/production/outsourcing-receipts', {
      receipt_no: `E23-RQ${genCode('Q')}`,
      outsourcing_order_id: earlyId,
      receipt_date: todayStr(),
      product_id: ctx.productIds[0],
      return_quantity: '10',
      quality_status: 'passed',
    });
    expect(fQ.status, `越界质检词建单应 400，实际=${fQ.status}`).toBe(400);
    expect(failureCode(fQ), '越界质检词机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // C) 重复发料：draft 门（order.rs:371-376）
    const issued = await seedOutsourcingOrder(page, 'DUP');
    const issuedId = requireNum(issued.id, 'DUP 订单');
    await apiCall(page, 'POST', `/production/outsourcing-orders/${issuedId}/issue`);
    const f1 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${issuedId}/issue`
    );
    expect(f1.status, '重复发料应 400').toBe(400);
    expect(failureCode(f1), '重复发料机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 未结算即关闭 → 拒（close 门仅 settled，order.rs:581-587），且状态不漂移
    const f2 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${issuedId}/close`
    );
    expect(f2.status, 'issued 直接 close 应 400').toBe(400);
    expect(failureCode(f2), '越级 close 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noDrift = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${issuedId}`
    );
    expectKeyValue(noDrift, 'status', 'issued', '越级动作被拒后仍 issued');

    // D) 超量收回：return>issue 时 confirm 拒（order.rs:56-60）
    const rcvBig = await apiCallRaw<Record<string, unknown>>(
      page,
      'POST',
      '/production/outsourcing-receipts',
      {
        receipt_no: `E23-RB${genCode('B')}`,
        outsourcing_order_id: issuedId,
        receipt_date: todayStr(),
        product_id: ctx.productIds[0],
        return_quantity: '101',
        quality_status: 'qualified',
      }
    );
    const bigId = requireNum(rcvBig.id, '建超量收回单');
    CLEANUP.push({
      path: `/production/outsourcing-receipts/${bigId}`,
      label: `receipt(big)#${bigId}`,
    });
    const f3 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-receipts/${bigId}/confirm`
    );
    expect(f3.status, '收回量>发料量确认应 400').toBe(400);
    expect(failureCode(f3), '超量收回机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // E) 单号唯一门：重复 order_no 建单拒（validate_order_no_unique，order.rs:112-120 区段）
    const f4 = await apiCallExpectFail(page, 'POST', '/production/outsourcing-orders', {
      order_no: String(issued.order_no),
      order_type: 'dyeing',
      supplier_id: ctx.supplierId,
      issue_date: todayStr(),
      issue_quantity: '1',
      material_cost: '1',
    });
    expect(f4.status, `重复单号应 400，实际=${f4.status} body=${JSON.stringify(f4)}`).toBe(400);
    expect(failureCode(f4), '重复单号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // F) 明细引用不存在的生产匹号 ⇒ 发料整单拒（piece_domain_service.rs:186-189），订单仍 draft
    const pieceOrder = await seedOutsourcingOrder(page, 'PIECE');
    const pieceId = requireNum(pieceOrder.id, 'PIECE 订单');
    const fakePieceNo = `E23-NOPE-${genCode('P')}`;
    await apiCall(page, 'POST', '/production/outsourcing-orders/items', {
      outsourcing_order_id: pieceId,
      product_id: ctx.productIds[0],
      quantity: '10',
      unit_cost: '1',
      piece_no: fakePieceNo,
    });
    const f5 = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${pieceId}/issue`
    );
    expect(f5.status, '假匹号发料应 400').toBe(400);
    expect(failureCode(f5), '假匹号机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒无痕：明细仍在（裸数组键口径）
    const its = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(
        page,
        'GET',
        `/production/outsourcing-orders/items/by-order/${pieceId}`
      ),
      'bare',
      '假匹号订单明细'
    );
    expect(its.length, '发料被拒不应吞明细').toBe(1);
    expectKeyValue(its[0], 'piece_no', fakePieceNo, '假匹号建单后应原样回读（不被发料门吞键）');
    const noDrift2 = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${pieceId}`
    );
    expectKeyValue(noDrift2, 'status', 'draft', '假匹号发料被拒后仍 draft');
  });

  test('23-03 回归钉（本波修复后应为绿）：加工费/运费/税额经契约录入回读（FEE 链路数值生效）', async ({
    page,
  }) => {
    // settle 语义要求订单携带 processing_fee/freight_fee/tax_amount（order.rs:484 注释
    // "需在订单更新时填入"）。本波源码修复：UpdateOutsourcingOrderRequest
    // （services/outsourcing_ops/types.rs）补齐三键（NOT NULL 列 v15/mod.rs:3247-3249，
    // 显式 null 拒清），PUT 送键即须可回读；结算 FEE 凭证按真实值计（后端集成测
    // tests/contract_wave5_trade_fields_roundtrip_test.rs 同口径锁定）。
    // 本例再红即缺陷①委外形态回归，禁止放宽成"键缺失容忍"。
    const order = await seedOutsourcingOrder(page, 'FEE');
    const orderId = requireNum(order.id, 'FEE 订单');
    await apiCall(page, 'PUT', `/production/outsourcing-orders/${orderId}`, {
      processing_fee: 100,
    });
    const reread = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/production/outsourcing-orders/${orderId}`
    );
    if (Number(reread.processing_fee) !== 100) {
      throw new Error(
        `加工费经契约录入回读回归（缺陷①委外形态）：draft 期 PUT processing_fee=100 后回读=` +
          JSON.stringify(reread.processing_fee) +
          `；应查 backend/src/services/outsourcing_ops/types.rs UpdateOutsourcingOrderRequest 的 ` +
          `processing_fee/freight_fee/tax_amount 三键与 backend/src/services/outsourcing_ops/order.rs ` +
          `update 写入接线（settle 语义 order.rs:484 ⇒ FEE 凭证 order.rs:517-535）。` +
          `实际键=${Object.keys(reread).join(',')}`
      );
    }
  });
});
