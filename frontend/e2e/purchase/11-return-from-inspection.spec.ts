// 采购 E2E 套件 — 11 质检不合格生成退货（跨页联动）
//
// 覆盖：completed + fail 的质检单行「生成退货」按钮 → 跳 /purchase-return?fromInspection=<id>
//   → 退货新建对话框以新建态预填（供应商/原因/退货日期/明细非空）→ 提交 → 断言新退货单落库。
//
// 与 quotation copy 同构：以源单 id 载入派生数据、以新建态呈现、保存走 POST /purchase/returns。
// 预填取数口径（契约以出参为准，逐个核对源码）：
// - 表头：GET /purchase/inspections/{id} = purchase_inspection::Model 原键
//   （order_id/receipt_id/supplier_id/defect_description/notes），退货表单写
//   purchaseOrderId/receiptId/supplierId/returnDate(当日)/reasonType/reason/remarks（usePrRtn.ts）。
// - 原因类型：purchase_return.reason_type 的取值词表在共用常量 constants/return-reason，
//   预填取该表 qualityDefect 档 = 落库原值「品质瑕疵」。
// - 明细：GET /purchase/inspections/{id}/items 直接序列化 purchase_inspection_item::Model
//   （backend/src/services/purchase_inspection_service.rs:323），业务列只有
//   product_id / item_name / qualified_quantity / unqualified_quantity / remark，
//   **没有** failed_quantity / passed_quantity / product_name / defect_reason 这些键；
//   退货行取 unqualified_quantity > 0 的行，退货数量 = Number(unqualified_quantity)
//   （DECIMAL 经 JSON 序列化为字符串，如 "50.0000"）。
// - 提交按钮文案 = purchaseReturn.form.button.submit = '确定'；
//   成功 toast = msg.success('createSuccess') = message.createSuccess = '创建成功'。
// 状态/结果词表真值（后端 models/status/purchase_inventory.rs）：
//   inspection_status: pending/completed（全小写）
//   inspection_result: 自由文本，本用例写 pass/fail/partial（piFmts 映射已知取值）
//   purchase_return.return_status 写入方原值 draft（create_return）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, tryCleanup } from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

