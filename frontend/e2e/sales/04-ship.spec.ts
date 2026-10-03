// P9-3 销售 E2E 套件 — 04 销售发货
// 覆盖范围：已审批订单行内「发货」→ DeliveryDialog 出库（真实对话框，非详情页按钮）
//
// 并行/种子隔离改造（PR #941 run #4656 红簇取证）：
// 原四例都从 /sales 列表抓 `.first()` 的「已审批」行。同分片 fullyParallel 多 worker 下四例抢
// 同一张共享 approved 行，且 04-04 真正发货后该行状态流转（approved→partially_shipped/shipped），
// 其余例就找不到 approved 行 → 成片红。属真 seed/并行串扰，补选择器解决不了。
// 改为：每例进入前用 API 造一张「本例专属、approved 态」的销售订单，用后端唯一 order_no 精确筛选
// 锚定该行后再操作，彻底脱离共享行。
//
// 出库第四维同步（用户 2026-10-02 口径：出库对染色布强制四维=缸号/色号/批次/匹号，PR #942）：
// 04-04/04-05 的发货行是染色布（色号非空），出库除四维库存行外还必须消耗同 tuple 的真实
// AVAILABLE 染色匹（后端 piece_domain_service.rs:606 outbound_piece_filter CAS）。
// seed 统一走 helpers.seedDyedOutboundBundle：库存行 batch=缸号 + 委外染色真实链同维造匹，
// 仓库改为确定性选定（pickDyeableWarehouse + 按名称在对话框挑该仓），不再"每仓灌一行碰运气"。
// 04-05 为第四维负例：UI 未选匹号被拦（前端提示）+ 直连 API 缺 piece_no 被 400
// VALIDATION_ERROR 且文案外显真实原因（≠脱敏常量「请求参数验证失败」）、被拒写无痕。
//
// 真实 UI 事实（据 SalesOrderTable.vue / DeliveryDialog.vue / useOlv.ts / locales 核对）：
// - 发货入口：/sales 列表已审批行（状态标签 '已审批'）行内按钮 sales.table.deliver = '发货'
//   → OrderListView.onDelivery → olv.prepareDelivery → 打开 DeliveryDialog。
//   不存在"详情页创建发货单/保存按钮/发货单号 DN-"等 UI。
// - DeliveryDialog（aria-label sales.delivery.dialogAriaLabel='销售发货对话框'，标题 '销售发货'）真实控件：
//   只读销售单号/客户；发货日期 date picker 占位 '选择日期'；仓库 el-select（label '仓库'）；
//   明细 el-table（aria-label '销售发货明细表'）列 产品/库存行/缸号/色号/批次/匹号/订单数量/已发货/
//   本次发货(el-input-number)/单价/备注；底部 '取消' / '确定发货'。
//   出库需先选仓库加载库存行，再按四维(色号/批次/缸号)选库存行；染色布行（color_no 非空）随后
//   出现「匹号」el-select（候选由 loadDeliveryPieces 按 产品+仓+缸+批+AVAILABLE 下推查询），
//   白坯行该列渲染 '—'；本次发货 el-input-number :max = 订单数量 - 已发货，超限输入会被钳制。
//   单明细行时对话框内 el-select 顺序：0=仓库、1=库存行、2=匹号。
// - 校验（handleSubmit）：未选仓库 → ElMessage.warning('请选择仓库')；未填发货日期 → '请选择发货日期'；
//   染色布未选匹号 → '染色布发货必须选定匹号（缸号/色号/批次/匹号 四维齐才可出库）'。
// - 成功：handleDeliverySubmit → shipSalesOrder → msg.success('shipSuccess') = '发货成功'。

import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';
import {
  ensureTestEntities,
  getCtx,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  tryCleanup,
  genCode,
  failureCode,
  APP_ERROR_CODES,
  verifyStockFourDim,
  seedDyedOutboundBundle,
  pickDyeableWarehouse,
  fetchAvailableDyedPieces,
  readDyedPieceByNo,
} from '../flow/helpers';

/** 后端 SalesOrderDetail 中本套件用到的字段 */
interface SalesOrderLite {
  id: number;
  order_no: string;
  status: string;
}

const CREATED_ORDER_IDS: number[] = [];
test.afterEach(async ({ page }) => {
  while (CREATED_ORDER_IDS.length) {
    const id = CREATED_ORDER_IDS.pop();
    if (id != null) await tryCleanup(page, 'DELETE', `/sales/orders/${id}`, `sales_order#${id}`);
  }
});

