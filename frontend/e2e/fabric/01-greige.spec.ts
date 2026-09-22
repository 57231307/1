// 面料管理 E2E 套件 — 01 坯布管理
// 创建时间: 2026-08-19
// 覆盖范围：坯布创建 → 入库 → 出库
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 01-03 / 01-04 造数据前置：创建一条坯布，令 GreigeTab.vue 行内入库/出库按钮渲染。
 * 注意（真缺陷，见 .monkeycode/doto.md，本轮不修，均在后端/契约层不在改动范围）：
 *  1) 编号列读 fabric_code，后端返回 fabric_no（greige_fabric.rs:14）→ 编号列恒空；
 *     供应商/库存列同理（supplier_name/quantity 后端无此字段）→ 故本用例按 fabric_name 定位。
 *  2) 前端 handleStock（fabric/index.vue:124）入库/出库发 { quantity }，
 *     后端 stock_in 要求 { warehouse_id, weight_kg, length_m }、stock_out 要求 { weight_kg/length_m }
 *     （greige_fabric_handler.rs:110/123）→ 点击必 400，入库/出库结果无法断言。
 * 采用方法二：仅硬断言坯布行与入库/出库按钮渲染（Tier A 无条件控件，不可见即红），不点击。
 * status 用非「在库」值以便用例后 DELETE 清理（后端仅拦截「在库」删除）。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedGreige(
  page: import('@playwright/test').Page
): Promise<{ id: number; name: string }> {
  const suffix = genCode('E2E-GF').slice(-6);
  const name = `E2E坯布${suffix}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/greige-fabrics', {
    fabric_no: `E2E-GF${suffix}`,
    fabric_name: name,
    fabric_type: '梭织',
    status: 'pending',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建坯布失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/production/greige-fabrics/${id}`, label: 'greige_fabric' });
  return { id, name };
}

test.describe('01 坯布管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入面料管理页面', async ({ page }) => {
    await page.goto('/fabric');
    await expect(page.getByText(/面料/)).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('tab', { name: /坯布/ })).toBeVisible();
  });

  test('01-02 新建坯布', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/面料编码/).fill(`FB-${Date.now()}`);
    await page.getByLabel(/面料名称/).fill('E2E 测试坯布');
    await page.getByLabel(/数量/).fill('1000');
    await page
      .getByRole('button', { name: /确认|保存|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });
  });

  test('01-03 坯布入库操作', async ({ page }) => {
    // 假绿清零：原 `const stockInBtn = page.getByRole('link', { name: /入库/ });
    // if (await stockInBtn.isVisible())` 两处恒空转——无数据时按钮不渲染；且入库控件是
    // el-button（role=button），getByRole('link') 永远匹配不到。改为造坯布 → 硬断言
    // 入库按钮渲染。点击发 { quantity } 与后端契约不符（见文件头缺陷2），故不点击/不断言结果。
    const { name } = await seedGreige(page);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: name }).first();
    await expect(
      row.getByRole('button', { name: '入库', exact: true }),
      `坯布 ${name} 的「入库」按钮应渲染`
    ).toBeVisible({ timeout: 30000 });
  });

  test('01-04 坯布出库操作', async ({ page }) => {
    // 假绿清零：同 01-03，出库按钮同为 el-button（role=button）。造坯布 → 硬断言出库按钮渲染；
    // 点击 body 契约不符（见文件头缺陷2），不点击/不断言结果。
    const { name } = await seedGreige(page);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /坯布/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: name }).first();
    await expect(
      row.getByRole('button', { name: '出库', exact: true }),
      `坯布 ${name} 的「出库」按钮应渲染`
    ).toBeVisible({ timeout: 30000 });
  });
});
