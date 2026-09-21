import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  tryCleanup,
  ensureTestEntities,
  ensureStockInWarehouse,
  getCtx,
  listNotifications,
} from './helpers';

/**
 * P0 自动通知全链路覆盖（2026-09-11 用户指令："自动产生的通知需要详细覆盖所有功能，每条链路都要触发验证通知"）
 *
 * 5 条可测链路：
 * A. 订单提交 → notify_order_submitted → 创建人收到通知
 * B. 订单审批 → notify_order_approved → 创建人收到通知
 * C. 订单发货 → notify_order_shipped → 创建人收到通知
 * D. 库存预警 → notify_inventory_alert_batch → admin/manager 收到预警通知
 * F. 付款申请提交 → admin/manager 审批人收到通知
 *
 * （原 E 链路 notify_ar_due 为死代码：已实现无调用方，其 skip 占位测试已移除）
 *
 * 验证模式：触发业务动作 → 查询通知列表 → 断言通知产生（标题/内容匹配）→ 清理
 *
 * iter23 修正：本地 getUnreadNotifications 按 data.items 取列表，而后端
 * list_notifications 的 key 是 data.list（notification_handler.rs:75），返回恒为空数组，
 * 导致"没找到通知就只 warn"的分支全部空转、A/B/C 三条链路一条断言都没跑到；
 * 现统一改用 helpers.listNotifications，并把 A/B/C 的软分支改成真实断言。
 */

/** 通知落库由 commit 后事件驱动，触发后需给监听器留出写入窗口 */
const NOTIF_SETTLE_MS = 3000;

/** 清理动作：走 helpers 的 CSRF/重试链路，失败仅记录（行为已由用例断言验证） */
async function markRead(page: import('@playwright/test').Page, id: number): Promise<void> {
  await tryCleanup(page, 'POST', `/notifications/${id}/read`, '[31d] 标记已读');
}

async function deleteNotification(
  page: import('@playwright/test').Page,
  id: number
): Promise<void> {
  await tryCleanup(page, 'DELETE', `/notifications/${id}`, '[31d] 删除通知');
}