/**
 * 用 API 造一张本例专属、approved 态的销售订单（POST → /submit → /approve），回查确认落态，
 * 返回 {id, orderNo}。每步后以真实 status 字面量断言，不达预期立即抛错（禁止默认值兜底）。
 * 订单产品取 ctx.productIds[0]（ensureTestEntities 已为其备好可出库四维库存）。
 */
async function seedApprovedOrder(page: Page): Promise<SalesOrderLite> {
  const ctx = getCtx();
  if (!ctx.customerId) throw new Error('前置缺失：ctx.customerId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const marker = genCode('S4');

  const created = await apiCall<SalesOrderLite>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: ctx.productIds[0], quantity: '10', unit_price: '25.00' }],
    notes: `E2E-S4-${marker}`,
  });
  const id = created.data?.id;
  const orderNo = created.data?.order_no;
  if (!id || !orderNo) {
    throw new Error(`前置失败：销售订单创建未返回 id/order_no：${JSON.stringify(created)}`);
  }
  CREATED_ORDER_IDS.push(id);

  await apiCall(page, 'POST', `/sales/orders/${id}/submit`);
  const afterSubmit = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
  if (afterSubmit.status !== 'pending') {
    throw new Error(`submit 后状态应为 pending（实际 ${afterSubmit.status}，id=${id}）`);
  }
  await apiCall(page, 'POST', `/sales/orders/${id}/approve`);
  const afterApprove = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
  if (afterApprove.status !== 'approved') {
    throw new Error(`approve 后状态应为 approved（实际 ${afterApprove.status}，id=${id}）`);
  }
  return { id, order_no: orderNo, status: afterApprove.status };
}

/** 用「订单号」筛选把列表收敛到本例那一行（跨分页/并发唯一稳定锚点）并断言命中 */
async function locateRowByOrderNo(page: Page, orderNo: string) {
  await page.goto('/sales');
  await page.getByPlaceholder('订单号').fill(orderNo);
  await page.getByRole('button', { name: '查询', exact: true }).click();
  const row = page.getByRole('row').filter({ hasText: orderNo });
  await expect(row, `按订单号 ${orderNo} 应筛选出本例专属行`).toHaveCount(1, { timeout: 30000 });
}

/**
 * 筛选到本例唯一行后按精确名取行内操作按钮。
 * 销售列表是 V2Table（el-table-v2），操作列 fixed:'right' 由 EP 渲染到独立 overlay Grid，
 * 其按钮与承载「订单号」的主 Grid 行分属不同 DOM row（EP row.mjs），故不能用 `row.getByRole('button')`
 * 作用域定位（会命中 0 个 → click 30s 超时）。唯一订单号已把整表筛到 1 行，页面级精确名即唯一命中
 * （对齐 production/01、system/02 既有 V2Table 范式）。
 */
function orderActionBtn(page: Page, name: string) {
  return page.getByRole('button', { name, exact: true }).first();
}

/** 打开本例已审批订单的发货对话框：筛到唯一行 → 点行内「发货」→ 返回 '销售发货' 对话框 */
async function openDeliveryDialog(page: Page, orderNo: string) {
  await locateRowByOrderNo(page, orderNo);
  await orderActionBtn(page, '发货').click();
  const dialog = page.getByRole('dialog', { name: '销售发货' });
  await expect(dialog).toBeVisible();
  return dialog;
}

