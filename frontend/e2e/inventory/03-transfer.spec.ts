// 库存管理 E2E 套件 — 03 库存调拨（创建 → 审批）
// 覆盖范围：正规页 /inventory-transfer 的调拨单创建（调出→调入仓库、明细行经真实库存行选四维）、审批
// 说明：库存页 /inventory 的「库存调拨」Tab 及老 TransferDialog 已随源码物理删除，
// 调拨入口统一为 /inventory 页头「库存调拨」按钮（router.push InventoryTransfer）。
// 本套件因此改测正规页 views/inventory-transfer 的真实交互路径。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  apiCall,
  apiCallRaw,
  ensureTestEntities,
  getCtx,
  pickDyeableWarehouse,
  seedDyedOutboundBundle,
  seedFourDimStockIn,
  seedGreigeStockIn,
} from '../flow/helpers';
import { pickSelect, pickSelectIn, fillFieldByLabel, escRe } from '../flow/ui-helpers';

interface TransferSeed {
  productId: number;
  productCode: string;
  fromWarehouseId: number;
  fromWarehouseName: string;
  toWarehouseName: string;
  /** 本批 seed 的四维库存行色号：正规页库存行下拉须据此选中它自己的行（而非盲选 index 0）。 */
  seedColorNo: string;
  /** 同 tuple（产品+调出仓+缸=批+色号）的真实 AVAILABLE 染色匹——UI 匹号下拉的唯一合法选项源 */
  seedPieceNos: string[];
}

// 调拨出库要求「调出仓库对该产品有足量库存」（inventory_move::check_from_warehouse_inventory），
// 且正规页 TransferFormDialogTab 的出库四维经「调出仓+产品的真实库存行」下拉
// （GET /inventory/stock）选定——若该产品在调出仓无库存行，则明细的下拉为空、无法建单。
// 批次 5a82561a 补匹号选择器后的新契约（判责 #4669 J-2）：
// - 染色布（色号非空）第四维匹号必选（TransferFormDialogTab.vue:110-126/614-622，
//   与后端 services/inv/fabric_class.rs 唯一判定同口径；不许为过用例放松）；
// - 可命中的真实染色匹只由写入方链产出：piece_domain_service.rs:518-556 委外染色回仓确认
//   生成的匹恒 batch_no == dye_lot_no == 缸号，且染色匹只允许成品仓/未设类型仓
//   （validate_warehouse_for_piece_type）——故 seed 必须用 helpers.seedDyedOutboundBundle
//   （库存行 batch=缸号 + 真实链 N 匹），调出仓必须经 pickDyeableWarehouse 选定；
//   旧 seedFourDimStockIn 以独立 batch 值灌行，按该 tuple 造不出匹，UI 第四步必死锁。
async function seedTransferSource(page: Page): Promise<TransferSeed> {
  await ensureTestEntities(page);
  const wh = await apiCallRaw<{ items: { id: number; warehouse_name: string }[] }>(
    page,
    'GET',
    '/warehouses?page=1&page_size=100'
  );
  const pr = await apiCallRaw<{ items: { id: number; product_code: string }[] }>(
    page,
    'GET',
    '/products?page=1&page_size=100'
  );
  const product = pr.items?.[0];
  expect(product?.id, '前置：调拨需至少一个产品').toBeTruthy();
  // 调出仓 = 可承载染色匹的真实仓库（未设类型仓优先，其次成品仓；胚布仓建匹必被后端拒绝）
  const fromWarehouse = await pickDyeableWarehouse(page);
  // 调入仓 = 与调出仓不同的另一个仓库（建单期仅记录维度归属，不校验仓库类型）
  const toWarehouse = wh.items?.find(w => w.id !== fromWarehouse.id);
  expect(toWarehouse?.id, '前置：调拨需至少两个仓库（调出/调入不可同仓）').toBeTruthy();
  const tag = Date.now().toString().slice(-6);
  const seedColorNo = `E2E-TRF-C${tag}`;
  const bundle = await seedDyedOutboundBundle(page, {
    productId: product!.id,
    warehouseId: fromWarehouse.id,
    colorNo: seedColorNo,
    quantityMeters: '5000',
    pieceCount: 2,
    context: `TRF${tag}`,
  });
  expect(
    bundle.pieces.length,
    `前置：seed 应产出 ≥1 匹 AVAILABLE 染色匹，实际=${bundle.pieces.length}`
  ).toBeGreaterThanOrEqual(1);
  return {
    productId: product!.id,
    productCode: product!.product_code,
    fromWarehouseId: fromWarehouse.id,
    fromWarehouseName: fromWarehouse.name,
    toWarehouseName: toWarehouse!.warehouse_name,
    seedColorNo,
    seedPieceNos: bundle.pieces.map(p => p.piece_no),
  };
}

