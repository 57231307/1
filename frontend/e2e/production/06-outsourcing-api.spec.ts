// 生产管理 E2E 套件 — 06 委外加工单 API 契约（发料明细 + 状态机 + 凭证）
// 覆盖范围（本批补 production A 档零覆盖端点，page.request 直连契约覆盖，CI shard36 实跑）：
//   PUT    /outsourcing-orders/items/{id}          更新发料明细（update_outsourcing_item）
//   DELETE /outsourcing-orders/items/{id}          删除发料明细（delete_outsourcing_item）
//   POST   /outsourcing-orders/{id}/processing     标记加工中（record_processing，前置 issued）
//   POST   /outsourcing-orders/{id}/cancel         取消订单（cancel_outsourcing_order，非 closed/cancelled 可取消）
//   POST   /outsourcing-vouchers                   创建委外凭证（create_outsourcing_voucher）
//   GET    /outsourcing-vouchers/by-no/{no}        按凭证号查询（get_outsourcing_voucher_by_no）
//   DELETE /outsourcing-vouchers/{id}              删除未过账凭证（delete_outsourcing_voucher）
// 契约来源：backend/src/routes/production、outsourcing_handler.rs、
//   services/outsourcing_ops/{order,order_item,voucher}.rs、types.rs；
//   状态机 draft→issued→processing→received→settled→closed，任意非 closed→cancelled
//   （models/status/wage_energy_chemical_business）。
//   金额列为 rust_decimal，出参序列化为字符串（如 "25.5000"），断言前 Number() 归一。
// 本批未纳入的邻近 outsourcing 端点（留后续批次，非豁免）：
//   outsourcing-orders/*/print、outsourcing-receipts/*/print（docx 下载族，需按二进制
//   Content-Type 与文件体真实内容断言，与本轮 JSON 契约族不同形态）；
//   outsourcing-receipts/by-no/*（收回单族，需另建 receipt，本轮聚焦发料明细+状态机+凭证）。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  tryCleanup,
} from '../flow/helpers';

/** 委外订单出参（键对齐 models/outsourcing_order.rs，snake_case；decimal 列为字符串） */
interface OutsourcingOrder {
  id: number;
  order_no: string;
  order_type: string;
  supplier_id: number;
  status: string;
  issue_quantity: string;
  issue_unit: string;
  material_cost: string;
}

/** 委外发料明细出参（键对齐 models/outsourcing_order_item.rs；GET by-order 返回裸数组） */
interface OutsourcingOrderItem {
  id: number;
  outsourcing_order_id: number;
  product_id: number;
  quantity: string;
  unit: string;
  unit_cost: string;
  total_cost: string;
}

/** 委外凭证出参（键对齐 models/outsourcing_voucher.rs；amount/tax_amount 为 decimal 字符串） */
interface OutsourcingVoucher {
  id: number;
  voucher_no: string;
  outsourcing_order_id: number;
  voucher_type: string;
  debit_account: string;
  credit_account: string;
  amount: string;
  tax_amount: string;
  is_posted: boolean;
}

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 经真实 API 建一张最小草稿委外单（order_type=dyeing，外键 supplier_id 取夹具）。
 * order_no 显式传唯一值（后端 validate_order_no_unique 要求提供且不可重复），
 * 便于跨用例/跨运行互不撞号；id 缺失即抛（apiCall 非 200 已抛，此处再核主键）。
 */
async function createDraftOrder(page: import('@playwright/test').Page, tag: string) {
  const ctx = getCtx();
  expect(ctx.supplierId, '前置：夹具需产出 supplierId').toBeTruthy();
  const order = await apiCallRaw<OutsourcingOrder>(page, 'POST', '/production/outsourcing-orders', {
    order_no: `E2E-OS-${tag}`,
    order_type: 'dyeing',
    supplier_id: ctx.supplierId,
    issue_date: new Date().toISOString().slice(0, 10),
    issue_quantity: 100,
    material_cost: 1000,
    processing_fee: 200,
    freight_fee: 50,
    tax_amount: 30,
  });
  expect(order.id, '建委外单应返回 id').toBeTruthy();
  // 初始态硬断 draft：源码若把新单落成其它状态，本断即红
  expect(order.status, `新委外单初始态应为 draft，实际 ${order.status}`).toBe('draft');
  CLEANUP.push({ path: `/production/outsourcing-orders/${order.id}`, label: 'outsourcing_order' });
  return order;
}

