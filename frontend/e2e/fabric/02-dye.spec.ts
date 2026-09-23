// 面料管理 E2E 套件 — 02 染色批次
// 创建时间: 2026-08-19
// 覆盖范围：染色批次创建 → 完成（pending → in_progress → completed）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 02-03 造数据前置：创建一条 status='inspecting'（验布中）的染色批次。
 * DyeTab.vue:64 完成按钮 v-if 要求 status ∈ {preparing,dyeing,washing,fixing,dehydrating,drying,inspecting}；
 * 后端 complete（dye_batch_handler.rs:280）仅接受 inspecting→stored（状态机规则表），
 * 故必须用 inspecting 才能同时满足「按钮渲染」与「完成流转成功」。批次号 batch_no 用于定位自身行。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/**
 * /fabric 页面的 Tab 真实文案（fabric.index.tabDye）为「染色批次」。
 * 原用例用 { name: /染色|批次/ } 会同时命中「染色批次」与「染色配方」两个 tab
 * （二者都含「染色」）→ getByRole('tab') strict-mode 违例。统一改用精确名定位。
 */
const DYE_TAB = '染色批次';

async function seedDyeBatch(
  page: import('@playwright/test').Page,
  status: string
): Promise<{ id: number; batchNo: string }> {
  const batchNo = genCode('E2E-DB');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
    batch_no: batchNo,
    color_no: `E2E-CN${batchNo.slice(-6)}`,
    planned_quantity: 100,
    status,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建染色批次失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/production/dye-batches/${id}`, label: 'dye_batch' });
  return { id, batchNo };
}

/** 按 batch_no 回查后端真实落库的染色批次 id（列表端点支持 batch_no 模糊过滤）。 */
async function findDyeBatchIdByNo(
  page: import('@playwright/test').Page,
  batchNo: string
): Promise<number | undefined> {
  const res = await apiCall<{ items?: Array<{ id?: number; batch_no?: string }> }>(
    page,
    'GET',
    `/production/dye-batches?batch_no=${encodeURIComponent(batchNo)}&page=1&page_size=50`
  );
  return res.data?.items?.find(it => it.batch_no === batchNo)?.id;
}

test.describe('02 染色批次', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 染色批次 Tab 可正常加载', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: DYE_TAB, exact: true }).click();
    // 染色批次表格按 aria-label（fabric.dyeTab.tableAriaLabel=「染色批次列表」）精确定位；
    // 原 `table, .el-table` 命中本页三个 Tab 的表格容器 → strict-mode 违例。
    await expect(page.getByLabel('染色批次列表')).toBeVisible({ timeout: 30000 });
  });

  test('02-02 新建染色批次', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: DYE_TAB, exact: true }).click();
    // 新建按钮真实文案 fabric.dyeTab.buttonCreate=「新建批次」
    await page.getByRole('button', { name: '新建批次' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 批次号在打开对话框时由 generateUniqueDocNo 预生成且输入框 readonly
    // （DyeFormDialogTab.vue:27 `<el-input readonly>`）——只读字段不可 fill（Playwright 判
    // not editable），原用例 getByLabel(/批次号/).fill(...) 属对只读自动单号的错误操作。
    // 真实契约是「自动填充」，故断其非空而非写入；随后填写可编辑的计划字段。
    const batchNoInput = dialog.getByLabel('批次号');
    await expect(batchNoInput).not.toHaveValue('');
    const batchNo = await batchNoInput.inputValue();
    await dialog.getByLabel('颜色').fill(`E2E 测试颜色 ${Date.now()}`);
    await dialog.getByLabel('计划数量').fill('500');
    await dialog.getByRole('button', { name: '确定' }).click();
    // 保存成功提示 fabric.common.success=「操作成功」
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // 回查并登记清理：UI 建的批次不在 seedDyeBatch 的 CLEANUP 内，须按自动生成的批次号定位删除
    const createdId = await findDyeBatchIdByNo(page, batchNo);
    expect(createdId, `新建后应能按批次号 ${batchNo} 回查到染色批次 id`).toBeTruthy();
    if (createdId)
      CLEANUP.push({ path: `/production/dye-batches/${createdId}`, label: 'dye_batch' });
  });

  test('02-03 染色批次可标记为完成', async ({ page }) => {
    // 假绿清零：原 `if (await completeBtn.isVisible())` 无数据/无匹配状态时零断言通过。
    // 改为造 inspecting 批次 → 完成按钮渲染 → 点击 → ElMessageBox.confirm →
    // completeDyeBatch(Path only，无 body，前后端契约一致) → 真实 toast。
    // 订正错误期望：DyeTab handleComplete 成功提示是 fabric.common.success=「操作成功」，
    // 原用例误写「完成成功」。
    const { batchNo } = await seedDyeBatch(page, 'inspecting');
    await page.goto('/fabric');
    await page.getByRole('tab', { name: DYE_TAB, exact: true }).click();
    await expect(page.getByLabel('染色批次列表')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: batchNo }).first();
    const completeBtn = row.getByRole('button', { name: '完成', exact: true });
    await expect(completeBtn, `批次 ${batchNo} 的「完成」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await completeBtn.click();
    await page.getByRole('button', { name: '确定' }).last().click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
  });
});