test.describe('库存管理 - 03 库存调拨（正规页 /inventory-transfer）', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('库存调拨页列表加载', async ({ page }) => {
    // 老「库存调拨 Tab 数据加载」经 /inventory 的 Tab，Tab 已删除——改为直达正规页并断言列表真实渲染。
    await page.goto('/inventory-transfer');
    await expect(page.getByRole('heading', { name: '库存调拨' })).toBeVisible({ timeout: 30000 });
    // TransferListTab 的 el-table aria-label=库存调拨列表（transferList.table.ariaLabel，zh-CN.ts:2260）。
    // 根因A（strict mode violation）：同页 el-pagination 的 aria-label=库存调拨列表分页
    // （transferList.table.paginationAriaLabel，zh-CN.ts:2270）含「库存调拨列表」子串，
    // getByLabel 默认子串匹配同时命中表格与分页两个 aria-label 宿主 → 报命中 2 元素。
    // 正解：以 exact 精确锚定表格的可访问名，保留「列表真实渲染」这一断言意图（不删、不放宽）。
    await expect(page.getByLabel('库存调拨列表', { exact: true })).toBeVisible({ timeout: 30000 });
  });

  test('创建库存调拨单（经正规页新建对话框，明细从真实库存行选四维）', async ({ page }) => {
    const seed = await seedTransferSource(page);
    await page.goto('/inventory-transfer');

    // 「新建」按钮：TransferListTab 内 v-permission='inventory:create'，文案 transferList.button.create
    const createBtn = page.getByRole('button', { name: '新建', exact: true }).first();
    await expect(createBtn, '正规页应渲染"新建"入口').toBeVisible({ timeout: 30000 });
    await createBtn.click();

    const dialog = page.locator('.el-dialog:visible').last();
    await expect(dialog).toBeVisible({ timeout: 30000 });
    // 表头 aria-label=新建调拨单对话框（transferForm.createDialogTitle）
    await expect(page.getByLabel('新建调拨单对话框')).toBeVisible({ timeout: 10000 });

    // 选择调出/调入仓库（选项 label 为真实仓库名，按名精确锚定，规避并发下单据顺序漂移）
    await pickSelectIn(dialog, page, '调出仓库', {
      optionText: new RegExp(`^${escRe(seed.fromWarehouseName)}$`),
      timeout: 30_000,
    });
    await pickSelectIn(dialog, page, '调入仓库', {
      optionText: new RegExp(`^${escRe(seed.toWarehouseName)}$`),
      timeout: 30_000,
    });

    // 明细行：选定调出/调入仓后，对话框内 el-select 顺序为
    // [调出仓库, 调入仓库, 产品, 库存行, 匹号(色号非空后才渲染)]。
    // 产品下拉（nth 2，无 form-item label，filterable）：按真实款号（label 前缀 code - name）锚定。
    await pickSelect(
      page,
      dialog.locator('.el-select').nth(2),
      new RegExp(`^${escRe(seed.productCode)} `),
      { timeout: 30_000 }
    );
    // 产品 change 即 await loadStockRows（TransferFormDialogTab.vue:404-411），无需另等
    // GET /inventory/stock 响应——原用例的 waitForResponse 注册在响应之后必然落空，且曾以
    // `.catch(() => {})` 静默吞错（成功失败无痕，违反显式日志红线），此处直接删除；
    // 库存行下拉（nth 3）内 pickSelect 自身等待选项可见，即等价于等待回源完成。
    // 库存行下拉：选项为该调出仓+产品下的真实四维行，按本用例 seed 的色号（唯一 tag）
    // 精确锚定它自己造的那行（stockRowLabel 含 `色号: <color_no>` 文本），
    // 确保选中的行与随后建单扣减的维度一致、且可用量充足（并发下不盲选 index 0）。
    await pickSelect(page, dialog.locator('.el-select').nth(3), seed.seedColorNo, {
      timeout: 30_000,
    });
    // 第四维匹号（5a82561a 新契约）：库存行选定后 item.color_no 非空 → 匹号下拉渲染为
    // nth(4)，选项只能来自 GET /inventory/pieces 的真实 AVAILABLE 匹；不选定则前端
    // pieceNoRequired 拦截、永不发请求。按本批 seed 产出的匹号精确锚定第一匹。
    expect(seed.seedPieceNos.length, '前置：seed 匹号候选非空').toBeGreaterThan(0);
    await pickSelect(page, dialog.locator('.el-select').nth(4), seed.seedPieceNos[0], {
      timeout: 30_000,
    });

    // 数量：明细行第一个 el-input-number 的内层 <input>，placeholder 直传原生 input
    // （EP input-number → el-input → input[placeholder]，本地 node_modules 源码核实）。
    // CI #4669 判红根因：getByRole('spinbutton', {name:'数量'}) 按可及名计算 0 命中
    // （role 由 EP onMounted setAttribute、名称回落 placeholder 的链条在真实浏览器不可靠），
    // 正解为按 DOM 属性直查并以 :visible 过滤到可见项——找不到即自然超时抛真实红，
    // 绝不改成「填不进就跳过」。
    const qtyInput = dialog.locator('input[placeholder="数量"]:visible').first();
    await expect(qtyInput, '明细数量输入应可见且可填').toBeVisible({ timeout: 10000 });
    await qtyInput.click({ clickCount: 3 });
    await qtyInput.fill('5');
    await page.keyboard.press('Tab');

    // 落库回读前置：记录当前该调出仓已存在的调拨单 id，提交后据此识别本次新建
    const before = await apiCallRaw<{ items: { id: number }[] }>(
      page,
      'GET',
      `/inventory/transfers?from_warehouse_id=${seed.fromWarehouseId}&page=1&page_size=100`
    );
    const beforeIds = new Set((before.items ?? []).map(t => t.id));

    // 提交按钮真实文案「保存」（transferForm.save）；成功后 ElMessage.success(t('message.operationSuccess'))='操作成功'
    await dialog.getByRole('button', { name: '保存', exact: true }).click();
    await expect(page.getByText('操作成功')).toBeVisible({ timeout: 30000 });

    // 走后端 API 回读命中：新建调拨单应落库、出现在 GET /inventory/transfers（同一调出仓），初始态 pending。
    const after = await apiCallRaw<{
      items: { id: number; from_warehouse_id: number; status: string }[];
    }>(
      page,
      'GET',
      `/inventory/transfers?from_warehouse_id=${seed.fromWarehouseId}&page=1&page_size=100`
    );
    const created = (after.items ?? []).filter(
      t => !beforeIds.has(t.id) && t.from_warehouse_id === seed.fromWarehouseId
    );
    expect(
      created.length,
      `回读：经正规页新建的调拨单应命中 GET /inventory/transfers（调出仓 ${seed.fromWarehouseId}）`
    ).toBeGreaterThanOrEqual(1);
    expect(
      String(created[0].status).toLowerCase(),
      `新建调拨单初始态应为 pending，实际=${created[0].status}`
    ).toBe('pending');
  });

  test('审批待审批调拨单', async ({ page }) => {
    // 真实造数：先补前置实体（≥2 仓库 + 产品），再用 API 落一张"待审批"调拨单。
    // 原实现用 `if (await approveBtn.isVisible())` 把整段交互包进可见性分支：
    // 空库时列表无 pending 单 → 审批按钮不渲染 → 一条断言都不执行却记为通过（假绿）。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：需要至少两个仓库（调出/调入）').toBeGreaterThanOrEqual(
      2
    );
    expect(ctx.productIds.length, '前置：需要至少一个产品').toBeGreaterThanOrEqual(1);

    // 根因C：调拨出库前置校验 inv/stock.rs::check_from_warehouse_inventory →
    // match_single_item_against_stocks 要求「调出仓对该产品有维度匹配的足量库存行」，
    // 否则返回 BUSINESS_ERROR（无匹配库存，4xx）。原实现直接 POST /inventory/transfers 而未造源库存
    // → 建单被正确拒绝、拿不到 data.id → 本用例红（后端行为正确，测试缺前置 seed）。
    //
    // 布种口径说明：本用例只验证「pending→审批→approved」状态流，与白坯/染色无关。
    // 由于 /inventory/stock/fabric 入库端点对 color_no 强制 length(min=1)
    // （backend/src/handlers/inventory_stock_handler_dto.rs:20-21，白坯空色号无法经此端点播种），
    // 故此处采用染色布口径（色号/缸号/批次齐全），并以 seedFourDimStockIn 在调出仓为「同一产品+同一
    // 四维」造一行足量库存——seed 维度与随后 POST 的调拨明细三字段逐一对应（不造假库存）。
    const tag = Date.now().toString().slice(-6);
    const colorNo = `E2E-AP-C${tag}`;
    const dyeLotNo = `E2E-AP-D${tag}`;
    const batchNo = `E2E-AP-B${tag}`;
    await seedFourDimStockIn(page, {
      productId: ctx.productIds[0],
      warehouseId: ctx.warehouseIds[0],
      colorNo,
      dyeLotNo,
      batchNo,
      quantityMeters: '1000',
    });

    const created = await apiCall<{ id?: number; transfer_no?: string }>(
      page,
      'POST',
      '/inventory/transfers',
      {
        from_warehouse_id: ctx.warehouseIds[0],
        to_warehouse_id: ctx.warehouseIds[1],
        transfer_date: new Date().toISOString(),
        notes: 'E2E 审批用例造数',
        items: [
          {
            product_id: ctx.productIds[0],
            quantity: '5',
            color_no: colorNo,
            dye_lot_no: dyeLotNo,
            batch_no: batchNo,
          },
        ],
      }
    );
    const transferId = created.data?.id;
    const transferNo = created.data?.transfer_no;
    expect(
      transferId,
      `调拨建单应返回 data.id，实际响应：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();
    expect(
      transferNo,
      `调拨建单应返回 data.transfer_no 供列表检索定位，实际响应：${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();

    // 进入正规页，按单号精确过滤，确保点击的是本用例刚造的 pending 单（非并发他人行）。
    await page.goto('/inventory-transfer');
    const filter = page.getByLabel('库存调拨筛选表单');
    await fillFieldByLabel(filter, page, '调拨单号', String(transferNo));
    await filter.getByRole('button', { name: '查询', exact: true }).click();

    // 硬断言（Tier A）：过滤后该待审批单必然渲染"审批"按钮（transferList.button.approve），缺失即缺陷
    const approveBtn = page.getByRole('button', { name: '审批', exact: true }).first();
    await expect(approveBtn, '待审批调拨单应渲染"审批"按钮').toBeVisible({ timeout: 30000 });
    await approveBtn.click();

    // 审批对话框（ApproveTransferDialogTab）：点"通过"（approveTransfer.pass）
    const approveDialog = page.locator('.el-dialog:visible').last();
    await expect(approveDialog).toBeVisible({ timeout: 15000 });
    await approveDialog.getByRole('button', { name: '通过', exact: true }).click();
    // 成功提示 approvePassed='审批通过'
    await expect(page.getByText('审批通过')).toBeVisible({ timeout: 30000 });

    // API 回读：真实状态迁移 pending→approved（GET /inventory/transfers/{id} 详情）
    const detail = await apiCallRaw<{ status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(String(detail.status).toLowerCase(), '审批后调拨单状态应为 approved').toBe('approved');
  });

  test('白坯调拨建单成功（color_no 空、免缸号，验证 is_dyed=false 宽松放行路径）', async ({
    page,
  }) => {
    // 根因覆盖：match_single_item_against_stocks 的白坯分支（is_dyed=false）——
    // 当 color_no 为空时仅按 款号+批次 匹配库存、不强制缸号，调拨建单应成功放行。
    // 上一波因 /stock/fabric 对 color_no 强制 min=1 而丢失此分支 e2e 覆盖，
    // 本用例经 POST /inventory/stock（通用 handler，不调 payload.validate()）播种白坯库存行
    // （color_no=''、无缸号），再以同维度建调拨单，走 API 回读确认落库。
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.warehouseIds.length, '前置：需至少两个仓库').toBeGreaterThanOrEqual(2);
    expect(ctx.productIds.length, '前置：需至少一个产品').toBeGreaterThanOrEqual(1);

    const tag = Date.now().toString().slice(-6);
    const batchNo = `E2E-GRG-B${tag}`;

    // seed：白坯库存行（color_no 空、batch_no 有、无缸号）
    await seedGreigeStockIn(page, {
      productId: ctx.productIds[0],
      warehouseId: ctx.warehouseIds[0],
      batchNo,
      quantityMeters: '2000',
    });

    // 建调拨单：明细只给 款号+批次，color_no 传空串（白坯口径），不带缸号
    const created = await apiCall<{ id?: number; transfer_no?: string }>(
      page,
      'POST',
      '/inventory/transfers',
      {
        from_warehouse_id: ctx.warehouseIds[0],
        to_warehouse_id: ctx.warehouseIds[1],
        transfer_date: new Date().toISOString(),
        notes: `E2E 白坯调拨用例 tag=${tag}`,
        items: [
          {
            product_id: ctx.productIds[0],
            quantity: '10',
            color_no: '',
            batch_no: batchNo,
          },
        ],
      }
    );
    const transferId = created.data?.id;
    expect(
      transferId,
      `白坯调拨建单应成功返回 data.id（验证 is_dyed=false 免缸号路径），实际响应：${JSON.stringify(created).slice(0, 300)}`
    ).toBeTruthy();

    // 回读 GET /inventory/transfers/{id}：状态 pending（建单初始态）、调出仓正确
    const detail = await apiCallRaw<{ id: number; from_warehouse_id: number; status: string }>(
      page,
      'GET',
      `/inventory/transfers/${transferId}`
    );
    expect(detail.id, '回读 id 应与建单返回一致').toBe(transferId);
    expect(detail.from_warehouse_id, '调出仓应为 seed 仓库').toBe(ctx.warehouseIds[0]);
    expect(
      String(detail.status).toLowerCase(),
      `白坯调拨单初始态应为 pending，实际=${detail.status}`
    ).toBe('pending');

    // 清理：删除本用例创建的调拨单（避免污染后续用例）
    await apiCall(page, 'DELETE', `/inventory/transfers/${transferId}`).catch(() => {});
  });
});
