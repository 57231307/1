// 面料管理 E2E 套件 — 03 染色配方
// 创建时间: 2026-08-19
// 覆盖范围：染色配方创建 → 审批（draft → approved，端到端点击 + 断真实 toast）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { fillFieldByLabel } from '../flow/ui-helpers';

/**
 * 03-03 造数据前置：创建一条 status='draft' 的染色配方，令 RecipeTab.vue 审批按钮
 * （canApprove：draft / pending_approval）渲染。配方号 recipe_no 用于定位自身行。
 *
 * 缺陷已修复（v15 词表收口）：
 * - 后端 dye_recipe.status 曾为中文词表（quality_dyeing.rs::dye_recipe），前端按钮/标签用
 *   英文 → 真实数据行永不渲染审批按钮。现统一为小写英文闭合词表，历史中文值由迁移回填，
 *   中文仅在 i18n 展示层（status='draft' 经 i18n 显示为「草稿」）。
 * - approveDyeRecipe 曾不发 body、后端曾要求 {approved_by:i32} → 点击必 400；现后端审批人改取
 *   服务端会话（AuthContext.user_id），该端点无请求体，前端不再传身份。
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
    await expect(page.getByLabel('染色配方列表')).toBeVisible({ timeout: 30000 });
  });

  test('03-02 新建染色配方', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    const dlg = page.locator('.el-dialog:visible').last();
    await expect(dlg).toBeVisible({ timeout: 30000 });
    // 旧用 getByLabel(/配方编号/)/getByLabel(/配方名称/)：RecipeFormDialogTab 真实 label 为
    // 「配方号」(readonly，新建时 generateUniqueDocNo 预生成) /「名称」/「颜色」/「面料类型」/
    // 「配方详情」——「配方编号」「配方名称」根本不存在 → getByLabel 定位超时（本用例红根因）。
    // 成功提示 fabric.common.success=「操作成功」（非「创建成功/保存成功」），提交按钮「确定」。
    await fillFieldByLabel(dlg, page, '名称', 'E2E 测试染色配方');
    await fillFieldByLabel(dlg, page, '颜色', 'E2E测试色');
    await fillFieldByLabel(dlg, page, '面料类型', '纯棉');
    await dlg.getByRole('button', { name: '确定' }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });

  test('03-03 草稿配方可审批（端到端点击 + 断真实 toast）', async ({ page }) => {
    // 假绿清零：原 `if (await approveBtn.isVisible())` 无草稿数据时零断言通过。
    // 词表收口 + approve body 补齐后：造 draft 配方 → 硬断言状态标签为「草稿」、审批按钮渲染
    // → 真实点击审批 + 确认 → 断成功 toast（审批人由后端按会话派生，无需请求体）。
    const { recipeNo } = await seedDraftRecipe(page);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /配方/ }).click();
    await expect(page.getByLabel('染色配方列表')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: recipeNo }).first();
    await expect(row.getByText('草稿'), `配方 ${recipeNo} 应渲染为「草稿」状态`).toBeVisible({
      timeout: 30000,
    });
    const approveBtn = row.getByRole('button', { name: '审批', exact: true });
    await expect(approveBtn, `草稿配方 ${recipeNo} 的「审批」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await approveBtn.click();
    // handleApprove（RecipeTab.vue:159）用 ElMessageBox.confirm 二次确认，确认按钮文案「确定」；
    // 限定到 .el-message-box 作用域点确认，避免误命中页面其它同名按钮。
    const msgBox = page.locator('.el-message-box');
    await msgBox.getByRole('button', { name: '确定' }).click();
    // 注：确认后 approveDyeRecipe(id) — 后端 validate_can_approve 允许 draft/pending_approval 审批，
    // 审批人取会话身份。若此断言仍红，属登录用户权限或后端 approve 落库侧缺陷（非选择器问题），
    // 不改断言方向掩盖，交回复核。
    await expect(page.getByText(/审批成功/)).toBeVisible({ timeout: 30000 });
  });

  test('03-04 chemical_formula 落库回读验证（创建后 GET 回读字段真写入）', async ({ page }) => {
    const recipeNo = genCode('E2E-CF');
    const formulaContent = '活性染料 红SP 2.5g/L; 纯碱 20g/L; 浴比 1:10; 温度 60度; 时间 45min';

    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_no: recipeNo,
      recipe_name: `CF回读测试${recipeNo.slice(-6)}`,
      color_code: recipeNo,
      color_name: 'CF测试色',
      fabric_type: '涤纶',
      chemical_formula: formulaContent,
      status: 'draft',
    });
    const id = created.data?.id;
    expect(id, `创建配方应返回 id，实际: ${JSON.stringify(created)}`).toBeTruthy();
    CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: 'dye_recipe_cf' });

    const fetched = await apiCall<{ chemical_formula?: string }>(
      page,
      'GET',
      `/production/dye-recipes/${id}`
    );
    expect(
      fetched.data?.chemical_formula,
      `GET 回读 chemical_formula 应与 POST 写入值一致，实际为 ${JSON.stringify(fetched.data?.chemical_formula)}`
    ).toBe(formulaContent);
  });

  test('03-05 chemical_formula 更新后回读验证（PUT 修改 → GET 确认新值持久化）', async ({
    page,
  }) => {
    const recipeNo = genCode('E2E-CFU');
    const originalFormula = '原始配方: 染料A 1g/L';
    const updatedFormula = '修改后配方: 染料A 2.5g/L; 助剂B 10g/L; 温度 95度';

    const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-recipes', {
      recipe_no: recipeNo,
      recipe_name: `CF更新回读${recipeNo.slice(-6)}`,
      color_code: recipeNo,
      color_name: 'CF更新色',
      fabric_type: '棉',
      chemical_formula: originalFormula,
      status: 'draft',
    });
    const id = created.data?.id;
    expect(id).toBeTruthy();
    CLEANUP.push({ path: `/production/dye-recipes/${id}`, label: 'dye_recipe_cf_update' });

    await apiCall(page, 'PUT', `/production/dye-recipes/${id}`, {
      chemical_formula: updatedFormula,
    });

    const fetched = await apiCall<{ chemical_formula?: string }>(
      page,
      'GET',
      `/production/dye-recipes/${id}`
    );
    expect(
      fetched.data?.chemical_formula,
      `PUT 更新后 GET 回读应返回新值；期望包含 "助剂B"，实际: ${fetched.data?.chemical_formula}`
    ).toBe(updatedFormula);
  });
});
