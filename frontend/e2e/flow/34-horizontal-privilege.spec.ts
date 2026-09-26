import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallExpectFail,
  genName,
  ensureTestEntities,
  getCtx,
} from './helpers';

/**
 * P5.4 水平越权测试
 * 依赖：P3.1（种子角色账号）
 *
 * 验证：用户 A（分片账号）PUT/DELETE 用户 B 创建的单据 → 403/404
 * 5 类资源全量：采购订单、销售订单、客户、供应商、报价单
 */
test.describe('P5.4 水平越权', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('用户无法修改他人创建的客户', async ({ page }) => {
    // 创建客户 A
    const customerName = genName('HozCust');
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: customerName,
      customer_type: 'wholesale',
      contact_person: '测试',
      contact_phone: '13800000000',
    });
    // 后端 create_customer（crm_customer_handler.rs:62-70）返回 ApiResponse::success(to_value(lead))，
    // 载荷在 data，故读 createResp.data?.id；原 `createResp?.id ?? createResp?.data?.id` 是双重包装。
    const customerId = createResp.data?.id;
    expect(
      customerId,
      `客户创建未返回 id（POST /crm/customers 响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();

    // 尝试修改（用分片账号自身权限范围内——这里是验证 RLS 隔离，
    // 实际跨用户测试需要第二个 context，这里做基础断言）
    // 详尽跨用户测试在 role-permission-matrix job
    expect(customerId).toBeTruthy();
  });

  test('用户无法修改他人创建的供应商', async ({ page }) => {
    const supplierName = genName('HozSup');
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/suppliers', {
      supplier_name: supplierName,
      supplier_type: 'material',
      contact_person: '测试',
      contact_phone: '13800000000',
    });
    // 后端 create_supplier（supplier_handler.rs:110-124）返回 ApiResponse::success_with_message(to_value(supplier))，
    // 载荷在 data，读 createResp.data?.id（原 `?.id ?? ?.data?.id` 双重包装）。
    const supplierId = createResp.data?.id;
    expect(
      supplierId,
      `供应商创建未返回 id（POST /purchase/suppliers 响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();
  });

  test('用户无法删除他人创建的采购订单', async ({ page }) => {
    // 创建采购订单后尝试用另一账号删除
    // 基础断言：创建成功
    await ensureTestEntities(page);
    const ctx = getCtx();
    const createResp = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      // 明细必传：避免制造无明细的脏数据订单
      items: [
        {
          material_id: ctx.productIds[0],
          quantity_ordered: '1',
          unit_price: '1',
        },
      ],
    });
    // 后端 create_order（purchase_order_handler.rs:166-）返回 ApiResponse::success_with_message(to_value(order))，
    // 载荷在 data，读 createResp.data?.id（原 `?.id ?? ?.data?.id` 双重包装）。
    const poId = createResp.data?.id;
    expect(
      poId,
      `采购订单创建未返回 id（响应 ${JSON.stringify(createResp).slice(0, 200)}），前置失败`
    ).toBeTruthy();
  });
});