/** 建一条发料明细（挂到指定委外单），quantity/unit_cost 走默认正值 */
async function createItem(
  page: import('@playwright/test').Page,
  orderId: number,
  overrides?: Record<string, unknown>
) {
  const ctx = getCtx();
  const item = await apiCallRaw<OutsourcingOrderItem>(
    page,
    'POST',
    '/production/outsourcing-orders/items',
    {
      outsourcing_order_id: orderId,
      product_id: ctx.productIds[0],
      quantity: 10,
      unit: 'kg',
      unit_cost: 5,
      ...overrides,
    }
  );
  expect(item.id, '创建发料明细应返回 id').toBeTruthy();
  return item;
}

/** 读取指定委外单的发料明细裸数组（GET .../items/by-order/{id} 出参为 Vec，非 {items}） */
async function listItems(page: import('@playwright/test').Page, orderId: number) {
  const res = await apiCall<OutsourcingOrderItem[]>(
    page,
    'GET',
    `/production/outsourcing-orders/items/by-order/${orderId}`
  );
  return res.data;
}

/**
 * 造一枚可用的胚布生产匹，返回匹号与入仓库。
 *
 * 发料门控（piece_domain_service::validate_pieces_for_issue）要求明细逐条引用
 * 真实存在且状态可用的生产匹，空明细或缺匹号都拒绝，故走到 issued 必须先有匹。
 * 链路与真实写入方一致：生产订单→流转卡→排产→备布→工序报工（匹记在报工收尾）。
 */
async function seedAvailableGreigePiece(
  page: import('@playwright/test').Page,
  tag: string
): Promise<{ pieceNo: string; warehouseId: number }> {
  const ctx = getCtx();
  const productId = ctx.productIds[0];
  if (!productId) throw new Error(`[${tag}] 前置：夹具需产出 productIds`);

  const whRes = await apiCallRaw<{ items?: Array<{ id?: number; warehouse_type?: string }> }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=200'
  );
  const whRows = Array.isArray(whRes.items) ? whRes.items : [];
  // 胚布匹入仓不得为成品仓（validate_warehouse_for_piece_type），优先取胚布仓
  const warehouse =
    whRows.find(w => w.warehouse_type === 'greige') ?? whRows.find(w => w.warehouse_type !== 'finished');
  const warehouseId = Number(warehouse?.id ?? 0);
  if (!warehouseId) throw new Error(`[${tag}] 无可用非成品仓，生产匹无法入仓，链断`);

  const planned = 100;
  const po = await apiCall<{ id?: number }>(page, 'POST', '/production/production-orders/orders', {
    order_no: `E2E-PO-${tag}`,
    product_id: productId,
    planned_quantity: planned,
  });
  const poId = po.data?.id;
  if (!poId) throw new Error(`[${tag}] 生产订单创建未返回 id：${JSON.stringify(po).slice(0, 200)}`);

  const card = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards', {
    production_order_id: poId,
    product_id: productId,
    product_name: `E2E委外胚布${tag}`,
    planned_fabric_weight: planned,
  });
  const cardId = card.data?.id;
  if (!cardId) throw new Error(`[${tag}] 流转卡创建未返回 id：${JSON.stringify(card).slice(0, 200)}`);

  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/schedule`, {});
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/start-preparing`);
  await apiCall(page, 'POST', `/production/flow-cards/${cardId}/complete-preparing`, {
    actual_fabric_weight: planned,
  });

  const step = await apiCall<{ id?: number }>(page, 'POST', '/production/flow-cards/steps/start', {
    flow_card_id: cardId,
  });
  const stepId = step.data?.id;
  if (!stepId) throw new Error(`[${tag}] 工序启动未返回 id：${JSON.stringify(step).slice(0, 200)}`);

  const pieceNo = `GR-${tag}-001`;
  await apiCall(page, 'POST', `/production/flow-cards/steps/${stepId}/complete`, {
    actual_quantity: planned,
    qualified_quantity: planned,
    pieces: [
      {
        piece_no: pieceNo,
        machine_no: 'M-E2E-ISSUE',
        machine_operator: 'E2E报工人',
        length: planned,
        weight: planned / 2,
        warehouse_id: warehouseId,
      },
    ],
  });
  return { pieceNo, warehouseId };
}

