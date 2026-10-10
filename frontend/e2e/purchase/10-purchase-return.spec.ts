// 采购 E2E 套件 — 10 采购退货（专用页 /purchase-return）
// 迁移来源：e2e/purchase-ext/03-return.spec.ts（枢纽 Tab 已删除）
//   03-01 Tab 加载        → 10-01
//   03-02 新建采购退货单  → 10-02
//
// 状态机真值（backend/src/models/status/purchase_inventory.rs 的 purchase_return，全小写：
//   draft/submitted/approved/rejected）：create_return 写 draft（services/purchase_return_service.rs:127）。
// 审批链（reject 端点仅 submitted 起拒）：submit DRAFT→SUBMITTED；approve SUBMITTED→APPROVED
//   （approval_reason 选填，落 approval_reason 列，留空省略）；reject SUBMITTED→REJECTED，拒绝理由
//   必填落 rejected_reason 专列（purchase_return_service.rs::reject_return），reason_detail 回归建单
//   明细语义不再被覆盖（本轮止毁缺陷回归——旧实现把拒绝理由写进 reason_detail，覆盖退货原因）。
//   approve 与 reject 互斥出边（APPROVED 不可再 reject），故本波分两条链各自验证。
// 建单必填（CreatePurchaseReturnRequest）：supplier_id、reason_type、return_date 为非 Option，
//   order_id/receipt_id/warehouse_id 为 Option；前端表单把“采购订单”作为必填项驱动，
//   选中订单后供应商由 handleOrderChange 自动派生（usePrRtn.ts:341）。
// 退货原因取值见 constants/return-reason.ts，value 为中文业务词，落库原值即中文（此处取“色差”）。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  failureCode,
  APP_ERROR_CODES,
  tryCleanup,
} from '../flow/helpers';
import { pickSelect, pickSelectIn } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

