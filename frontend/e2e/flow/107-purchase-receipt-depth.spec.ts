// 采购收货到确认入库的深度链用例（真实建单、质检分流、让步改判、四维库存读回、终态门控）
//
// 本文件把 收货 到 入库 的整条链按后端真实实现逐段钉死，覆盖四件事。
//   一、收货单真实建单与四维明细。建单写入方在 services 层 purchase_receipt_ops 的构建函数，
//       初始 receipt_status 落大写 DRAFT、inspection_status 落大写 PENDING，单号由
//       DocumentNumberGenerator 以 PR 前缀生成。明细四维（批次、色号、缸号、等级）按行原样
//       落库并可读回，主表 total_quantity 与 total_quantity_alt 为按行求和。
//   二、质检结论对入库的真实门控口径。权威词表在 models 下 status 目录的 purchase_inventory.rs。
//       质检行自身状态为小写 pending、completed，结论为小写 pass、fail、partial。回写收货单
//       的映射为 pass 到 PASSED，fail 与 partial 都到 REJECTED，是整单级阻断，不存在任何
//       按数量拆分入库的旁路。不合格数量 reject_quantity 只落在质检行自身并参与质量得分
//       计算（合格量除以合格量加不合格量再乘一百），既不扣减入库量，也不自动生成退货。
//       确认入库时库存按明细行 quantity 全额累加，本用例按实现真实行为钉住该口径，即
//       partial 结论经改判放行后，入库总量等于收货全量而非合格量。
//   三、状态流转词表逐字符比对。收货主状态词表为大写 DRAFT、COMPLETED（状态机语义为 DRAFT
//       直接流转到 COMPLETED，confirm 即终态，没有中间写入点产生 CONFIRMED），检验状态词表为
//       大写 PENDING、PASSED、REJECTED、CONCESSION_ACCEPTED。库存行 stock_status 为中文 正常，
//       quality_status 为中文 合格，grade 为中文 一等品、二等品、等外品。全部读回值用 toBe
//       逐字符比较，不做 toLowerCase 归一，不放行词表外取值。另以 in_progress 作为 status
//       过滤值查询收货列表，证明库里不存在该取值的行。
//   四、门控链与留痕。确认端点按顺序过 存在性、DRAFT 门、明细数、供应商黑名单、质检门。
//       质检门的三族文案属可外显族（HTTP 400 加 BUSINESS_ERROR，message 原样回显），用例断
//       文案片段。DRAFT 门与终态的修改删除门属脱敏族，只钉状态码、机器码与文案非空，
//       这是设计而非缺陷。让步接收的合法前驱为 PENDING 或 REJECTED，复检改判的唯一合法前驱
//       为 CONCESSION_ACCEPTED。词表外改判结论与空白理由属校验族 VALIDATION_ERROR 且可外显。
//       让步与改判的理由、操作人、时间落专用列，改判理由按 trim 后落库，rejudge_count 累加。
//
// 造数纪律。每支用例用 genCode 生成唯一 tag，色号、缸号、批次三个维度全部带 tag 隔离，
// 夹具不删除，靠唯一取值避免共享库干扰（与同车道 105 号盘点用例一致）。全部打真实后端，
// 不用 page.route 一类造响应的手段，不硬编码任何实体 ID。
import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  getCtx,
  genCode,
  verifyStockFourDim,
  failureCode,
  APP_ERROR_CODES,
} from './helpers';
import type { ApiFailureResult } from './helpers';
import { pickListArray } from './ui-helpers';

/** 读回响应里的一个键并转成数字，键缺席或非有限值直接抛错，不以默认值掩盖缺键。 */
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

/** 数量按业务精度比较（容差 0.01），失败信息带期望与实际。 */
function expectQuantity(actual: number, expected: number, label: string): void {
  expect(Math.abs(actual - expected), `${label}，期望=${expected} 实际=${actual}`).toBeLessThan(
    0.01
  );
}

/** 把响应值收窄为字典，缺席、非对象或数组直接抛错，不用强转掩盖形状异常。 */
function asRecord(value: unknown, label: string): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) {
    throw new Error(`${label} 应为对象，实际=${JSON.stringify(value)?.slice(0, 200)}`);
  }
  return value as Record<string, unknown>;
}

/** 时间戳列断言，落库必须有值且为可读字符串，失败信息带实际值。 */
function expectTimestamp(value: unknown, label: string): void {
  expect(
    typeof value === 'string' && value.trim().length > 0,
    `${label} 应为非空时间字符串，实际=${JSON.stringify(value)}`
  ).toBe(true);
}

/**
 * 断言失败体属可外显拒绝族，即 HTTP 恰为 400、机器码精确、文案含指定片段。
 * 用于质检门、让步门、改判门与校验族这些 message 原样回显的分支。
 */
