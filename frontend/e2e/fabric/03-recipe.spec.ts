// 面料管理 E2E 套件 — 03 染色配方
// 创建时间: 2026-08-19
// 覆盖范围：染色配方创建 → 审批（draft → approved，端到端点击 + 断真实 toast）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 03-03 造数据前置：创建一条 status='draft' 的染色配方，令 RecipeTab.vue 审批按钮
 * （canApprove：draft / pending_approval）渲染。配方号 recipe_no 用于定位自身行。
 *
 * 缺陷已修复（v15 词表收口）：
 * - 后端 dye_recipe.status 曾为中文词表（quality_dyeing.rs::dye_recipe），前端按钮/标签用
 *   英文 → 真实数据行永不渲染审批按钮。现统一为小写英文闭合词表，历史中文值由迁移回填，
 *   中文仅在 i18n 展示层（status='draft' 经 i18n 显示为「草稿」）。
 * - approveDyeRecipe 曾不发 body，后端 approve_recipe 要求 {approved_by:i32} → 点击必 400。
 *   现前端从登录用户 userStore.userInfo.id 取真实 ID 传入（applyAuthMocks 下 /auth/me 返回 id=1）。
 * 故本用例恢复端到端：硬断言状态标签 + 点击审批 + 断真实「审批成功」toast。
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

  test('03-03 草稿配方可审批（端到端点击 + 断真实 toast）', async ({ page }) => {
    // 假绿清零：原 `if (await approveBtn.isVisible())` 无草稿数据时零断言通过。
    // 词表收口 + approve body 补齐后：造 draft 配方 → 硬断言状态标签为「草稿」、审批按钮渲染
    // → 真实点击审批 + 确认 → 断成功 toast（点击必成，不再因缺 approved_by 而 400）。
    const { recipeNo } = await seedDraftRecipe(page);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: recipeNo }).first();
    await expect(row.getByText('草稿'), `配方 ${recipeNo} 应渲染为「草稿」状态`).toBeVisible({
      timeout: 30000,
    });
    const approveBtn = row.getByRole('button', { name: '审批', exact: true });
    await expect(approveBtn, `草稿配方 ${recipeNo} 的「审批」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await approveBtn.click();
    await page.getByRole('button', { name: /确定|确认|OK/ }).click();
    await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 });
  });
});
