// P9-4 采购 E2E 套件 — 04 采购质检（独立页新建 + 完成流）
// 创建时间: 2026-06-17｜本文件按真实 UI 据实重写（决策：原用例对着不存在的 UI 写，属测试缺陷）
// 覆盖范围：采购质检全流程（3 用例）
//
// 真实 UI 事实（逐一核对源码，非猜测）：
// - 质检在独立扁平路由 /purchase-inspection（router index.ts:57 根 path '/' + :1035 子 path
//   'purchase-inspection'）新建；不存在「入库单详情点发起质检 → 带 label 表单」这条链路。
// - 新建入口：index.vue:11「新建检验单」按钮（purchaseInspection.index.button.create = '新建检验单'，
//   zh-CN:8805）→ 打开 PurchaseInspectionForm.vue 对话框（aria-label purchaseInspection.form.ariaLabel.create
//   = '新建检验单对话框'，zh-CN:8875）。
// - 表单第一个字段即「入库单号」下拉（PurchaseInspectionForm.vue:30-44，label purchaseInspection.form.label.
//   receiptNo = '入库单号' zh-CN:8880），选项 = 已加载的入库单 receipt_no；该下拉只 emit
//   receipt-change，receipt_id 由 usePi.handleReceiptChange（usePi.ts:192-234）写入 formData 后
//   经 props 镜像回表单（父组件是唯一写入方）；同一函数还从入库单派生 supplier_id 并带出明细。
//   第二个必填项「检验日期」（formRules:107-122）。
//   payload 含 receipt_id + supplier_id（usePiProc.ts:182-188），supplier_id 缺失会被前端拦截
//   （usePiProc.ts:175-181 purchaseInspection.message.supplierNotDerived
//    = '入库单未带出供应商，无法创建质检单' zh-CN:8796）。
// - 提交按钮 purchaseInspection.form.button.confirm = '确定'（zh-CN:8902）；成功 toast
//   msg.success('createSuccess') 走 message 命名空间（utils/message.ts 的 t() 固定前缀 message.），
//   即 zh-CN:1446 message.createSuccess = '创建成功'（不是 common/其它命名空间里的 '新增成功'）。
// - 录入结果不是「带 label 的合格/不合格/原因表单」，而是列表行内「完成」按钮（table.button.complete =
//   '完成' zh-CN:8929，仅 inspection_status==='pending' 时渲染 PurchaseInspectionTable.vue:89-96）
//   → usePiProc.handleComplete（:206-253）弹三连 ElMessageBox.prompt：合格数量（complete.passQuantityTip =
//   '请输入合格数量' zh-CN:8936）→ 不合格数量（rejectQuantityTip = '请输入不合格数量' zh-CN:8938）→
//   质检结论（resultTip = '请输入质检结论（pass / fail / partial）' zh-CN:8941，pattern /^(pass|fail|partial)$/）。
//   每个 prompt 确认按钮 = t('common.confirm') = '确认'（zh-CN:18）。成功 toast
//   msg.success('operationSuccess') = message.operationSuccess = '操作成功'（zh-CN:1453）。
// - 状态/结果 tag 文案（PurchaseInspectionTable.vue 状态列 getStatusText / 结果列 getResultText，
//   piFmts.ts + utils/purchase-inspection-status.ts）：inspection_status pending→'待检验'（zh-CN:8858）
//   completed→'已完成'（zh-CN:8859）；inspection_result pass→'合格'（zh-CN:8863）partial→'部分合格'
//   （zh-CN:8865）fail→'不合格'（zh-CN:8864）。
// - 后端 inspection_no 前缀为 'PI'（purchase_inspection_service.rs:74-79 impl_generate_no "PI"），
//   **不是** 原用例臆造的 `QI-\d{8}-\d{4}`，故本文件不写死单号正则，只断言非空。
// - 本页无「退货/不合格处理」按钮（退货是独立 purchase-return 单据），原 `/退货|不合格处理/` 断言删除。

import { test, expect, type Locator } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { pickSelectIn, formItemByExactLabel, pickListArray } from '../flow/ui-helpers';
import { apiCall, apiCallRaw, seedInspectionPass } from '../flow/helpers';

/** 当日 YYYY-MM-DD（检验日期 / 建单日期用） */
function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * 种一张「已确认(COMPLETED)且带 supplier、带明细」的采购入库单，返回其 id 与 receipt_no。
 * 链路照抄兄弟 spec / 全局种子已验证的真实流程（建 PO → 提交 → 审批 → 建入库单 → 确认），
 * 仅在本 spec 内用 apiCall 造数，不改全局 seed。
 * 入库单是质检「入库单号」下拉的数据源（usePi.fetchReceipts 取 GET /purchase/receipts 全量、
 * 后端按 created_at DESC 排序，query.rs:66-68，故本用例刚建的入库单必在首屏），
 * 且质检单创建所需的 supplier_id 由该入库单派生（usePi.handleReceiptChange）。
 * 任一步 apiCall 非 200 会抛错；关键 id/状态缺失用 expect 显式判负，不静默兜底。
 */