function expectDisplayableRejection(
  result: ApiFailureResult,
  code: string,
  needle: string,
  label: string
): void {
  expect(
    result.status,
    `${label}：应恰为 HTTP 400，实际 status=${result.status} code=${failureCode(
      result
    )} message=${result.message}`
  ).toBe(400);
  expect(
    failureCode(result),
    `${label}：机器码应为 ${code}，实际 code=${JSON.stringify(
      result.code
    )} message=${result.message}`
  ).toBe(code);
  expect(
    typeof result.message === 'string' && result.message.includes(needle),
    `${label}：文案应含片段 ${needle}，实际 message=${JSON.stringify(result.message)}`
  ).toBe(true);
}

/**
 * 断言失败体属脱敏拒绝族，只钉 HTTP 400、机器码与文案非空，绝不比对文案内容。
 * DRAFT 门与终态修改删除门由 AppError 的脱敏构造函数产出，message 会被归一，
 * 比对文案是假绿反模式，这里只证明 被拒且不是裸 5xx、不是 404、不是权限门。
 */
function expectMaskedRejection(result: ApiFailureResult, code: string, label: string): void {
  expect(
    result.status,
    `${label}：应恰为 HTTP 400（脱敏业务拒绝的正确契约），实际 status=${
      result.status
    } code=${failureCode(result)}`
  ).toBe(400);
  expect(
    failureCode(result),
    `${label}：机器码应为 ${code}，实际 code=${JSON.stringify(result.code)}`
  ).toBe(code);
  expect(
    typeof result.message === 'string' && result.message.trim().length > 0,
    `${label}：失败体应含非空 message，实际 message=${JSON.stringify(result.message)}`
  ).toBe(true);
}

