// 本文件守的缺陷族：状态机门被绕过（跳状态推进）、非法流转被放行后状态回跳或半程写入、
// 发货前置门（订单外产品/路径载荷错配/染色布缺维）失效，以及取消发货不真正反向回滚账本
// （库存、订单行 shipped_quantity、发货单状态、匹消耗四处必须同时回位，缺一处即账实脱节）。
// 每个负例三件套：精确 HTTP 码 + 精确机器码 +（可外显族）拒绝文案点名原因；
// 并强制「被拒零漂移」回读：状态、行数量、库存量、发货单集合在拒绝后逐项与原值相等。
// 状态比较值与写入方逐字符同源：sales_order 小写，sales_delivery 小写 pending/shipped/cancelled。
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
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  tryCleanup,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

type Row = Record<string, unknown>;

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

function toScaled(v: unknown, scale: number, label: string): bigint {
  const s = typeof v === 'string' ? v.trim() : typeof v === 'number' ? String(v) : '';
  if (s === '') throw new Error(`${label}：Decimal 出参非 string|number，raw=${JSON.stringify(v)}`);
  const m = /^(-?)(\d+)(?:\.(\d+))?$/.exec(s);
  if (!m) throw new Error(`${label}：Decimal 出参形态非法，raw="${s}"`);
  const [, sign, intPart, frac = ''] = m;
  const padded = (intPart + frac).padEnd(intPart.length + scale, '0');
  const value = BigInt(padded);
  return sign === '-' ? -value : value;
}

async function readOrder(page: import('@playwright/test').Page, id: number): Promise<Row> {
  return apiCallRaw<Row>(page, 'GET', `/sales/orders/${id}`);
}

function orderLine(detail: Row, productId: number): Row {
  const lines = detail.items as Row[];
  const hit = lines.find(l => Number(l.product_id) === productId);
  if (!hit) throw new Error(`订单明细缺产品 ${productId} 的行，实际=${JSON.stringify(lines)}`);
  return hit;
}

async function seedOrder(
  page: import('@playwright/test').Page,
  productId: number,
  quantity: string
): Promise<{ id: number; order_no: string; status: string }> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  const created = await apiCallRaw<Row>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [
      {
        product_id: productId,
        quantity,
        unit_price: '9.00',
        tax_percent: '0',
        quantity_tolerance_pct: '0',
      },
    ],
    notes: `E2E-GATE-${genCode('G')}`,
  });
  const id = requireNum(created.id, '建销售订单');
  CLEANUP.push({ path: `/sales/orders/${id}`, label: `so#${id}` });
  const detail = await readOrder(page, id);
  return { id, order_no: String(detail.order_no), status: String(detail.status) };
}