test.describe('06 委外加工单 API 契约', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('06-01 更新发料明细：quantity/unit_cost 覆盖并按新值回读', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `ITEM-UPD-${stamp}`);
    const item = await createItem(page, order.id, { quantity: 10, unit_cost: 5 });
    // 前置基线：total_cost = quantity*unit_cost（后端 compute）= 50
    expect(Number(item.total_cost), '创建明细 total_cost 应为 quantity×unit_cost').toBe(50);

    // PUT 覆盖 quantity→25.5、unit_cost→4
    const updated = await apiCallRaw<OutsourcingOrderItem>(
      page,
      'PUT',
      `/production/outsourcing-orders/items/${item.id}`,
      { quantity: 25.5, unit_cost: 4 }
    );
    expect(Number(updated.quantity), 'PUT 出参 quantity 应回显新值 25.5').toBe(25.5);
    expect(Number(updated.unit_cost), 'PUT 出参 unit_cost 应回显新值 4').toBe(4);

    // 落库回读：by-order 列表内该明细三值一致（源码若未持久化或未重算 total_cost，本断即红）
    const rows = await listItems(page, order.id);
    const target = rows.find(r => r.id === item.id);
    expect(target, '回读应命中被更新的明细').toBeTruthy();
    expect(Number(target!.quantity), '回读 quantity 应为 25.5').toBe(25.5);
    expect(Number(target!.unit_cost), '回读 unit_cost 应为 4').toBe(4);
    expect(Number(target!.total_cost), '回读 total_cost 应为 25.5×4=102').toBe(102);
  });

  test('06-02 更新发料明细拒绝 NOT NULL 列显式清空', async ({ page }) => {
    // 后端 UpdateOutsourcingOrderItemRequest：quantity/unit_cost 为 NOT NULL 列，
    // 显式 null（double_option 的 Some(None)）被 service 以 business_displayable 拒绝。
    const stamp = Date.now();
    const order = await createDraftOrder(page, `ITEM-NULL-${stamp}`);
    const item = await createItem(page, order.id);

    const fail = await apiCallExpectFail(
      page,
      'PUT',
      `/production/outsourcing-orders/items/${item.id}`,
      { quantity: null }
    );
    expect(
      fail.status,
      `清空 NOT NULL quantity 应返回 4xx，实际 ${fail.status}`
    ).toBeGreaterThanOrEqual(400);
    expect(fail.status, '应为业务拒绝 4xx 而非 5xx').toBeLessThan(500);

    // 拒绝不得改动存量：回读 quantity 仍为创建值 10
    const rows = await listItems(page, order.id);
    const target = rows.find(r => r.id === item.id);
    expect(Number(target!.quantity), '非法清空被拒后 quantity 应仍为 10').toBe(10);
  });

  test('06-03 删除发料明细后 by-order 回读不再命中', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `ITEM-DEL-${stamp}`);
    const kept = await createItem(page, order.id, { quantity: 3 });
    const doomed = await createItem(page, order.id, { quantity: 7 });

    const del = await apiCall(page, 'DELETE', `/production/outsourcing-orders/items/${doomed.id}`);
    expect(del.code, 'DELETE 明细应返回成功信封码').toBe(200);

    // 回读：被删明细消失，同单其它明细保留（源码若软删未过滤或误删他行，本断即红）
    const rows = await listItems(page, order.id);
    expect(
      rows.some(r => r.id === doomed.id),
      '被删明细不应再出现在 by-order 列表'
    ).toBe(false);
    expect(
      rows.some(r => r.id === kept.id),
      '同单其它明细应仍存在'
    ).toBe(true);
  });

  test('06-04 issued 订单可标记加工中并回读终值', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `PROC-${stamp}`);
    // 发料前置：后端按匹校验，空明细与缺匹号都会被拒，先造一枚真实可用的胚布生产匹并挂进明细
    const piece = await seedAvailableGreigePiece(page, `PROC${stamp}`);
    await createItem(page, order.id, {
      quantity: 100,
      piece_no: piece.pieceNo,
      dye_lot_no: `LOT-${stamp}`,
      warehouse_id: piece.warehouseId,
    });

    // draft → issued（POST /issue 为已覆盖前置端点，此处仅用作 processing 的合法前置）
    const issued = await apiCallRaw<OutsourcingOrder>(
      page,
      'POST',
      `/production/outsourcing-orders/${order.id}/issue`
    );
    expect(issued.status, `发料后状态应为 issued，实际 ${issued.status}`).toBe('issued');

    // issued → processing
    const proc = await apiCallRaw<OutsourcingOrder>(
      page,
      'POST',
      `/production/outsourcing-orders/${order.id}/processing`
    );
    expect(proc.status, `标记加工后状态应为 processing，实际 ${proc.status}`).toBe('processing');

    // GET 详情回读二次确认（迁移响应与库内状态必须一致）
    const detail = await apiCallRaw<OutsourcingOrder>(
      page,
      'GET',
      `/production/outsourcing-orders/${order.id}`
    );
    expect(detail.status, '回读委外单状态应为 processing').toBe('processing');
  });

  test('06-05 非 issued 态标记加工中被拒且不改状态', async ({ page }) => {
    // 后端 record_processing 门：status != ISSUED → business 400（"仅已发料(issued)状态可标记加工中"）
    const stamp = Date.now();
    const order = await createDraftOrder(page, `PROC-DENY-${stamp}`); // 仍为 draft

    const fail = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${order.id}/processing`
    );
    expect(fail.status, `draft 态加工应 4xx，实际 ${fail.status}`).toBeGreaterThanOrEqual(400);
    expect(fail.status, '状态门拒绝应为 4xx 非 5xx').toBeLessThan(500);

    const detail = await apiCallRaw<OutsourcingOrder>(
      page,
      'GET',
      `/production/outsourcing-orders/${order.id}`
    );
    expect(detail.status, '非法跃迁被拒后状态应仍为 draft').toBe('draft');
  });

  test('06-06 草稿订单可取消并回读 cancelled 终态', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `CANCEL-${stamp}`);

    const cancelled = await apiCallRaw<OutsourcingOrder>(
      page,
      'POST',
      `/production/outsourcing-orders/${order.id}/cancel`
    );
    expect(cancelled.status, `取消后状态应为 cancelled，实际 ${cancelled.status}`).toBe(
      'cancelled'
    );

    // 取消是软性终态转移：回读一致，且已取消不可重复取消（cancel 门：CANCELLED → business）
    const detail = await apiCallRaw<OutsourcingOrder>(
      page,
      'GET',
      `/production/outsourcing-orders/${order.id}`
    );
    expect(detail.status, '回读状态应为 cancelled').toBe('cancelled');

    const again = await apiCallExpectFail(
      page,
      'POST',
      `/production/outsourcing-orders/${order.id}/cancel`
    );
    expect(again.status, '重复取消应被拒（4xx）').toBeGreaterThanOrEqual(400);
    expect(again.status, '重复取消拒绝应为 4xx 非 5xx').toBeLessThan(500);
    // 已成功取消，撤下 order 清理项（cancel 终态仍可能拒删，交由 tryCleanup 容错即可，保留无妨）
  });

  test('06-07 创建凭证并回读全字段一致', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `VOCH-${stamp}`);
    const vno = `E2E-VCH-${stamp}`;

    const v = await apiCallRaw<OutsourcingVoucher>(
      page,
      'POST',
      '/production/outsourcing-vouchers',
      {
        voucher_no: vno,
        outsourcing_order_id: order.id,
        voucher_type: 'issue',
        debit_account: '1405',
        credit_account: '1403',
        amount: 1234.5,
        tax_amount: 100,
        voucher_date: new Date().toISOString().slice(0, 10),
        remarks: 'E2E 发料凭证',
      }
    );
    CLEANUP.push({
      path: `/production/outsourcing-vouchers/${v.id}`,
      label: 'outsourcing_voucher',
    });

    expect(v.id, '创建凭证应返回 id').toBeTruthy();
    expect(v.voucher_no, '凭证号应回显提交值').toBe(vno);
    expect(v.outsourcing_order_id, '凭证应挂在所建委外单下').toBe(order.id);
    expect(v.voucher_type, '凭证类型应回显 issue').toBe('issue');
    expect(Number(v.amount), '凭证金额应回显 1234.5').toBe(1234.5);
    expect(v.is_posted, '新建凭证应为未过账').toBe(false);
  });

  test('06-08 非法凭证类型被拒', async ({ page }) => {
    // validate_voucher_type：非 issue/fee/receipt/loss → business 400
    const stamp = Date.now();
    const order = await createDraftOrder(page, `VOCH-BAD-${stamp}`);

    const fail = await apiCallExpectFail(page, 'POST', '/production/outsourcing-vouchers', {
      voucher_no: `E2E-VCHBAD-${stamp}`,
      outsourcing_order_id: order.id,
      voucher_type: 'advance',
      debit_account: '1405',
      credit_account: '1403',
      amount: 10,
      voucher_date: new Date().toISOString().slice(0, 10),
    });
    expect(fail.status, `非法 voucher_type 应 4xx，实际 ${fail.status}`).toBeGreaterThanOrEqual(
      400
    );
    expect(fail.status, '类型校验拒绝应为 4xx 非 5xx').toBeLessThan(500);
  });

  test('06-09 按凭证号查询命中同一凭证', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `VOCH-BYNO-${stamp}`);
    const vno = `E2E-VCH-BYNO-${stamp}`;

    const created = await apiCallRaw<OutsourcingVoucher>(
      page,
      'POST',
      '/production/outsourcing-vouchers',
      {
        voucher_no: vno,
        outsourcing_order_id: order.id,
        voucher_type: 'fee',
        debit_account: '5001',
        credit_account: '1122',
        amount: 250,
        voucher_date: new Date().toISOString().slice(0, 10),
      }
    );
    CLEANUP.push({
      path: `/production/outsourcing-vouchers/${created.id}`,
      label: 'outsourcing_voucher',
    });

    const byNo = await apiCallRaw<OutsourcingVoucher>(
      page,
      'GET',
      `/production/outsourcing-vouchers/by-no/${vno}`
    );
    // 主键与关键业务字段必须与创建响应逐一对齐（by-no 命中错行/错型即红）
    expect(byNo.id, 'by-no 应命中刚创建的同一凭证主键').toBe(created.id);
    expect(byNo.voucher_no, 'by-no 回显凭证号').toBe(vno);
    expect(byNo.voucher_type, 'by-no 回显凭证类型 fee').toBe('fee');
    expect(Number(byNo.amount), 'by-no 回显金额 250').toBe(250);
  });

  test('06-10 删除未过账凭证后按号查询落空 404', async ({ page }) => {
    const stamp = Date.now();
    const order = await createDraftOrder(page, `VOCH-DEL-${stamp}`);
    const vno = `E2E-VCH-DEL-${stamp}`;

    const created = await apiCallRaw<OutsourcingVoucher>(
      page,
      'POST',
      '/production/outsourcing-vouchers',
      {
        voucher_no: vno,
        outsourcing_order_id: order.id,
        voucher_type: 'loss',
        debit_account: '1901',
        credit_account: '1405',
        amount: 88,
        voucher_date: new Date().toISOString().slice(0, 10),
      }
    );

    // 未过账凭证可删：DELETE 成功
    const del = await apiCall(page, 'DELETE', `/production/outsourcing-vouchers/${created.id}`);
    expect(del.code, 'DELETE 未过账凭证应返回成功信封码').toBe(200);

    // 删除后按号查询应 404（get_by_no 缺失 → AppError::not_found），证明真删而非软残留。
    // 用 created.voucher_no 属性访问取号，保持与 06-09 同一 by-no 路径归一口径。
    const fail = await apiCallExpectFail(
      page,
      'GET',
      `/production/outsourcing-vouchers/by-no/${created.voucher_no}`
    );
    expect(fail.status, `已删凭证按号查询应 404，实际 ${fail.status}`).toBe(404);
  });
});