function todayStr(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * 经由 API 种一张 completed+fail 的质检单（附入库单明细行），返回 { inspectionId, inspectionNo, supplierId }。
 */
async function seedCompletedFailInspection(page: import('@playwright/test').Page): Promise<{
  inspectionId: number;
  inspectionNo: string;
  supplierId: number;
  productId: number;
}> {
  // 前置种子实体
  const suppliers = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/purchase/suppliers?page=1&page_size=1'),
    'items',
    'seed supplier'
  );
  const supplierId = suppliers[0]?.id;
  expect(supplierId, '种子缺供应商').toBeTruthy();

  const warehouses = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/warehouses?page=1&page_size=1'),
    'items',
    'seed warehouse'
  );
  const warehouseId = warehouses[0]?.id;
  expect(warehouseId, '种子缺仓库').toBeTruthy();

  const departments = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/departments?page=1&page_size=1'),
    'items',
    'seed department'
  );
  const departmentId = departments[0]?.id;
  expect(departmentId, '种子缺部门').toBeTruthy();

  const products = pickListArray<{ id: number }>(
    await apiCallRaw(page, 'GET', '/products?page=1&page_size=1'),
    'items',
    'seed product'
  );
  const productId = products[0]?.id;
  expect(productId, '种子缺产品').toBeTruthy();

  const prod = await apiCallRaw<{ code?: string; name?: string; unit?: string }>(
    page,
    'GET',
    `/products/${productId}`
  );
  const unitMaster = prod.unit ?? '米';

  const ts = Date.now().toString().slice(-8);
  const today = todayStr();

  // 1) 建采购订单 + 提交 + 审批
  const po = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
    supplier_id: supplierId,
    order_date: today,
    warehouse_id: warehouseId,
    department_id: departmentId,
    notes: `E2E-RTN-PO-${ts}`,
    items: [{ material_id: productId, quantity_ordered: '200', unit_price: '10.00' }],
  });
  const poId = po.data?.id;
  expect(poId, `建 PO 未返回 id`).toBeTruthy();
  await apiCall(page, 'POST', `/purchase/orders/${poId}/submit`, {});
  await apiCall(page, 'POST', `/purchase/orders/${poId}/approve`, {});

  // 2) 建入库单（**故意不确认**）
  //    「质检合格方可入库」门控（backend ensure_receipt_inspection_allows_flow）下，
  //    质检不合格的收货单本来就不允许确认入库——本用例被测的正是"判不合格 → 一键生成退货"，
  //    业务语义是货从未入库、直接退供，不是"先入库再退"。
  //    逐条核对过：11-01 只断言 completed+fail 行的「生成退货」按钮可见；
  //    11-02 只断言跳转 /purchase-return 且预填非空；11-03 只把退货单提交（submit）。
  //    库存扣减发生在 approve_return（purchase_return_service.rs:286 起，
  //    其内 deduct_stock_for_return_items:307），submit_return(:241) 不触库存，
  //    因此移除这步确认不会让任何断言失去前置——保留它反而要用"先 pass 再 fail"的
  //    复合质检把被测语义改写成"已入库货物复验不合格"，那是另一条链路。
  const rcv = await apiCall<{ id?: number }>(page, 'POST', '/purchase/receipts', {
    supplier_id: supplierId,
    order_id: poId,
    receipt_date: today,
    warehouse_id: warehouseId,
    department_id: departmentId,
    notes: `E2E-RTN-RCV-${ts}`,
    items: [
      {
        line_no: 1,
        material_id: productId,
        material_code: prod.code,
        material_name: prod.name,
        batch_no: `E2E-RB${ts}`,
        color_code: 'E2E-RC',
        lot_no: `E2E-RL${ts}`,
        grade: '一等品',
        quantity: '200',
        quantity_alt: '0',
        unit_master: unitMaster,
        unit_price: '10.00',
      },
    ],
  });
  const rcvId = rcv.data?.id;
  expect(rcvId, `建入库单未返回 id`).toBeTruthy();

  // 3) 建质检单
  const insp = await apiCall<{ id?: number; inspection_no?: string }>(
    page,
    'POST',
    '/purchase/inspections',
    {
      receipt_id: rcvId,
      order_id: poId,
      supplier_id: supplierId,
      inspection_date: today,
      notes: `E2E-RTN-INSP-${ts}`,
    }
  );
  const inspectionId = insp.data?.id;
  expect(inspectionId, `建质检单未返回 id`).toBeTruthy();
  CLEANUP.push({ path: `/purchase/inspections/${inspectionId}`, label: 'purchase_inspection' });

  // 4) 添加一条质检明细（不合格数量 > 0）
  await apiCall(page, 'POST', `/purchase/inspections/${inspectionId}/items`, {
    product_id: productId,
    item_name: prod.name ?? '测试产品',
    qualified_quantity: 150,
    unqualified_quantity: 50,
  });

  // 5) 完成质检（result = fail）
  await apiCall(page, 'POST', `/purchase/inspections/${inspectionId}/complete`, {
    pass_quantity: 150,
    reject_quantity: 50,
    inspection_result: 'fail',
  });

  const detail = await apiCallRaw<{ inspection_no?: string; inspection_status?: string }>(
    page,
    'GET',
    `/purchase/inspections/${inspectionId}`
  );
  expect(detail.inspection_status, '质检单应为 completed 态').toBe('completed');

  return {
    inspectionId: inspectionId as number,
    inspectionNo: detail.inspection_no ?? '',
    supplierId: supplierId as number,
    productId: productId as number,
  };
}

