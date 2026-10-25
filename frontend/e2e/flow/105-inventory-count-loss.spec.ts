// 库存盘点盘亏方向深度用例（与 13 号盘点用例互补，13 只断盘盈方向的量增与状态流转）
//
// 本文件只覆盖盘亏（实盘低于账面）这一从未被断言过的方向，并把三件事钉在用例里。
//   一、盘亏过账后的量减。写入方是 services 层的 inventory_count_service.rs，
//       apply_stock_adjustments 把 quantity_on_hand 与 quantity_available 两列同时
//       改写为实盘量，差额 quantity_difference 为实盘减账面（负值即盘亏）。
//       本用例断两列都精确减到实盘量、差额为负且绝对值等于预期损失量，
//       不接受只断"行数变化"或只断审批接口返回 200。
//   二、盘点状态词表逐字符等于写入方词表。唯一权威是 models 的 status 目录中
//       purchase_inventory.rs 的 inventory_count 模块，三值全为小写
//       pending、in_review、completed，词表里没有 in_progress。
//       建单落 pending，submit_for_approval 落 in_review，approve_count 落 completed，
//       reject_count 从 in_review 退回 pending。本用例对每一次流转的读回值用 toBe 精确比较，
//       不用 toLowerCase 归一，也不放行任何词表外的值。另用 status=in_progress 过滤列表，
//       证明库里不存在该取值的行（写入方从不写它）。
//   三、驳回不落账。reject_count 只改主表状态，库存调整只发生在 approve_count 内，
//       所以驳回后库存两列必须分文未动。这是"状态说已处理、库存一分没动"类账实分裂的
//       反向证明，也是盘亏唯一可回退的窗口。
//   四、非法值两族分界。缺必填键在提取器解码阶段被拒，由全局 catch_panic 响应层归一为
//       400 加 code VALIDATION_ERROR 的统一信封，不是 axum 默认的 422 纯文本；
//       负实盘属服务层非负校验拒绝，同码但走 handler 校验分支。两族都不得产生任何写入。
//
// 明细快照口径（写死在用例里，供失败时定位）。create_count 的
// create_count_items_txn 用 stock.quantity_on_hand 作为 quantity_before 的快照源，
// 不是 quantity_available。实盘初始化为 0，差异初始化为 0。
// 因此本用例先读回账面行自己的 quantity_on_hand 作为期望基线，不拿可用量当账面量比对。
//
// 造数纪律。每支用例用 seedFourDimStockIn 建一行专属四维库存（色号、缸号、批次每轮唯一），
// 建盘点单时用后端真实支持的 stock_ids 把明细限定在这一行，
// 避免共享仓库里其它库存行进明细导致统计字段与断言基线不可控。
// 全部打真实后端，不用 page.route 一类造响应的手段。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  getCtx,
  genCode,
  seedFourDimStockIn,
  verifyStockFourDim,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';
import { pickListArray } from './ui-helpers';

/** 读回响应里的一个键并转成数字，键缺席或非有限值直接抛错，不用 ?? 0 掩盖缺键。 */
function readNumber(obj: Record<string, unknown>, key: string, label: string): number {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label} 响应缺少后端真实键 ${key}，实际键=${Object.keys(obj).join(',')}`);
  }
  const value = Number(obj[key]);
  if (!Number.isFinite(value)) {
    throw new Error(`${label} 键 ${key} 不可解析为有限数字，raw=${String(obj[key])}`);
  }
  return value;
}

/** 金额与数量按货币精度比较（容差 0.01），失败信息带期望与实际。 */
function expectQuantity(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}，期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

/** 盘点明细读回，按 stock_id 定位本用例专属行，定位不到即抛错而不是拿第一条蒙过。 */
function pickCountItem(
  detail: Record<string, unknown>,
  stockId: number,
  label: string
): Record<string, unknown> {
  const items = pickListArray<Record<string, unknown>>(detail, 'items', label);
  const row = items.find(it => Number(it.stock_id) === stockId);
  if (!row) {
    throw new Error(
      `${label} 明细里没有库存行 ${stockId}，实际明细 stock_id=${items
        .map(it => String(it.stock_id))
        .join(',')}`
    );
  }
  return row;
}

/** 读回盘点详情（出参为 CountResponse，主表列与 items 明细同层）。 */
async function readCountDetail(
  page: import('@playwright/test').Page,
  countId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/inventory/counts/${countId}`);
}