test.describe('04 销售发货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('04-01 已审批订单行内「发货」打开发货对话框', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const dialog = await openDeliveryDialog(page, orderNo);
    // 底部真实按钮 sales.delivery.confirmDelivery = '确定发货'
    await expect(dialog.getByRole('button', { name: '确定发货' })).toBeVisible();
  });

  test('04-02 未选仓库直接确定发货被拦截', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const dialog = await openDeliveryDialog(page, orderNo);
    // 未选仓库即点确定发货
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实校验 sales.delivery.warehouseRequired = '请选择仓库'
    await expect(page.getByText('请选择仓库')).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
  });

  test('04-03 本次发货数量受订单可发数量钳制', async ({ page }) => {
    const { order_no: orderNo } = await seedApprovedOrder(page);
    const dialog = await openDeliveryDialog(page, orderNo);
    // 先选仓库以启用库存行与本次发货上限计算
    await pickSelectIn(dialog, page, '仓库');
    const qty = dialog.getByRole('spinbutton').first();
    // 输入远超订单数量（订单量 10）的值
    await qty.fill('99999');
    await page.keyboard.press('Tab');
    // el-input-number :max 生效：失焦后实际值被钳制到可发上限（≠ 99999）
    await expect(qty).not.toHaveValue('99999');
  });

  test('04-04 填写发货四维（含匹号）并确定发货成功后给出成功提示', async ({ page }) => {
    test.setTimeout(240_000);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    // 确定性选仓（替代旧"每仓灌一行、下拉首项碰运气"）：选可承载染色匹的仓库，
    // 对话框按仓库名称精确选到该仓，出库四维（库存行+匹）全部落在该仓，可精确归因。
    const target = await pickDyeableWarehouse(page);
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '100',
      pieceCount: 1,
      context: 'S4-04',
    });
    const piece = bundle.pieces[0];

    const { id, order_no: orderNo } = await seedApprovedOrder(page);
    const dialog = await openDeliveryDialog(page, orderNo);
    // 仓库：按本例目标仓名称选定（option label = warehouse_name）
    await pickSelectIn(dialog, page, '仓库', { optionText: target.name });
    // 发货日期
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    // 库存行（对话框第 2 个 el-select）：以本例唯一缸号锚定，候选文本含「缸号: {dyeLotNo}」
    await pickSelect(page, dialog.locator('.el-select').nth(1), bundle.dyeLotNo);
    // 第四维（匹号）发货前显式判红：候选来自与 UI 同口径的下推查询，
    // 命中不到本例匹 = 真实链路/UI 候选断了，直接抛中文原因，禁止 skip/放宽。
    const candidates = await fetchAvailableDyedPieces(page, {
      productId,
      warehouseId: target.id,
      dyeLotNo: bundle.dyeLotNo,
    });
    if (!candidates.some(p => p.piece_no === piece.piece_no)) {
      throw new Error(
        `[04-04] 发货对话框匹候选为空/未含本例匹 ${piece.piece_no}（按 产品+仓+缸+批+AVAILABLE ` +
          `回读命中 ${candidates.length} 匹）——UI 第四维候选链路真实断裂，判红不 skip`
      );
    }
    const pieceSel = dialog.locator('.el-select').nth(2);
    await pickSelect(page, pieceSel, piece.piece_no);
    // 读 el-select 已选值的正确选择器（不能用 getByLabel().toHaveValue）
    await expect(
      pieceSel.locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)'),
      `匹号第四维应选中本例真实匹 ${piece.piece_no}`
    ).toContainText(piece.piece_no);
    // 本次发货数量
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // 真实出库端点成功 → msg.success('shipSuccess') = '发货成功'
    await expect(page.getByText('发货成功')).toBeVisible({ timeout: 30000 });
    // 后端真实流转：成功出库后订单应离开纯 approved（部分/整单发货态），
    // 只断言"状态已流转"这一确定事实，不写死具体 token（partial/shipped 词表随版本可能调整）
    const after = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(after.status, `发货后状态应离开 approved（实际 ${after.status}，id=${id}）`).not.toBe(
      'approved'
    );
    // 匹消耗回读（写后必回读，不靠 toast）：四维 tuple 回读该匹必须 AVAILABLE→SHIPPED
    const consumed = await readDyedPieceByNo(page, {
      productId,
      warehouseId: target.id,
      dyeLotNo: bundle.dyeLotNo,
      batchNo: bundle.dyeLotNo,
      pieceNo: piece.piece_no,
    });
    expect(consumed, `发货后应能按四维 tuple 回读到匹 ${piece.piece_no}`).toBeTruthy();
    expect(
      String(consumed!.status),
      `发货后匹状态应流转为 SHIPPED（词表 models/status/purchase_inventory.rs::inventory_piece，大写），实际 ${consumed!.status}`
    ).toBe('SHIPPED');
  });

  test('04-05 染色布缺匹号发货：UI 拦截 + API 400 外显真实原因且被拒无痕', async ({ page }) => {
    test.setTimeout(240_000);
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    if (!productId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    const target = await pickDyeableWarehouse(page);
    // 四维库存行 + 真实 AVAILABLE 染色匹都齐——拒绝只能源于"没带匹号"这一缺维本身，
    // 而非缺库存/缺匹的旁路原因（否则负例测的是别的东西）。
    const bundle = await seedDyedOutboundBundle(page, {
      productId,
      warehouseId: target.id,
      quantityMeters: '100',
      pieceCount: 1,
      context: 'S4-05',
    });
    const piece = bundle.pieces[0];
    const { id, order_no: orderNo } = await seedApprovedOrder(page);

    // ── UI 层：前三维+数量齐填、唯独不选匹号 → 前端拦下并给第四维提示（不发请求）──
    const dialog = await openDeliveryDialog(page, orderNo);
    await pickSelectIn(dialog, page, '仓库', { optionText: target.name });
    await dialog.getByPlaceholder('选择日期').fill('2026-12-31');
    await page.keyboard.press('Enter');
    await pickSelect(page, dialog.locator('.el-select').nth(1), bundle.dyeLotNo);
    await dialog.getByRole('spinbutton').first().fill('1');
    await dialog.getByRole('button', { name: '确定发货' }).click();
    // locales sales.delivery.pieceNoRequired 原文
    await expect(
      page.getByText('染色布发货必须选定匹号（缸号/色号/批次/匹号 四维齐才可出库）')
    ).toBeVisible();
    await expect(page.locator('.el-message--success')).toHaveCount(0);
    await dialog.getByRole('button', { name: '取消' }).click();

    // ── API 层：绕开 UI 直连 POST /ship，带染色布三维（色号/缸号/批次）省略 piece_no。
    //    后端 inventory_deduction.rs:242 require_outbound_dimensions → fabric_class.rs:81
    //    normalize_outbound_piece_no：缺维属字段必填族 VALIDATION，且 fix(outbound) 已改回
    //    可外显真实原因（validation_displayable），出参必须点名「匹号」、≠脱敏常量。──
    const rejected = await apiCallExpectFail(page, 'POST', `/sales/orders/${id}/ship`, {
      order_id: id,
      warehouse_code: target.code,
      items: [
        {
          product_id: productId,
          quantity: '1',
          color_no: bundle.colorNo,
          dye_lot_no: bundle.dyeLotNo,
          batch_no: bundle.dyeLotNo,
        },
      ],
    });
    expect(
      rejected.status,
      `染色布缺匹号发货应 400，实际 status=${rejected.status} code=${rejected.code ?? ''}`
    ).toBe(400);
    expect(
      failureCode(rejected),
      `缺维机器码应为 VALIDATION_ERROR（字段必填族），实际 code=${rejected.code ?? ''} message=${rejected.message ?? ''}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const rejectMsg = String(rejected.message ?? '');
    // 外显钉：≠脱敏常量（fix(outbound) 前的旧行为），且必须点名缺失的第四维「匹号」
    expect(rejectMsg, `拒绝文案不得是脱敏常量，实际=${rejectMsg}`).not.toBe('请求参数验证失败');
    expect(rejectMsg, `拒绝文案应含「匹号」，实际=${rejectMsg}`).toContain('匹号');

    // ── 被拒的写必须无痕：订单仍 approved、库存可用量未动、匹仍 AVAILABLE ──
    const still = await apiCallRaw<SalesOrderLite>(page, 'GET', `/sales/orders/${id}`);
    expect(still.status, `被拒发货不得流转订单状态，实际=${still.status}`).toBe('approved');
    const untouchedPiece = await readDyedPieceByNo(page, {
      productId,
      warehouseId: target.id,
      dyeLotNo: bundle.dyeLotNo,
      batchNo: bundle.dyeLotNo,
      pieceNo: piece.piece_no,
    });
    expect(untouchedPiece, '被拒发货不得消耗匹').toBeTruthy();
    expect(
      String(untouchedPiece!.status),
      '被拒后匹状态应仍为 AVAILABLE（CAS 未命中即整单拒绝回滚）'
    ).toBe('AVAILABLE');
    const stockAfter = await verifyStockFourDim(page, productId, bundle.colorNo, bundle.dyeLotNo, {
      batchNo: bundle.dyeLotNo,
      warehouseId: target.id,
    });
    expect(stockAfter, '四维库存行回读应存在').toBeTruthy();
    expect(
      Number(stockAfter!.quantity_available),
      `被拒发货后可用量应无痕（仍 100），实际 ${stockAfter!.quantity_available}`
    ).toBe(100);
  });
});
