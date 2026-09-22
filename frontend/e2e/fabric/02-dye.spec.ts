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

test.describe('02 染色批次', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('02-01 染色批次 Tab 可正常加载', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    await expect(page.locator('table, .el-table')).toBeVisible({ timeout: 30000 });
  });

  test('02-02 新建染色批次', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    await page.getByRole('button', { name: /新建|创建/ }).click();
    await expect(page.locator('.el-dialog')).toBeVisible({ timeout: 30000 });
    await page.getByLabel(/批次号/).fill(`DB-${Date.now()}`);
    await page.getByLabel(/颜色/).fill('E2E 测试颜色');
    await page.getByLabel(/计划数量/).fill('500');
    await page
      .getByRole('button', { name: /确认|保存|提交/ })
      .last()
      .click();
    await expect(page.getByText(/创建成功|保存成功/)).toBeVisible({ timeout: 30000 });
  });

  test('02-03 染色批次可标记为完成', async ({ page }) => {
    // 假绿清零：原 `if (await completeBtn.isVisible())` 无数据/无匹配状态时零断言通过。
    // 改为造 inspecting 批次 → 完成按钮渲染 → 点击 → ElMessageBox.confirm →
    // completeDyeBatch(Path only，无 body，前后端契约一致) → 真实 toast。
    // 订正错误期望：DyeTab handleComplete 成功提示是 fabric.common.success=「操作成功」，
    // 原用例误写「完成成功」。
    const { batchNo } = await seedDyeBatch(page, 'inspecting');
    await page.goto('/fabric');
    await page.getByRole('tab', { name: /染色|批次/ }).click();
    await expect(page.locator('.el-table')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: batchNo }).first();
    const completeBtn = row.getByRole('button', { name: '完成', exact: true });
    await expect(completeBtn, `批次 ${batchNo} 的「完成」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await completeBtn.click();
    await page
      .getByRole('button', { name: /确定|确认/ })
      .last()
      .click();
    await expect(page.getByText(/操作成功/)).toBeVisible({ timeout: 30000 });
  });
});