/** 收货单详情读回（出参 PurchaseReceiptDto，主表列与富化列同层）。 */
async function readReceipt(
  page: import('@playwright/test').Page,
  receiptId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/receipts/${receiptId}`);
}

/** 质检单详情读回（出参为 purchase_inspection 行自身，状态与结论是小写词表）。 */
async function readInspection(
  page: import('@playwright/test').Page,
  inspectionId: number
): Promise<Record<string, unknown>> {
  return apiCallRaw<Record<string, unknown>>(page, 'GET', `/purchase/inspections/${inspectionId}`);
}

/** 收货明细读回并按行号升序整理，端点出参是数组本体，非对象包数组直接抛错。 */
async function readReceiptItems(
  page: import('@playwright/test').Page,
  receiptId: number
): Promise<Array<Record<string, unknown>>> {
  const data = await apiCallRaw<unknown>(page, 'GET', `/purchase/receipts/${receiptId}/items`);
  if (!Array.isArray(data)) {
    throw new Error(
      `收货单 ${receiptId} 明细读回应为数组，实际=${JSON.stringify(data)?.slice(0, 200)}`
    );
  }
  return [...(data as Array<Record<string, unknown>>)].sort(
    (a, b) => readNumber(a, 'line_no', '明细行号') - readNumber(b, 'line_no', '明细行号')
  );
}

/** 按四维读回库存行集合（同一批次下可能按等级分桶成多行，这里不做取首条收敛）。 */
async function readStockRowsByDims(
  page: import('@playwright/test').Page,
  productId: number,
  warehouseId: number,
  dims: { colorNo: string; dyeLotNo: string; batchNo: string }
): Promise<Array<Record<string, unknown>>> {
  const path =
    `/inventory/stock?product_id=${productId}&warehouse_id=${warehouseId}` +
    `&color_no=${encodeURIComponent(dims.colorNo)}&dye_lot_no=${encodeURIComponent(
      dims.dyeLotNo
    )}&batch_no=${encodeURIComponent(dims.batchNo)}&page=1&page_size=50`;
  const res = await apiCallRaw<Record<string, unknown>>(page, 'GET', path);
  const rows = pickListArray<Record<string, unknown>>(res, 'items', `四维库存读回 ${path}`);
  return rows.filter(
    r =>
      String(r.batch_no) === dims.batchNo &&
      String(r.color_no) === dims.colorNo &&
      String(r.dye_lot_no) === dims.dyeLotNo
  );
}

/** 收货明细行的四维与数量载荷（用例侧造数描述，字段名对齐后端建单请求键）。 */
interface ReceiptSeedItem {
  lineNo: number;
  qty: string;
  alt: string;
  grade: string;
  colorNo: string;
  dyeLotNo: string;
  batchNo: string;
}

/**
 * API 建一张本用例专属的 DRAFT 收货单（不挂采购订单，order_id 显式传 null，
 * 走无来源单的直收链），明细逐行携带四维。物料编码、名称、主单位取自真实产品行，
 * 不手写假码。返回收货单 id。
 */
async function seedDraftReceipt(
  page: import('@playwright/test').Page,
  opts: {
    productId: number;
    warehouseId: number;
    supplierId: number;
    materialCode: string;
    materialName: string;
    unitMaster: string;
    marker: string;
    items: ReceiptSeedItem[];
  }
): Promise<number> {
  const created = await apiCall<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
    order_id: null,
    supplier_id: opts.supplierId,
    receipt_date: new Date().toISOString().slice(0, 10),
    warehouse_id: opts.warehouseId,
    notes: `E2E-收货深度链-${opts.marker}`,
    items: opts.items.map(it => ({
      line_no: it.lineNo,
      material_id: opts.productId,
      material_code: opts.materialCode,
      material_name: opts.materialName,
      batch_no: it.batchNo,
      color_code: it.colorNo,
      lot_no: it.dyeLotNo,
      grade: it.grade,
      quantity: it.qty,
      quantity_alt: it.alt,
      unit_master: opts.unitMaster,
    })),
  });
  const id = readNumber(asRecord(created.data, '收货建单响应'), 'id', '收货建单');
  return id;
}

/** 建质检单并返回其 id（体三键与后端 CreatePurchaseInspectionRequest 对齐）。 */
async function createInspectionForReceipt(
  page: import('@playwright/test').Page,
  receiptId: number,
  supplierId: number
): Promise<number> {
  const created = await apiCall<Record<string, unknown>>(page, 'POST', '/purchase/inspections', {
    receipt_id: receiptId,
    supplier_id: supplierId,
    inspection_date: new Date().toISOString().slice(0, 10),
  });
  return readNumber(asRecord(created.data, '质检建单响应'), 'id', '质检单');
}

/** 录入质检结论（合格量、不合格量、结论三键与 CompleteInspectionRequest 对齐）。 */
async function completeInspection(
  page: import('@playwright/test').Page,
  inspectionId: number,
  payload: { passQuantity: string; rejectQuantity: string; result: string }
): Promise<void> {
  await apiCall(page, 'POST', `/purchase/inspections/${inspectionId}/complete`, {
    pass_quantity: payload.passQuantity,
    reject_quantity: payload.rejectQuantity,
    inspection_result: payload.result,
  });
}

/** 断言一行四维库存的落库真值（三列同值累加口径、公斤列、状态词表与版本）。 */
function expectStockRowTruth(
  row: Record<string, unknown>,
  expected: {
    meters: number;
    kg: number;
    grade: string;
    version: number;
    label: string;
  }
): void {
  const label = expected.label;
  expectQuantity(
    readNumber(row, 'quantity_on_hand', `${label} 在库量`),
    expected.meters,
    `${label} quantity_on_hand 应等于明细行数量全额`
  );
  expectQuantity(
    readNumber(row, 'quantity_available', `${label} 可用量`),
    expected.meters,
    `${label} quantity_available 与在库量同写同值（入库不产生预留）`
  );
  expectQuantity(
    readNumber(row, 'quantity_meters', `${label} 米数`),
    expected.meters,
    `${label} quantity_meters 与在库量同写同值`
  );
  expectQuantity(
    readNumber(row, 'quantity_kg', `${label} 公斤数`),
    expected.kg,
    `${label} quantity_kg 应等于明细行 quantity_alt 累加`
  );
  expect(
    String(row.grade),
    `${label} 等级应逐字符等于明细行 grade，词表为 一等品、二等品、等外品`
  ).toBe(expected.grade);
  expect(
    String(row.stock_status),
    `${label} 库存状态应逐字符等于写入方中文词表值 正常`
  ).toBe('正常');
  expect(
    String(row.quality_status),
    `${label} 质量状态应逐字符等于入库写入的中文词表值 合格（入库前提是质检 PASSED）`
  ).toBe('合格');
  expect(
    readNumber(row, 'version', `${label} 乐观锁版本`),
    `${label} version 应为 ${expected.version}（新建为 0，同维度二次累加走更新分支加一）`
  ).toBe(expected.version);
}

test.describe('采购收货到入库深度链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('收货建单、partial 分流经让步改判放行，确认入库四维读回全量（不合格数不扣减入库量）', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const supplierId = ctx.supplierId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(supplierId, '前置供应商缺失').toBeTruthy();

    const tag = genCode('PRD1');
    const dimA = {
      colorNo: `E2E-PRD1-AC${tag}`,
      dyeLotNo: `E2E-PRD1-AL${tag}`,
      batchNo: `E2E-PRD1-AB${tag}`,
    };
    const dimB = {
      colorNo: `E2E-PRD1-BC${tag}`,
      dyeLotNo: `E2E-PRD1-BL${tag}`,
      batchNo: `E2E-PRD1-BB${tag}`,
    };

    // 真实产品行取物料码与主单位，禁手写假码
    const prod = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
    const prodCode = String(prod.code);
    const prodName = String(prod.name);
    const prodUnit = String(prod.unit);
    expect(prodCode.trim().length > 0, `产品物料码读回应非空，实际 ${prodCode}`).toBe(true);
    expect(prodName.trim().length > 0, `产品名称读回应非空，实际 ${prodName}`).toBe(true);
    expect(prodUnit.trim().length > 0, `产品主单位读回应非空，实际 ${prodUnit}`).toBe(true);

    const receiptId = await seedDraftReceipt(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      supplierId: supplierId!,
      materialCode: prodCode,
      materialName: prodName,
      unitMaster: prodUnit,
      marker: tag,
      items: [
        { lineNo: 1, qty: '100', alt: '40', grade: '一等品', ...dimA },
        { lineNo: 2, qty: '60', alt: '20', grade: '一等品', ...dimB },
      ],
    });

    // 建单读回，主表逐字符落大写词表，PR 前缀由编号生成器决定
    const headDraft = await readReceipt(page, receiptId);
    expect(
      String(headDraft.receipt_no).startsWith('PR'),
      `单号应由 DocumentNumberGenerator 以 PR 前缀生成，实际 ${String(headDraft.receipt_no)}`
    ).toBe(true);
    expect(
      String(headDraft.receipt_status),
      `建单后 receipt_status 应逐字符等于词表值 DRAFT，实际 ${String(headDraft.receipt_status)}`
    ).toBe('DRAFT');
    expect(
      String(headDraft.inspection_status),
      `建单后 inspection_status 应逐字符等于词表值 PENDING，实际 ${String(
        headDraft.inspection_status
      )}`
    ).toBe('PENDING');
    expect(Number(headDraft.supplier_id), '供应商应为建单传入的会话前置供应商').toBe(supplierId!);
    expect(Number(headDraft.warehouse_id), '仓库应为建单传入的前置仓库').toBe(warehouseId!);
    expect(
      headDraft.order_id === null || headDraft.order_id === undefined,
      `无来源单直收链的 order_id 应为空，实际 ${String(headDraft.order_id)}`
    ).toBe(true);
    expectQuantity(
      readNumber(headDraft, 'total_quantity', '主表总数量'),
      160,
      'total_quantity 应为两行 quantity 求和'
    );
    expectQuantity(
      readNumber(headDraft, 'total_quantity_alt', '主表副单位总量'),
      60,
      'total_quantity_alt 应为两行 quantity_alt 求和'
    );
    expect(
      headDraft.confirmed_at === null || headDraft.confirmed_at === undefined,
      'DRAFT 阶段 confirmed_at 不应有值'
    ).toBe(true);
    expect(readNumber(headDraft, 'rejudge_count', '改判次数'), '建单时改判次数应为 0').toBe(0);

    // 明细四维如实读回（行出参里产品键名是 product_id，与建单入参 material_id 不同名）
    const items = await readReceiptItems(page, receiptId);
    expect(items.length, '明细应为两行').toBe(2);
    const item1 = items[0];
    const item2 = items[1];
    expect(Number(item1.line_no), '首行行号应为 1').toBe(1);
    expect(Number(item1.product_id), '明细产品应为前置产品').toBe(productId!);
    expect(String(item1.material_code), '明细物料码应等于真实产品码').toBe(prodCode);
    expect(String(item1.material_name), '明细物料名应等于真实产品名').toBe(prodName);
    expect(String(item1.batch_no), '首行批次应如实落库').toBe(dimA.batchNo);
    expect(String(item1.color_code), '首行色号应如实落库').toBe(dimA.colorNo);
    expect(String(item1.lot_no), '首行缸号应如实落库').toBe(dimA.dyeLotNo);
    expect(String(item1.grade), '首行等级应逐字符等于 一等品').toBe('一等品');
    expectQuantity(readNumber(item1, 'quantity', '首行数量'), 100, '首行 quantity 应为提交值');
    expectQuantity(readNumber(item1, 'quantity_alt', '首行副单位量'), 40, '首行 alt 应为提交值');
    expect(String(item2.batch_no), '次行批次应如实落库').toBe(dimB.batchNo);
    expectQuantity(readNumber(item2, 'quantity', '次行数量'), 60, '次行 quantity 应为提交值');

    // 入库前四维读回无行，证明库存行只由确认入库写入
    const stockABefore = await verifyStockFourDim(page, productId!, dimA.colorNo, dimA.dyeLotNo, {
      batchNo: dimA.batchNo,
      warehouseId: warehouseId!,
    });
    const stockBBefore = await verifyStockFourDim(page, productId!, dimB.colorNo, dimB.dyeLotNo, {
      batchNo: dimB.batchNo,
      warehouseId: warehouseId!,
    });
    expect(stockABefore, '确认入库前两维 A 不应存在库存行').toBeNull();
    expect(stockBBefore, '确认入库前两维 B 不应存在库存行').toBeNull();

    // 未检确认被拒，可外显族文案含 PENDING 门片段，且拒绝无任何副作用
    const failNoInspection = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/confirm`
    );
    expectDisplayableRejection(
      failNoInspection,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '质检尚未完成',
      '质检未完成的确认入库'
    );
    const headAfterFail1 = await readReceipt(page, receiptId);
    expect(String(headAfterFail1.receipt_status), '被拒的确认不得改写主状态').toBe('DRAFT');
    expect(String(headAfterFail1.inspection_status), '被拒的确认不得改写检验状态').toBe('PENDING');
    expect(
      await verifyStockFourDim(page, productId!, dimA.colorNo, dimA.dyeLotNo, {
        batchNo: dimA.batchNo,
        warehouseId: warehouseId!,
      }),
      '被拒的确认不得产生库存行'
    ).toBeNull();

    // 建质检单，行自身状态逐字符落小写 pending
    const inspectionId = await createInspectionForReceipt(page, receiptId, supplierId!);
    const inspPending = await readInspection(page, inspectionId);
    expect(
      String(inspPending.inspection_status),
      `质检行初始状态应逐字符等于小写词表值 pending，实际 ${String(
        inspPending.inspection_status
      )}`
    ).toBe('pending');
    expect(Number(inspPending.receipt_id), '质检行应挂在本次收货单').toBe(receiptId);

    // 录入 partial 结论（合格 120，不合格 40），小写词表读回与得分
    await completeInspection(page, inspectionId, {
      passQuantity: '120',
      rejectQuantity: '40',
      result: 'partial',
    });
    const inspDone = await readInspection(page, inspectionId);
    expect(String(inspDone.inspection_status), '完成后应逐字符落小写 completed').toBe('completed');
    expect(String(inspDone.inspection_result), '结论应逐字符落小写 partial').toBe('partial');
    expectQuantity(
      readNumber(inspDone, 'pass_quantity', '质检合格量'),
      120,
      'pass_quantity 应等于提交值'
    );
    expectQuantity(
      readNumber(inspDone, 'reject_quantity', '质检不合格量'),
      40,
      'reject_quantity 应等于提交值且只落质检行自身'
    );
    expectQuantity(
      readNumber(inspDone, 'quality_score', '质量得分'),
      75,
      'quality_score 应为 120 除以 160 乘一百即 75'
    );

    // partial 经同源映射回写收货单为整单级 REJECTED（阻断全部入库，不按量拆分）
    const headRejected = await readReceipt(page, receiptId);
    expect(
      String(headRejected.inspection_status),
      `partial 应逐字符回写为 REJECTED（词表映射 fail、partial 都到 REJECTED），实际 ${String(
        headRejected.inspection_status
      )}`
    ).toBe('REJECTED');
    expect(String(headRejected.receipt_status), 'REJECTED 不改主状态').toBe('DRAFT');

    // REJECTED 下确认被拒（可外显族），库存仍零行
    const failRejected = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/confirm`
    );
    expectDisplayableRejection(
      failRejected,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '质检不合格的收货单不能',
      'REJECTED 态的确认入库'
    );
    expect(
      await verifyStockFourDim(page, productId!, dimA.colorNo, dimA.dyeLotNo, {
        batchNo: dimA.batchNo,
        warehouseId: warehouseId!,
      }),
      'REJECTED 阻断期不得产生库存行'
    ).toBeNull();

    // 从 REJECTED 办理让步接收（合法前驱），理由操作人时间落专用列
    const concessionReason = '让步接收理由-尺寸轻微偏差特采评估';
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/concession`, {
      reason: concessionReason,
    });
    const headConcession = await readReceipt(page, receiptId);
    expect(
      String(headConcession.inspection_status),
      '让步后应逐字符落 CONCESSION_ACCEPTED'
    ).toBe('CONCESSION_ACCEPTED');
    expect(String(headConcession.receipt_status), '让步不改主状态，仍应为 DRAFT').toBe('DRAFT');
    expect(
      String(headConcession.concession_reason),
      'concession_reason 应原样落库专用列'
    ).toBe(concessionReason);
    expect(
      readNumber(headConcession, 'concession_by', '让步操作人') > 0,
      'concession_by 应由会话派生为非空操作人'
    ).toBe(true);
    expectTimestamp(headConcession.concession_at, '让步时间 concession_at');

    // 让步态不等于放行，确认仍被拒且文案点名出路
    const failConcession = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/confirm`
    );
    expectDisplayableRejection(
      failConcession,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '让步接收的收货单需先复检改判',
      '让步态的确认入库'
    );

    // 改判坏结论属校验族且可外显，空白理由同族拒绝
    const badToken = await apiCallExpectFail(page, 'POST', `/purchase/receipts/${receiptId}/rejudge`, {
      inspection_result: 'excellent',
      reason: '词表外结论探针',
    });
    expectDisplayableRejection(
      badToken,
      APP_ERROR_CODES.VALIDATION_ERROR,
      '改判结论只能是',
      '词表外改判结论'
    );
    const blankReason = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/rejudge`,
      { inspection_result: 'pass', reason: '   ' }
    );
    expectDisplayableRejection(
      blankReason,
      APP_ERROR_CODES.VALIDATION_ERROR,
      '请先填写复检改判理由',
      '空白改判理由'
    );
    const headAfterFail2 = await readReceipt(page, receiptId);
    expect(
      String(headAfterFail2.inspection_status),
      '两次被拒的改判不得改写检验状态'
    ).toBe('CONCESSION_ACCEPTED');
    expect(
      readNumber(headAfterFail2, 'rejudge_count', '改判次数'),
      '被拒的改判不得累加次数'
    ).toBe(0);

    // 复检改判为 pass，理由按 trim 后落库，映射目标为 PASSED
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/rejudge`, {
      inspection_result: 'pass',
      reason: '  复检确认合格放行  ',
    });
    const headPassed = await readReceipt(page, receiptId);
    expect(String(headPassed.inspection_status), '改判后应逐字符落 PASSED').toBe('PASSED');
    expect(
      String(headPassed.rejudge_reason),
      'rejudge_reason 应落 trim 后的原文（首尾空白不入库）'
    ).toBe('复检确认合格放行');
    expect(readNumber(headPassed, 'rejudge_count', '改判次数'), '成功改判一次计数应为 1').toBe(1);
    expect(
      readNumber(headPassed, 'rejudge_by', '改判操作人') > 0,
      'rejudge_by 应由会话派生'
    ).toBe(true);
    expectTimestamp(headPassed.rejudge_at, '改判时间 rejudge_at');

    // 确认入库，主状态逐字符落 COMPLETED（词表无 CONFIRMED 中间写入点）
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    const headDone = await readReceipt(page, receiptId);
    expect(
      String(headDone.receipt_status),
      `确认后应逐字符等于词表值 COMPLETED，实际 ${String(headDone.receipt_status)}`
    ).toBe('COMPLETED');
    expectTimestamp(headDone.confirmed_at, '确认时间 confirmed_at');
    expect(
      readNumber(headDone, 'confirmed_by', '确认人') > 0,
      'confirmed_by 应落会话操作人'
    ).toBe(true);
    expect(String(headDone.inspection_status), '确认后检验状态保持 PASSED').toBe('PASSED');

    // 四维库存读回，两行各按本行全额落库（这就是分流真值，40 不合格未从入库量扣除）
    const stockA = await verifyStockFourDim(page, productId!, dimA.colorNo, dimA.dyeLotNo, {
      batchNo: dimA.batchNo,
      warehouseId: warehouseId!,
    });
    const stockB = await verifyStockFourDim(page, productId!, dimB.colorNo, dimB.dyeLotNo, {
      batchNo: dimB.batchNo,
      warehouseId: warehouseId!,
    });
    if (!stockA || !stockB) {
      throw new Error('确认入库后两行四维库存必须存在，读回为空说明写回链断了');
    }
    expectStockRowTruth(stockA, {
      meters: 100,
      kg: 40,
      grade: '一等品',
      version: 0,
      label: '两维A行',
    });
    expectStockRowTruth(stockB, {
      meters: 60,
      kg: 20,
      grade: '一等品',
      version: 0,
      label: '两维B行',
    });
    const onHandTotal =
      readNumber(stockA, 'quantity_on_hand', 'A 在库量') +
      readNumber(stockB, 'quantity_on_hand', 'B 在库量');
    expectQuantity(
      onHandTotal,
      160,
      '入库总量应为收货全量 160 而非质检合格量 120（reject_quantity 不参与扣减，是整单门控不是按量分流）'
    );

    // 重复确认被 DRAFT 门拒绝（脱敏族），库存零副作用
    const dupConfirm = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/confirm`
    );
    expectMaskedRejection(dupConfirm, APP_ERROR_CODES.BUSINESS_ERROR, '重复确认入库');
    const stockADup = await verifyStockFourDim(page, productId!, dimA.colorNo, dimA.dyeLotNo, {
      batchNo: dimA.batchNo,
      warehouseId: warehouseId!,
    });
    if (!stockADup) {
      throw new Error('重复确认后 A 行仍应存在');
    }
    expectQuantity(
      readNumber(stockADup, 'quantity_on_hand', '重复确认后 A 在库量'),
      100,
      '被拒的重复确认不得二次累加在库量'
    );
    expect(
      readNumber(stockADup, 'version', '重复确认后 A 版本'),
      '被拒的重复确认不得推进乐观锁版本（累加未发生）'
    ).toBe(0);
  });

  test('同四维三行按行累加，库存按等级分桶各自成行，两行合计入一等品行', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const supplierId = ctx.supplierId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(supplierId, '前置供应商缺失').toBeTruthy();

    const tag = genCode('PRD2');
    const dims = {
      colorNo: `E2E-PRD2-C${tag}`,
      dyeLotNo: `E2E-PRD2-L${tag}`,
      batchNo: `E2E-PRD2-B${tag}`,
    };

    const prod = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
    const receiptId = await seedDraftReceipt(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      supplierId: supplierId!,
      materialCode: String(prod.code),
      materialName: String(prod.name),
      unitMaster: String(prod.unit),
      marker: tag,
      items: [
        // 前两行四维与等级完全相同（70 加 30），确认时先后落到同一库存行累加
        { lineNo: 1, qty: '70', alt: '30', grade: '一等品', ...dims },
        { lineNo: 2, qty: '30', alt: '10', grade: '一等品', ...dims },
        // 第三行仅等级不同，维度键含等级，应独立成行
        { lineNo: 3, qty: '25', alt: '10', grade: '二等品', ...dims },
      ],
    });

    const headDraft = await readReceipt(page, receiptId);
    expectQuantity(
      readNumber(headDraft, 'total_quantity', '主表总数量'),
      125,
      'total_quantity 应为三行求和'
    );
    expectQuantity(
      readNumber(headDraft, 'total_quantity_alt', '主表副单位总量'),
      50,
      'total_quantity_alt 应为三行 alt 求和'
    );

    // 整单全数合格放行（pass 到 PASSED 的同源映射）
    const inspectionId = await createInspectionForReceipt(page, receiptId, supplierId!);
    await completeInspection(page, inspectionId, {
      passQuantity: '125',
      rejectQuantity: '0',
      result: 'pass',
    });
    const headPassed = await readReceipt(page, receiptId);
    expect(
      String(headPassed.inspection_status),
      'pass 结论应逐字符回写为 PASSED'
    ).toBe('PASSED');

    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    const headDone = await readReceipt(page, receiptId);
    expect(String(headDone.receipt_status), '确认后应逐字符落 COMPLETED').toBe('COMPLETED');

    // 该四维组合下应恰好两行，按 grade 分桶
    const rows = await readStockRowsByDims(page, productId!, warehouseId!, dims);
    expect(rows.length, '同一批次色号缸号下应按等级分成两行').toBe(2);
    const rowFirst = rows.find(r => String(r.grade) === '一等品');
    const rowSecond = rows.find(r => String(r.grade) === '二等品');
    if (!rowFirst || !rowSecond) {
      throw new Error(`四维行按等级分桶失败，实际等级值=${rows.map(r => String(r.grade)).join('、')}`);
    }
    expectStockRowTruth(rowFirst, {
      meters: 100,
      kg: 40,
      grade: '一等品',
      version: 1,
      label: '一等品累加行',
    });
    expectStockRowTruth(rowSecond, {
      meters: 25,
      kg: 10,
      grade: '二等品',
      version: 0,
      label: '二等品独立行',
    });
    const firstRowCount = rows.filter(r => String(r.grade) === '一等品').length;
    expect(firstRowCount, '同四维同等级必须合并为一行而不是两行').toBe(1);
  });

  test('词表外取值过滤恒零行，让步改判前驱门、终态修改删除门全部生效且无副作用', async ({
    page,
  }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0];
    const warehouseId = ctx.warehouseIds[0];
    const supplierId = ctx.supplierId;
    expect(productId, '前置产品缺失').toBeTruthy();
    expect(warehouseId, '前置仓库缺失').toBeTruthy();
    expect(supplierId, '前置供应商缺失').toBeTruthy();

    // 词表外过滤探针，写入方从不产生 in_progress，等值下推必命中零行
    const listByIllegalToken = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/purchase/receipts?status=in_progress&page=1&page_size=10'
    );
    const rows = pickListArray<Record<string, unknown>>(
      listByIllegalToken,
      'items',
      '按 in_progress 过滤收货列表'
    );
    expect(rows.length, 'in_progress 不是 receipt_status 词表取值，命中行数应为 0').toBe(0);
    expect(
      readNumber(listByIllegalToken, 'total', '按 in_progress 过滤总数'),
      'in_progress 过滤的 total 应为 0'
    ).toBe(0);
    // 通用分页信封四键齐备并回显（本端点数组键是 items，与盘点列表的 counts 不同名）
    for (const key of ['items', 'total', 'page', 'page_size']) {
      expect(
        Object.prototype.hasOwnProperty.call(listByIllegalToken, key),
        `收货列表响应应含分页键 ${key}，实际键=${Object.keys(listByIllegalToken).join(',')}`
      ).toBe(true);
    }
    expect(readNumber(listByIllegalToken, 'page', '分页页码'), '回显页码应为提交的 1').toBe(1);
    expect(
      readNumber(listByIllegalToken, 'page_size', '分页每页量'),
      '回显每页量应为提交的 10'
    ).toBe(10);

    // 合法取值 COMPLETED 的等值过滤，返回行必须全部逐字符等于该值
    const listCompleted = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      '/purchase/receipts?status=COMPLETED&page=1&page_size=10'
    );
    const completedRows = pickListArray<Record<string, unknown>>(
      listCompleted,
      'items',
      '按 COMPLETED 过滤收货列表'
    );
    for (const row of completedRows) {
      expect(
        String(row.receipt_status),
        `COMPLETED 过滤命中行必须逐字符等于 COMPLETED，实际 ${String(row.receipt_status)}`
      ).toBe('COMPLETED');
    }

    const tag = genCode('PRD3');
    const dims = {
      colorNo: `E2E-PRD3-C${tag}`,
      dyeLotNo: `E2E-PRD3-L${tag}`,
      batchNo: `E2E-PRD3-B${tag}`,
    };
    const prod = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
    const receiptId = await seedDraftReceipt(page, {
      productId: productId!,
      warehouseId: warehouseId!,
      supplierId: supplierId!,
      materialCode: String(prod.code),
      materialName: String(prod.name),
      unitMaster: String(prod.unit),
      marker: tag,
      items: [{ lineNo: 1, qty: '5', alt: '2', grade: '一等品', ...dims }],
    });

    // PENDING 直接改判被前驱门拒绝（改判的唯一合法前驱是让步态）
    const failRejudgeFromPending = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/rejudge`,
      { inspection_result: 'pass', reason: '未让步直接改判探针' }
    );
    expectDisplayableRejection(
      failRejudgeFromPending,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '仅处于让步接收状态的收货单可复检改判',
      'PENDING 态直接改判'
    );

    // 空白理由让步被校验族拒绝，随后从合法前驱 PENDING 办理让步成功
    const failBlankConcede = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/concession`,
      { reason: '   ' }
    );
    expectDisplayableRejection(
      failBlankConcede,
      APP_ERROR_CODES.VALIDATION_ERROR,
      '请先填写让步接收理由',
      '空白让步理由'
    );
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/concession`, {
      reason: '让步接收理由-首检外观瑕疵',
    });
    const headConcession = await readReceipt(page, receiptId);
    expect(
      String(headConcession.inspection_status),
      'PENDING 是让步合法前驱，应逐字符落 CONCESSION_ACCEPTED'
    ).toBe('CONCESSION_ACCEPTED');

    // 让步后改判 fail 回 REJECTED，再让步（REJECTED 同为合法前驱），再改判 pass 到 PASSED
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/rejudge`, {
      inspection_result: 'fail',
      reason: '复检初判仍不合格',
    });
    const headRejectedAgain = await readReceipt(page, receiptId);
    expect(String(headRejectedAgain.inspection_status), 'fail 应逐字符落 REJECTED').toBe(
      'REJECTED'
    );
    expect(readNumber(headRejectedAgain, 'rejudge_count', '改判次数'), '一次改判计数应为 1').toBe(
      1
    );
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/concession`, {
      reason: '让步接收理由-二次特采评估',
    });
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/rejudge`, {
      inspection_result: 'pass',
      reason: '终检确认合格放行',
    });
    const headPassed = await readReceipt(page, receiptId);
    expect(String(headPassed.inspection_status), '二次改判应逐字符落 PASSED').toBe('PASSED');
    expect(readNumber(headPassed, 'rejudge_count', '改判次数'), '两次改判计数应为 2').toBe(2);
    expect(String(headPassed.concession_reason), '让步理由应保留最近一次值').toBe(
      '让步接收理由-二次特采评估'
    );

    // 放行确认入库，产生唯一四维库存行
    await apiCall(page, 'POST', `/purchase/receipts/${receiptId}/confirm`);
    const stockRow = await verifyStockFourDim(page, productId!, dims.colorNo, dims.dyeLotNo, {
      batchNo: dims.batchNo,
      warehouseId: warehouseId!,
    });
    if (!stockRow) {
      throw new Error('确认入库后四维库存行必须存在');
    }
    expectQuantity(
      readNumber(stockRow, 'quantity_on_hand', '在库量'),
      5,
      '在库量应为明细行全额 5'
    );

    // 终态门，COMPLETED 后让步与改判均被可外显门拒绝且状态零漂移
    const failConcedeDone = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/concession`,
      { reason: '终态让步探针' }
    );
    expectDisplayableRejection(
      failConcedeDone,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '已确认入库的收货单不允许办理让步接收',
      'COMPLETED 态办理让步'
    );
    const failRejudgeDone = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/receipts/${receiptId}/rejudge`,
      { inspection_result: 'fail', reason: '终态改判探针' }
    );
    expectDisplayableRejection(
      failRejudgeDone,
      APP_ERROR_CODES.BUSINESS_ERROR,
      '已确认入库的收货单不允许复检改判',
      'COMPLETED 态复检改判'
    );

    // 终态修改与删除属脱敏门，只钉契约不钉文案。被拒后状态与库存均无副作用
    const failPut = await apiCallExpectFail(page, 'PUT', `/purchase/receipts/${receiptId}`, {
      notes: '终态修改探针',
    });
    expectMaskedRejection(failPut, APP_ERROR_CODES.BUSINESS_ERROR, 'COMPLETED 态修改收货单');
    const failDelete = await apiCallExpectFail(page, 'DELETE', `/purchase/receipts/${receiptId}`);
    expectMaskedRejection(failDelete, APP_ERROR_CODES.BUSINESS_ERROR, 'COMPLETED 态删除收货单');

    const headFinal = await readReceipt(page, receiptId);
    expect(String(headFinal.receipt_status), '被拒的修改删除不得回退主状态').toBe('COMPLETED');
    expect(String(headFinal.inspection_status), '被拒的改判不得回退检验状态').toBe('PASSED');
    expect(readNumber(headFinal, 'rejudge_count', '改判次数'), '被拒改判不得累加计数').toBe(2);
    expect(String(headFinal.notes), '被拒修改不得改写备注').toBe(`E2E-收货深度链-${tag}`);
    const stockFinal = await verifyStockFourDim(page, productId!, dims.colorNo, dims.dyeLotNo, {
      batchNo: dims.batchNo,
      warehouseId: warehouseId!,
    });
    if (!stockFinal) {
      throw new Error('被拒的删除请求后库存行仍应存在');
    }
    expectQuantity(
      readNumber(stockFinal, 'quantity_on_hand', '终态门后在库量'),
      5,
      '被拒的删除不得回滚库存'
    );
    expect(
      readNumber(stockFinal, 'version', '终态门后版本'),
      '终态门拒绝期不得发生任何累加（版本保持 0）'
    ).toBe(0);
  });
});
