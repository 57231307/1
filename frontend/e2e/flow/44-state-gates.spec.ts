import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectBadRequest,
  tryCleanup,
  ensureTestEntities,
  getCtx,
  genCode,
} from './helpers';

/**
 * L1-L2 状态门负例矩阵（flow/44-state-gates）
 *
 * 设计原则（rule provenance）：每条用例对应后端一条真实业务规则，标注代码位置。
 * 全部负例断言 HTTP 400+ 且校验错误消息关键词——防止后端规则被静默移除。
 * 数据全部动态创建（零 seed 依赖），清理用 tryCleanup。
 */

const CLEANUP: Array<{ path: string; label: string }> = [];
afterEachCleanupHook();
function afterEachCleanupHook() {
  test.afterEach(async ({ page }) => {
    for (const c of CLEANUP.reverse()) {
      await tryCleanup(page, 'DELETE', c.path, c.label);
    }
    CLEANUP.length = 0;
  });
}

test.describe.serial('44d 凭证状态门负例（voucher_ops/workflow.rs 规则表）', () => {
  let voucherId: number | undefined;

  /** 确保会计科目 1001/1002 存在（subjects 表无 seed，凭证校验科目必须存在） */
  async function ensureSubjects(page: import('@playwright/test').Page): Promise<void> {
    const list = await apiCallRaw<Array<{ code?: string }>>(page, 'GET', '/subjects');
    const codes = new Set((list ?? []).map(s => s.code));
    for (const [code, name] of [
      ['1001', '库存现金'],
      ['1002', '银行存款'],
    ] as const) {
      if (!codes.has(code)) {
        await apiCall(page, 'POST', '/subjects', {
          code,
          name,
          level: 1,
          balance_direction: 'debit',
        });
      }
    }
  }

  /** 创建一张借贷平衡的草稿凭证（可指定不平衡金额注入负例） */
  async function createVoucher(
    page: import('@playwright/test').Page,
    debit: string,
    credit: string
  ): Promise<number | undefined> {
    await ensureSubjects(page);
    const r = await apiCall<{ id?: number }>(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          line_no: 1,
          subject_code: '1001',
          subject_name: '库存现金',
          debit,
          credit: '0',
          summary: '44d 状态门负例-借方',
        },
        {
          line_no: 2,
          subject_code: '1002',
          subject_name: '银行存款',
          debit: '0',
          credit,
          summary: '44d 状态门负例-贷方',
        },
      ],
    });
    const id = r?.data?.id;
    if (id) CLEANUP.push({ path: `/vouchers/${id}`, label: `[44d] 凭证${id}` });
    return id;
  }

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44d-1 借贷不平衡：创建被拒（crud.rs:123-128 create 时校验平衡）', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'POST', '/vouchers', {
      voucher_type: '记',
      voucher_date: new Date().toISOString().slice(0, 10),
      items: [
        { subject_code: '1001', debit: '100.00', credit: '0.00', summary: '44d-1 借方' },
        { subject_code: '1002', debit: '0.00', credit: '99.00', summary: '44d-1 贷方' },
      ],
    });
    expectBadRequest(r, '借贷不平衡凭证创建应被拒（借100 != 贷99）');
  });

  test('44d-2 借贷平衡：draft 直接过账被拒（workflow.rs:131-133 仅 reviewed 可过账）', async ({
    page,
  }) => {
    const id = await createVoucher(page, '100.00', '100.00');
    expect(id, '平衡凭证创建失败').toBeTruthy();
    const r = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/post`);
    expectBadRequest(r, 'draft 凭证直接过账应被拒（仅 reviewed 可过账）');
  });

  test('44d-3 状态不可逆：draft 直接审核被拒（需先 submit）', async ({ page }) => {
    const id = await createVoucher(page, '100.00', '100.00');
    expect(id, '凭证创建失败').toBeTruthy();
    const r = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/review`);
    expectBadRequest(r, 'draft 凭证直接审核应被拒（需先 submit → reviewed）');
  });

  test('44d-4 状态机不可逆：提交→提交重复被拒（防重复提交）', async ({ page }) => {
    const id = await createVoucher(page, '100.00', '100.00');
    expect(id, '凭证创建失败').toBeTruthy();
    const r1 = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/submit`);
    expect(r1.status, '平衡凭证首次提交应成功').toBeLessThan(300);
    const r2 = await apiCallExpectFail(page, 'POST', `/vouchers/${id}/submit`);
    expectBadRequest(r2, '已提交凭证再次提交应被拒（draft→submitted 不可逆）');
  });
});

test.describe.serial('44b 采购订单状态门负例（po/contract.rs + receipt.rs 规则表）', () => {
  let orderId: number | undefined;

  async function createOrder(page: import('@playwright/test').Page): Promise<number | undefined> {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const r = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      items: [
        {
          material_id: ctx.productIds[0],
          quantity: 100,
          unit_price: '10.50',
        },
      ],
    });
    const id = r?.data?.id;
    if (id) CLEANUP.push({ path: `/purchase/orders/${id}`, label: `[44b] PO${id}` });
    return id;
  }

  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44b-1 无明细提交被拒（contract.rs:86-88 至少一行明细）', async ({ page }) => {
    await ensureTestEntities(page);
    const ctx = getCtx();
    const r = await apiCallExpectFail(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      expected_delivery_date: new Date(Date.now() + 7 * 86400000).toISOString().split('T')[0],
      items: [],
    });
    expectBadRequest(r, '空明细采购订单创建/提交应被拒');
  });

  test('44b-2 二次提交拦截（contract.rs:63-71 仅 DRAFT/REJECTED 可提交）', async ({ page }) => {
    orderId = await createOrder(page);
    expect(orderId, 'PO 创建失败').toBeTruthy();
    const r1 = await apiCallExpectFail(page, 'POST', `/purchase/orders/${orderId}/submit`);
    expect(r1.status, '首次提交应成功').toBeLessThan(300);
    const r2 = await apiCallExpectFail(page, 'POST', `/purchase/orders/${orderId}/submit`);
    expectBadRequest(r2, '已提交订单二次提交应被拒（防重复提交幂等）');
  });

  test('44b-3 DRAFT 直接审批被拒（contract.rs:160-165 仅 PENDING_APPROVAL 可审批）', async ({
    page,
  }) => {
    const id = await createOrder(page);
    expect(id, 'PO 创建失败').toBeTruthy();
    const r = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/approve`);
    expectBadRequest(r, 'DRAFT 订单直接审批应被状态门拒绝');
  });

  test('44b-4 不存在订单操作返回 4xx（路由健壮性）', async ({ page }) => {
    const r = await apiCallExpectFail(page, 'POST', '/purchase/orders/99999999/approve');
    expectBadRequest(r, '不存在订单的审批应 4xx');
  });

  test('44b-5 close 状态门（lifecycle.rs:43 订单状态不允许关闭）', async ({ page }) => {
    const id = await createOrder(page);
    expect(id, 'PO 创建失败').toBeTruthy();
    // DRAFT 不在可关闭状态集
    const r = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/close`);
    expectBadRequest(r, 'DRAFT 订单关闭应被拒（订单状态不允许关闭）');
  });

  test('44b-6 取消状态门：DRAFT 可取消（正向）+ 二次取消被拒', async ({ page }) => {
    const id = await createOrder(page);
    expect(id, 'PO 创建失败').toBeTruthy();
    const r1 = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/cancel`, {
      reason: 'E2E 44b-6 取消测试',
    });
    expect(r1.status, 'DRAFT 取消应成功（contract.rs:271-274）').toBeLessThan(300);
    const r2 = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/cancel`, {
      reason: 'E2E 44b-6 二次取消',
    });
    expectBadRequest(r2, 'CANCELLED 为终态，二次取消应被拒');
    // 取消后提交被拒（终态拦截）
    const r3 = await apiCallExpectFail(page, 'POST', `/purchase/orders/${id}/submit`);
    expectBadRequest(r3, '已取消订单提交应被拒');
  });
});

test.describe.serial('44g 唯一性/幂等负例（盘点规则抽样）', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('44g-1 部门名称重复创建被拒（department_service.rs:83-91 先查后插）', async ({ page }) => {
    const name = `44g唯一部门${genCode('U')}`;
    const r1 = await apiCall<{ id?: number }>(page, 'POST', '/departments', {
      name,
      description: '44g 唯一性负例',
    });
    const id = r1?.data?.id;
    expect(id, '首次创建应成功').toBeTruthy();
    if (id) CLEANUP.push({ path: `/departments/${id}`, label: '[44g] 部门' });
    const r2 = await apiCallExpectFail(page, 'POST', '/departments', {
      name,
      description: '44g 唯一性负例-重名',
    });
    expectBadRequest(r2, '重名部门创建应被拒');
  });

  test('44g-2 凭证列表可达（对照 44d 负例的 sanity check）', async ({ page }) => {
    const list = await apiCallExpectFail(page, 'GET', '/vouchers?page=1&page_size=1');
    expect(list.status, '凭证列表应可达').toBeLessThan(300);
  });
});
