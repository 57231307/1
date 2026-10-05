// 采购 E2E 套件 — 09 采购价格（专用页 /purchase-price）
// 迁移来源：e2e/purchase-ext/02-price.spec.ts（枢纽 Tab 已删除）
//   02-01 Tab 加载          → 09-01
//   02-02 新建采购价格      → 09-02
//
// 状态机真值（backend/src/models/status/sales.rs::price_approval，全小写 pending/approved/rejected/inactive）：
//   create_price 写 pending（purchase_price_service.rs::create_price 写 price_approval::PENDING），
//   批准端点写 approved（::approve_price，含"仅 pending 可批"状态门；approve 端点 approval_reason 必填、
//     approved=false 一律 400 VALIDATION_ERROR——approve 单一职责只受理批准）、
//   拒绝端点写 rejected（::reject_price，同"仅 pending 可拒"状态门，reason 必填落 rejected_reason 列）、
//   停用写 inactive（::update_price 按 price_approval::ALL 白名单透传）。前端比较对象见 views/purchase-price/composables/ppFmts.ts。
// 后端 list_prices 仅支持 product_id / supplier_id / status 过滤（无 keyword），
// 故回读按建单返回的 product_id 检索列表 + 按 id 回读详情，二者结合确认真实落库。
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  ensureTestEntities,
  expectStateGateRejection,
  failureCode,
  getCtx,
  tryCleanup,
} from '../flow/helpers';
import { pickSelect } from '../flow/ui-helpers';
import type { Page } from '@playwright/test';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * 展开当前可见 el-select 并点选其第一个选项（建单只需任一有效产品/供应商）。
 * 原实现点完外层 .el-select 后直接 `option.click()`，dropdown 内首个 option 尚在动画
 * → "element is not stable" 30s 超时假红（#4654 09-02 根因）。改用向后兼容 pickSelect：
 * 点 .el-select__wrapper（非外层/只读 input）并先 waitFor option 可见再点，消除不稳定。
 */
async function pickFirstOption(page: Page, labelText: string): Promise<void> {
  const dlg = page.locator('.el-dialog:visible');
  const trigger = dlg.locator('.el-form-item').filter({ hasText: labelText }).first();
  await pickSelect(page, trigger.locator('.el-select').first());
}

/**
 * API 建一条 pending 采购价格并登记清理，返回 {id, price}。
 * price 取 `9876.` + 毫秒尾数（恰 6 位小数）：列表按 ID DESC 排序
 * （purchase_price_service.rs::get_prices_list 的 order_by Id Desc），新行必居第一页；
 * 唯一小数位保证表格行仅凭价格文本（formatCurrency 定长 ¥9876.xxxxxx）即可
 * 与其他历史行区分，不依赖产品/供应商名（同产品可有多条价目）。
 * 断言建单出参 status=pending（后端写入侧 price_approval::PENDING），
 * 前置不满足立即抛错，不带病继续。
 */
async function seedPendingPrice(page: Page): Promise<{ id: number; price: string }> {
  const ctx = getCtx();
  if (!ctx.supplierId) throw new Error('前置缺失：ctx.supplierId 未就绪');
  if (!ctx.productIds[0]) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
  const price = `9876.${String(Date.now() % 1_000_000).padStart(6, '0')}`;
  const created = await apiCallRaw<{ id: number; status: string }>(
    page,
    'POST',
    '/purchase/purchase-prices',
    {
      product_id: ctx.productIds[0],
      supplier_id: ctx.supplierId,
      price,
      unit: 'meter',
      price_type: 'STANDARD',
    }
  );
  if (!created.id) throw new Error(`价目 seed 建单未返回 id：${JSON.stringify(created)}`);
  if (created.status !== 'pending') {
    throw new Error(`新建价格状态应为 pending（实际 ${created.status}，id=${created.id}）`);
  }
  CLEANUP.push({ path: `/purchase/purchase-prices/${created.id}`, label: 'purchase_price' });
  return { id: created.id, price };
}

