// 面料管理 E2E 套件 — 02 染色批次
// 创建时间: 2026-08-19
// 覆盖范围：染色批次创建 → 完成（pending → in_progress → completed）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, genDyeLotNo, tryCleanup } from '../flow/helpers';

/**
 * 02-03 造数据前置：创建一条 status='inspecting'（验布中）的染色批次。
 * DyeTab.vue:64 完成按钮 v-if 要求 status ∈ {preparing,dyeing,washing,fixing,dehydrating,drying,inspecting}；
 * 后端 complete（dye_batch_handler.rs:280）仅接受 inspecting→stored（状态机规则表），
 * 故必须用 inspecting 才能同时满足「按钮渲染」与「完成流转成功」。批次号 batch_no 用于定位自身行。
 *
 * 缸号/色号前置（CI J 族判责：属测试缺前置，后端校验正当不放松）
 * 后端新建归一 resolve_dye_color_identity（dye_batch_handler.rs:203-251）要求——色号非空即
 * 染色布，dye_lot_no 必填（缺 ⇒ 400「染色布必须提供缸号」，与出库四维唯一判定
 * services/inv/fabric_class.rs:60-65 同口径），且色号必须在色卡明细档案（color_card_item）
 * 中唯一命中（color_code 等值反查，档案无此色 ⇒ 400）。globalSeed 只建色卡头（
 * global-setup.ts:1239-1255）与四维库存行（固定色号 'E2E-SEED-COLOR' 仅落库存行，不建
 * color_card_item 档案），未为本用例准备「色号在色卡档案存在 + 真实缸号」前置，
 * 故逐用例自建专属色卡+色号（新色卡状态 draft，色号可挂 ——
 * color_card_item_service.rs:65 EDITABLE_CARD_STATUSES=[DRAFT]）。
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
  status: string,
  colorNo: string
): Promise<{ id: number; batchNo: string }> {
  const batchNo = genCode('E2E-DB');
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/dye-batches', {
    batch_no: batchNo,
    color_no: colorNo,
    // 染色布缸号必填（dye_batch_handler.rs:224-228），取全仓统一取号器生成真实缸号
    dye_lot_no: genDyeLotNo(),
    planned_quantity: 100,
    status,
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建染色批次失败：${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/production/dye-batches/${id}`, label: 'dye_batch' });
  return { id, batchNo };
}

/**
 * 建立「色号在色卡档案中存在」的真实前置：新建专属色卡 → 挂唯一色号明细。
 * 色号必须全局唯一（genCode 时间戳+随机）：resolve_dye_color_identity 对同色号多条记录
 * 显式业务错「无法唯一定位」（dye_batch_handler.rs:241-246），复用既有色号会污染判定。
 * 载荷字段对照 ColorItemDto（handlers/color_card_item_dto.rs:12-53）：
 * color_code/color_name/rgb_r/g/b/hex_value(#RRGGBB) 必填。
 */
async function seedColorCardItem(page: import('@playwright/test').Page): Promise<string> {
  const colorCode = genCode('E2E-DYECOL');
  const card = await apiCall<{ id?: number }>(page, 'POST', '/color-cards', {
    card_no: genCode('E2E-DYECC'),
    card_name: `E2E 染色批次前置色卡 ${colorCode}`,
    card_type: 'CUSTOM',
  });
  const cardId = card.data?.id;
  if (!cardId) throw new Error(`前置色卡创建失败：${JSON.stringify(card)}`);
  const item = await apiCall<{ color_code?: string }>(
    page,
    'POST',
    `/color-cards/${cardId}/items`,
    {
      color_code: colorCode,
      color_name: 'E2E 染色前置色号',
      rgb_r: 220,
      rgb_g: 20,
      rgb_b: 60,
      hex_value: '#DC143C',
    }
  );
  if (item.data?.color_code !== colorCode) {
    throw new Error(
      `色号前置创建后回显不一致（期望 ${colorCode}，实际响应：${JSON.stringify(item)}` +
        '）——若此处红，根因是色卡状态门控/端点契约漂移，属后端问题，不得在本用例放宽'
    );
  }
  console.warn(`[02-dye] 色号前置就绪：card=${cardId} color_code=${colorCode}`);
  return colorCode;
}

/**
 * 按 batch_no 回查后端真实落库的染色批次记录（列表端点支持 batch_no 模糊过滤）。
 * 端点：GET /production/dye-batches（routes/production.rs:30 → list_dye_batches）；
 * 出参键 = DyeBatchDto（dye_batch_handler.rs:71），含 color_code / color_name / color_no。
 */
async function findDyeBatchByNo(
  page: import('@playwright/test').Page,
  batchNo: string
): Promise<
  | {
      id?: number;
      batch_no?: string;
      color_code?: string;
      color_name?: string;
      color_no?: string;
    }
  | undefined
