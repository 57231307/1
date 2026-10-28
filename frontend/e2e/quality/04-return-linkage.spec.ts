// 质量管理 E2E 套件 — 04 不合格质检 → 采购退货联动：退货明细字段真实回读
// 覆盖范围：建质检单+明细(unqualified_quantity) → 回读明细字段持久化 →
//           创建退货单 → 按不合格数量添加退货明细 → 回读退货明细 quantity_returned = unqualified_quantity
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, ensureTestEntities, getCtx, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

interface InspectionItemRow {
  id: number;
  product_id: number;
  item_name: string;
  qualified_quantity: string | number;
  unqualified_quantity: string | number;
  remark: string | null;
}

// 采购质检单终止腿与回读断言。质检单没有根记录删除端点，整单的唯一合法终止是
// 待检状态经 complete 跃迁为已完成（后端 purchase_inspection_service 完成前置为 pending）。
// 调用方为本文件三个用例各自自建的质检单。入参为页面对象、质检单 id、合格合计与不合格合计。
// 传给后端 pass_quantity、reject_quantity、inspection_result，结论取值域为权威表
// purchase_inspection_result 的小写码 pass、fail、partial，此处按合格与不合格并存的语义取 partial。
// 跃迁后回读 GET 库里 inspection_status 落库值等于 completed，证明终止腿真实生效而非只回状态码。
async function completeInspectionAndVerify(
  page: import('@playwright/test').Page,
  inspectionId: number,
  passQuantity: number,
  rejectQuantity: number
): Promise<void> {
  await apiCall(page, 'POST', `/purchase/inspections/${inspectionId}/complete`, {
    pass_quantity: passQuantity,
    reject_quantity: rejectQuantity,
    inspection_result: 'partial',
  });
  const closed = await apiCallRaw<{ inspection_status: string }>(
    page,
    'GET',
    `/purchase/inspections/${inspectionId}`
  );
  expect(
    closed.inspection_status,
    `complete 终止腿生效后质检单状态应回读为 completed，实际值 ${closed.inspection_status}`
  ).toBe('completed');
}

