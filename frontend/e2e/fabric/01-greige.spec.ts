// 面料管理 E2E 套件 — 01 坯布管理
// 创建时间: 2026-08-19
// 覆盖范围：坯布创建 → 入库 → 出库
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, genCode, tryCleanup } from '../flow/helpers';
import { pickSelectIn, fillFieldByLabel } from '../flow/ui-helpers';

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
 *   （greigeFabrics.index.messageStockInSuccess / messageStockOutSuccess）。
 * status 用非「在库」值以便用例后 DELETE 清理（后端仅拦截「在库」删除）。
 */
const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

async function seedGreige(
  page: import('@playwright/test').Page,
): Promise<{ id: number; name: string }> {
  // 只建坯布档案，不带数量列。weight_kg 与 length_m 是当前库存两列，quantity_kg 与
  // quantity_meters 是累计入库两列，两组字段在 greige_fabric 表里独立存储，出库门控读的是
  // 累计入库两列。建单时把当前库存凑出来但累计入库仍为空，等价于凭空造库存，与业务事实
  // 可出库量必须由真实入库支撑不符，因此建档腿不再接受数量注入，数量一律通过后端
  // stock-in 端点走真实入库登记。
  const suffix = genCode('E2E-GF').slice(-6);
  const name = `E2E坯布${suffix}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/production/greige-fabrics', {
    fabric_no: `E2E-GF${suffix}`,
    fabric_name: name,
    fabric_type: '梭织',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建坯布失败 ${JSON.stringify(created)}`);
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

/**
 * SeaORM DECIMAL 序列化为 JSON 字符串（rust_decimal 默认 Display 语义，前端 api 类型
 * greige-fabric.ts 也把这四列声明成 string 或 null 两态）。写后读回要把它们当数字比较，
 * 不能吞 null。判据严格。null 与 undefined 和 NaN 一律抛错让用例真红，不用 Number 的隐式
 * 0 把字段缺失洗成数值为 0，避免脏读回蒙过断言。
 */
function toExactNumber(v: string | number | null | undefined, fieldLabel: string): number {
  if (v === null || v === undefined) {
    throw new Error(`读回字段 ${fieldLabel} 为空，期望数值，实际 ${String(v)}`);
  }
  const n = Number(v);
  if (Number.isNaN(n)) {
    throw new Error(`读回字段 ${fieldLabel} 不是数值，实际 ${String(v)}`);
  }
  return n;
}

/**
 * 通过后端 stock_in handler 做真实入库登记。handler 累加 weight_kg 与 length_m 到当前库存
 * 两列，同时把同一份量加进 quantity_kg 与 quantity_meters 累计入库两列。出库门控正是读后
 * 两列，走这条腿才会把库存台账建起来。调用方是本 spec 的 01-04 出库用例，入参是 fabricId
 * 与仓库 id 与本次入库量，传给 POST /production/greige-fabrics/{id}/stock-in，存储落到
 * greige_fabric 表同一行的四列。
 */
async function seedStockIn(
  page: import('@playwright/test').Page,
  fabricId: number,
  warehouseId: number,
  weightKg: number,
  lengthM: number
): Promise<void> {
  await apiCall(page, 'POST', `/production/greige-fabrics/${fabricId}/stock-in`, {
    warehouse_id: warehouseId,
    weight_kg: weightKg,
    length_m: lengthM,
  });
}

/**
 * 读回一条坯布的库存四列与状态。用于写后读回，钉的是后端 greige_fabric 一行的真值，不是
 * 前端 store 或 UI 展示。GET /production/greige-fabrics/{id} 由 get_greige_fabric handler
 * 返回完整 Model 序列化，四列按 SeaORM DECIMAL 到 JSON 字符串的两态声明，读回时用
 * toExactNumber 严格转换。
 */
async function readGreigeStock(
  page: import('@playwright/test').Page,
  fabricId: number
): Promise<{
  weight_kg: number;
  quantity_kg: number;
  length_m: number;
  quantity_meters: number;
  status: string | null;
}> {
  const res = await apiCall<{
    weight_kg?: string | number | null;
    quantity_kg?: string | number | null;
    length_m?: string | number | null;
    quantity_meters?: string | number | null;
    status?: string | null;
  }>(page, 'GET', `/production/greige-fabrics/${fabricId}`);
  return {
    weight_kg: toExactNumber(res.data.weight_kg, `坯布 ${fabricId} weight_kg`),
    quantity_kg: toExactNumber(res.data.quantity_kg, `坯布 ${fabricId} quantity_kg`),
    length_m: toExactNumber(res.data.length_m, `坯布 ${fabricId} length_m`),
    quantity_meters: toExactNumber(res.data.quantity_meters, `坯布 ${fabricId} quantity_meters`),
    status: res.data.status ?? null,
  };
}