test.describe('09 采购价格', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
    await ensureTestEntities(page);
  });

  test('09-01 进入采购价格列表页', async ({ page }) => {
    await page.goto('/purchase-price');
    await expect(page.getByText('采购价格管理')).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: '新建价格' })).toBeVisible();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
  });

  test('09-02 新建采购价格（UI 建单 → 列表按 product_id 检索 + 详情回读确认落库）', async ({
    page,
  }) => {
    await page.goto('/purchase-price');
    await expect(page.getByText('采购价格管理')).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: '新建价格' }).click();
    const dlg = page.locator('.el-dialog:visible');
    await expect(dlg).toBeVisible({ timeout: 30000 });

    await pickFirstOption(page, '产品');
    await pickFirstOption(page, '供应商');
    await dlg
      .locator('.el-form-item')
      .filter({ hasText: '采购价格' })
      .first()
      .locator('input')
      .first()
      .fill('123.456');
    // 生效日期（后端建单默认写当天，但表单已提供且原用例填此字段，保持等价）
    const effInput = dlg
      .locator('.el-form-item')
      .filter({ hasText: '生效日期' })
      .first()
      .locator('input')
      .first();
    await effInput.click();
    await effInput.fill('2026-08-01');
    await page.keyboard.press('Enter');

    // 只绑 200：CSRF token 一次性消费下 UI 首个 POST 可能 403（backend/src/middleware/csrf.rs 的
    // consume_csrf_token 一次性消费与轮换，前端 axios 拦截器静默重放，request.ts:197-223）。不过滤状态码就会命中
    // 那个 data=null 的 403 中间态，"建单响应未返回 id"成为真红假象（同 11-03 族）。
    // 真被业务拒绝时既无 200 也无 toast，两处断言都判红——不放宽，只收紧。
    const createdResp = page
      .waitForResponse(
        res =>
          res.url().includes('/purchase/purchase-prices') &&
          res.request().method() === 'POST' &&
          res.status() === 200,
        { timeout: 30000 }
      )
      .catch(() => null);
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('保存成功')).toBeVisible({ timeout: 30000 });

    const resp = await createdResp;
    expect(resp, '未捕获到建单 POST 响应').not.toBeNull();
    const body = (await resp!.json()) as {
      data: {
        id: number;
        product_id: number;
        supplier_id: number;
        price: number | string;
        status: string;
      };
    };
    const created = body.data;
    expect(created?.id, '建单响应未返回 id').toBeTruthy();
    CLEANUP.push({ path: `/purchase/purchase-prices/${created.id}`, label: 'purchase_price' });

    // 详情回读：价格值正确，状态为 pending（后端写入侧 price_approval::PENDING）
    expect(Number(created.price), '建单返回价格应与提交值一致').toBeCloseTo(123.456, 2);
    expect(created.status, '新建价格状态应为 pending').toBe('pending');

    // 列表检索回读：按 product_id 过滤（后端真实支持的过滤键），确认新记录出现在列表中
    const list = await apiCallRaw<Array<{ id: number }>>(
      page,
      'GET',
      `/purchase/purchase-prices?product_id=${created.product_id}&page=1&page_size=50`
    );
    expect(
      list.some(r => r.id === created.id),
      `列表按 product_id=${created.product_id} 检索未命中新建价格 ${created.id}`
    ).toBe(true);
  });

  test('09-03 审批 pending 采购价格（UI 点审批 → API 回读 approved + 列表行状态回显）', async ({
    page,
  }) => {
    const { id, price } = await seedPendingPrice(page);

    await page.goto('/purchase-price');
    await expect(page.getByText('采购价格管理')).toBeVisible({ timeout: 30000 });
    const row = page
      .locator('.el-table__row')
      .filter({ hasText: `¥${price}` })
      .first();
    await expect(row, '新建 pending 行应出现在列表第一页（后端 ID DESC）').toBeVisible({
      timeout: 30000,
    });

    // 缺陷面本体（F3 修复前 pending 行零出边=死态）：审批出边出现，且编辑/停用
    // 仍按 approved 门控缺席——同时钉住"补了审批"与"没把其它门控顺手放宽"。
    await expect(row.getByRole('button', { name: '审批' })).toBeVisible();
    await expect(row.getByRole('button', { name: '编辑' })).toHaveCount(0);
    await expect(row.getByRole('button', { name: '停用' })).toHaveCount(0);

    // 只绑 200：CSRF token 一次性消费下 UI 首个 POST 可能 403 中间态（同 09-02 注）
    const approveResp = page
      .waitForResponse(
        res =>
          res.url().includes(`/purchase/purchase-prices/${id}/approve`) &&
          res.request().method() === 'POST' &&
          res.status() === 200,
        { timeout: 30000 }
      )
      .catch(() => null);
    await row.getByRole('button', { name: '审批' }).click();
    const msgBox = page.locator('.el-message-box:visible');
    // approve 现要求通过理由必填：usePpProc.handleApprove 先经 promptApprovalReason(true) 采集
    // （ElMessageBox.prompt，textarea），空/纯空白被 inputValidator 拦在提交前 ⇒ 必须先填理由再确定
    await expect(msgBox, '点审批应弹「通过理由」输入框（填写→批准，取消→中止）').toBeVisible({
      timeout: 10000,
    });
    await msgBox.getByRole('textbox').fill('E2E 采购价目通过理由');
    await msgBox.locator('.el-message-box__btns button:has-text("确定")').click();

    const resp = await approveResp;
    expect(resp, '未捕获到审批 POST 响应').not.toBeNull();
    const envelope = (await resp!.json()) as { code: number };
    expect(envelope.code, '审批端点成功信封 code 应为 200').toBe(200);

    // toast 仅作即时反馈信号——本仓红线：禁止只断 toast
    await expect(page.getByText('审批成功')).toBeVisible({ timeout: 10000 });

    // API 回读：状态迁移真实落库，审批留痕字段成对写入（approve_price :194-196）
    const after = await apiCallRaw<{
      status: string;
      approved_by: number | null;
      approved_at: string | null;
    }>(page, 'GET', `/purchase/purchase-prices/${id}`);
    expect(after.status, '审批后状态应落库为 approved').toBe('approved');
    expect(after.approved_by, '审批后 approved_by 应留痕（非空）').not.toBeNull();
    expect(after.approved_at, '审批后 approved_at 应留痕（非空）').not.toBeNull();

    // UI 回读：成功链刷新列表（usePpProc.handleApprove → refresh.getList），
    // 行状态标签翻为"已批准"，审批出边消失、编辑/停用出边出现（状态机出边闭环）
    await expect(row.getByText('已批准'), '行状态标签应回显为已批准').toBeVisible({
      timeout: 30000,
    });
    await expect(row.getByRole('button', { name: '审批' })).toHaveCount(0);
    await expect(row.getByRole('button', { name: '编辑' })).toBeVisible();
    await expect(row.getByRole('button', { name: '停用' })).toBeVisible();
  });

  test('09-04 审批端点状态门与仅批准语义（负例按 status + 信封机器码双钉，不断言文案）', async ({
    page,
  }) => {
    const { id } = await seedPendingPrice(page);

    // 先经 API 批准一次（pending 门放行 → approved），回读钉住正例基线。
    // approve 现要求 approval_reason 必填（缺失/空白即 400 VALIDATION_ERROR），基线批准须带理由。
    await apiCall(page, 'POST', `/purchase/purchase-prices/${id}/approve`, {
      approved: true,
      approval_reason: 'E2E 采购价目批准基线',
    });
    const afterFirst = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/purchase/purchase-prices/${id}`
    );
    expect(afterFirst.status, 'API 批准应先流转为 approved（回读钉，非 toast）').toBe('approved');

    // 二次批准：命中 approve_price 状态门（purchase_price_service.rs::approve_price 要求
    // 前置状态恰为 price_approval::PENDING，否则 AppError::business）⇒ 契约钉 HTTP 400
    //（utils/error.rs 的 BusinessError 映射）+ 机器码 BUSINESS_ERROR
    //（utils/error.rs 的 code 映射表）。文案永久脱敏，禁止断言 message 原文。
    // 必须带 approval_reason：handler 先于状态门校验通过理由必填（缺理由会先撞 400
    // VALIDATION_ERROR，触达不到状态门）——补理由让链真正走到 service 状态门后仍断 BUSINESS_ERROR。
    const repeat = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/purchase-prices/${id}/approve`,
      {
        approved: true,
        approval_reason: 'E2E 采购价目重复批准负例',
      }
    );
    expectStateGateRejection(repeat, '对已批准采购价格再次批准应被状态门拒绝');
    expect(
      failureCode(repeat),
      '状态门拒绝机器码应为 BUSINESS_ERROR（AppError::business），非状态门族内其它码'
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 拒绝不得破坏已达成状态（事务未提交即返回）
    const stillApproved = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/purchase/purchase-prices/${id}`
    );
    expect(stillApproved.status, '被拒的二次批准不得改变已落库状态').toBe('approved');

    // approved=false：approve 端点单一职责——本端点只受理批准，approved=false 不做任何写入，
    // 直接 validation_displayable ⇒ 400 + VALIDATION_ERROR（purchase_price_handler.rs::approve_price，
    // 映射见 utils/error.rs）。拒绝动作有独立 /reject 端点（见 09-05），不经由 approve。
    const rejectAttempt = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/purchase-prices/${id}/approve`,
      { approved: false }
    );
    expect(rejectAttempt.status, 'approved=false 应返回 HTTP 400').toBe(400);
    expect(failureCode(rejectAttempt), 'approved=false 机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
  });

  test('09-05 拒绝 pending 采购价格（reject 端点：理由必填落 rejected_reason + 空理由/缺通过理由负例）', async ({
    page,
  }) => {
    const { id } = await seedPendingPrice(page);

    // 正例：带理由拒绝 → pending→rejected 终态 + 逐字回读 rejected_reason（不止断状态，防"理由丢失用例仍绿"）
    const rejectReason = `E2E-PP-拒绝-${Date.now()}`;
    await apiCall(page, 'POST', `/purchase/purchase-prices/${id}/reject`, { reason: rejectReason });
    const after = await apiCallRaw<{
      status: string;
      rejected_reason: string | null;
      approval_reason: string | null;
    }>(page, 'GET', `/purchase/purchase-prices/${id}`);
    expect(after.status, '拒绝后状态应落库为 rejected').toBe('rejected');
    expect(after.rejected_reason, '拒绝理由应逐字落 rejected_reason 专列').toBe(rejectReason);
    // 两动作两列：拒绝只写 rejected_reason，不得挪用 approval_reason
    expect(after.approval_reason, '拒绝不得写入 approval_reason（两动作两列）').toBeNull();

    // 状态门：rejected 终态再拒绝 → 400 BUSINESS_ERROR（reject_price 仅 pending 可拒）
    const repeatReject = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/purchase-prices/${id}/reject`,
      { reason: 'E2E-PP-重复拒绝' }
    );
    expectStateGateRejection(repeatReject, '已拒绝价目再次拒绝应被状态门拦截');
    expect(failureCode(repeatReject), '重复拒绝机器码应为 BUSINESS_ERROR').toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );

    // 空/纯空白理由 → 400 VALIDATION_ERROR（reason 服务端必填，trim 非空门）
    const newPending = await seedPendingPrice(page);
    const emptyReason = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/purchase-prices/${newPending.id}/reject`,
      { reason: '   ' }
    );
    expect(emptyReason.status, '纯空白拒绝理由应返回 HTTP 400').toBe(400);
    expect(failureCode(emptyReason), '纯空白拒绝理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );

    // approve 缺通过理由 → 400 VALIDATION_ERROR（必填档）
    const approveNoReason = await apiCallExpectFail(
      page,
      'POST',
      `/purchase/purchase-prices/${newPending.id}/approve`,
      { approved: true }
    );
    expect(approveNoReason.status, 'approve 缺 approval_reason 应返回 HTTP 400').toBe(400);
    expect(failureCode(approveNoReason), 'approve 缺理由机器码应为 VALIDATION_ERROR').toBe(
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    // 缺理由的 approve 被拒后状态不得改变（仍是 pending）
    const stillPending = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/purchase/purchase-prices/${newPending.id}`
    );
    expect(stillPending.status, '缺理由 approve 被拒不得改变状态').toBe('pending');
  });
});