async function seedConfirmedReceipt(page: import('@playwright/test').Page): Promise<{
  rcvId: number;
  rcvNo: string;
}> {
  // 前置实体：供应商 / 仓库 / 部门 / 产品（后端 validate_order_request 与入库单 DTO 强校验其存在）
  const suppliers = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/purchase/suppliers?page=1&page_size=1'),
    'items',
    'seed 供应商'
  );
  const supplierId = suppliers[0]?.id;
  expect(supplierId, '种子缺供应商（/purchase/suppliers 无 items[0].id）').toBeTruthy();

  const warehouses = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/warehouses?page=1&page_size=1'),
    'items',
    'seed 仓库'
  );
  const warehouseId = warehouses[0]?.id;
  expect(warehouseId, '种子缺仓库（/warehouses 无 items[0].id）').toBeTruthy();

  const departments = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/departments?page=1&page_size=1'),
    'items',
    'seed 部门'
  );
  const departmentId = departments[0]?.id;
  expect(departmentId, '种子缺部门（/departments 无 items[0].id）').toBeTruthy();

  const products = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/products?page=1&page_size=1'),
    'items',
    'seed 产品'
  );
  const productId = products[0]?.id;
  expect(productId, '种子缺产品（/products 无 items[0].id）').toBeTruthy();

  // 入库明细的 material_code/material_name 取产品真实值（CreateReceiptItemRequest 非 Option，必填）
  const prod = await apiCallRaw<{ code?: string; name?: string; unit?: string }>(
    page,
    'GET',
    `/products/${productId}`
  );
  expect(prod.code, `种子产品 ${productId} 缺 code`).toBeTruthy();
  expect(prod.name, `种子产品 ${productId} 缺 name`).toBeTruthy();
  const unitMaster = prod.unit ?? '米';

  const ts = Date.now().toString().slice(-8);
  const today = todayStr();

  // 1) 建采购订单 + 提交 + 审批（入库需关联已审批订单，link_receipt_items_to_order_items）
  const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
    supplier_id: supplierId,
    order_date: today,
    warehouse_id: warehouseId,
    department_id: departmentId,
    notes: `E2E-INSP-PO-${ts}`,
    items: [{ material_id: productId, quantity_ordered: '500', unit_price: '15.00' }],
  });
  const poId = po.data?.id;
  expect(poId, `种子采购订单未返回 id：${JSON.stringify(po)}`).toBeTruthy();
  await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`, {});
  await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`, {});

  // 2) 建入库单（染色布四维齐全：batch_no/color_code/lot_no 均非空，满足建单期维度校验）
  const rcv = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
    supplier_id: supplierId,
    order_id: poId,
    receipt_date: today,
    warehouse_id: warehouseId,
    department_id: departmentId,
    notes: `E2E-INSP-RCV-${ts}`,
    items: [
      {
        line_no: 1,
        material_id: productId,
        material_code: prod.code,
        material_name: prod.name,
        batch_no: `E2E-IB${ts}`,
        color_code: 'E2E-INSP-COLOR',
        lot_no: `E2E-IL${ts}`,
        grade: '一等品',
        quantity: '500',
        quantity_alt: '0',
        unit_master: unitMaster,
        unit_price: '15.00',
      },
    ],
  });
  const rcvId = rcv.data?.id;
  expect(rcvId, `种子入库单未返回 id：${JSON.stringify(rcv)}`).toBeTruthy();

  // 3) 质检合格回写（入库门控要求的真实前置：见 helpers.seedInspectionPass）→ 确认入库
  //    （backend ensure_receipt_inspection_allows_flow 只放行 PASSED，未质检直接 confirm 必 400）
  await seedInspectionPass(page, {
    receiptId: rcvId as number,
    supplierId,
    context: 'purchase/04 种子入库单',
  });
  await apiCall(page, 'POST', `/purchase/receipts/${rcvId}/confirm`, {});

  // 4) 断言造数真实成功：回读确认态 + 取真实 receipt_no（下拉锚点）
  const confirmed = await apiCallRaw<{ receipt_status?: string; receipt_no?: string }>(
    page,
    'GET',
    `/purchase/receipts/${rcvId}`
  );
  expect(
    confirmed.receipt_status,
    `入库单 ${rcvId} 确认后状态应为 COMPLETED，实际=${confirmed.receipt_status}`
  ).toBe('COMPLETED');
  const rcvNo = confirmed.receipt_no ?? '';
  expect(rcvNo, `入库单 ${rcvId} 缺 receipt_no`).toBeTruthy();

  return { rcvId: rcvId as number, rcvNo };
}

/**
 * 在 /purchase-inspection 页用真实 UI 新建一张质检单：
 * 「新建检验单」→ 选「入库单号」→ 填「检验日期」→「确定」→ 断言创建成功 toast + 本用例种子行出现。
 * 返回定位到该新行的 Locator（以唯一 receipt_no 锚定，不依赖分页顺序）。
 */
