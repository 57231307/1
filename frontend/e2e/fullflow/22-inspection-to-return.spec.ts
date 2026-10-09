// 交易域全流程契约级 E2E — 22 质检到退货（入库单→质检独立建单→完成三连 prompt→生成退货→落库回读）
//
// 定位：用户两类缺陷在本链的靶心：
//   ①「创建保存不完整、再编辑显示不出来」→ UI 建质检单后 GET 回读 receipt_id/supplier_id/order_id/
//      inspection_date 全键；退货单落库后回读主表 + 明细（明细 DTO 键 material_id/quantity_returned
//      即前端编辑回填键，缺任一即"显示不出来"，purchase_return_service.rs:1053-1074）。
//   ②「提交/保存报请求错误」→ 负例锁具体 status + 机器码（BUSINESS_ERROR/NOT_FOUND），禁放宽。
// 取数口径：入库单/质检明细/完成动作虽以 API 驱动，但**建质检单与「完成」录入、点「生成退货」与
// 提交**均走真实 UI（与 purchase/04、purchase/11 的既有 DOM 事实同源），本例自建唯一链路，
// 用后端生成的唯一 receipt_no/inspection_no 精确锚定行；afterEach 尽力清理。
// 单号诚实事实：本页派生单号前缀为 **PI**（purchase_inspection_service.rs:74-79 impl_generate_no "PI"），
// 不是 QI；断言只验 inspection_no 非空，不写死正则。本页无退货按钮——退货入口是质检列表行操作
// 「生成退货」（purchase/11 源码事实）。
//
// 契约真值源：
//   质检状态词表（小写）   backend/src/models/status/purchase_inventory.rs:114-120（pending/completed）
//   完成三连 prompt        usePiProc.handleComplete（purchase/04 头注释：合格数量→不合格数量→结论，
//     pattern /^(pass|fail|partial)$/，确认按钮文案 '确认'，成功 toast '操作成功'）
//   完成后端落库           services/purchase_inspection_service.rs:169-215：pass_quantity/reject_quantity/
//     inspection_result/quality_score=(pass/(pass+reject))*100（:221-231）/inspection_status=completed；
//     仅 pending 可完成（:184-189），非 pending 重复完成 business 拒（脱敏站点锁 code）
//   入库单词表（大写）     models/status/purchase_inventory.rs:35-42（DRAFT/CONFIRMED/COMPLETED）
//   退货状态词表（小写）   同上 :98-110（draft/submitted/approved/rejected）
//   退货主表回读键         models/purchase_return.rs:11-32（return_no/receipt_id/order_id/supplier_id/
//     return_date/reason_type/reason_detail/return_status/total_quantity/total_quantity_alt/total_amount）
//   退货明细回读键         services/purchase_return_service.rs:1053-1074 PurchaseReturnItemDto
//     （material_id ← product_id 别名、quantity_returned ← quantity 别名、四维 color_no/dye_lot_no/
//      batch_no 全列直出）；GET /purchase/returns/{id}/items 的 data 为**裸数组**（handler:226-237）
//   生成退货入口门控       completed 且 result∈{fail,partial} 行显示「生成退货」（purchase/11）
//   原因预填               constants/return-reason qualityDefect 档 = 落库原值「品质瑕疵」
// 诚实标注（写入侧真实缺口，报告同步）：purchase complete_inspection 不回写
//   purchase_receipt.inspection_status（PENDING/PASSED/REJECTED 词表列，
//   models/status/purchase_inventory.rs:163-190 与 from_inspection_result 只被通用质检记录域使用），
//   本文件不断该回写（断不存在=假绿，断存在=误红），列入后端缺口清单；
//   inspection_result 后端为**自由文本**（service:199 直接 Set(req.inspection_result)），
//   词表 pass/fail/partial 仅前端 prompt pattern 把关——API 直送任意词会照单全收，
//   该缺口只报告不加宽松断言掩盖。
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
  seedInspectionPass,
} from '../flow/helpers';
import { pickSelectIn, formItemByExactLabel, pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

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

function expectDecimal(obj: Record<string, unknown>, key: string, expected: number, label: string) {
  if (!Object.prototype.hasOwnProperty.call(obj, key)) {
    throw new Error(`${label}：缺数值键 "${key}"，实际键=${Object.keys(obj).join(',')}`);
  }
  const n = Number(obj[key]);
  if (!Number.isFinite(n))
    throw new Error(`${label}：${key} 不可解析，raw=${JSON.stringify(obj[key])}`);
  expect(Math.abs(n - expected), `${label}：${key} 期望=${expected} 实际=${n}`).toBeLessThan(0.005);
}

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/**
 * API 种一张 COMPLETED 入库单（全四维明细，染色布口径同 purchase/04），
 * 返回 { poId, rcvId, rcvNo, productId, supplierId, batchNo, colorCode, lotNo }。
 * 该链是质检「入库单号」下拉的数据源；供应商/产品由后端从入库单派生（usePi.handleReceiptChange）。
 */
async function seedConfirmedReceipt(page: Page): Promise<{
  poId: number;
  rcvId: number;
  rcvNo: string;
  productId: number;
  supplierId: number;
  batchNo: string;
}> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  if (!ctx.warehouseIds[0]) throw new Error('前置缺失：ctx.warehouseIds[0] 未就绪');
  if (!ctx.departmentIds[0]) throw new Error('前置缺失：ctx.departmentIds[0] 未就绪');
  const productId = ctx.productIds[0];
  const supplierId = ctx.supplierId;
  const prod = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/products/${productId}`);
  if (!prod.code || !prod.name)
    throw new Error(`产品 ${productId} 缺 code/name（CreateReceiptItemRequest 非 Option 必填）`);

  const ts = genCode('F22');
  const po = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/orders', {
    supplier_id: supplierId,
    order_date: todayStr(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-F22-PO-${ts}`,
    items: [{ material_id: productId, quantity_ordered: '200', unit_price: '10.00' }],
  });
  const poId = requireNum(po.id, '建 PO');
  CLEANUP.push({ path: `/purchase/orders/${poId}`, label: `purchase_order#${poId}` });
  await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`);
  await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`);

  const batchNo = `E22B${ts}`;
  const rcv = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/purchase/receipts', {
    supplier_id: supplierId,
    order_id: poId,
    receipt_date: todayStr(),
    warehouse_id: ctx.warehouseIds[0],
    department_id: ctx.departmentIds[0],
    notes: `E2E-F22-RCV-${ts}`,
    items: [
      {
        line_no: 1,
        material_id: productId,
        material_code: prod.code,
        material_name: prod.name,
        batch_no: batchNo,
        color_code: `E22C${ts}`,
        lot_no: `E22L${ts}`,
        grade: '一等品',
        quantity: '200',
        quantity_alt: '50',
        unit_master: prod.unit ?? '米',
        unit_price: '10.00',
      },
    ],
  });
  const rcvId = requireNum(rcv.id, '建入库单');
  CLEANUP.push({ path: `/purchase/receipts/${rcvId}`, label: `purchase_receipt#${rcvId}` });
  // 门控前置：门控落地后未质检的收货单确认必 400（helpers.seedInspectionPass 走
  // 建质检单→complete(pass)→回读 PASSED 的真实回写链）。
  // 本用例被测动作仍是随后在 UI 新建并完成的那张质检单（pass/fail 由用例决定），
  // 种子里这次质检只是数据前置，没有替代被测动作。
  await seedInspectionPass(page, {
    receiptId: rcvId,
    supplierId,
    context: '22 种子入库单',
  });
  await apiCall(page, 'POST', `/purchase/receipts/${rcvId}/confirm`);
  const confirmed = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/purchase/receipts/${rcvId}`
  );
  expectKeyValue(confirmed, 'receipt_status', 'COMPLETED', `入库单 ${rcvId} 确认回读`);
  const rcvNo = String(confirmed.receipt_no ?? '');
  if (!rcvNo) throw new Error(`入库单 ${rcvId} 缺 receipt_no（下拉锚点）`);
  return { poId, rcvId, rcvNo, productId, supplierId, batchNo };
}

/**
 * UI 在 /purchase-inspection 新建质检单（第一字段=入库单号下拉 receipt_id），
 * 捕获 POST /purchase/inspections 响应拿本例 id；再 GET 详情逐字段回读（缺陷①）。
 */
async function createInspectionViaUI(
  page: Page,
  rcvNo: string,
  anchors: { rcvId: number; poId: number; supplierId: number }
): Promise<{ id: number; inspection_no: string; notes: unknown }> {
  await page.goto('/purchase-inspection');
  const createBtn = page.getByRole('button', { name: '新建检验单', exact: true });
  await expect(createBtn).toBeVisible({ timeout: 30_000 });
  await createBtn.click();
  const dialog = page.locator('.el-dialog:visible').first();
  await expect(dialog).toBeVisible({ timeout: 30_000 });

  // 入库单号下拉：选项文本 = receipt_no（PurchaseInspectionForm.vue:30-44）
  await pickSelectIn(dialog, page, '入库单号', { optionText: rcvNo });
  // 已选值回读（EP filterable 单选：非透明 placeholder 项才承载真实选中值）
  await expect(
    formItemByExactLabel(dialog, '入库单号')
      .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
      .first()
  ).toContainText(rcvNo, { timeout: 10_000 });

  const dateInput = formItemByExactLabel(dialog, '检验日期').locator('input').first();
  await dateInput.click();
  await dateInput.fill(todayStr());
  await dateInput.press('Enter');

  // 只绑 200：CSRF token 一次性消费下 UI 首个 POST 可能 403（csrf.rs:110 consume +
  // :216-224 轮换，前端 axios 用恢复头静默重放，request.ts:197-223）；不过滤状态码就会
  // 命中 data=null 的 403 中间态，"质检单创建响应 id"落空即真红假象（同 11-03 族）。
  const createdResp = page
    .waitForResponse(
      r =>
        r.request().method() === 'POST' &&
        /\/purchase\/inspections(\?|$)/.test(r.url()) &&
        !/\/items/.test(r.url()) &&
        r.status() === 200,
      { timeout: 30_000 }
    )
    .catch(() => null);
  await dialog.getByRole('button', { name: '确定', exact: true }).click();
  // 真实 toast = message.createSuccess = '创建成功'（purchase/04 头注释）
  await expect(page.locator('.el-message').filter({ hasText: '创建成功' }).first()).toBeVisible({
    timeout: 30_000,
  });
  const resp = await createdResp;
  expect(resp, `UI 建质检单未捕获 POST /purchase/inspections（入库单 ${rcvNo}）`).not.toBeNull();
  const body = (await resp!.json()) as { data?: { id?: number } };
  const id = requireNum(body.data?.id, '质检单创建响应 id');
  CLEANUP.push({ path: `/purchase/inspections/${id}`, label: `purchase_inspection#${id}` });

  // ①全字段回读：下拉派生的三个外键必须真实落库
  const detail = await apiCallRaw<Record<string, unknown>>(
    page,
    'GET',
    `/purchase/inspections/${id}`
  );
  expectKeyValue(detail, 'receipt_id', anchors.rcvId, '质检单回读 receipt_id');
  expectKeyValue(detail, 'order_id', anchors.poId, '质检单回读 order_id（自入库单派生）');
  expectKeyValue(
    detail,
    'supplier_id',
    anchors.supplierId,
    '质检单回读 supplier_id（自入库单派生）'
  );
  expectKeyValue(detail, 'inspection_status', 'pending', '新建质检词值 pending（小写）');
  expectKeyValue(detail, 'inspection_date', todayStr(), '质检单回读 inspection_date');
  const inspection_no = String(detail.inspection_no ?? '');
  if (!inspection_no)
    throw new Error(`质检单缺 inspection_no，实际键=${Object.keys(detail).join(',')}`);
  return { id, inspection_no, notes: detail.notes };
}