test.describe('04 不合格质检 → 退货明细回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('04-01 质检明细 unqualified_quantity 落库回读', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;

    // 创建采购质检单
    const inspection = await apiCallRaw<{ id: number }>(page, 'POST', '/purchase/inspections', {
      supplier_id: ctx.supplierId,
      order_id: ctx.purchaseOrderId,
      inspection_date: new Date().toISOString().slice(0, 10),
      inspection_type: 'incoming',
    });
    expect(inspection.id, '创建质检单应返回 id').toBeTruthy();

    // 添加质检明细：合格 80，不合格 20
    const unqualifiedQty = '20';
    const item = await apiCallRaw<InspectionItemRow>(
      page,
      'POST',
      `/purchase/inspections/${inspection.id}/items`,
      {
        product_id: productId,
        item_name: '布面疵点检验',
        qualified_quantity: '80',
        unqualified_quantity: unqualifiedQty,
        remark: 'E2E 不合格联动测试',
      }
    );
    expect(item.id, '创建质检明细应返回 id').toBeTruthy();

    // GET 回读质检明细列表 → 验证 unqualified_quantity 真实落库
    const itemsResult = await apiCallRaw<{ items: InspectionItemRow[] }>(
      page,
      'GET',
      `/purchase/inspections/${inspection.id}/items`
    );
    expect(
      Array.isArray(itemsResult.items),
      `质检明细应返回 items 数组，实际: ${JSON.stringify(itemsResult).slice(0, 200)}`
    ).toBe(true);
    const found = itemsResult.items.find(i => i.id === item.id);
    expect(found, `回读列表应包含刚创建的明细 id=${item.id}`).toBeDefined();
    expect(
      Number(found!.unqualified_quantity),
      `回读 unqualified_quantity 应为 20，实际: ${found!.unqualified_quantity}`
    ).toBe(20);
    expect(Number(found!.qualified_quantity), `回读 qualified_quantity 应为 80`).toBe(80);

    // 用例断言全部达成后，走整单终止腿把本轮质检单跃迁为已完成并回读落库状态
    await completeInspectionAndVerify(page, inspection.id, 80, Number(unqualifiedQty));
  });

  test('04-02 退货明细 quantity_returned 按质检不合格数预填并回读正确', async ({ page }) => {
    const ctx = getCtx();
    const productId = ctx.productIds[0] || 1;
    const unqualifiedQty = 15;

    // Step1: 创建采购质检单 + 明细（模拟不合格）
    const inspection = await apiCallRaw<{ id: number }>(page, 'POST', '/purchase/inspections', {
      supplier_id: ctx.supplierId,
      order_id: ctx.purchaseOrderId,
      inspection_date: new Date().toISOString().slice(0, 10),
      inspection_type: 'incoming',
    });
    expect(inspection.id).toBeTruthy();

    await apiCall(page, 'POST', `/purchase/inspections/${inspection.id}/items`, {
      product_id: productId,
      item_name: '色差检验',
      qualified_quantity: '85',
      unqualified_quantity: String(unqualifiedQty),
      remark: '联动退货预填测试',
    });

    // Step2: 读取质检明细，取 unqualified_quantity 作为退货数量来源
    const itemsResp = await apiCallRaw<{ items: InspectionItemRow[] }>(
      page,
      'GET',
      `/purchase/inspections/${inspection.id}/items`
    );
    const inspItem = itemsResp.items?.find(i => Number(i.unqualified_quantity) > 0);
    expect(inspItem, '应存在不合格数量>0 的明细项').toBeDefined();
    const returnQty = Number(inspItem!.unqualified_quantity);
    expect(returnQty, '不合格数量应为 15').toBe(unqualifiedQty);

    // Step3: 创建退货单
    const returnOrder = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', {
      order_id: ctx.purchaseOrderId,
      supplier_id: ctx.supplierId,
      return_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      reason_type: '品质瑕疵',
      reason_detail: `质检不合格 ${returnQty} 件，联动退货`,
    });
    const returnId = returnOrder.data?.id;
    expect(
      returnId,
      `退货单创建应返回 id: ${JSON.stringify(returnOrder).slice(0, 200)}`
    ).toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${returnId}`, label: 'purchase_return' });

    // Step4: 添加退货明细，quantity_returned = 质检 unqualified_quantity
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 1,
      material_id: productId,
      quantity_returned: String(returnQty),
      unit_price: '25.00',
    });

    // Step5: GET 退货单明细 → 验证 quantity_returned 精确等于 unqualified_quantity
    // 判责 K-3（测试缺陷，信封口径）：GET /purchase/returns/:id/items 是「子表明细
    // 全量列表」（无 page/page_size 查询参），后端 handler 出参 = ApiResponse<Vec<
    // PurchaseReturnItemDto>>（handlers/purchase_return_handler.rs:225-236），前端契约
    // 早已按同一单形状声明 ApiResponse<PurchaseReturnItem[]>
    //（api/purchase-return.ts:192-193），check-api-envelope 分类映射中
    // 「Vec<T> ↔ 前端裸数组」为合法配对。PaginatedResponse{items,total,page,page_size}
    //（utils/response.rs:34）唯一约束的是**分页列表**，不覆盖本端点。
    // 原用例把分页信封强加于此 → 恒取 .items undefined 判红。此处对齐权威单形状：
    // 只接受数组载荷，出现其他形状即真实判红（禁双形状探测兼容）。
    const returnItems = await apiCallRaw<
      Array<{
        material_id: number;
        quantity_returned: string | number;
      }>
    >(page, 'GET', `/purchase/returns/${returnId}/items`);
    expect(
      Array.isArray(returnItems),
      `退货明细应为载荷数组（与该端点前后端一致的裸数组契约），实际: ${JSON.stringify(
        returnItems
      ).slice(0, 200)}`
    ).toBe(true);
    const rItem = returnItems.find(i => i.material_id === productId);
    expect(rItem, `退货明细应包含 material_id=${productId} 的行`).toBeDefined();
    expect(
      Number(rItem!.quantity_returned),
      `退货 quantity_returned(${rItem!.quantity_returned}) 应精确等于质检 unqualified_quantity(${unqualifiedQty})`
    ).toBe(unqualifiedQty);

    // 用例断言全部达成后，走整单终止腿把本轮质检单跃迁为已完成并回读落库状态
    await completeInspectionAndVerify(page, inspection.id, 85, unqualifiedQty);
  });

  test('04-03 多条不合格明细 → 退货预填各行数量独立正确', async ({ page }) => {
    const ctx = getCtx();
    expect(ctx.productIds.length, '需要至少 2 个产品 ID 用于多条明细测试').toBeGreaterThanOrEqual(
      2
    );
    const product1 = ctx.productIds[0];
    const product2 = ctx.productIds[1];
    const qty1 = 10;
    const qty2 = 25;

    // 建质检单 + 两条不合格明细
    const inspection = await apiCallRaw<{ id: number }>(page, 'POST', '/purchase/inspections', {
      supplier_id: ctx.supplierId,
      order_id: ctx.purchaseOrderId,
      inspection_date: new Date().toISOString().slice(0, 10),
    });

    await apiCall(page, 'POST', `/purchase/inspections/${inspection.id}/items`, {
      product_id: product1,
      item_name: '检验项A',
      qualified_quantity: '90',
      unqualified_quantity: String(qty1),
    });
    await apiCall(page, 'POST', `/purchase/inspections/${inspection.id}/items`, {
      product_id: product2,
      item_name: '检验项B',
      qualified_quantity: '75',
      unqualified_quantity: String(qty2),
    });

    // 创建退货单，两行分别按各自 unqualified_quantity 填入
    const returnOrder = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', {
      order_id: ctx.purchaseOrderId,
      supplier_id: ctx.supplierId,
      return_date: new Date().toISOString().slice(0, 10),
      warehouse_id: ctx.warehouseIds[0],
      reason_type: '品质瑕疵',
    });
    const returnId = returnOrder.data?.id;
    expect(returnId).toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${returnId}`, label: 'return_multi' });

    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 1,
      material_id: product1,
      quantity_returned: String(qty1),
      unit_price: '12.00',
    });
    await apiCall(page, 'POST', `/purchase/returns/${returnId}/items`, {
      line_no: 2,
      material_id: product2,
      quantity_returned: String(qty2),
      unit_price: '18.00',
    });

    // 回读退货明细 → 两行各自 quantity_returned 正确
    //（同 04-02 Step5 判责：本端点权威单形状＝载荷数组，非 items 包裹）
    const resp = await apiCallRaw<
      Array<{
        material_id: number;
        quantity_returned: string | number;
      }>
    >(page, 'GET', `/purchase/returns/${returnId}/items`);
    expect(
      Array.isArray(resp),
      `退货明细应为载荷数组（裸数组契约），实际: ${JSON.stringify(resp).slice(0, 200)}`
    ).toBe(true);
    const row1 = resp.find(i => i.material_id === product1);
    const row2 = resp.find(i => i.material_id === product2);
    expect(row1, `退货明细应含 material=${product1}`).toBeDefined();
    expect(row2, `退货明细应含 material=${product2}`).toBeDefined();
    expect(Number(row1!.quantity_returned), `第1行退货数量应为 ${qty1}`).toBe(qty1);
    expect(Number(row2!.quantity_returned), `第2行退货数量应为 ${qty2}`).toBe(qty2);

    // 用例断言全部达成后走整单终止腿，合格与不合格各取两条明细合计并回读落库状态
    await completeInspectionAndVerify(page, inspection.id, 90 + 75, qty1 + qty2);
  });
});
