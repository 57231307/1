// 面料管理 E2E 套件 — 03 染色配方
// 创建时间: 2026-08-19
// 覆盖范围：染色配方创建 → 审批（draft → approved，端到端点击 + 断真实 toast）
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { fillFieldByLabel, formItemByExactLabel } from '../flow/ui-helpers';
import { columnIndex, rowByText, rowCellText } from '../flow/table-column';

/**
 * 03-03 造数据前置：创建一条 status='draft' 的染色配方，令 /dye-recipe 页
 * （views/dye-recipe/index.vue）审批按钮（canApprove：draft / pending_approval）渲染。
 * 配方号 recipe_no 用于定位自身行。
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

  test('03-01 染色配方页可正常加载', async ({ page }) => {
    await page.goto('/dye-recipe');
    await expect(page.getByLabel('配方列表表格')).toBeVisible({ timeout: 30000 });
  });

  test('03-02 新建染色配方', async ({ page }) => {
    await page.goto('/dye-recipe');
    await page.getByRole('button', { name: '新建配方' }).click();
    const dlg = page.locator('.el-dialog:visible').last();
    await expect(dlg).toBeVisible({ timeout: 30000 });
    const recipeNo = genCode('E2E-DRUI');
    const recipeName = 'E2E 测试染色配方';
    const colorName = 'E2E测试色';
    const contentText = 'E2E 测试配方内容';
    // 字段文案与控件类型以本页组件为唯一依据，中文值逐个对 locales/zh-CN.ts 的
    // dyeRecipe.index.* 核实（fillFieldByLabel→formItemByExactLabel 对 .el-form-item__label
    // 走行首尾锚定精确匹配，简称/近义词哪怕少两字都恒 0 命中）：
    // - colRecipeNo=配方编号、colRecipeName=配方名称、colColorNo=色号、colColorName=颜色名称
    //   四项都是 el-input（真 <input>），可走 fillFieldByLabel；色号绑定 formData.color_no，
    //   提交时由本页映射为后端 CreateDyeRecipeRequest.color_code（color_code 建表 NOT NULL）。
    // - labelContent=配方内容：el-input type="textarea"（绑定 content），DOM 是真 <textarea>，
    //   fillFieldByLabel 只锚 input 故不可用，改为按同一精确 label 锚 form-item 后填 textarea；
    //   该项在 formRules 里必填，不填则 validate 失败、POST 根本不发。
    // - labelRemarks=备注：同为 textarea、非必填，本用例不填。
    // 提交按钮 buttonConfirm=确认；成功提示 messageSaveSuccess=保存成功。
    await fillFieldByLabel(dlg, page, '配方编号', recipeNo);
    await fillFieldByLabel(dlg, page, '配方名称', recipeName);
    await fillFieldByLabel(dlg, page, '色号', recipeNo);
    await fillFieldByLabel(dlg, page, '颜色名称', colorName);
    await expect(
      formItemByExactLabel(dlg, '色号').locator('input').first(),
      '「色号」项必须是真 <input> 且收到提交值（定位到却没填进值同样判红）'
    ).toHaveValue(recipeNo);
    await expect(
      formItemByExactLabel(dlg, '颜色名称').locator('input').first(),
      '「颜色名称」项必须是真 <input> 且收到提交值'
    ).toHaveValue(colorName);
    const contentArea = formItemByExactLabel(dlg, '配方内容').locator('textarea').first();
    await expect(
      contentArea,
      '「配方内容」在组件里是 el-input type="textarea"，必须渲染为真 <textarea>；' +
        '若渲染成 <input> 说明控件回退，属真实缺陷，不得改回只锚 input 的填法'
    ).toBeVisible({ timeout: 30000 });
    await contentArea.fill(contentText);
    await expect(contentArea, '「配方内容」textarea 必须真收到填写值').toHaveValue(contentText);
    await dlg.getByRole('button', { name: '确认' }).click();
    await expect(page.getByText('保存成功')).toBeVisible({ timeout: 30000 });
    const created = await apiCall<{
      items?: Array<{
        id?: number;
        recipe_no?: string;
        recipe_name?: string;
        color_code?: string;
        color_name?: string;
        chemical_formula?: string | null;
      }>;
    }>(
      page,
      'GET',
      `/production/dye-recipes?recipe_no=${encodeURIComponent(recipeNo)}&page=1&page_size=50`
    );
    const row = created.data?.items?.find(it => it.recipe_no === recipeNo);
    expect(row, `新建后应能按配方编号 ${recipeNo} 从后端回查到配方`).toBeTruthy();
    // 逐键回读弹窗真提交进去的四个值：正文只有经 content→chemical_formula 映射才会落库，
    // 少这一层映射时该列恒 NULL（整表单直传会被后端 serde 静默丢弃），只断 toast 看不出来
    expect(
      row?.recipe_name,
      `回读 recipe_name 应等于「配方名称」提交值，实际 ${JSON.stringify(row?.recipe_name)}`
    ).toBe(recipeName);
    expect(
      row?.color_code,
      `回读 color_code 应等于「色号」提交值（色号未映射进 color_code 即落空串），实际 ${JSON.stringify(
        row?.color_code
      )}`
    ).toBe(recipeNo);
    expect(
      row?.color_name,
      `回读 color_name 应等于「颜色名称」提交值，实际 ${JSON.stringify(row?.color_name)}`
    ).toBe(colorName);
    expect(
      row?.chemical_formula,
      `回读 chemical_formula 应等于「配方内容」textarea 的填写值，实际 ${JSON.stringify(
        row?.chemical_formula
      )}`
    ).toBe(contentText);
    if (row?.id) CLEANUP.push({ path: `/production/dye-recipes/${row.id}`, label: 'dye_recipe' });

    // UI 面回读：按配方编号筛选（本页 filter 的「配方编号」= dyeRecipe.index.filterRecipeNo，
    // 生效键是「查询」按钮 buttonQuery，输入框未绑 @keyup.enter），再逐格读「色号」「颜色名称」列
    // （列头 = colColorNo/colColorName，正文 prop = color_no/color_name）。
    // toast 绿不代表列取到了真值：列恒空是本仓反复出现过的缺陷族，必须按列头下标逐格断。
    const table = page.locator('.el-table[aria-label="配方列表表格"]').first();
    await table.waitFor({ state: 'visible', timeout: 30000 });
    const filterGet = page.waitForResponse(
      res =>
        res.url().includes('/api/v1/erp/production/dye-recipes') &&
        res.url().includes('recipe_no=') &&
        res.status() === 200,
      { timeout: 20000 }
    );
    await fillFieldByLabel(page, page, '配方编号', recipeNo);
    await page.getByRole('button', { name: '查询' }).first().click();
    await filterGet;
    // 结构门（取格之前）：列头 th 数与目标行 td 数必须相等——列绑定取证（列头 ↔ 同序单元格）
    // 就是靠这个对齐关系成立的。本仓 element-plus 2.14.4 的固定列是同一张表内的 sticky 单元格，
    // 不复制列；唯一的表头额外格是 td.el-table__fixed-right-patch（td 不是 th），
    // 所以 th 计数与行内 td 计数天然相等，本页（操作列 fixed="right"）也一样，本门在当前版本恒过。
    // 留着它是防回退：一旦依赖升级改了口径（固定列另起包装层、或插入 gutter 格），
    // 这里给出一条带两个计数与实际列头清单的可读失败；判据不许随之降级成整行 hasText。
    const headerCells = table.locator(
      '.el-table__header-wrapper thead th.el-table__cell:not(.gutter)'
    );
    const headerCount = await headerCells.count();
    const headerTexts = (await headerCells.allInnerTexts()).map(text => text.trim());
    const uiRow = await rowByText(table, recipeNo);
    const bodyCellCount = await uiRow.locator('> td').count();
    expect(
      bodyCellCount,
      `列头与单元格数量不齐，列绑定取证赖以成立的「列头 ↔ 同序单元格」对齐被破坏：` +
        `th=${headerCount} td=${bodyCellCount}，列头清单=${JSON.stringify(headerTexts)}。` +
        `本仓 element-plus 2.14.4 的固定列是同表内 sticky 单元格、不复制列（表头额外格只有 ` +
        `td.el-table__fixed-right-patch，不是 th），故本门本应恒过；判红即渲染口径变了，` +
        `按实际 DOM 修依赖或修取格选择器，不许把判据降级成整行 hasText 绕过`
    ).toBe(headerCount);
    const colorNoIdx = await columnIndex(table, '色号');
    const colorNameIdx = await columnIndex(table, '颜色名称');
    expect(await rowCellText(uiRow, colorNoIdx), '刷新/筛选后「色号」列应渲染弹窗提交值').toBe(
      recipeNo
    );
    expect(
      await rowCellText(uiRow, colorNameIdx),
      '刷新/筛选后「颜色名称」列应渲染弹窗提交值'
    ).toBe(colorName);
  });

  test('03-03 草稿配方可审批（端到端点击 + 断真实 toast）', async ({ page }) => {
    // 硬断言：造 draft 配方 → 状态标签「草稿」、审批按钮渲染 → 真实点击 + 确认 → 断成功 toast。
    const { recipeNo } = await seedDraftRecipe(page);
    await page.goto('/dye-recipe');
    await expect(page.getByLabel('配方列表表格')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: recipeNo }).first();
    await expect(row.getByText('草稿'), `配方 ${recipeNo} 应渲染为「草稿」状态`).toBeVisible({
      timeout: 30000,
    });
    const approveBtn = row.getByRole('button', { name: '审核通过', exact: true });
    await expect(approveBtn, `草稿配方 ${recipeNo} 的「审核通过」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await approveBtn.click();
    const msgBox = page.locator('.el-message-box');
    await msgBox.getByRole('button', { name: '确定' }).click();
    // 审批人取会话身份；若此断言仍红，属登录用户权限或后端 approve 落库侧缺陷，不得放宽断言。
    await expect(page.getByText(/审核通过成功/)).toBeVisible({ timeout: 30000 });
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
