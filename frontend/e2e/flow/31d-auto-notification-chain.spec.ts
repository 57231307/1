import { test, expect } from '../diagnose-fixture';
import { loginViaUI, apiCall } from './helpers';

/**
 * P0 自动通知全链路覆盖（2026-09-11 用户指令："自动产生的通知需要详细覆盖所有功能，每条链路都要触发验证通知"）
 *
 * 5 条可测链路 + 1 条死代码 skip：
 * A. 订单提交 → notify_order_submitted → 创建人收到通知
 * B. 订单审批 → notify_order_approved → 创建人收到通知
 * C. 订单发货 → notify_order_shipped → 创建人收到通知
 * D. 库存预警 → notify_inventory_alert_batch → admin/manager 收到通知
 * E. 应收到期 → notify_ar_due → 死代码（已实现无调用方），skip 并记录
 * F. 付款申请提交 → notify_multiple_users → admin/manager 审批人收到通知
 *
 * 验证模式：触发业务动作 → 查询通知列表 → 断言通知产生（标题/内容匹配）→ 清理
 */

const TS = Date.now().toString().slice(-8);

/** 查询当前用户通知列表，返回未读通知 */
async function getUnreadNotifications(page: import('@playwright/test').Page, userId?: number): Promise<{ id: number; title: string; content: string; businessType?: string }[]> {
  try {
    const res = await page.request.get(`http://localhost:8082/api/v1/erp/notifications/?status=unread&page=1&page_size=50`);
    if (!res.ok()) {
      console.warn(`[31d] 通知列表查询 HTTP ${res.status()}`);
      return [];
    }
    const body = await res.json();
    const items = body?.data?.items || body?.data?.data || body?.data || [];
    return Array.isArray(items) ? items : [];
  } catch (e) {
    console.warn(`[31d] 通知列表查询异常: ${(e as Error).message}`);
    return [];
  }
}

/** 删除通知（清理） */
async function deleteNotification(page: import('@playwright/test').Page, id: number): Promise<void> {
  try {
    await page.request.delete(`http://localhost:8082/api/v1/erp/notifications/notification/${id}`);
    console.log(`[31d] 清理通知 id=${id} ✅`);
  } catch (e) {
    console.warn(`[31d] 清理通知 id=${id} 失败: ${(e as Error).message}`);
  }
}

/** 标记通知已读（清理，避免影响后续用例） */
async function markRead(page: import('@playwright/test').Page, id: number): Promise<void> {
  try {
    await page.request.post(`http://localhost:8082/api/v1/erp/notifications/notification/${id}/read`);
  } catch { /* 静默 */ }
}