test.describe('10 采购退货', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('10-01 进入采购退货列表页', async ({ page }) => {
    await page.goto('/purchase-return');
    await expect(page.getByText('采购退货').first()).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建退货单' })).toBeVisible();
    await expect(page.locator('.el-table').first()).toBeVisible({ timeout: 30000 });
  });

  test('10-02 新建采购退货单（UI 建单 → 详情回读 + 列表按单号检索确认落库）', async ({ page }) => {
    await page.goto('/purchase-return');
    await expect(page.getByRole('button', { name: '新建退货单' })).toBeVisible();
    await page.getByRole('button', { name: '新建退货单' }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });

    // 采购订单（filterable el-select，label '采购订单'）：pickSelectIn 点 wrapper→选首项，
    // 选中后供应商由后端/handleOrderChange 自动派生。
    await pickSelectIn(dlg, page, '采购订单');

    // 退货日期（该 date-picker 带 value-format=YYYY-MM-DD，填入即字符串，无时区退化）
    const dateInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货日期' })
      .first()
      .locator('input')
      .first();
    await dateInput.click();
    await dateInput.fill('2026-08-19');
    await page.keyboard.press('Enter');

    // 原因类型（固定中文词表，label '原因类型'，取“色差”）+ 退货原因详情
    await pickSelectIn(dlg, page, '原因类型', { optionText: '色差' });
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '退货原因' })
      .first()
      .locator('textarea')
      .first()
      .fill('E2E 迁移用例：色差退货');

    // 退货明细：选中订单后已自动生成一行，补一行并给首个产品赋值（quantity 默认 1）。
    // 明细行内 el-select 无 form-item label 可锚定，用向后兼容 pickSelect：点其
    // .el-select__wrapper（非外层/只读 input）并先 waitFor option 可见再点，消除
    // 原 `.el-select').click()`+`dropdown__item').first().click()` 的“element not
    // stable / not visible” 30s 超时（#4654 10-02 同类根因）。
    await dlg.getByRole('button', { name: '添加明细' }).click();
    await pickSelect(page, dlg.locator('.el-table .el-select').last());

    const createdResp = page
      .waitForResponse(
        res =>
          res.request().method() === 'POST' &&
          res.url().includes('/purchase/returns') &&
          !res.url().includes('/items') &&
          // CSRF 令牌一次性消费：并发下 UI 提交首个 POST 可能返回 403（后端经
          // X-New-CSRF-Token 下发恢复头，前端 axios 拦截器自动重放第二次 POST 成功，
          // 见 api/request.ts:197-223）。此处若不过滤，waitForResponse 会命中 403 中间态——
          // 它是 AppError 形状、data=null，读 id 落空即「建单响应未返回 id」的真红假象。
          // 后端 create_purchase_return（handlers/purchase_return_handler.rs:79）经
          // ApiResponse::success_with_message 序列化 purchase_return::Model，data.id 确凿返回；
          // 与 flow/ui-helpers waitCreateResponse 同则跳过 403，绑定真实业务响应（200 或真实 4xx 失败）。
          res.status() !== 403,
        { timeout: 30000 }
      )
      .catch(() => null);

    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('创建成功')).toBeVisible({ timeout: 30000 });

    const resp = await createdResp;
    expect(resp, '未捕获到建单 POST 响应').not.toBeNull();
    const body = (await resp!.json()) as {
      data: {
        id: number;
        return_no: string;
        supplier_id: number;
        reason_type: string;
        return_status: string;
      };
    };
    const created = body.data;
    expect(created?.id, '建单响应未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${created.id}`, label: 'purchase_return' });

    // 详情回读：状态 draft、原因类型逐字等于落库中文“色差”、供应商为真实外键
    const detail = await apiCallRaw<{
      id: number;
      return_no: string;
      return_status: string;
      reason_type: string;
      supplier_id: number;
    }>(page, 'GET', `/purchase/returns/${created.id}`);
    expect(detail.id).toBe(created.id);
    expect(detail.return_status, '新建退货单应为 draft 态').toBe('draft');
    expect(detail.reason_type, '退货原因类型应逐字落库为中文“色差”').toBe('色差');
    expect(detail.supplier_id, '供应商应为后端派生的真实 id（>0）').toBeGreaterThan(0);

    // 列表检索回读：后端 list 支持 keyword 匹配 return_no（PaginatedResponse ⇒ data.items）
    const list = await apiCallRaw<{
      items: Array<{ id: number; return_no: string }>;
      total: number;
    }>(page, 'GET', `/purchase/returns?keyword=${created.return_no}&page=1&page_size=20`);
    expect(
      list.items.some(r => r.id === created.id),
      `列表按单号 ${created.return_no} 检索未命中新建退货单`
    ).toBe(true);
  });

  test('10-03 reject 链：submit→拒绝（理由必填落 rejected_reason + reason_detail 未被覆盖 + 空理由负例）', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
    // 建一张 draft 退货单，reason_detail 写入唯一码——reject 后必须逐字不变（双列锁另一列）
    const reasonDetail = `E2E-PR10-退货明细原因-${Date.now()}`;
    const created = await apiCall<{ id?: number; return_no?: string; return_status: string }>(
      page,
      'POST',
      '/purchase/returns',
      {
        supplier_id: ctx.supplierId,
        return_date: new Date().toISOString().slice(0, 10),
        reason_type: '色差',
        reason_detail: reasonDetail,
      }
    );
    const id = created.data?.id;
    expect(id, '退货单建单未返回 id').toBeTruthy();
    expect(created.data.return_status, '新建退货单应为 draft 态').toBe('draft');
    CLEANUP.push({ path: `/purchase/returns/${id}`, label: 'purchase_return' });

    // draft → submitted（reject 状态门仅接受 submitted）
    await apiCall(page, 'POST', `/purchase/returns/${id}/submit`);

    // 带理由拒绝 → submitted→rejected + 逐字回读 rejected_reason；reason_detail 不得被覆盖
    const rejectReason = `E2E-PR10-拒绝理由-${Date.now()}`;
    await apiCall(page, 'POST', `/purchase/returns/${id}/reject`, { reason: rejectReason });
    const after = await apiCallRaw<{
      return_status: string;
      rejected_reason: string | null;
      reason_detail: string | null;
      approval_reason: string | null;
    }>(page, 'GET', `/purchase/returns/${id}`);
    expect(after.return_status, '拒绝后状态应为 rejected').toBe('rejected');
    expect(after.rejected_reason, '拒绝理由应逐字落 rejected_reason 专列').toBe(rejectReason);
    // 止毁双列锁：reject 不得再覆盖 reason_detail（应保持建单值）
    expect(after.reason_detail, 'reject 不得覆盖 reason_detail（应保持建单值）').toBe(reasonDetail);
    expect(after.approval_reason, 'reject 不得写入 approval_reason（两动作两列）').toBeNull();

    // 空/纯空白理由 → 400 VALIDATION_ERROR（reject 服务端必填理由）
    const c2 = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', {
      supplier_id: ctx.supplierId,
      return_date: new Date().toISOString().slice(0, 10),
      reason_type: '色差',
    });
    const id2 = c2.data?.id;
    expect(id2, '第二张退货单建单未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${id2}`, label: 'purchase_return' });
    await apiCall(page, 'POST', `/purchase/returns/${id2}/submit`);
    const emptyReason = await apiCallExpectFail(page, 'POST', `/purchase/returns/${id2}/reject`, {
      reason: '   ',
    });
    expect(emptyReason.status, '纯空白拒绝理由应返回 HTTP 400').toBe(400);
    expect(failureCode(emptyReason), '纯空白拒绝理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    const stillSubmitted = await apiCallRaw<{ return_status: string }>(
      page,
      'GET',
      `/purchase/returns/${id2}`
    );
    expect(stillSubmitted.return_status, '空理由 reject 被拒不得改变状态').toBe('submitted');
  });

  test('10-04 approve 链（选填理由）：缺 approval_reason 不应触发 400 VALIDATION_ERROR（留空省略档）', async ({
    page,
  }) => {
    const ctx = getCtx();
    if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/returns', {
      supplier_id: ctx.supplierId,
      return_date: new Date().toISOString().slice(0, 10),
      reason_type: '色差',
    });
    const id = created.data?.id;
    expect(id, '退货单建单未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/returns/${id}`, label: 'purchase_return' });
    await apiCall(page, 'POST', `/purchase/returns/${id}/submit`);

    // approve 的 approval_reason 为选填：缺理由不得被必填校验拦成 400 VALIDATION_ERROR。
    // （本仓 reject 域必填档才有该组合；选填档必须能透传到 service 层，由库存/状态门决定结果。）
    const approveRes = await apiCallExpectFail(page, 'POST', `/purchase/returns/${id}/approve`);
    expect(
      approveRes.status === 400 && failureCode(approveRes) === APP_ERROR_CODES.VALIDATION_ERROR,
      `采购退货 approve 为选填理由档，缺 approval_reason 不应命中 400 VALIDATION_ERROR，实际 status=${approveRes.status} code=${failureCode(
        approveRes
      )}`
    ).toBe(false);
  });
});