/**
 * 建一行专属四维库存，返回该行读回结果（含 id、warehouse_id 与账面量）。
 *
 * 不在建好当场删除。同一行库存既是盘点建单 fetch_stocks_for_count_txn 按 stock_ids 过滤的
 * 唯一明细来源，又是过账前后每次读回落账值的比对对象，删了就没有前置可用。清理交给色号、
 * 缸号、批次三者唯一取值隔离（与同车道其它库存用例一致），不落库删除动作。
 */
async function seedLossStock(
  page: import('@playwright/test').Page,
  productId: number,
  warehouseId: number,
  tag: string,
  quantityMeters: string
): Promise<Record<string, unknown>> {
  const row = await seedFourDimStockIn(page, {
    productId,
    warehouseId,
    colorNo: `E2E-LOSS-C${tag}`,
    dyeLotNo: `E2E-LOSS-D${tag}`,
    batchNo: `E2E-LOSS-B${tag}`,
    quantityMeters,
  });
  return row;
}

/** 按四维读回同一行库存（盘亏前后的比对都走这一步，不缓存旧对象）。 */
async function rereadStock(
  page: import('@playwright/test').Page,
  productId: number,
  warehouseId: number,
  tag: string
): Promise<Record<string, unknown>> {
  const row = await verifyStockFourDim(page, productId, `E2E-LOSS-C${tag}`, `E2E-LOSS-D${tag}`, {
    batchNo: `E2E-LOSS-B${tag}`,
    warehouseId,
  });
  if (!row) {
    throw new Error(`四维库存行读回为空，product=${productId} warehouse=${warehouseId} tag=${tag}`);
  }
  return row;
}

/** 建盘点单并把明细限定在专属库存行，返回盘点单 id。 */
async function createCountForStock(
  page: import('@playwright/test').Page,
  warehouseId: number,
  stockId: number,
  notes: string
): Promise<number> {
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/inventory/counts', {
    warehouse_id: warehouseId,
    count_date: new Date().toISOString(),
    notes,
    stock_ids: [stockId],
  });
  const countId = readNumber(created, 'id', '盘点建单');
  // 建单出参就是 CountResponse，明细快照在同一响应里，先钉住"只盘这一行"
  const items = pickListArray<Record<string, unknown>>(created, 'items', '盘点建单明细');
  expect(items.length, `建单明细应只有专属库存行 ${stockId} 一条，实际 ${items.length} 条`).toBe(1);
  return countId;
}