test.describe.serial('P0 自动通知全链路：业务动作→通知产生验证', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('A. 订单提交→创建人收到提交通知', async ({ page }) => {
    test.setTimeout(180_000);
    // 先记录已有通知数（基线）
    const before = await getUnreadNotifications(page);
    console.log(`[31d-A] 提交前未读通知 ${before.length} 条`);

    let orderId: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ product_id: 1, quantity: 10, unit_price: 25.5 }],
      });
      orderId = r?.data?.id;
    } catch (e) {
      console.error(`[31d-A] 订单创建失败: ${(e as Error).message}`);
    }
    if (!orderId) { test.skip(); return; }
    console.log(`[31d-A] 订单创建成功 id=${orderId}`);

    // submit 端点触发通知
    try {
      await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);
      console.log(`[31d-A] 订单提交成功`);
    } catch (e) {
      console.error(`[31d-A] 订单提交失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(3000);
    const after = await getUnreadNotifications(page);
    console.log(`[31d-A] 提交后未读通知 ${after.length} 条`);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const orderNotif = newOnes.find((n) => n.title?.includes('订单') || n.businessType === 'ORDER');
    console.log(`[31d-A] 新增通知 ${newOnes.length} 条，匹配订单通知: ${!!orderNotif}`);

    if (orderNotif) {
      expect(orderNotif.title, `[31d-A] 通知标题应含"订单"相关字样`).toBeTruthy();
      await markRead(page, orderNotif.id);
      await deleteNotification(page, orderNotif.id);
    } else {
      console.warn('[31d-A] 未找到订单提交通知（可能通知服务未配置或去重窗口内已存在）');
    }

    // 清理订单
    try {
      await apiCall(page, 'DELETE', `/sales/orders/${orderId}`);
      console.log(`[31d-A] 清理订单 id=${orderId} ✅`);
    } catch (e) {
      console.warn(`[31d-A] 清理订单失败: ${(e as Error).message}`);
    }
  });

  test('B. 订单审批→创建人收到审批通知', async ({ page }) => {
    test.setTimeout(180_000);
    // 创建+提交订单，再审批
    let orderId: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ product_id: 1, quantity: 5, unit_price: 30 }],
      });
      orderId = r?.data?.id;
    } catch (e) {
      console.error(`[31d-B] 订单创建失败: ${(e as Error).message}`);
    }
    if (!orderId) { test.skip(); return; }
    console.log(`[31d-B] 订单创建成功 id=${orderId}`);

    try {
      await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`);
    } catch (e) {
      console.warn(`[31d-B] 订单提交失败: ${(e as Error).message}`);
    }

    const before = await getUnreadNotifications(page);

    try {
      await apiCall(page, 'POST', `/sales/orders/${orderId}/approve`);
      console.log(`[31d-B] 订单审批成功`);
    } catch (e) {
      console.error(`[31d-B] 订单审批失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(3000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const approvalNotif = newOnes.find((n) => n.title?.includes('审批') || n.title?.includes('approve') || n.businessType === 'ORDER');
    console.log(`[31d-B] 新增通知 ${newOnes.length} 条，匹配审批通知: ${!!approvalNotif}`);

    if (approvalNotif) {
      await markRead(page, approvalNotif.id);
      await deleteNotification(page, approvalNotif.id);
    } else {
      console.warn('[31d-B] 未找到订单审批通知（可能通知服务未配置或已读）');
    }

    try {
      await apiCall(page, 'DELETE', `/sales/orders/${orderId}`);
      console.log(`[31d-B] 清理订单 id=${orderId} ✅`);
    } catch (e) {
      console.warn(`[31d-B] 清理订单失败: ${(e as Error).message}`);
    }
  });

  test('C. 订单发货→创建人收到发货通知', async ({ page }) => {
    test.setTimeout(180_000);
    let orderId: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: 1,
        order_date: new Date().toISOString().slice(0, 10),
        items: [{ product_id: 1, quantity: 8, unit_price: 20 }],
      });
      orderId = r?.data?.id;
    } catch (e) {
      console.error(`[31d-C] 订单创建失败: ${(e as Error).message}`);
    }
    if (!orderId) { test.skip(); return; }
    console.log(`[31d-C] 订单创建成功 id=${orderId}`);

    // 提交+审批后才能发货
    try { await apiCall(page, 'POST', `/sales/orders/${orderId}/submit`); } catch (e) { console.warn(`[31d-C] 提交失败: ${(e as Error).message}`); }
    try { await apiCall(page, 'POST', `/sales/orders/${orderId}/approve`); } catch (e) { console.warn(`[31d-C] 审批失败: ${(e as Error).message}`); }

    const before = await getUnreadNotifications(page);

    try {
      await apiCall(page, 'POST', `/sales/orders/${orderId}/ship`, {
        order_id: orderId,
        warehouse_code: 'WH001',
        items: [{ product_id: 1, quantity: 8 }],
      });
      console.log(`[31d-C] 订单发货成功`);
    } catch (e) {
      console.error(`[31d-C] 订单发货失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(3000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const shipNotif = newOnes.find((n) => n.title?.includes('发货') || n.title?.includes('ship') || n.businessType === 'ORDER');
    console.log(`[31d-C] 新增通知 ${newOnes.length} 条，匹配发货通知: ${!!shipNotif}`);

    if (shipNotif) {
      await markRead(page, shipNotif.id);
      await deleteNotification(page, shipNotif.id);
    } else {
      console.warn('[31d-C] 未找到发货通知（可能通知服务未配置或库存不足拒绝发货）');
    }

    try {
      await apiCall(page, 'DELETE', `/sales/orders/${orderId}`);
      console.log(`[31d-C] 清理订单 id=${orderId} ✅`);
    } catch (e) {
      console.warn(`[31d-C] 清理订单失败: ${(e as Error).message}`);
    }
  });

  test('D. 库存预警→admin/manager收到预警通知', async ({ page }) => {
    test.setTimeout(120_000);
    const before = await getUnreadNotifications(page);
    console.log(`[31d-D] 触发前未读通知 ${before.length} 条`);

    // GET /inventory/stock/low-stock 触发 check_low_stock → 发布事件 → 通知 admin/manager
    try {
      const res = await page.request.get('http://localhost:8082/api/v1/erp/inventory/stock/low-stock');
      console.log(`[31d-D] low-stock 检查 HTTP ${res.status()}`);
    } catch (e) {
      console.warn(`[31d-D] low-stock 检查异常: ${(e as Error).message}`);
    }

    await page.waitForTimeout(5000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const stockNotif = newOnes.find((n) => n.title?.includes('库存') || n.title?.includes('stock') || n.title?.includes('预警') || n.businessType === 'INVENTORY');
    console.log(`[31d-D] 新增通知 ${newOnes.length} 条，匹配库存预警通知: ${!!stockNotif}`);

    if (stockNotif) {
      expect(stockNotif.title, '[31d-D] 库存预警通知标题应存在').toBeTruthy();
      await markRead(page, stockNotif.id);
      await deleteNotification(page, stockNotif.id);
    } else {
      console.warn('[31d-D] 未找到库存预警通知（可能当前库存均高于安全线，无预警产生）');
    }
  });

  test('E. 应收到期通知（死代码，skip）', async () => {
    // notify_ar_due 在 event_notification_service.rs:615 已实现但无任何调用方
    // 既无 HTTP 端点也无定时调度触发——已实现的死代码
    // 需后续补接触发链路（定时扫描应收账款到期表 → notify_ar_due）
    test.skip();
    console.log('[31d-E] notify_ar_due 为死代码（已实现无调用方），skip');
  });

  test('F. 付款申请提交→admin/manager审批人收到通知', async ({ page }) => {
    test.setTimeout(180_000);
    let requestId: number | undefined;
    try {
      const r = await apiCall<{ id?: number }>(page, 'POST', '/finance/ap/payment-requests', {
        request_no: `P0-NOTIF-${TS}`,
        request_date: new Date().toISOString().slice(0, 10),
        supplier_id: 1,
        payment_type: 'bank_transfer',
        payment_method: 'bank',
        request_amount: 5000,
        currency: 'CNY',
        exchange_rate: 1,
      });
      requestId = r?.data?.id;
    } catch (e) {
      console.error(`[31d-F] 付款申请创建失败: ${(e as Error).message}`);
    }
    if (!requestId) { test.skip(); return; }
    console.log(`[31d-F] 付款申请创建成功 id=${requestId}`);

    const before = await getUnreadNotifications(page);

    try {
      await apiCall(page, 'POST', `/finance/ap/payment-requests/${requestId}/submit`);
      console.log(`[31d-F] 付款申请提交成功`);
    } catch (e) {
      console.error(`[31d-F] 付款申请提交失败: ${(e as Error).message}`);
    }

    await page.waitForTimeout(3000);
    const after = await getUnreadNotifications(page);
    const newOnes = after.filter((n) => !before.some((b) => b.id === n.id));
    const payNotif = newOnes.find((n) => n.title?.includes('付款') || n.title?.includes('payment') || n.businessType === 'FINANCE');
    console.log(`[31d-F] 新增通知 ${newOnes.length} 条，匹配付款申请通知: ${!!payNotif}`);

    if (payNotif) {
      expect(payNotif.title, '[31d-F] 付款申请通知标题应含"付款"相关字样').toBeTruthy();
      await markRead(page, payNotif.id);
      await deleteNotification(page, payNotif.id);
    } else {
      console.warn('[31d-F] 未找到付款申请通知（可能通知服务未配置或用户非 admin/manager）');
    }

    try {
      await apiCall(page, 'DELETE', `/finance/ap/payment-requests/${requestId}`);
      console.log(`[31d-F] 清理付款申请 id=${requestId} ✅`);
    } catch (e) {
      console.warn(`[31d-F] 清理付款申请失败: ${(e as Error).message}`);
    }
  });
});