// 入库需真实仓库，warehouse_id 是后端 StockInRequest 的必填主键。返回 id 与 name 两个字段，
// UI 入库腿用 name 做下拉选中，API 入库腿用 id 做请求体。
async function seedWarehouse(
  page: import('@playwright/test').Page
): Promise<{ id: number; name: string }> {
  const suffix = genCode('E2E-WH').slice(-6);
  const name = `E2E坯布仓${suffix}`;
  const created = await apiCall<{ id?: number }>(page, 'POST', '/warehouses', {
    warehouse_name: name,
    warehouse_code: `E2E-WH${suffix}`,
    warehouse_type: 'greige',
  });
  const id = created.data?.id;
  if (!id) throw new Error(`创建仓库失败 ${JSON.stringify(created)}`);
  CLEANUP.push({ path: `/warehouses/${id}`, label: 'warehouse' });
  return { id, name };
}

async function openGreigeTabAndLocateRow(page: import('@playwright/test').Page, name: string) {
  await page.goto('/greige-fabrics');
  // 坯布列表唯一入口为 /greige-fabrics，表格按 aria-label「坯布列表表格」精确定位
  await expect(page.getByLabel('坯布列表表格')).toBeVisible({ timeout: 30000 });
  return page.getByRole('row').filter({ hasText: name }).first();
}