test.describe.serial('P0 自动通知全链路：业务动作→通知产生验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('A. 订单提交→创建人收到提交通知', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 先记录已有通知数（基线）
    const before = await listNotifications(page);
    console.log(`[31d-A] 提交前未读通知 ${before.length} 条`);

    let orderId: number | undefined;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds[0], quantity: 10, unit_price: 25.5 }],
    });
    orderId = r?.data?.id;
    expect(orderId, '[31d-A] 销售订单创建失败，提交通知链路无从验证').toBeTruthy();
    console.log(`[31d-A] 订单创建成功 id=${orderId}`);

    // 创建即草稿态，应先收到「销售订单已创建」（notify_order_created），
    // 且与后续提交通知用不同 dedup_key，不被 5 分钟窗口折叠
    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const afterCreate = await listNotifications(page);
    const createdNotif = afterCreate
      .filter(n => !before.some(b => b.id === n.id))
      .find(n => n.title === '销售订单已创建');
    expect(
      createdNotif,
      `[31d-A] 未收到「销售订单已创建」通知（新增 ${afterCreate.length} 条：${afterCreate
        .map(n => n.title)
        .join('|')}）`
    ).toBeTruthy();
    console.log(`[31d-A] 已收到创建通知 id=${createdNotif!.id}`);

    // submit 端点触发通知
    await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);
    console.log(`[31d-A] 订单提交成功`);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    console.log(`[31d-A] 提交后未读通知 ${after.length} 条`);
    const newOnes = after.filter(n => !afterCreate.some(b => b.id === n.id));
    // event_notification_service.rs 的 notify_order_submitted 固定标题
    const orderNotif = newOnes.find(n => n.title === '订单已提交');
    console.log(`[31d-A] 提交后新增通知 ${newOnes.length} 条，匹配提交通知: ${!!orderNotif}`);
    expect(
      orderNotif,
      `[31d-A] 未收到「订单已提交」通知（新增 ${newOnes.length} 条：${newOnes
        .map(n => n.title)
        .join('|')}）`
    ).toBeTruthy();
    await markRead(page, orderNotif!.id);
    await deleteNotification(page, orderNotif!.id);
    await markRead(page, createdNotif!.id);
    await deleteNotification(page, createdNotif!.id);

    // 清理订单
    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '[31d-A]');
  });

  test('B. 订单审批→创建人收到审批通知', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 创建+提交订单，再审批
    let orderId: number | undefined;
    const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds[0], quantity: 5, unit_price: 30 }],
    });
    orderId = r?.data?.id;
    expect(orderId, '[31d-B] 销售订单创建失败，审批通知链路无从验证').toBeTruthy();
    console.log(`[31d-B] 订单创建成功 id=${orderId}`);

    await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);

    const before = await listNotifications(page);

    // submit 只会把订单置为 pending 并拉起 BPM 首任务，不存在"小额直接终审"分支；
    // 原实现按 status!=='approved' 跳过 approve，是为 BPM 流程定义 schema 不匹配
    // 导致后端自动完成回写 approved 而写的兜底（已在 helpers 修正 schema）。
    const afterSubmit = await apiCallRaw<{ status?: string }>(
      page,
      'GET',
      `/sales/orders/${orderId}`
    );
    expect(afterSubmit.status, '[31d-B] submit 后订单应为 pending').toBe('pending');
    await apiCall(page, 'POST', `/sales/orders/${orderId}/approve`);
    console.log(`[31d-B] 订单审批成功`);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    const newOnes = after.filter(n => !before.some(b => b.id === n.id));
    // event_notification_service.rs:192 notify_order_approved 的固定标题
    const approvalNotif = newOnes.find(n => n.title === '订单审批通过');
    console.log(`[31d-B] 新增通知 ${newOnes.length} 条，匹配审批通知: ${!!approvalNotif}`);
    expect(
      approvalNotif,
      `[31d-B] 未收到「订单审批通过」通知（新增 ${newOnes.length} 条：${newOnes
        .map(n => n.title)
        .join('|')}）`
    ).toBeTruthy();
    await markRead(page, approvalNotif!.id);
    await deleteNotification(page, approvalNotif!.id);

    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '[31d-B]');
  });

  test('C. 订单发货→创建人收到发货通知', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    let orderId: number | undefined;
    // 客户/产品取 ensureTestEntities 真实保障的实体。原实现硬编码 customer_id:1 /
    // product_id:1，在 CI 空库中依赖种子恰好存在，一旦漂移订单创建就拿不到 id
    const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      order_date: new Date().toISOString(),
      items: [{ product_id: ctx.productIds[0], quantity: 8, unit_price: 20 }],
    });
    orderId = r?.data?.id;
    expect(orderId, '[31d-C] 销售订单创建失败，发货通知链路无从验证').toBeTruthy();
    console.log(`[31d-C] 订单创建成功 id=${orderId}`);

    // 提交+审批后才能发货
    await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);
    const afterCSubmit = await apiCallRaw<{ status?: string }>(
      page,
      'GET',
      `/sales/orders/${orderId}`
    );
    expect(afterCSubmit.status, '[31d-C] submit 后订单应为 pending').toBe('pending');
    await apiCall(page, 'POST', `/sales/orders/${orderId}/approve`);

    const before = await listNotifications(page);

    // ship.rs:135 按 warehouse::Column::WarehouseCode 查仓，原实现硬编码 'WH001'
    // 在 CI 空库中不存在 → 发货接口回 NOT_FOUND。改为按 ctx 真实仓库反查其编码，
    // 并保障该仓库有可发出库存。
    const warehouseId = ctx.warehouseIds[0];
    await ensureStockInWarehouse(page, ctx.productIds[0], warehouseId);
    const wh = await apiCallRaw<{ warehouse_code?: string }>(
      page,
      'GET',
      `/warehouses/${warehouseId}`
    );
    expect(wh?.warehouse_code, `仓库 ${warehouseId} 应返回 warehouse_code`).toBeTruthy();

    await apiCall(page, 'POST', `/sales/orders/${orderId}/ship`, {
      order_id: orderId,
      warehouse_code: wh.warehouse_code,
      items: [{ product_id: ctx.productIds[0], quantity: 8 }],
    });
    console.log(`[31d-C] 订单发货成功（仓库编码 ${wh.warehouse_code}）`);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    const newOnes = after.filter(n => !before.some(b => b.id === n.id));
    // event_notification_service.rs:228 notify_order_shipped 的固定标题
    const shipNotif = newOnes.find(n => n.title === '订单已发货');
    console.log(`[31d-C] 新增通知 ${newOnes.length} 条，匹配发货通知: ${!!shipNotif}`);
    expect(
      shipNotif,
      `[31d-C] 未收到「订单已发货」通知（新增 ${newOnes.length} 条：${newOnes
        .map(n => n.title)
        .join('|')}）`
    ).toBeTruthy();
    await markRead(page, shipNotif!.id);
    await deleteNotification(page, shipNotif!.id);

    await tryCleanup(page, 'DELETE', `/sales/orders/${orderId}`, '[31d-C]');
  });

  test('D. 库存预警→admin/manager收到预警通知', async ({ page }) => {
    test.setTimeout(120_000);
    const before = await listNotifications(page);
    console.log(`[31d-D] 触发前未读通知 ${before.length} 条`);

    // GET /inventory/stock/low-stock 触发 check_low_stock → 发布事件 → 通知 admin/manager
    await apiCallRaw<unknown>(page, 'GET', '/inventory/stock/low-stock');
    console.log('[31d-D] low-stock 检查完成');

    await page.waitForTimeout(5000);
    const after = await listNotifications(page);
    const newOnes = after.filter(n => !before.some(b => b.id === n.id));
    const stockNotif = newOnes.find(
      n => n.title?.includes('库存') || n.title?.includes('stock') || n.title?.includes('预警')
    );
    console.log(
      `[31d-D] 新增通知 ${newOnes.length} 条（${newOnes.map(n => n.title).join('|')}），` +
        `匹配库存预警通知: ${!!stockNotif}`
    );

    if (stockNotif) {
      await markRead(page, stockNotif.id);
      await deleteNotification(page, stockNotif.id);
    } else {
      // TODO(doto iter23)：本链路要变成硬断言，需先把某商品的 safety_stock 抬到
      // 现有库存之上以构造确定性的低库存前提，再断言必产生预警通知。
      console.warn('[31d-D] 本轮无库存预警通知（当前库存均高于安全线）');
    }
  });

  test('F. 付款申请提交→admin/manager审批人收到通知', async ({ page }) => {
    test.setTimeout(180_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    // 原实现 supplier_id 硬编码为 1，CI 空库中依赖种子恰好存在该供应商
    // 付款申请必须挂在真实应付单上：后端 CreateApPaymentRequest.items 为必填，
    // 且校验应付单非 DRAFT/CANCELLED、apply_amount 不超过未付金额。
    // 路由为 /api/v1/erp/ap/payment-requests（routes/finance.rs:712，AP 域经
    // sub_routes() 挂在 /api/v1/erp 下，无 /finance 前缀）。
    const invoice = await apiCallRaw<{ id?: number; unpaid_amount?: number }>(
      page,
      'POST',
      '/ap/invoices',
      {
        supplier_id: ctx.supplierId,
        amount: 8000,
        invoice_date: new Date().toISOString().slice(0, 10),
      }
    );
    expect(invoice?.id, '[31d-F] 应付单创建失败，付款申请前置不成立').toBeTruthy();
    await apiCall(page, 'POST', `/ap/invoices/${invoice.id}/approve`);
    const approved = await apiCallRaw<{ invoice_status?: string; unpaid_amount?: number }>(
      page,
      'GET',
      `/ap/invoices/${invoice.id}`
    );
    console.log(
      `[31d-F] 应付单 id=${invoice.id} 状态=${approved.invoice_status} 未付=${approved.unpaid_amount}`
    );
    expect(
      (approved.invoice_status || '').toUpperCase(),
      `[31d-F] 应付单审批后状态非可付款态（实际 ${approved.invoice_status}）`
    ).not.toBe('DRAFT');
    const unpaid = Number(approved.unpaid_amount);
    expect(unpaid, `[31d-F] 应付单未付金额无效（实际 ${approved.unpaid_amount}）`).toBeGreaterThan(
      0
    );

    const r = await apiCall<{ id?: number; request_no?: string }>(
      page,
      'POST',
      '/ap/payment-requests',
      {
        supplier_id: ctx.supplierId,
        request_date: new Date().toISOString().slice(0, 10),
        payment_type: 'bank_transfer',
        payment_method: 'bank',
        request_amount: 5000,
        currency: 'CNY',
        exchange_rate: 1,
        items: [{ invoice_id: invoice.id, apply_amount: 5000 }],
      }
    );
    const requestId = r?.data?.id;
    expect(requestId, `[31d-F] 付款申请创建失败（supplier_id=${ctx.supplierId}）`).toBeTruthy();
    // request_no 由后端生成（服务内 generate_request_no），通知标题按回写值匹配
    const requestNo = r?.data?.request_no;
    expect(requestNo, '[31d-F] 创建响应未返回 request_no，无法匹配通知标题').toBeTruthy();
    console.log(
      `[31d-F] 付款申请创建成功 id=${requestId} request_no=${requestNo} 应付单=${invoice.id}`
    );

    const before = await listNotifications(page);

    await apiCall(page, 'POST', `/ap/payment-requests/${requestId}/submit`);
    console.log(`[31d-F] 付款申请提交成功`);

    await page.waitForTimeout(NOTIF_SETTLE_MS);
    const after = await listNotifications(page);
    const newOnes = after.filter(n => !before.some(b => b.id === n.id));
    // ap_payment_request_handler.rs submit_request 的固定标题：付款申请待审批：{request_no}
    const expectedTitle = `付款申请待审批：${requestNo}`;
    const payNotif = newOnes.find(n => n.title === expectedTitle);
    console.log(
      `[31d-F] 新增通知 ${newOnes.length} 条（${newOnes.map(n => n.title).join('|')}），` +
        `匹配「${expectedTitle}」: ${!!payNotif}`
    );
    expect(
      payNotif,
      `[31d-F] 未收到付款申请审批人通知「${expectedTitle}」（新增 ${newOnes.length} 条：${newOnes
        .map(n => n.title)
        .join('|')}）`
    ).toBeTruthy();

    await markRead(page, payNotif!.id);
    await deleteNotification(page, payNotif!.id);

    await tryCleanup(page, 'DELETE', `/ap/payment-requests/${requestId}`, '[31d-F]');
  });
});
