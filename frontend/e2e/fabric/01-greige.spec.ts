// 面料管理 E2E 套件 — 01 坯布管理
// 创建时间: 2026-08-19
// 覆盖范围：坯布创建 → 入库 → 出库
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';

/**
 * 前后端契约缺陷（曾致本套件只能"硬断按钮渲染"，现已修复，恢复真实点击 + 真实 toast）：
 *  1) 列表编号列曾读 fabric_code，后端 greige_fabric::Model 实为 fabric_no
 *     （backend/src/models/greige_fabric.rs:14）→ 已改绑 fabric_no；供应商列因后端列表
 *     端点仅返回 supplier_id、无 supplier_name，已暂移除；库存 quantity 列后端不存在，
 *     已改绑真实字段 weight_kg / length_m。
 *  2) 入库/出库对话框曾发 { quantity }，后端 stock_in 要求 { warehouse_id, weight_kg, length_m }、
 *     stock_out 要求 { weight_kg / length_m }（backend/src/handlers/greige_fabric_handler.rs:110/123）
 *     → 点击必 400；现对话框按契约采集，真实点击可成功。
 * toast 文案取自前端 src/locales：入库「入库成功」、出库「出库成功」
 *   （fabric.index.messageStockInSuccess / messageStockOutSuccess）。
 * status 用非「在库」值以便用例后 DELETE 清理（后端仅拦截「在库」删除）。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedGreige(
  page: import('@playwright/test').Page,
  extra?: { weight_kg?: number; length_m?: number }
): Promise<{ id: number; name: string }> {
  const suffix = genCode('E2E-GF').slice(-6);
  const name = `E2E坯布${suffix}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/greige-fabrics', {
    fabric_no: `E2E-GF${suffix}`,
    fabric_name: name,
    fabric_type: '梭织',
    status: 'pending',
    ...(extra ?? {}),
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建坯布失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/production/greige-fabrics/${id}`, label: 'greige_fabric' });
  return { id, name };
}

/**
 * 按手填 fabric_no 从列表回查后端真实落库记录（列表端点支持 fabric_no 模糊过滤）。
 * 端点：GET /production/greige-fabrics（routes/production.rs:67 → list_greige_fabrics）；
 * 出参键 = greige_fabric::Model.fabric_no（models/greige_fabric.rs:14，NOT NULL）。
 */
async function findGreigeByFabricNo(
  page: import('@playwright/test').Page,
  fabricNo: string
): Promise<{ id?: number; fabric_no?: string; fabric_name?: string } | undefined> {
  const res = await apiCall<{
    items?: Array<{ id?: number; fabric_no?: string; fabric_name?: string }>;
  }>(
    page,
    'GET',
    `/production/greige-fabrics?fabric_no=${encodeURIComponent(fabricNo)}&page=1&page_size=50`
  );
  return res.data?.items?.find(it => it.fabric_no === fabricNo);
}