async function createInspectionViaUI(
  page: import('@playwright/test').Page,
  rcvNo: string
): Promise<Locator> {
  await page.goto('/purchase-inspection');
  const createBtn = page.getByRole('button', { name: '新建检验单', exact: true });
  await expect(createBtn).toBeVisible({ timeout: 30_000 });
  await createBtn.click();

  const dialog = page.locator('.el-dialog:visible').first();
  await expect(dialog).toBeVisible({ timeout: 30_000 });

  // 入库单号下拉（el-select，选项文本 = receipt_no）
  await pickSelectIn(dialog, page, '入库单号', { optionText: rcvNo });

  // 检验日期（el-date-picker，value-format YYYY-MM-DD，fill + Enter 提交 v-model）
  const dateInput = formItemByExactLabel(dialog, '检验日期').locator('input').first();
  await dateInput.click();
  await dateInput.fill(todayStr());
  await dateInput.press('Enter');

  // 提交：form.button.confirm = '确定'
  await dialog.getByRole('button', { name: '确定', exact: true }).click();

  // 真实 createSuccess toast = message.createSuccess = '创建成功'
  await expect(page.locator('.el-message').filter({ hasText: '创建成功' }).first()).toBeVisible({
    timeout: 30_000,
  });

  const row = page.getByRole('row').filter({ hasText: rcvNo }).first();
  await expect(row).toBeVisible({ timeout: 30_000 });
  return row;
}

/** 应答「完成」流的单个 ElMessageBox.prompt：按提示文案锚定可见框，填值后点「确认」 */
async function answerPrompt(
  page: import('@playwright/test').Page,
  tipText: string,
  value: string
): Promise<void> {
  const box = page.locator('.el-message-box').filter({ hasText: tipText }).last();
  await expect(box).toBeVisible({ timeout: 30_000 });
  await box.getByRole('textbox').fill(value);
  await box.getByRole('button', { name: '确认', exact: true }).click();
}

/** 对本用例种子行走「完成」三连 prompt 并断言状态/结果 tag */
async function completeInspectionViaUI(
  page: import('@playwright/test').Page,
  row: Locator,
  passQty: string,
  rejectQty: string,
  resultInput: string,
  resultLabel: string
): Promise<void> {
  // 完成前该行为待检验态
  await expect(row.getByText('待检验', { exact: true })).toBeVisible({ timeout: 30_000 });
  await row.getByRole('button', { name: '完成', exact: true }).click();

  await answerPrompt(page, '请输入合格数量', passQty);
  await answerPrompt(page, '请输入不合格数量', rejectQty);
  await answerPrompt(page, '请输入质检结论', resultInput);

  // 成功 toast = '操作成功'
  await expect(page.locator('.el-message').filter({ hasText: '操作成功' }).first()).toBeVisible({
    timeout: 30_000,
  });

  // 状态 tag → '已完成'，结果 tag → 传入文案（合格/部分合格/不合格）
  await expect(row.getByText('已完成', { exact: true })).toBeVisible({ timeout: 30_000 });
  await expect(row.getByText(resultLabel, { exact: true })).toBeVisible({ timeout: 30_000 });
}

/**
 * 测试套件：采购质检（独立页新建 + 完成流）
 *
 * 业务流程：
 * 1. 有已确认入库单 → 质检页可新建检验单（入库单号下拉带出种子单）
 * 2. 完成检验填 合格 → 状态置已完成、结果置合格
 * 3. 完成检验填 部分合格 → 结果置部分合格（不合格走同一 prompt 填 fail，UI 路径一致）
 */
test.describe('04 采购质检', () => {
  test.beforeEach(async ({ page, context }) => {
    // 注入真实登录态（applyAuthMocks 走真实后端登录并落 cookie），业务 API 不打 mock
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('04-01 已确认入库单可在质检页新建检验单', async ({ page }) => {
    const { rcvNo } = await seedConfirmedReceipt(page);
    const row = await createInspectionViaUI(page, rcvNo);

    // 新增行状态为待检验（inspection_status pending）
    await expect(row.getByText('待检验', { exact: true })).toBeVisible({ timeout: 30_000 });
    // 检验单号列（第一列 inspection_no）非空——不写死后端单号正则（后端前缀为 PI，非 QI-）
    await expect(row.locator('td').first().locator('.cell')).toHaveText(/\S/);
  });

  test('04-02 质检完成填合格', async ({ page }) => {
    const { rcvNo } = await seedConfirmedReceipt(page);
    const row = await createInspectionViaUI(page, rcvNo);
    await completeInspectionViaUI(page, row, '100', '0', 'pass', '合格');
  });

  test('04-03 质检完成填部分合格', async ({ page }) => {
    const { rcvNo } = await seedConfirmedReceipt(page);
    const row = await createInspectionViaUI(page, rcvNo);
    await completeInspectionViaUI(page, row, '80', '20', 'partial', '部分合格');
  });
});