/** 应答「完成」流单个 ElMessageBox.prompt（确认按钮文案 '确认'，purchase/04 事实） */
async function answerPrompt(page: Page, tipText: string, value: string) {
  const box = page.locator('.el-message-box').filter({ hasText: tipText }).last();
  await expect(box).toBeVisible({ timeout: 30_000 });
  await box.getByRole('textbox').fill(value);
  await box.getByRole('button', { name: '确认', exact: true }).click();
}

/** 对本例行点「完成」→ 三连 prompt → 断 toast 与行 tag */
async function completeViaUI(
  page: Page,
  inspectionNo: string,
  pass: string,
  failQty: string,
  result: string,
  resultLabel: string
) {
  const row = page.getByRole('row').filter({ hasText: inspectionNo }).first();
  await expect(row.getByText('待检验', { exact: true })).toBeVisible({ timeout: 30_000 });
  await row.getByRole('button', { name: '完成', exact: true }).click();
  await answerPrompt(page, '请输入合格数量', pass);
  await answerPrompt(page, '请输入不合格数量', failQty);
  await answerPrompt(page, '请输入质检结论', result);
  await expect(page.locator('.el-message').filter({ hasText: '操作成功' }).first()).toBeVisible({
    timeout: 30_000,
  });
  await expect(row.getByText('已完成', { exact: true })).toBeVisible({ timeout: 30_000 });
  await expect(row.getByText(resultLabel, { exact: true })).toBeVisible({ timeout: 30_000 });
}

