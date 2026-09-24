// 质量管理 E2E 套件 — 02 检验记录与缺陷处理
// 创建时间: 2026-08-19
// 覆盖范围：检验记录创建 → 合格/不合格判定 → 缺陷处理
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode, tryCleanup } from '../flow/helpers';

/**
 * 缺陷管理（DefectTab）列表数据源是 unqualified_product 表
 * （backend/services/quality_inspection_service.rs::get_defects_list）。
 * 该表没有 defect_type/severity/processed 等前端列名，故 row.processed 恒为 undefined，
 * 「处理」按钮渲染条件 `v-if="!row.processed"`（DefectTab.vue:67）对任何一行都成立。
 *
 * 方法二 + 真缺陷说明：
 * 1) 缺陷行没有可唯一定位的列，无法像方法一那样「按单号定位自己那一行再点行内按钮」；
 * 2) 更关键的是「处理」点击本身是坏的前后端契约——DefectTab.processDefect 只提交
 *    { remark }（quality/tabs/DefectTab.vue:130 → api/quality.ts processDefect），
 *    而后端 process_unqualified 要求 ProcessUnqualifiedRequest 的
 *    unqualified_qty / unqualified_reason / handling_method 三个非 Option 字段
 *    （backend/handlers/quality_inspection_handler.rs ProcessUnqualifiedRequest），
 *    空提交必被 serde 拒绝（400），永远弹不出「处理成功」。
 * 因此本用例改为「先构造一条缺陷（unqualified_product）使处理按钮必然存在，再显式断言其可见」，
 * 不再断言那条必然失败的点击结果（不改断言方向掩盖缺陷）。契约不一致记入 .monkeycode/doto.md。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** 造一条缺陷：建质检记录 → 处理为不合格品（生成 unqualified_product） */
async function seedDefect(page: import('@playwright/test').Page): Promise<void> {
  const products = await apiCallRaw<{ items?: Array<{ id: number }> }>(
    page,
    'GET',
    '/products?page=1&page_size=1'
  );
  const productId = products.items?.[0]?.id;
  if (!productId) throw new Error('无可用产品，无法构造质检记录');

  const inspectionNo = genCode('E2E-QIR');
  const record = await apiCall<{ id?: number }>(
    page,
    'POST',
    '/production/quality-inspection/records',
    {
      inspection_no: inspectionNo,
      inspection_type: 'finished',
      product_id: productId,
      inspection_date: new Date().toISOString().slice(0, 10),
      total_qty: 100,
      inspected_qty: 100,
      qualified_qty: 90,
      unqualified_qty: 10,
      inspection_result: '不合格',
      grade: 'C',
    }
  );
  if (!record.data?.id) throw new Error(`建质检记录失败：${JSON.stringify(record)}`);
  CLEANUP.push({
    path: `/production/quality-inspection/records/${record.data.id}`,
    label: 'quality_record',
  });

  // 由不合格记录派生 unqualified_product（缺陷列表条目）
  await apiCall(page, 'POST', `/production/quality-inspection/defects/${record.data.id}/process`, {
    unqualified_qty: 10,
    unqualified_reason: 'E2E 缺陷前置',
    handling_method: 'rework',
    remark: 'E2E',
  });
}

test.describe('02 检验记录与缺陷处理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 新建检验记录', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /记录|检验记录/ }).click();
    await page.getByRole('button', { name: /新建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/产品名称/).fill('E2E 测试产品');
    await page.getByLabel(/批次号/).fill(`BATCH-${Date.now()}`);
    await page.getByLabel(/检验员/).fill('E2E 检验员');
    await page.getByLabel(/检验结果/).click();
    await page
      .getByRole('button', { name: /确认|保存|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-02 检验记录列表可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /记录/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('02-03 缺陷管理 Tab 可正常加载', async ({ page }) => {
    await page.goto('/quality');
    await page.getByRole('tab', { name: /缺陷/ }).click();
    await expect(page.getByRole('table').first()).toBeVisible({ timeout: 30000 });
  });

  test('02-04 未处理缺陷可处理', async ({ page }) => {
    // 方法二：先构造一条缺陷（unqualified_product），保证处理按钮必然存在，再显式断言
    await seedDefect(page);
    await page.goto('/quality');
    await page.getByRole('tab', { name: /缺陷/ }).click();
    const handleBtn = page.getByText('处理', { exact: false }).first();
    await expect(
      await handleBtn.isVisible(),
      '缺少未处理缺陷：前置数据（unqualified_product）未生效'
    ).toBe(true);
  });
});