test.describe('01 坯布管理', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('01-01 进入坯布管理页面', async ({ page }) => {
    await page.goto('/greige-fabrics');
    await expect(page.getByRole('heading', { name: '坯布管理' })).toBeVisible({
      timeout: 30000,
    });
    await expect(page.getByLabel('坯布列表表格')).toBeVisible();
  });

  test('01-02 新建坯布（API 造数 + UI 列表回显验证）', async ({ page }) => {
    const { name } = await seedGreige(page);
    await page.goto('/greige-fabrics');
    await expect(page.getByLabel('坯布列表表格')).toBeVisible({ timeout: 30000 });
    const row = page.getByRole('row').filter({ hasText: name }).first();
    await expect(row, `新建坯布「${name}」应出现在 /greige-fabrics 坯布列表`).toBeVisible({
      timeout: 30000,
    });
    // 验证 fabric_type 真实渲染到行内（seedGreige 传入 '梭织'，若后端 NOT NULL 未落库则列空）
    await expect(
      row.getByText('梭织'),
      `坯布列表应渲染 fabric_type='梭织'（seedGreige 真实传入）`
    ).toBeVisible({ timeout: 10000 });
    // 回查后端确认 fabric_type 落库（彻底消除覆盖盲区）
    const suffix = name.replace('E2E坯布', '');
    const created = await findGreigeByFabricNo(page, `E2E-GF${suffix}`);
    expect(created, `应能从后端回查到坯布`).toBeTruthy();
    // seedGreige 已登记清理（CLEANUP.push 内含 id），此处无需重复
  });

  test('01-03 坯布入库操作', async ({ page }) => {
    const wh = await seedWarehouse(page);
    const { name } = await seedGreige(page);
    const row = await openGreigeTabAndLocateRow(page, name);
    await row.getByRole('button', { name: '入库', exact: true }).click();

    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    await pickSelectIn(dialog, page, '仓库', { optionText: wh.name });
    await fillFieldByLabel(dialog, page, '重量(kg)', '50');
    await page.keyboard.press('Tab');
    await fillFieldByLabel(dialog, page, '长度(m)', '100');
    await page.keyboard.press('Tab');
    await dialog.getByRole('button', { name: '确认' }).click();

    await expect(page.getByText('入库成功')).toBeVisible({ timeout: 30000 });
  });

  test('01-04 坯布出库操作', async ({ page }) => {
    // 出库门控读的是累计入库两列，stock_out handler 会分别校验出库重量不大于 quantity_kg、
    // 出库长度不大于 quantity_meters。建单时把 weight_kg 与 length_m 凑成非零并不能让门控
    // 放行，凑数等同一笔没有入库台账的库存，与业务事实不符。前置走真实入库腿。seedWarehouse
    // 建仓，seedGreige 建空白档案，seedStockIn 通过后端 stock_in 端点把当前库存与累计入库
    // 四列同步抬起来。入库量与出库量对齐到 10 kg 与 20 m，出库后当前库存与累计入库双双向零，
    // 状态随之翻到已出库（stock_out handler 在剩余双 0 时走 STOCKED_OUT 分支），afterEach
    // 的 DELETE 清理才能过 delete_greige_fabric 的在库门控，不留脏行。
    const INBOUND_WEIGHT_KG = 10;
    const INBOUND_LENGTH_M = 20;
    const OUTBOUND_WEIGHT_KG = 10;
    const OUTBOUND_LENGTH_M = 20;

    const wh = await seedWarehouse(page);
    const { id: fabricId, name } = await seedGreige(page);
    await seedStockIn(page, fabricId, wh.id, INBOUND_WEIGHT_KG, INBOUND_LENGTH_M);

    // 入库腿落地自检。如果这一步没把 weight_kg 与 quantity_kg 抬起来，UI 出库就会撞回同一
    // 条 400 出库重量大于累计入库，红因会被误导成出库腿；这里先正向钉一次读回，把入库确实
    // 落库与出库确实扣库两个动作分开验证，出库失败时能一眼定位是腿错了还是门控错了。
    const afterInbound = await readGreigeStock(page, fabricId);
    expect(
      afterInbound.weight_kg,
      `入库腿必须把当前库存 weight_kg 抬到 ${INBOUND_WEIGHT_KG}，实际读回 ${afterInbound.weight_kg}`
    ).toBe(INBOUND_WEIGHT_KG);
    expect(
      afterInbound.quantity_kg,
      `入库腿必须把累计入库 quantity_kg 同步抬到 ${INBOUND_WEIGHT_KG}（stock_in 与当前库存同源累加），实际读回 ${afterInbound.quantity_kg}`
    ).toBe(INBOUND_WEIGHT_KG);
    expect(
      afterInbound.length_m,
      `入库腿必须把当前长度 length_m 抬到 ${INBOUND_LENGTH_M}，实际读回 ${afterInbound.length_m}`
    ).toBe(INBOUND_LENGTH_M);
    expect(
      afterInbound.quantity_meters,
      `入库腿必须把累计米数 quantity_meters 同步抬到 ${INBOUND_LENGTH_M}，实际读回 ${afterInbound.quantity_meters}`
    ).toBe(INBOUND_LENGTH_M);

    const row = await openGreigeTabAndLocateRow(page, name);
    await row.getByRole('button', { name: '出库', exact: true }).click();

    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    await fillFieldByLabel(dialog, page, '重量(kg)', String(OUTBOUND_WEIGHT_KG));
    await page.keyboard.press('Tab');
    await fillFieldByLabel(dialog, page, '长度(m)', String(OUTBOUND_LENGTH_M));
    await page.keyboard.press('Tab');
    await dialog.getByRole('button', { name: '确认' }).click();

    await expect(page.getByText('出库成功')).toBeVisible({ timeout: 30000 });

    // 写后读回。断言对象是后端 greige_fabric 一行的四列与状态，正向钉扣减确实落库，反向排除
    // 只弹 toast 未扣库存与只扣当前库存不扣累计入库两条假绿路径。差值就是入库量减出库量，
    // 库里剩余应等于该差值；本用例取全额出库，差值为 0，剩余归零同时把状态翻到已出库。若状态
    // 停在在库说明扣减或状态判定任一环节失真，afterEach 的 DELETE 也会撞在库门控。
    const afterOutbound = await readGreigeStock(page, fabricId);
    const expectedRemainingWeight = INBOUND_WEIGHT_KG - OUTBOUND_WEIGHT_KG;
    const expectedRemainingLength = INBOUND_LENGTH_M - OUTBOUND_LENGTH_M;
    expect(
      afterOutbound.weight_kg,
      `出库后当前库存 weight_kg 应为 ${expectedRemainingWeight}，即入库 ${INBOUND_WEIGHT_KG} 减 出库 ${OUTBOUND_WEIGHT_KG}，实际读回 ${afterOutbound.weight_kg}`
    ).toBe(expectedRemainingWeight);
    expect(
      afterOutbound.quantity_kg,
      `出库后累计入库 quantity_kg 应同步扣到 ${expectedRemainingWeight}（stock_out 与当前库存同源扣减），实际读回 ${afterOutbound.quantity_kg}`
    ).toBe(expectedRemainingWeight);
    expect(
      afterOutbound.length_m,
      `出库后当前长度 length_m 应为 ${expectedRemainingLength}，即入库 ${INBOUND_LENGTH_M} 减 出库 ${OUTBOUND_LENGTH_M}，实际读回 ${afterOutbound.length_m}`
    ).toBe(expectedRemainingLength);
    expect(
      afterOutbound.quantity_meters,
      `出库后累计米数 quantity_meters 应同步扣到 ${expectedRemainingLength}，实际读回 ${afterOutbound.quantity_meters}`
    ).toBe(expectedRemainingLength);
    expect(
      afterOutbound.status,
      `剩余双 0 时状态应翻到已出库（stock_out 走 STOCKED_OUT 分支），实际读回 status ${String(afterOutbound.status)}；停在在库意味着扣减或状态判定失真，afterEach DELETE 会被在库门控挡住`
    ).toBe('已出库');
  });
});