> {
  const res = await apiCall<{
    items?: Array<{
      id?: number;
      batch_no?: string;
      color_code?: string;
      color_name?: string;
      color_no?: string;
    }>;
  }>(
    page,
    'GET',
    `/production/dye-batches?batch_no=${encodeURIComponent(batchNo)}&page=1&page_size=50`
  );
  return res.data?.items?.find(it => it.batch_no === batchNo);
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
    // 真实契约是「自动填充」，故断其非空而非写入，并取作回查锚点。
    const batchNoInput = dialog.getByLabel('批次号');
    await expect(batchNoInput).not.toHaveValue('');
    const batchNo = await batchNoInput.inputValue();
    // 源码缺陷修复（d60ecdd3）后「颜色」绑定 formData.color_no（DyeFormDialogTab.vue:32），
    // 与后端 create DTO 字段一致（CreateDyeBatchRequest.color_no，dye_batch_handler.rs:45）；
    // 后端用它同时写 color_code/color_name/color_no（dye_batch_handler.rs:188-193）。
    // 真实契约是「手填色号 + 真实缸号 + 色号有档案」三者齐备才允许建染色批次
    //（dye_batch_handler.rs:203-251 染色身份归一）——先建色卡档案前置，再据其填表。
    const colorNo = await seedColorCardItem(page);
    await dialog.getByLabel('颜色').fill(colorNo);
    await dialog.getByLabel('染色批号').fill(genDyeLotNo());
    await dialog.getByLabel('计划数量').fill('500');
    await dialog.getByRole('button', { name: '确定' }).click();
    // 保存成功提示 fabric.common.success=「操作成功」
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });
    // 真实落库回查（核心，消除覆盖盲区）：按批次号命中后端记录，断其落库 color_no/color_code
    // 与手填逐字相等，证明「颜色」真实写入而非落系统默认色；若修复失效则此项失败。
    const created = await findDyeBatchByNo(page, batchNo);
    expect(created, `新建后应能按批次号 ${batchNo} 从后端回查到染色批次`).toBeTruthy();
    expect(created?.color_no, `落库 color_no 应等于用户手填值 ${colorNo}（未被默认色覆盖）`).toBe(
      colorNo
    );
    expect(
      created?.color_code,
      `落库 color_code 应等于用户手填值 ${colorNo}（后端按 color_no 反查色卡档案派生）`
    ).toBe(colorNo);
    // 回查并登记清理：UI 建的批次不在 seedDyeBatch 的 CLEANUP 内，须按批次号定位删除
    const createdId = created?.id;
    expect(createdId, `回查到的批次应含 id 以便清理`).toBeTruthy();
    if (createdId)
      CLEANUP.push({ path: `/production/dye-batches/${createdId}`, label: 'dye_batch' });
  });

  test('02-03 染色批次可标记为完成', async ({ page }) => {
    // 假绿清零：原 `if (await completeBtn.isVisible())` 无数据/无匹配状态时零断言通过。
    // 改为造 inspecting 批次 → 完成按钮渲染 → 点击 → 完工登记对话框强制采集实际产出三值
    // （DyeTab.vue:98 handleComplete 打开的是 CompleteDyeBatchDialog，
    // 原 ElMessageBox.confirm 形态已废弃；后端 CompleteDyeBatchRequest 三值必填，
    // 缺一即 400 且状态不推进）→ 提交成功提示 dyeBatch.complete.messageSuccess=「完工登记成功」。
    const colorNo = await seedColorCardItem(page);
    const { batchNo } = await seedDyeBatch(page, 'inspecting', colorNo);
    await page.goto('/fabric');
    await page.getByRole('tab', { name: DYE_TAB, exact: true }).click();
    await expect(page.getByLabel('染色批次列表')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: batchNo }).first();
    const completeBtn = row.getByRole('button', { name: '完成', exact: true });
    await expect(completeBtn, `批次 ${batchNo} 的「完成」按钮应渲染`).toBeVisible({
      timeout: 30000,
    });
    await completeBtn.click();
    const completeDialog = page.locator('.el-dialog:visible').last();
    await expect(completeDialog, '完工登记对话框应渲染').toBeVisible({ timeout: 15000 });
    // 三值输入 = 对话框内仅有的三个 el-input-number（DOM 序与实际落布重量/长度/坯布投料一致）；
    // fill 后 Tab 触发 blur 同步 v-model（el-input-number 不在 input 事件即提交模型）
    const spin = completeDialog.getByRole('spinbutton');
    await expect(spin, '完工对话框应采集实际产出三值').toHaveCount(3);
    await spin.nth(0).fill('100');
    await page.keyboard.press('Tab');
    await spin.nth(1).fill('5000');
    await page.keyboard.press('Tab');
    await spin.nth(2).fill('120');
    await page.keyboard.press('Tab');
    await completeDialog.getByRole('button', { name: '确认完工', exact: true }).click();
    await expect(page.getByText('完工登记成功')).toBeVisible({ timeout: 30000 });
  });
});