test.describe('22 质检到退货全流程契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('22-01 质检独立建单（UI）→ 明细/完成权威字段回读：pass 行 quality_score=80 且不显示「生成退货」', async ({
    page,
  }) => {
    const seed = await seedConfirmedReceipt(page);
    const insp = await createInspectionViaUI(page, seed.rcvNo, seed);

    // API 补明细（后端键名以 purchase_inspection_item Model 为准：qualified/unqualified_quantity，
    // purchase/11 头注释——本页**没有** failed/passed_quantity 这些键，不臆测别名）
    await apiCall(page, 'POST', `/purchase/inspections/${insp.id}/items`, {
      product_id: seed.productId,
      item_name: 'E2E-F22 检验项',
      qualified_quantity: 200,
      unqualified_quantity: 50,
    });
    const itemRows = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/inspections/${insp.id}/items`),
      // 该端点出参 data 是对象 {items,total,inspection_id}
      // （purchase_inspection_handler.rs:200-213 list_inspection_items →
      //   ApiResponse::success(json!{items,total,inspection_id})），非裸数组。
      // apiCallRaw 返回信封内层 data（helpers.ts:1495），故须以 'items' 直读 data.items；
      // 原声明 'bare' 与真实形状不符会让 pickListArray 抛「列表契约失配」——测试声明错，非源码缺陷。
      'items',
      `质检单 ${insp.id} 明细`
    );
    expect(itemRows.length, '明细应含 1 行').toBe(1);
    expectDecimal(itemRows[0], 'qualified_quantity', 200, '明细合格量回读');
    expectDecimal(itemRows[0], 'unqualified_quantity', 50, '明细不合格量回读');

    // UI 完成：200 合格 / 50 不合格 / pass → tag 已完成/合格
    await completeViaUI(page, insp.inspection_no, '200', '50', 'pass', '合格');

    // 后端权威回读：pass/fail 量、结论、quality_score=200/250*100=80（service:221-231）
    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/inspections/${insp.id}`
    );
    expectKeyValue(after, 'inspection_status', 'completed', '完成回读词值（小写）');
    expectKeyValue(after, 'inspection_result', 'pass', '结论回读（写入即提交值，service:199）');
    expectDecimal(after, 'pass_quantity', 200, '主表合格量');
    expectDecimal(after, 'reject_quantity', 50, '主表不合格量');
    expectDecimal(after, 'quality_score', 80, '质量得分=(200/250)*100（service:221-231）');

    // 门控另一证：completed 行再点「完成」——按钮只在 pending 渲染（PurchaseInspectionTable.vue:89-96），
    // 完成后的行应无「完成」按钮
    const row = page.getByRole('row').filter({ hasText: insp.inspection_no }).first();
    await expect(row.getByRole('button', { name: '完成', exact: true })).toHaveCount(0);

    // pass 行不显示「生成退货」（退货入口仅 fail/partial，purchase/11 门控事实）
    await expect(row.getByRole('button', { name: '生成退货', exact: true })).toHaveCount(0);
  });

  test('22-02 fail 行「生成退货」：预填回读→提交→退货单主表+明细全键落库回读（缺陷①靶心）', async ({
    page,
  }) => {
    const seed = await seedConfirmedReceipt(page);
    const insp = await createInspectionViaUI(page, seed.rcvNo, seed);
    await apiCall(page, 'POST', `/purchase/inspections/${insp.id}/items`, {
      product_id: seed.productId,
      item_name: 'E2E-F22 检验项',
      qualified_quantity: 150,
      unqualified_quantity: 50,
    });
    await completeViaUI(page, insp.inspection_no, '150', '50', 'fail', '不合格');

    // 行操作「生成退货」→ /purchase-return 新建对话框（预填派生）
    const inspRow = page.getByRole('row').filter({ hasText: insp.inspection_no }).first();
    const rtnBtn = inspRow.getByRole('button', { name: '生成退货', exact: true });
    await expect(
      rtnBtn,
      `completed+fail 行必须提供「生成退货」入口（purchase/11-01 同源门控）`
    ).toBeVisible({
      timeout: 10_000,
    });
    await rtnBtn.click();
    await page.waitForURL('**/purchase-return**', { timeout: 30_000 });
    const dlg = page.locator('.el-dialog:visible').first();
    await expect(dlg).toBeVisible({ timeout: 30_000 });

    // 预填回读（非空判据用真实选中项，禁止放宽成"可见即通过"）
    await expect(
      formItemByExactLabel(dlg, '供应商')
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '派生预填：供应商必须自质检单带出'
    ).not.toHaveText('', { timeout: 10_000 });
    await expect(
      formItemByExactLabel(dlg, '原因类型')
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '派生预填：原因类型=品质瑕疵（constants/return-reason qualityDefect 档落库原值）'
    ).toContainText('品质瑕疵', { timeout: 10_000 });
    const firstItemRow = dlg.locator('.el-table .el-table__row').first();
    await expect(
      firstItemRow
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first(),
      '派生预填：明细行产品必须带出（为空即"保存数据不完整"回归）'
    ).not.toHaveText('', { timeout: 10_000 });
    await expect(firstItemRow.getByRole('spinbutton').first()).toHaveValue('50', {
      timeout: 10_000,
    });

    // 补录退货原因（必填）并提交
    await formItemByExactLabel(dlg, '退货原因')
      .locator('textarea')
      .first()
      .fill('E2E-F22 质检不合格自动生成');
    // 只绑 200，避开 CSRF 一次性消费的 403 中间态（成因见本文件 ② 建质检单处同注释）
    const createdResp = page
      .waitForResponse(
        r =>
          r.request().method() === 'POST' &&
          /\/purchase\/returns(\?|$)/.test(r.url()) &&
          !/\/items/.test(r.url()) &&
          r.status() === 200,
        { timeout: 30_000 }
      )
      .catch(() => null);
    await dlg.getByRole('button', { name: '确定', exact: true }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30_000 });
    const resp = await createdResp;
    expect(resp, '未捕获 POST /purchase/returns 响应').not.toBeNull();
    const body = (await resp!.json()) as { data?: { id?: number; return_no?: string } };
    const rtnId = requireNum(body.data?.id, '退货单创建响应 id');
    const rtnNo = String(body.data?.return_no ?? '');
    if (!rtnNo) throw new Error(`退货单响应缺 return_no，实际=${JSON.stringify(body.data)}`);
    CLEANUP.push({ path: `/purchase/returns/${rtnId}`, label: `purchase_return#${rtnId}` });

    // 主表全键回读（models/purchase_return.rs:11-32）
    const rtn = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/returns/${rtnId}`
    );
    expectKeyValue(rtn, 'return_no', rtnNo, '退货单号回读');
    expectKeyValue(
      rtn,
      'return_status',
      'draft',
      '退货初始词值小写 draft（purchase_inventory.rs:100）'
    );
    expectKeyValue(rtn, 'supplier_id', seed.supplierId, '退货供应商=质检派生供应商');
    expectKeyValue(
      rtn,
      'receipt_id',
      seed.rcvId,
      '退货 receipt_id 派生自质检所引用入库单（缺陷①）'
    );
    expectKeyValue(rtn, 'order_id', seed.poId, '退货 order_id 派生回读');
    expectKeyValue(rtn, 'reason_type', '品质瑕疵', '原因类型落库原值');
    expectKeyValue(rtn, 'reason_detail', 'E2E-F22 质检不合格自动生成', '原因详情落库回读');
    expectDecimal(rtn, 'total_quantity', 50, '主表数量合计=明细不合格量 50');

    // 明细回读：裸数组 + 前端回填键名逐一对齐（PurchaseReturnItemDto :1053-1074）
    const lines = pickListArray<Record<string, unknown>>(
      await apiCallRaw<unknown>(page, 'GET', `/purchase/returns/${rtnId}/items`),
      'bare',
      `退货单 ${rtnId} 明细`
    );
    expect(lines.length, '退货应含 1 行明细（仅 unqualified_quantity>0 的行进入）').toBe(1);
    expectKeyValue(
      lines[0],
      'material_id',
      seed.productId,
      '明细回填键 material_id（←product_id 别名，:1057/:1083）'
    );
    expectDecimal(
      lines[0],
      'quantity_returned',
      50,
      '明细回填键 quantity_returned（←quantity 别名，:1060）'
    );
    for (const k of ['color_no', 'dye_lot_no', 'batch_no']) {
      expect(
        typeof lines[0][k] === 'string',
        `明细四维键 "${k}" 应为字符串直出（:1071-1073），实际=${JSON.stringify(lines[0][k])}`
      ).toBe(true);
    }

    // 状态机延伸：submitted/approve 门词值（draft→submit）
    await apiCall(page, 'POST', `/purchase/returns/${rtnId}/submit`);
    const sub = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/returns/${rtnId}`
    );
    expectKeyValue(sub, 'return_status', 'submitted', 'submit 回读词值（:103）');
  });

  test('22-03 门控负例：重复 complete 业务拒且无痕；不存在站点 404/NOT_FOUND', async ({ page }) => {
    const seed = await seedConfirmedReceipt(page);
    const insp = await createInspectionViaUI(page, seed.rcvNo, seed);
    await apiCall(page, 'POST', `/purchase/inspections/${insp.id}/complete`, {
      pass_quantity: 10,
      reject_quantity: 0,
      inspection_result: 'pass',
    });

    // completed 再 complete → 400 BUSINESS（service:184-189，business 脱敏站点锁 code）
    const dup = await apiCallExpectFail(page, 'POST', `/purchase/inspections/${insp.id}/complete`, {
      pass_quantity: 999,
      reject_quantity: 0,
      inspection_result: 'pass',
    });
    expect(dup.status, `重复完成应 400，实际=${dup.status} body=${JSON.stringify(dup)}`).toBe(400);
    expect(failureCode(dup), '重复完成机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const after = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/inspections/${insp.id}`
    );
    expectDecimal(after, 'pass_quantity', 10, '被拒后合格量不得被二次写入覆盖（半改即红）');
    expectKeyValue(after, 'inspection_status', 'completed', '被拒后状态无痕');

    // 不存在的质检单 → 404 NOT_FOUND（统一失败信封）
    const gone = await apiCallExpectFail(page, 'GET', '/purchase/inspections/2147483647');
    expect(gone.status, '不存在质检单 GET 应 404').toBe(404);
    expect(failureCode(gone), '不存在机器码').toBe('NOT_FOUND');

    // supplier_id 缺失 → 400 VALIDATION_ERROR 且 message 可外显（validation_displayable，
    // purchase_inspection_service.rs:94-97 "采购验收单缺少供应商ID"——非脱敏站点必须锁真实文案）
    const noSup = await apiCallExpectFail(page, 'POST', '/purchase/inspections', {
      receipt_id: seed.rcvId,
      order_id: seed.poId,
      inspection_date: todayStr(),
      notes: 'E2E-F22-NOSUP',
    });
    expect(
      noSup.status,
      `缺供应商建质检应 400，实际=${noSup.status} body=${JSON.stringify(noSup)}`
    ).toBe(400);
    expect(failureCode(noSup), '缺供应商机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(noSup.message, '可外显站点必须回具体原因文案').toBe('采购验收单缺少供应商ID');

    // completed 单再 PUT 修改 → 400 BUSINESS（update 门 service:136-141）
    const failUpd = await apiCallExpectFail(page, 'PUT', `/purchase/inspections/${insp.id}`, {
      notes: '违规修改',
    });
    expect(failUpd.status, 'completed 后 PUT 应 400').toBe(400);
    expect(failureCode(failUpd), 'completed 后 PUT 机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const noDrift = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/purchase/inspections/${insp.id}`
    );
    expectKeyValue(noDrift, 'notes', insp.notes, 'PUT 被拒后备注不得半改');
  });
});