// 入库需真实仓库：新建一个坯布仓供对话框选择，用例后清理
async function seedWarehouse(page: import('@playwright/test').Page): Promise<string> {
  const suffix = genCode('E2E-WH').slice(-6);
  const name = `E2E坯布仓${suffix}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
    warehouse_name: name,
    warehouse_code: `E2E-WH${suffix}`,
    warehouse_type: 'greige',
  });
  if (!created.data?.id) throw new Error(`创建仓库失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/warehouses/${created.data.id}`, label: 'warehouse' });
  return name;
}

async function openGreigeTabAndLocateRow(page: import('@playwright/test').Page, name: string) {
  await page.goto('/fabric');
  await page.getByRole('tab', { name: '坯布管理', exact: true }).click();
  // 坯布表格按 aria-label（fabric.greigeTab.tableAriaLabel=「坯布列表」）精确定位：
  // 原 `.el-table` 会同时命中 /fabric 页三个 Tab 各自的表格容器（染色批次列表 / 坯布列表 /
  // 染色配方列表）→ strict-mode 违例。
  await expect(page.getByLabel('坯布列表')).toBeVisible({ timeout: 30000 });
  return page.getByRole('row').filter({ hasText: name }).first();
}

test.describe('01 坯布管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入面料管理页面', async ({ page }) => {
    await page.goto('/fabric');
    // 页面归属以面包屑「面料管理」（el-breadcrumb__inner role=link）为唯一锚点：
    // 原 getByText(/面料/) 会同时命中侧边菜单「面料管理」「面料列表」、面包屑与列名
    // 「面料类型」共 4 处 → strict-mode 违例。
    await expect(page.getByRole('link', { name: '面料管理', exact: true })).toBeVisible({
      timeout: 30000,
    });
    await expect(page.getByRole('tab', { name: '坯布管理', exact: true })).toBeVisible();
  });

  test('01-02 新建坯布', async ({ page }) => {
    await page.goto('/fabric');
    await page.getByRole('tab', { name: '坯布管理', exact: true }).click();
    // 新建按钮真实文案 fabric.greigeTab.buttonCreate=「新建坯布」
    await page.getByRole('button', { name: '新建坯布' }).click();
    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 真实对话框字段（GreigeFormDialogTab.vue）为 编号/名称/供应商/幅宽/克重/成分。
    // 源码缺陷修复（d60ecdd3）后「编号」绑定 formData.fabric_no（GreigeFormDialogTab.vue:31），
    // 与后端 create DTO 字段一致（CreateGreigeFabricRequest.fabric_no，greige_fabric_handler.rs:37；
    // 缺省时后端才自动生成 GF-<ts>-<rand>，见 handler:192）。原缺陷为 fabric_code 被 serde
    // 忽略、用户编号恒落系统号——修复后应能真实验证「手填编号原样落库、不被默认号覆盖」。
    const name = `E2E坯布UI${Date.now()}`;
    const fabricNo = `GF-E2E-${Date.now()}`;
    await dialog.getByLabel('编号').fill(fabricNo);
    await dialog.getByLabel('名称').fill(name);
    await dialog.getByRole('button', { name: '确定' }).click();
    // 保存成功提示 fabric.common.success=「操作成功」（原用例断 /创建成功|保存成功/ 与实际文案不符）
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // UI 行回查：列表按名称命中新建行
    await expect(
      page.getByRole('row').filter({ hasText: name }).first(),
      `新建坯布「${name}」应真实落库并出现在坯布列表`
    ).toBeVisible({ timeout: 30000 });
    // 真实落库回查（核心，消除覆盖盲区）：按手填编号命中后端记录，断其落库 fabric_no
    // 与用户输入逐字相等，证明确实写入、未被系统默认号覆盖；若修复失效则此项失败。
    const created = await findGreigeByFabricNo(page, fabricNo);
    expect(created, `应能按手填编号 ${fabricNo} 从后端回查到坯布`).toBeTruthy();
    expect(
      created?.fabric_no,
      `落库 fabric_no 应等于用户手填值 ${fabricNo}（未被系统默认号覆盖）`
    ).toBe(fabricNo);
    const createdId = created?.id;
    expect(createdId, `回查到的坯布应含 id 以便清理`).toBeTruthy();
    if (createdId)
      CLEANUP.push({ path: `/production/greige-fabrics/${createdId}`, label: 'greige_fabric' });
  });

  test('01-03 坯布入库操作', async ({ page }) => {
    // 契约修复后恢复真实点击：采集 仓库 + 重量(kg) + 长度(m) 提交，断真实「入库成功」toast。
    const whName = await seedWarehouse(page);
    const { name } = await seedGreige(page);
    const row = await openGreigeTabAndLocateRow(page, name);
    await row.getByRole('button', { name: '入库', exact: true }).click();

    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 仓库选择（不给默认值，必须手动选）
    await dialog.getByRole('combobox').click();
    await page.getByRole('option', { name: whName }).click();
    await dialog.getByLabel(/重量/).fill('50');
    await dialog.getByLabel(/长度/).fill('100');
    await dialog.getByRole('button', { name: /确认|保存|提交/ }).click();

    await expect(page.getByText('入库成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 坯布出库操作', async ({ page }) => {
    // 出库无仓库字段；先经 API 造出库存（weight_kg/length_m），再从 UI 出库并断「出库成功」toast。
    const { name } = await seedGreige(page, { weight_kg: 100, length_m: 100 });
    const row = await openGreigeTabAndLocateRow(page, name);
    await row.getByRole('button', { name: '出库', exact: true }).click();

    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    await dialog.getByLabel(/重量/).fill('10');
    await dialog.getByLabel(/长度/).fill('20');
    await dialog.getByRole('button', { name: /确认|保存|提交/ }).click();

    await expect(page.getByText('出库成功')).toBeVisible({ timeout: 30000 });
  });
});