test.describe('库存盘点盘亏方向', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('盘亏过账，可用量与在库量按损失量精确减少，差额为负且状态逐字符落词表', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();

    const tag = genCode('LOSS');
    const stock = await seedLossStock(page, productId!, warehouseId!, tag, '100');
    const stockId = readNumber(stock, 'id', '四维库存行');
    const bookOnHand = readNumber(stock, 'quantity_on_hand', '账面在库量');
    const bookAvailable = readNumber(stock, 'quantity_available', '账面可用量');
    const lossQty = 30;
    const actualQty = bookOnHand - lossQty;

    // 建单快照口径，quantity_before 取账面在库量而非可用量
    const countId = await createCountForStock(page, warehouseId!, stockId, `E2E 盘亏 ${tag}`);
    const detailCreated = await readCountDetail(page, countId);
    expect(
      String(detailCreated.status),
      `建单后状态应逐字符等于写入方词表值 pending，实际 ${String(detailCreated.status)}`
    ).toBe('pending');
    expect(
      readNumber(detailCreated, 'total_items', '建单总项数'),
      '明细限定在专属行，总项数应为 1'
    ).toBe(1);
    expect(
      readNumber(detailCreated, 'counted_items', '建单已盘项数'),
      '未录入实盘时已盘项数应为 0'
    ).toBe(0);
    const itemCreated = pickCountItem(detailCreated, stockId, '建单后盘点详情');
    expectQuantity(
      readNumber(itemCreated, 'quantity_before', '明细账面量'),
      bookOnHand,
      '明细 quantity_before 应等于账面在库量（写入方取 stock.quantity_on_hand）'
    );

    // 录入盘亏实盘量
    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [{ stock_id: stockId, quantity_actual: String(actualQty), notes: `E2E 盘亏 ${tag}` }],
    });
    const detailRecorded = await readCountDetail(page, countId);
    const itemRecorded = pickCountItem(detailRecorded, stockId, '录入后盘点详情');
    expectQuantity(
      readNumber(itemRecorded, 'quantity_actual', '实盘量'),
      actualQty,
      '实盘量应等于提交值'
    );
    const difference = readNumber(itemRecorded, 'quantity_difference', '盘点差异');
    expect(
      difference < 0,
      `盘亏方向的 quantity_difference 必须为负（实盘减账面），实际 ${difference}`
    ).toBe(true);
    expectQuantity(difference, -lossQty, `盘亏差异应精确等于负的损失量 ${-lossQty}`);
    expect(
      readNumber(detailRecorded, 'counted_items', '已盘项数'),
      '录入一条实盘后已盘项数应为 1'
    ).toBe(1);
    expect(
      readNumber(detailRecorded, 'variance_items', '差异项数'),
      '存在盘亏差异，差异项数应为 1'
    ).toBe(1);

    // 提交审批，状态逐字符落 in_review
    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    const detailSubmitted = await readCountDetail(page, countId);
    expect(
      String(detailSubmitted.status),
      `提交后状态应逐字符等于写入方词表值 in_review，实际 ${String(detailSubmitted.status)}`
    ).toBe('in_review');
    // 未过账前库存分文未动，落账只发生在 approve
    const stockBeforeApprove = await rereadStock(page, productId!, warehouseId!, tag);
    expectQuantity(
      readNumber(stockBeforeApprove, 'quantity_available', '审批前可用量'),
      bookAvailable,
      '提交审批（未过账）不应改变库存可用量'
    );

    // 审批过账
    await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`);
    const detailApproved = await readCountDetail(page, countId);
    expect(
      String(detailApproved.status),
      `审批后状态应逐字符等于写入方词表值 completed，实际 ${String(detailApproved.status)}`
    ).toBe('completed');

    // 过账真值，两列都减到实盘量，且减幅恰为损失量
    const stockAfter = await rereadStock(page, productId!, warehouseId!, tag);
    const availableAfter = readNumber(stockAfter, 'quantity_available', '过账后可用量');
    const onHandAfter = readNumber(stockAfter, 'quantity_on_hand', '过账后在库量');
    expectQuantity(availableAfter, actualQty, '盘亏过账后可用量应精确减到实盘量');
    expectQuantity(onHandAfter, actualQty, '盘亏过账后在库量应精确减到实盘量');
    expectQuantity(
      bookAvailable - availableAfter,
      lossQty,
      `可用量减少量应恰为盘亏损失量 ${lossQty}`
    );
    expect(
      availableAfter > bookAvailable,
      `盘亏不得把可用量抬高（前 ${bookAvailable} 后 ${availableAfter}）`
    ).toBe(false);

    // 同一单号在库里恒一行，重复提交不静默产生第二行
    const countNo = String(detailApproved.count_no);
    const listByNo = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/inventory/counts?count_no=${encodeURIComponent(countNo)}&page=1&page_size=10`
    );
    const rowsByNo = pickListArray<Record<string, unknown>>(
      listByNo,
      'counts',
      '按单号过滤盘点列表'
    );
    expect(
      rowsByNo.filter(r => String(r.count_no) === countNo).length,
      `同一盘点单号 ${countNo} 在库里应只有一行`
    ).toBe(1);
  });

  test('盘亏驳回不落账，状态从 in_review 退回 pending，重新走完后才落账', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();

    const tag = genCode('LSRJ');
    const stock = await seedLossStock(page, productId!, warehouseId!, tag, '80');
    const stockId = readNumber(stock, 'id', '四维库存行');
    const bookOnHand = readNumber(stock, 'quantity_on_hand', '账面在库量');
    const bookAvailable = readNumber(stock, 'quantity_available', '账面可用量');

    const countId = await createCountForStock(page, warehouseId!, stockId, `E2E 盘亏驳回 ${tag}`);
    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [{ stock_id: stockId, quantity_actual: '20' }],
    });
    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    const submitted = await readCountDetail(page, countId);
    expect(String(submitted.status), '驳回前状态应为 in_review').toBe('in_review');

    // 驳回，主表退回 pending，库存两列不得动
    await apiCall(page, 'POST', `/inventory/counts/${countId}/reject`);
    const rejected = await readCountDetail(page, countId);
    expect(
      String(rejected.status),
      `驳回后状态应逐字符退回词表值 pending，实际 ${String(rejected.status)}`
    ).toBe('pending');
    const stockAfterReject = await rereadStock(page, productId!, warehouseId!, tag);
    expectQuantity(
      readNumber(stockAfterReject, 'quantity_available', '驳回后可用量'),
      bookAvailable,
      '驳回不过账，可用量必须保持账面值'
    );
    expectQuantity(
      readNumber(stockAfterReject, 'quantity_on_hand', '驳回后在库量'),
      bookOnHand,
      '驳回不过账，在库量必须保持账面值'
    );
    // 驳回后完成时间与审批人三列必须仍为空（只在 approve 写）
    expect(
      rejected.completed_at === null || rejected.completed_at === undefined,
      `驳回后 completed_at 不应有值，实际 ${String(rejected.completed_at)}`
    ).toBe(true);

    // 驳回可重新录入并走完，落账只发生在第二次 approve
    await apiCall(page, 'POST', `/inventory/counts/${countId}/record`, {
      items: [{ stock_id: stockId, quantity_actual: '60' }],
    });
    const reRecorded = pickCountItem(await readCountDetail(page, countId), stockId, '重录后详情');
    expectQuantity(
      readNumber(reRecorded, 'quantity_difference', '重录差异'),
      -(bookOnHand - 60),
      `重新录入后差异应按账面 ${bookOnHand} 与实盘 60 重算为负`
    );
    await apiCall(page, 'POST', `/inventory/counts/${countId}/submit`);
    await apiCall(page, 'POST', `/inventory/counts/${countId}/approve`);
    const stockFinal = await rereadStock(page, productId!, warehouseId!, tag);
    expectQuantity(
      readNumber(stockFinal, 'quantity_available', '最终可用量'),
      60,
      '第二次审批后才落账，可用量应减到 60'
    );
    expectQuantity(
      readNumber(stockFinal, 'quantity_on_hand', '最终在库量'),
      60,
      '第二次审批后才落账，在库量应减到 60'
    );
  });

  test('盘亏边界与非法值，实盘为零合法过账，重复录入覆盖而不累加统计，缺键与负值均被拒且无副作用', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();

    // 全损边界，实盘 0 允许（写入方只拒严格负数），差异为负满额
    const tagZero = genCode('LSZ0');
    const stockZero = await seedLossStock(page, productId!, warehouseId!, tagZero, '50');
    const stockZeroId = readNumber(stockZero, 'id', '全损四维库存行');
    const zeroCountId = await createCountForStock(
      page,
      warehouseId!,
      stockZeroId,
      `E2E 盘亏全损 ${tagZero}`
    );
    await apiCall(page, 'POST', `/inventory/counts/${zeroCountId}/record`, {
      items: [{ stock_id: stockZeroId, quantity_actual: '0' }],
    });
    const zeroItem = pickCountItem(
      await readCountDetail(page, zeroCountId),
      stockZeroId,
      '全损录入后详情'
    );
    expectQuantity(readNumber(zeroItem, 'quantity_actual', '全损实盘量'), 0, '实盘零应被接受');
    expectQuantity(
      readNumber(zeroItem, 'quantity_difference', '全损差异'),
      -50,
      '全损差异应为负满额 -50'
    );
    await apiCall(page, 'POST', `/inventory/counts/${zeroCountId}/submit`);
    await apiCall(page, 'POST', `/inventory/counts/${zeroCountId}/approve`);
    const stockZeroAfter = await rereadStock(page, productId!, warehouseId!, tagZero);
    expectQuantity(
      readNumber(stockZeroAfter, 'quantity_available', '全损过账后可用量'),
      0,
      '全损过账后可用量应为 0'
    );

    // 重复录入同一行，实盘量与差异按最后一次覆盖，统计字段不得累加成 2
    const tagTwice = genCode('LS2X');
    const stockTwice = await seedLossStock(page, productId!, warehouseId!, tagTwice, '90');
    const stockTwiceId = readNumber(stockTwice, 'id', '重复录入四维库存行');
    const twiceCountId = await createCountForStock(
      page,
      warehouseId!,
      stockTwiceId,
      `E2E 盘亏重复录入 ${tagTwice}`
    );
    await apiCall(page, 'POST', `/inventory/counts/${twiceCountId}/record`, {
      items: [{ stock_id: stockTwiceId, quantity_actual: '40' }],
    });
    await apiCall(page, 'POST', `/inventory/counts/${twiceCountId}/record`, {
      items: [{ stock_id: stockTwiceId, quantity_actual: '70' }],
    });
    const twiceDetail = await readCountDetail(page, twiceCountId);
    const twiceItem = pickCountItem(twiceDetail, stockTwiceId, '二次录入后详情');
    expectQuantity(
      readNumber(twiceItem, 'quantity_actual', '末次实盘量'),
      70,
      '重复录入应按末次值覆盖'
    );
    expectQuantity(
      readNumber(twiceItem, 'quantity_difference', '末次差异'),
      -20,
      '重复录入的差异应按末次实盘重算'
    );
    expect(
      readNumber(twiceDetail, 'counted_items', '重复录入后已盘项数'),
      '重复录入同一行不得把已盘项数累加'
    ).toBe(1);
    expect(
      readNumber(twiceDetail, 'variance_items', '重复录入后差异项数'),
      '重复录入同一行不得把差异项数累加'
    ).toBe(1);

    // 非法值，缺 quantity_actual 键的请求必须被拒（400 参数族），且对被拒的单不产生任何写入
    const missingKey = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/counts/${twiceCountId}/record`,
      { items: [{ stock_id: stockTwiceId }] }
    );
    const missingKeyCode = failureCode(missingKey);
    expect(
      missingKey.status,
      `缺 quantity_actual 键应恰为 400，实际 status=${missingKey.status} code=${missingKeyCode}`
    ).toBe(400);
    expect(missingKeyCode, `缺必填键属请求解码拒绝族，实际 code=${missingKeyCode}`).toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(
      typeof missingKey.message === 'string' && missingKey.message.trim().length > 0,
      `400 失败体应含非空 message，实际 message=${String(missingKey.message)}`
    ).toBe(true);
    // 非法值二：负数由服务层非负校验拒绝，走业务信封同码但不同路径
    const negativeQty = await apiCallExpectFail(
      page,
      'POST',
      `/inventory/counts/${twiceCountId}/record`,
      { items: [{ stock_id: stockTwiceId, quantity_actual: '-1' }] }
    );
    expect(
      negativeQty.status,
      `实盘负数应被拒且恰为 400，实际 status=${negativeQty.status} code=${failureCode(negativeQty)}`
    ).toBe(400);
    expect(
      failureCode(negativeQty),
      `负实盘属校验拒绝族，实际 code=${failureCode(negativeQty)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      typeof negativeQty.message === 'string' && negativeQty.message.trim().length > 0,
      `负值拒绝应带非空可读文案，实际 message=${String(negativeQty.message)}`
    ).toBe(true);
    // 被拒的录入不得改动已落库的末次实盘量与统计
    const detailAfterIllegal = await readCountDetail(page, twiceCountId);
    const itemAfterIllegal = pickCountItem(detailAfterIllegal, stockTwiceId, '非法请求后详情');
    expectQuantity(
      readNumber(itemAfterIllegal, 'quantity_actual', '非法请求后的实盘量'),
      70,
      '被拒的录入请求不得改写实盘量'
    );
    expect(String(detailAfterIllegal.status), '被拒的录入请求不得改写主表状态').toBe('pending');
  });

  test('词表外取值不入库，按 in_progress 过滤盘点列表恒零行', async ({ page }) => {
    // 写入方词表只有 pending、in_review、completed 三值，没有 in_progress。
    // 列表筛选把该值原样下推到 status 等值条件，命中 0 行即为"库里不存在该取值"的证据。
    const listByIllegalToken = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/inventory/counts?status=in_progress&page=1&page_size=10'
    );
    const rows = pickListArray<Record<string, unknown>>(
      listByIllegalToken,
      'counts',
      '按 in_progress 过滤盘点列表'
    );
    expect(rows.length, 'in_progress 不是 inventory_count 词表取值，命中行数应为 0').toBe(0);
    expect(
      readNumber(listByIllegalToken, 'total', '按 in_progress 过滤总数'),
      'in_progress 过滤的 total 应为 0'
    ).toBe(0);

    // 同一请求的分页信封四键必须齐（本端点出参是 CountListResponse，数组键为 counts，
    // 连同 total、page、page_size 四键，与通用 PaginatedResponse 的 items 键不同名）
    for (const key of ['counts', 'total', 'page', 'page_size']) {
      expect(
        Object.prototype.hasOwnProperty.call(listByIllegalToken, key),
        `盘点列表响应应含分页键 ${key}，实际键=${Object.keys(listByIllegalToken).join(',')}`
      ).toBe(true);
    }
    expect(readNumber(listByIllegalToken, 'page', '分页页码'), '回显页码应为请求提交的 1').toBe(1);
    expect(
      readNumber(listByIllegalToken, 'page_size', '分页每页量'),
      '回显每页量应为请求提交的 10'
    ).toBe(10);
  });
});
