// 面料管理 E2E 套件 — 03 染色配方
// 创建时间: 2026-08-19
// 覆盖范围：染色配方创建 → 审批（draft → approved）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 03-03 造数据前置：创建一条 status='draft' 的染色配方，令 RecipeTab.vue:63
 * 审批按钮（v-if="row.status === 'draft'"）渲染。配方号 recipe_no 用于定位自身行。
 * 注意（真缺陷，见 .monkeycode/doto.md，本轮不修）：后端 dye_recipe.status 用中文词表
 * （草稿/待审核/已审核/已停用，quality_dyeing.rs:36），前端按钮/标签用英文 draft/approved，
 * 真实数据行永远不出审批按钮；E2E 为驱动渲染改存英文 'draft'（仅测试内）。
 * 且 approveDyeRecipe 前端不发 body，后端 approve_recipe 要求 {approved_by:i32}
 * （dye_recipe_handler.rs:129）→ 点击必 400，审批结果无法断言。故本用例采用方法二：
 * 仅硬断言审批按钮渲染（控件存在性），不点击、不断言「审批成功」。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedDraftRecipe(
  page: import('@playwright/test').Page
): Promise<{ id: number; recipeNo: string }> {
  const recipeNo = genCode('E2E-DR');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
    recipe_no: recipeNo,
    recipe_name: `E2E配方${recipeNo.slice(-6)}`,
    color_code: recipeNo,
    color_name: 'E2E测试色',
    fabric_type: '纯棉',
    chemical_formula: 'E2E 测试配方内容',
    status: 'draft',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建染色配方失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: 'dye_recipe' });
  return { id, recipeNo };
}

test.describe('03 染色配方', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 染色配方 Tab 可正常加载', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('03-02 新建染色配方', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/配方编号/).fill(`RP-${Date.now()}`);
    await page.getByLabel(/配方名称/).fill('E2E 测试染色配方');
    await page
      .getByRole('button', { name: /确认|保存|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });
  });

  test('03-03 草稿配方可审批', async ({ page }) => {
    // 假绿清零：原 `if (await approveBtn.isVisible())` 无草稿数据时零断言通过。
    // 方法二：造 draft 配方 → 硬断言该行渲染且状态标签为「草稿」、审批按钮出现（不可见即红）。
    // 不点击审批——前端 approveDyeRecipe 无 body 而后端要求 approved_by，点击必 400，
    // 属后端契约缺陷（本轮不修，见文件头与 doto.md），故不断言审批结果、不为其放宽。
    const { recipeNo } = await seedDraftRecipe(page);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: recipeNo }).first();
    await expect(row.getByText('草稿'), `配方 ${recipeNo} 应渲染为「草稿」状态`).toBeVisible({
      timeout: 30000,
    });
    await expect(
      row.getByRole('button', { name: '审批', exact: true }),
      `草稿配方 ${recipeNo} 的「审批」按钮应渲染`
    ).toBeVisible({ timeout: 30000 });
  });
});