test.describe('11 质检不合格生成退货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('11-01 质检 completed+fail 行显示「生成退货」按钮', async ({ page }) => {
    const { inspectionNo } = await seedCompletedFailInspection(page);
    await page.goto('/purchase-inspection');
    const row = page.getByRole('row').filter({ hasText: inspectionNo }).first();
    await expect(row).toBeVisible({ timeout: 30_000 });
    await expect(row.getByRole('button', { name: '生成退货', exact: true })).toBeVisible({
      timeout: 10_000,
    });
  });

  test('11-02 点击「生成退货」跳转退货新建对话框且预填非空', async ({ page }) => {
    const { inspectionId } = await seedCompletedFailInspection(page);

    await page.goto('/purchase-inspection');
    // 通过 API 获取 inspectionNo 用于行定位
    const inspDetail = await apiCallRaw<{ inspection_no: string }>(
      page,
      'GET',
      `/purchase/inspections/${inspectionId}`
    );
    const inspRow = page.getByRole('row').filter({ hasText: inspDetail.inspection_no }).first();
    await expect(inspRow).toBeVisible({ timeout: 30_000 });
    const btn = inspRow.getByRole('button', { name: '生成退货', exact: true });
    await expect(btn).toBeVisible({ timeout: 10_000 });
    await btn.click();

    // 跳转后应到 /purchase-return（URL 含 purchase-return）
    await page.waitForURL('**/purchase-return**', { timeout: 30_000 });

    // 退货新建对话框可见
    const dlg = page.locator('.el-dialog:visible').first();
    await expect(dlg).toBeVisible({ timeout: 30_000 });
    await expect(dlg.locator('.el-dialog__title')).toContainText('新建退货单');

    // 预填验证：供应商下拉值非空（filterable 单选读 placeholder 项，input-wrapper 恒空）
    const supplierSelect = dlg
      .locator('.el-form-item')
      .filter({ hasText: '供应商' })
      .first()
      .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)');
    await expect(supplierSelect.first()).not.toHaveText('', { timeout: 10_000 });

    // 退货日期已预填
    const dateInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货日期' })
      .first()
      .locator('input')
      .first();
    const dateVal = await dateInput.inputValue();
    expect(dateVal, '退货日期应被预填').toBeTruthy();

    // 原因类型 = 品质瑕疵（预填自质检 fail）。filterable 单选的真实选中值渲染在
    // `.el-select__selected-item.el-select__placeholder`（非 input-wrapper），已选且收起时不带
    // `is-transparent`（见 EP select2.mjs:234-246：input-wrapper 恒空、placeholder 项才承载 label）。
    const reasonTypeSelect = dlg
      .locator('.el-form-item')
      .filter({ hasText: '原因类型' })
      .first()
      .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)');
    await expect(reasonTypeSelect.first()).toContainText('品质瑕疵', { timeout: 10_000 });

    // 明细表有预填行，且该行真实带出产品与不合格数量（seed 写 unqualified_quantity=50）
    const itemsTable = dlg.locator('.el-table').first();
    const firstRow = itemsTable.locator('.el-table__row').first();
    await expect(firstRow).toBeVisible({ timeout: 10_000 });
    // 产品 el-select 预填 productId：读真实选中项(placeholder 项非透明即确已选中)，空即预填未带产品
    await expect(
      firstRow
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first()
    ).not.toHaveText('', {
      timeout: 10_000,
    });
    await expect(firstRow.getByRole('spinbutton').first()).toHaveValue('50', { timeout: 10_000 });
  });

  test('11-03 提交预填退货单后生成新退货单并落库', async ({ page }) => {
    const { inspectionId, supplierId } = await seedCompletedFailInspection(page);

    await page.goto('/purchase-inspection');
    const inspDetail = await apiCallRaw<{ inspection_no: string }>(
      page,
      'GET',
      `/purchase/inspections/${inspectionId}`
    );
    const inspRow = page.getByRole('row').filter({ hasText: inspDetail.inspection_no }).first();
    await expect(inspRow).toBeVisible({ timeout: 30_000 });
    await inspRow.getByRole('button', { name: '生成退货', exact: true }).click();

    await page.waitForURL('**/purchase-return**', { timeout: 30_000 });

    const dlg = page.locator('.el-dialog:visible').first();
    await expect(dlg).toBeVisible({ timeout: 30_000 });

    // 补填退货原因详情（表单必填项）
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货原因' })
      .first()
      .locator('textarea')
      .first()
      .fill('E2E 质检不合格自动生成');

    // 明细行的产品必须由预填带出（不再"为空则手选"：手选会把「预填未带产品」的真实缺陷吃成绿灯）
    // 读真实选中项(placeholder 项非透明即确已选中)，input-wrapper 恒空不可用作判据
    const firstRow = dlg.locator('.el-table .el-table__row').first();
    await expect(
      firstRow
        .locator('.el-select__selected-item.el-select__placeholder:not(.is-transparent)')
        .first()
    ).not.toHaveText('', {
      timeout: 10_000,
    });

    // 监听 POST 建单请求
    const createdResp = page
      .waitForResponse(
        res =>
          res.request().method() === 'POST' &&
          res.url().includes('/purchase/returns') &&
          !res.url().includes('/items'),
        { timeout: 30_000 }
      )
      .catch(() => null);

    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30_000 });

    const resp = await createdResp;
    expect(resp, '未捕获到建单 POST 响应').not.toBeNull();
    const body = (await resp!.json()) as { data: { id: number; return_no: string } };
    const created = body.data;
    expect(created?.id, '建单响应未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${created.id}`, label: 'purchase_return' });

    // 回读确认退货单存在且关联了正确的 supplier
    const detail = await apiCallRaw<{
      id: number;
      return_status: string;
      supplier_id: number;
      reason_type: string;
    }>(page, 'GET', `/purchase/returns/${created.id}`);
    expect(detail.id).toBe(created.id);
    expect(detail.return_status, '新建退货单应为 draft 态').toBe('draft');
    expect(detail.supplier_id, '供应商应与质检单一致').toBe(supplierId);
    expect(detail.reason_type, '原因类型应为品质瑕疵').toBe('品质瑕疵');
  });
});