test.describe('93 销售链路非法流转与取消发货回滚', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('93-01 订单状态机负例族：跳步审批/提前完成/终态回退全部被拒且零漂移，机敏文案点名当前态', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds 为空');

    // A) 草稿跳步审批（draft→approve）：被拒且仍 draft
    const orderA = await seedOrder(page, productId, '10');
    expect(orderA.status, '新订单起点词值 draft').toBe('draft');
    const fApproveDraft = await apiCallExpectFail(
      page,
      'POST',
      `/sales/orders/${orderA.id}/approve`
    );
    expect(fApproveDraft.status, '草稿直接审批应 400').toBe(400);
    expect(failureCode(fApproveDraft), '草稿审批机器码 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const aStill = await readOrder(page, orderA.id);
    expect(aStill.status, '被拒后不得漂出 draft').toBe('draft');

    // B) 草稿直接完成（跳 submit/approve/ship）：被拒
    const fCompleteDraft = await apiCallExpectFail(
      page,
      'POST',
      `/sales/orders/${orderA.id}/complete`
    );
    expect(fCompleteDraft.status, '草稿直接完成应 400').toBe(400);
    expect(failureCode(fCompleteDraft), '草稿完成机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // C) pending 态发货：拒绝并点名可发货态（该门文案可外显，钉文案非脱敏常量）
    await apiCall(page, 'POST', `/sales/orders/${orderA.id}/submit`);
    const pending = await readOrder(page, orderA.id);
    expect(pending.status, 'submit 回读 pending').toBe('pending');
    const fShipPending = await apiCallExpectFail(page, 'POST', `/sales/orders/${orderA.id}/ship`, {
      order_id: orderA.id,
      warehouse_code: 'WH-NOT-USED',
      items: [
        { product_id: productId, quantity: '1', batch_no: 'B', color_no: 'C', dye_lot_no: 'D' },
      ],
    });
    expect(fShipPending.status, 'pending 发货应 400').toBe(400);
    expect(failureCode(fShipPending), 'pending 发货机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const shipMsg = String(fShipPending.message ?? '');
    expect(shipMsg, '发货门文案必须点名可发货前置态，不得是脱敏常量').toContain(
      '仅已审批或部分发货'
    );
    // D) pending 重复提交：拒绝文案点名当前态 pending（回显自单据本身，属可外显族）
    const fResubmit = await apiCallExpectFail(page, 'POST', `/sales/orders/${orderA.id}/submit`);
    expect(fResubmit.status, '重复提交应 400').toBe(400);
    expect(failureCode(fResubmit), '重复提交机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const resubmitMsg = String(fResubmit.message ?? '');
    expect(resubmitMsg, `提交门文案应点名当前状态 pending，实际=${resubmitMsg}`).toContain(
      'pending'
    );
    const pStill = await readOrder(page, orderA.id);
    expect(pStill.status, '被拒重复提交后仍 pending').toBe('pending');

    // E) 推满到 shipped→completed，再回退全部被拒（终态不可逆）
    await apiCall(page, 'POST', `/sales/orders/${orderA.id}/approve`);
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '10',
      pieceCount: 1,
      context: 'E93-01E',
    });
    await apiCall(page, 'POST', `/sales/orders/${orderA.id}/ship`, {
      order_id: orderA.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: productId,
          quantity: '10',
          batch_no: bundle.dyeLotNo,
          color_no: bundle.colorNo,
          dye_lot_no: bundle.dyeLotNo,
          piece_no: bundle.pieces[0].piece_no,
        },
      ],
    });
    const shipped = await readOrder(page, orderA.id);
    expect(shipped.status, '全额发货回读 shipped').toBe('shipped');
    await apiCall(page, 'POST', `/sales/orders/${orderA.id}/complete`);
    const completed = await readOrder(page, orderA.id);
    expect(completed.status, '完成回读 completed（终态起点）').toBe('completed');

    for (const action of ['submit', 'approve', 'ship'] as const) {
      const f = await apiCallExpectFail(
        page,
        'POST',
        `/sales/orders/${orderA.id}/${action}`,
        action === 'ship'
          ? {
              order_id: orderA.id,
              warehouse_code: target.code,
              items: [
                {
                  product_id: productId,
                  quantity: '1',
                  batch_no: bundle.dyeLotNo,
                  color_no: bundle.colorNo,
                  dye_lot_no: bundle.dyeLotNo,
                  piece_no: bundle.pieces[0].piece_no,
                },
              ],
            }
          : undefined
      );
      expect(f.status, `completed 终态执行 ${action} 应 400`).toBe(400);
      expect(failureCode(f), `completed 终态 ${action} 机器码`).toBe(
        APP_ERROR_CODES.BUSINESS_ERROR
      );
    }
    const tStill = await readOrder(page, orderA.id);
    expect(tStill.status, '三连拒后终态 completed 纹丝不动').toBe('completed');
    // 终态订单行数量守恒：完成后再被拒的 ship 不得累加 shipped_quantity
    expect(toScaled(orderLine(tStill, productId).shipped_quantity, 4, '终态行已发量')).toBe(
      toScaled(orderLine(tStill, productId).quantity, 4, '终态行数量')
    );

    // F) 已拒绝单不可提交/发货
    const orderR = await seedOrder(page, productId, '10');
    await apiCall(page, 'POST', `/sales/orders/${orderR.id}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${orderR.id}/reject`, { reason: 'E2E 门控样本' });
    const rejected = await readOrder(page, orderR.id);
    expect(rejected.status, 'reject 回读小写 rejected').toBe('rejected');
    for (const action of ['submit', 'approve'] as const) {
      const f = await apiCallExpectFail(page, 'POST', `/sales/orders/${orderR.id}/${action}`);
      expect(f.status, `rejected 单执行 ${action} 应 400`).toBe(400);
      expect(failureCode(f), `rejected 单 ${action} 机器码`).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    }
    const rStill = await readOrder(page, orderR.id);
    expect(rStill.status, 'rejected 是稳定态，被拒后不变').toBe('rejected');
    expect(String(rStill.rejected_reason), '拒绝理由必须真实落库可回读').toBe('E2E 门控样本');
  });

  test('93-02 发货前置门：订单外产品/路径载荷错配/染色布缺维三拒，均整单拒绝且账本无痕', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const otherProductId = ctx.productIds[3] ?? ctx.productIds[1];
    if (!productId || !otherProductId || productId === otherProductId)
      throw new Error('前置缺失：需要两个不同产品');

    const order = await seedOrder(page, productId, '50');
    await apiCall(page, 'POST', `/sales/orders/${order.id}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${order.id}/approve`);
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '50',
      pieceCount: 1,
      context: 'E93-02',
    });

    // A) 路径 id 与载荷 order_id 错配 → BAD_REQUEST（防以错配绕归属门/静默发往他单）
    const fMismatch = await apiCallExpectFail(page, 'POST', `/sales/orders/${order.id}/ship`, {
      order_id: order.id + 1,
      warehouse_code: target.code,
      items: [],
    });
    expect(fMismatch.status, '路径/载荷错配应 400').toBe(400);
    expect(failureCode(fMismatch), '错配机器码 BAD_REQUEST').toBe(APP_ERROR_CODES.BAD_REQUEST);

    // B) 订单外产品混入 → 整单拒绝并点名（历史上被按 0 单价静默处理即「发货成功两头空」）
    const fForeign = await apiCallExpectFail(page, 'POST', `/sales/orders/${order.id}/ship`, {
      order_id: order.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: otherProductId,
          quantity: '5',
          batch_no: bundle.dyeLotNo,
          color_no: bundle.colorNo,
          dye_lot_no: bundle.dyeLotNo,
          piece_no: bundle.pieces[0].piece_no,
        },
      ],
    });
    expect(fForeign.status, '订单外产品发货应 400').toBe(400);
    expect(failureCode(fForeign), '订单外产品机器码 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(String(fForeign.message ?? ''), '拒绝文案必须点名订单未包含产品').toContain(
      '未包含的产品'
    );

    // C) 染色布缺批次/缸号维 → VALIDATION_ERROR 且文案点名缺失维度（缺维属必填字段族）
    const fDim = await apiCallExpectFail(page, 'POST', `/sales/orders/${order.id}/ship`, {
      order_id: order.id,
      warehouse_code: target.code,
      items: [{ product_id: productId, quantity: '5' }],
    });
    expect(fDim.status, '缺维发货应 400').toBe(400);
    expect(failureCode(fDim), '缺维机器码 VALIDATION_ERROR').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(String(fDim.message ?? ''), '缺维拒绝文案不得是脱敏常量').not.toBe('请求参数验证失败');
    expect(String(fDim.message ?? ''), '缺维拒绝应点名批次/批号').toContain('批');

    // 三拒无痕：订单仍 approved、行未发量、库存分毫未动、发货单集合为空
    const still = await readOrder(page, order.id);
    expect(still.status, '三连拒后订单保持 approved').toBe('approved');
    expect(toScaled(orderLine(still, productId).shipped_quantity, 4, '被拒后行已发量')).toBe(0n);
    const stockRows = pickListArray<Row>(
      await apiCallRaw<Row>(
        page,
        'GET',
        `/inventory/stock?product_id=${productId}&warehouse_id=${target.id}&color_no=${encodeURIComponent(bundle.colorNo)}&page=1&page_size=20`
      ),
      'items',
      '被拒后的四维库存行'
    );
    const stockRow = stockRows.find(r => r.dye_lot_no === bundle.dyeLotNo);
    expect(stockRow, '四维库存行必须仍可回读').toBeTruthy();
    expect(toScaled(stockRow!.quantity_available, 4, '被拒后可用量')).toBe(
      toScaled('50', 4, '备货量')
    );
    expect(toScaled(stockRow!.quantity_shipped, 4, '被拒后已发量')).toBe(0n);
    const dlv = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    expect(
      pickListArray<Row>(dlv, 'list', '被拒后发货单列表').length,
      '被拒发货不得产生发货单'
    ).toBe(0);
  });

  test('93-03 取消发货全账本反向回滚：库存/行已发量/订单状态/发货单状态四处同时回位，二次取消被拒', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[1] ?? ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds 为空');

    const order = await seedOrder(page, productId, '40');
    await apiCall(page, 'POST', `/sales/orders/${order.id}/submit`);
    await apiCall(page, 'POST', `/sales/orders/${order.id}/approve`);
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '40',
      pieceCount: 1,
      context: 'E93-03',
    });
    await apiCall(page, 'POST', `/sales/orders/${order.id}/ship`, {
      order_id: order.id,
      warehouse_code: target.code,
      items: [
        {
          product_id: productId,
          quantity: '40',
          batch_no: bundle.dyeLotNo,
          color_no: bundle.colorNo,
          dye_lot_no: bundle.dyeLotNo,
          piece_no: bundle.pieces[0].piece_no,
        },
      ],
    });
    const shipped = await readOrder(page, order.id);
    expect(shipped.status, '发货后 shipped').toBe('shipped');
    const dlvEnv = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    const dlvRow = pickListArray<Row>(dlvEnv, 'list', '发货单列表')[0];
    expect(dlvRow, '前置：应有一张发货单').toBeTruthy();
    const deliveryId = requireNum(dlvRow.id, '发货单 id');

    // 取消发货（原因必填）→ 四处账本同时回位
    await apiCall(page, 'POST', `/sales/orders/${order.id}/deliveries/${deliveryId}/cancel`, {
      reason: 'E2E 反向回滚验证',
    });
    const reverted = await readOrder(page, order.id);
    expect(reverted.status, '全部发货被取消后订单回退 approved（不是 shipped 残留）').toBe(
      'approved'
    );
    expect(toScaled(orderLine(reverted, productId).shipped_quantity, 4, '取消后行已发量')).toBe(0n);
    const dlvAfter = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    const dlvRowAfter = pickListArray<Row>(dlvAfter, 'list', '取消后发货单列表').find(
      r => Number(r.id) === deliveryId
    );
    expect(dlvRowAfter, '取消不删行：发货单仍应可回读（留痕）').toBeTruthy();
    expect(dlvRowAfter!.status, '发货单状态小写 cancelled').toBe('cancelled');
    const stockRows = pickListArray<Row>(
      await apiCallRaw<Row>(
        page,
        'GET',
        `/inventory/stock?product_id=${productId}&warehouse_id=${target.id}&color_no=${encodeURIComponent(bundle.colorNo)}&page=1&page_size=20`
      ),
      'items',
      '取消后四维库存行'
    );
    const stockRow = stockRows.find(r => r.dye_lot_no === bundle.dyeLotNo);
    expect(stockRow, '取消后四维库存行仍可回读').toBeTruthy();
    expect(toScaled(stockRow!.quantity_available, 4, '取消后可用量必须回位 40')).toBe(
      toScaled('40', 4, '备货量')
    );
    expect(toScaled(stockRow!.quantity_shipped, 4, '取消后已发量归零')).toBe(0n);
    // 匹消耗反向：SHIPPED→AVAILABLE（第四维回位与扣减同为强制项）
    const pieceRows = pickListArray<Row>(
      await apiCallRaw<Row>(
        page,
        'GET',
        `/inventory/pieces?product_id=${productId}&warehouse_id=${target.id}&dye_lot_no=${encodeURIComponent(bundle.dyeLotNo)}&page=1&page_size=20`
      ),
      'items',
      '取消后匹行'
    );
    const piece = pieceRows.find(r => r.piece_no === bundle.pieces[0].piece_no);
    expect(piece, '取消后应仍回读到该匹').toBeTruthy();
    expect(String(piece!.status), '取消发货后匹必须回位 AVAILABLE').toBe('AVAILABLE');

    // 二次取消同一发货单被拒（仅 shipped 可取消），且回位结果不被二次取消破坏
    const fRecancel = await apiCallExpectFail(
      page,
      'POST',
      `/sales/orders/${order.id}/deliveries/${deliveryId}/cancel`,
      { reason: '再取消一次' }
    );
    expect(fRecancel.status, 'cancelled 发货单再取消应 400').toBe(400);
    expect(failureCode(fRecancel), '再取消机器码 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const finalOrder = await readOrder(page, order.id);
    expect(finalOrder.status, '被拒二次取消后订单仍 approved').toBe('approved');

    // 手动创建的 pending 发货单不可取消（同门：仅 shipped 可取消，pending 是未出库预留态）
    const pendingDlv = await apiCallRaw<Row>(page, 'POST', `/sales/orders/${order.id}/deliveries`, {
      warehouse_id: target.id,
    });
    const pendingId = requireNum(pendingDlv.id, '建 pending 发货单');
    expect(pendingDlv.status, '手动创建发货单词值 pending').toBe('pending');
    const fPendingCancel = await apiCallExpectFail(
      page,
      'POST',
      `/sales/orders/${order.id}/deliveries/${pendingId}/cancel`,
      { reason: 'E2E 门控' }
    );
    expect(fPendingCancel.status, 'pending 发货单取消应 400').toBe(400);
    expect(failureCode(fPendingCancel), 'pending 取消机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const pendingStill = await apiCallRaw<Row>(page, 'GET', `/sales/orders/${order.id}/deliveries`);
    const pendingRow = pickListArray<Row>(pendingStill, 'list', '取消被拒后的发货单列表').find(
      r => Number(r.id) === pendingId
    );
    expect(pendingRow!.status, '被拒后 pending 发货单状态不变').toBe('pending');
  });
});
