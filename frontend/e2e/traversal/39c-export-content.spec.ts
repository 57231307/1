import { test, expect } from '../diagnose-fixture';
import JSZip from 'jszip';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  ensureTestEntities,
  getCtx,
  genCode,
  seedColorCardArchive,
} from '../flow/helpers';

/**
 * 39c 导出内容断言（doto 2026-09-09 导出内容缺口）
 *
 * 现有 39 spec 仅断言 2xx/3xx + 文件名后缀；本 spec 深度断言导出内容：
 * 1. /products/export + /customers/export（敏感清单里，但 admin 持有审批令牌链路
 *    复杂——改用非敏感端点做内容断言；敏感端点内容断言在 39b 审批链令牌侧扩展）
 * 2. /warehouses/export（非敏感，基础数据必有）
 * 3. /stock/export（库存，ensureTestEntities 建过库存）
 * 4. 采购订单 / 销售订单 / 色卡发放 / 固定资产四腿为丙档：先经真实端点写入一行，
 *    再解包导出 xlsx，把水印行声明的条数、工作表数据行数与列表独立读回的服务端
 *    总数三者对成等式（交付行数 == 服务端全量）。任何一腿回退到分页或封顶旁路时
 *    交付行数小于服务端总数，等式即红；该判据与后端契约锁
 *    contract_export_full_rowset / contract_wave18_order_export_full_rowset /
 *    contract_order_export_rowset_lock 同源。工作表行数 = 水印行 1 + 标题行 1 +
 *    数据行（六腿均为带水印形态，水印文案与 handler 逐字同源）。
 *
 * xlsx = zip 容器：
 * - xl/sharedStrings.xml（字符串表）或 xl/worksheets/sheet1.xml（内联字符串）
 * - 断言列头（中文表头）与数据行数 ≥ 列表 API 行数（内容与源数据一致）
 *
 * 全部真实后端 + 每步显式日志
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** xlsx 解包提取全部文本（sharedStrings + sheet1 内联 + inlineStr） */
async function extractXlsxText(body: Buffer): Promise<{ text: string; sheetRows: number }> {
  const zip = await JSZip.loadAsync(body);
  let text = '';
  const shared = await zip.file('xl/sharedStrings.xml')?.async('string');
  if (shared) text += shared;
  // 遍历全部 worksheet（多 sheet 场景）
  const sheetFiles = Object.keys(zip.files).filter(
    f => f.startsWith('xl/worksheets/') && f.endsWith('.xml')
  );
  let sheetRows = 0;
  for (const f of sheetFiles) {
    const xml = await zip.file(f)!.async('string');
    text += xml;
    // xml 恒存在（刚 async('string') 读出）；无 <row> 时 match 返回 null，此处 ?? 0 表示"该 sheet 无数据行"，
    // 是行计数归零的正确语义，非"字段缺失伪装成空集合"，保留。
    sheetRows += (xml.match(/<row[ >]/g) ?? []).length;
  }
  return { text, sheetRows };
}

/**
 * 订单腿水印行取数：文案与销售/采购 handler 的 WatermarkConfig.extra 逐字同源
 * （采购订单导出（交付 {n} 条，服务端全量 {m} 条，完整）/ 销售订单导出同型）。
 * 取不到该形态即抛错判红——封顶或分页旁路回潮时水印会带出不全数字或缺失该句式。
 */
function watermarkCounts(text: string, label: string): { delivered: number; server: number } {
  const m = text.match(new RegExp(`${label}（交付 (\\d+) 条，服务端全量 (\\d+) 条，完整）`));
  if (!m) {
    throw new Error(
      `导出文件水印行缺少 ${label} 的交付/服务端全量真实数字形态：${text.slice(0, 400)}`
    );
  }
  return { delivered: Number(m[1]), server: Number(m[2]) };
}

/**
 * 载体腿水印行取数：文案与固定资产/色卡发放 handler 逐字同源
 * （固定资产导出（共 {n} 条）/ 色卡发放记录导出（共 {n} 条））。
 */
function watermarkTotal(text: string, label: string): number {
  const m = text.match(new RegExp(`${label}（共 (\\d+) 条）`));
  if (!m) {
    throw new Error(`导出文件水印行缺少 ${label} 的共条数形态：${text.slice(0, 400)}`);
  }
  return Number(m[1]);
}

test.describe('39c 导出内容断言', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('仓库导出 xlsx 内容与列表数据一致（列头+行数）', async ({ page }) => {
    // 与 playwright.config.ts 单测默认 420s 基线一致（该值专为 ensureTestEntities
    // 需 UI 创建 10+ 实体、50 分片并发同库的耗时设定）。原 120s 覆盖低于基线，
    // 内联 ensureTestEntities 在末段（/auth/me）踩上限。
    test.setTimeout(420_000);
    // 造数据：ensureTestEntities 保证至少 2 个仓库，导出内容匹配不再依赖种子数据缺失而 skip
    await ensureTestEntities(page);

    // ---- 1. 列表 API 行数（源数据基准）----
    const list = await apiCallRaw<{
      items: Array<{ id: number; name?: string; code?: string }>;
      total: number;
    }>(page, 'GET', '/warehouses?page=1&page_size=100');
    // 先断言出参形态：字段缺失不得伪装成空集合
    expect(
      Array.isArray(list.items),
      `仓库列表缺 items 数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    const listCount = list.items.length;
    expect(listCount, 'ensureTestEntities 后仓库列表不应为空').toBeGreaterThan(0);
    console.log(`[39c] 仓库列表行数=${listCount}（total=${list.total}）`);

    // ---- 2. 真实导出（已注册 catalog.rs:91 /warehouses/export，有数据必须 200）----
    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/warehouses/export`);
    const status = exportResp.status();
    console.log(`[39c] /warehouses/export → ${status}`);
    expect(
      status,
      `仓库导出应 200（端点已注册且已造数据）；${status} 说明导出处理器坏掉，判红`
    ).toBe(200);

    const body = await exportResp.body();
    expect(body.length, `xlsx 应 >512B，实际 ${body.length}B`).toBeGreaterThan(512);
    const contentType = exportResp.headers()['content-type'] ?? '';
    console.log(`[39c] Content-Type=${contentType}`);

    // ---- 3. xlsx 解包内容断言 ----
    const { text, sheetRows } = await extractXlsxText(body);
    expect(text.length, 'xlsx 解包应提取到文本内容').toBeGreaterThan(0);
    console.log(`[39c] xlsx 解包：${text.length}B 文本 / ${sheetRows} 行`);

    // 列头断言：仓库名称/编码类中文表头应出现（sharedStrings 或内联）
    const hasHeader =
      text.includes('仓库') ||
      text.includes('名称') ||
      text.includes('编码') ||
      text.includes('code');
    expect(hasHeader, '导出应包含仓库相关列头').toBeTruthy();

    // 行数断言：导出数据行 ≥ 列表行数（列表分页 page_size=100，导出全量）
    if (listCount > 0) {
      expect(
        sheetRows,
        `导出数据行 ${sheetRows} 应 ≥ 列表行数 ${listCount}（内容完整性）`
      ).toBeGreaterThanOrEqual(listCount);
    }
    // 名称内容匹配：列表第一条名称应出现在导出文本中
    const first = list.items?.[0];
    if (first?.name) {
      expect(
        text.includes(first.name),
        `导出应包含列表首条名称 "${first.name}"（内容与源数据一致）`
      ).toBeTruthy();
      console.log(`[39c] ✅ 内容匹配："${first.name}" 出现在导出文件`);
    }
  });

  test('库存导出 xlsx 行数与列表一致', async ({ page }) => {
    // 与 playwright.config.ts 单测默认 420s 基线一致（该值专为 ensureTestEntities
    // 需 UI 创建 10+ 实体、50 分片并发同库的耗时设定）。原 120s 覆盖低于基线，
    // 内联 ensureTestEntities 在末段（/auth/me）踩上限。
    test.setTimeout(420_000);
    // 造数据：ensureTestEntities 会建库存行；导出处理器已注册则必须 200，不再 skip
    await ensureTestEntities(page);

    const list = await apiCallRaw<{
      items: Array<{ id: number; batch_no?: string; product_name?: string }>;
      total: number;
    }>(page, 'GET', '/inventory/stock?page=1&page_size=100');
    expect(
      Array.isArray(list.items),
      `库存列表缺 items 数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    const listCount = list.items.length;
    const firstText = list.items[0]?.batch_no ?? '';
    console.log(`[39c] 库存列表行数=${listCount}，首条=${firstText}`);

    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/inventory/stock/export`);
    const status = exportResp.status();
    console.log(`[39c] /stock/export → ${status}`);
    expect(
      status,
      `库存导出应 200（端点已注册 inventory.rs:59）；${status} 说明导出处理器坏掉，判红`
    ).toBe(200);

    const body = await exportResp.body();
    expect(body.length, '库存 xlsx 应 >512B').toBeGreaterThan(512);
    const { text, sheetRows } = await extractXlsxText(body);
    console.log(`[39c] 库存 xlsx：${text.length}B / ${sheetRows} 行`);

    if (listCount > 0) {
      expect(sheetRows, `导出行 ${sheetRows} 应 ≥ 列表行 ${listCount}`).toBeGreaterThanOrEqual(
        listCount
      );
    }
    if (firstText) {
      expect(text.includes(firstText), `导出应包含首条批号 ${firstText}（内容一致）`).toBeTruthy();
      console.log(`[39c] ✅ 库存批号 ${firstText} 内容匹配`);
    }
  });

  test('采购订单导出丙档：写草稿单后读回，交付数=服务端全量=列表total', async ({ page }) => {
    test.setTimeout(420_000);
    await ensureTestEntities(page);
    const ctx = getCtx();

    // 写：一张草稿采购单（载荷对照 flow/18-resilience-boundary 已验证的真实契约形态，
    // 金额为 Decimal 字符串入参）
    const created = await apiCall<{ id?: number }>(page, 'POST', '/purchase/orders', {
      supplier_id: ctx.supplierId,
      warehouse_id: ctx.warehouseIds[0],
      department_id: ctx.departmentIds[0],
      order_date: new Date().toISOString().slice(0, 10),
      items: [
        {
          material_id: ctx.productIds[0],
          quantity_ordered: '1',
          unit_price: '1',
          tax_rate: 13,
        },
      ],
    });
    expect(created.data?.id, `丙档前置建采购单失败：${JSON.stringify(created)}`).toBeTruthy();

    // 服务端总数独立读回：列表 PaginatedResponse.total（与导出腿同一 admin 视角）
    const list = await apiCallRaw<{ items: unknown[]; total: number }>(
      page,
      'GET',
      '/purchase/orders?page=1&page_size=100'
    );
    expect(
      Array.isArray(list.items),
      `采购订单列表缺 items 数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    expect(list.total, '写入后采购订单服务端总数应至少为 1').toBeGreaterThanOrEqual(1);
    console.log(`[39c] 采购订单列表 total=${list.total}`);

    // 读回：导出文件水印数字与数据行数
    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/purchase/orders/export`);
    expect(
      exportResp.status(),
      `采购订单导出应 200，实际 ${exportResp.status()}（端点单数挂载，复数即判红）`
    ).toBe(200);
    const { text, sheetRows } = await extractXlsxText(await exportResp.body());
    const marks = watermarkCounts(text, '采购订单导出');
    console.log(
      `[39c] 采购订单：水印交付=${marks.delivered} 水印服务端=${marks.server} 文件行=${sheetRows}`
    );

    // 丙类等式：交付 == 服务端全量 == 列表独立 total；数据行（去水印行与标题行）同值
    expect(marks.delivered, '水印交付条数应等于水印服务端全量').toBe(marks.server);
    expect(marks.server, '水印服务端全量应等于列表独立读回 total').toBe(list.total);
    expect(sheetRows - 2, '采购订单导出数据行应等于服务端全量（截断旁路回潮即红）').toBe(
      list.total
    );
  });

  test('销售订单导出丙档：写订单后读回，交付数=服务端全量=列表total', async ({ page }) => {
    test.setTimeout(420_000);
    await ensureTestEntities(page);
    const ctx = getCtx();
    expect(ctx.customerId, '销售订单腿需要 ensureTestEntities 备好客户').toBeTruthy();

    // 写：一张销售订单（载荷对照 dashboard/02-bi-aggregate-truth 已验证形态）
    const created = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
      customer_id: ctx.customerId,
      items: [{ product_id: ctx.productIds[0], quantity: 1, unit_price: 10 }],
    });
    expect(created.data?.id, `丙档前置建销售单失败：${JSON.stringify(created)}`).toBeTruthy();

    const list = await apiCallRaw<{ items: unknown[]; total: number }>(
      page,
      'GET',
      '/sales/orders?page=1&page_size=100'
    );
    expect(
      Array.isArray(list.items),
      `销售订单列表缺 items 数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    expect(list.total, '写入后销售订单服务端总数应至少为 1').toBeGreaterThanOrEqual(1);
    console.log(`[39c] 销售订单列表 total=${list.total}`);

    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/sales/orders/export`);
    expect(exportResp.status(), `销售订单导出应 200，实际 ${exportResp.status()}`).toBe(200);
    const { text, sheetRows } = await extractXlsxText(await exportResp.body());
    const marks = watermarkCounts(text, '销售订单导出');
    console.log(
      `[39c] 销售订单：水印交付=${marks.delivered} 水印服务端=${marks.server} 文件行=${sheetRows}`
    );

    expect(marks.delivered, '水印交付条数应等于水印服务端全量').toBe(marks.server);
    expect(marks.server, '水印服务端全量应等于列表独立读回 total').toBe(list.total);
    expect(sheetRows - 2, '销售订单导出数据行应等于服务端全量（截断旁路回潮即红）').toBe(
      list.total
    );
  });

  test('色卡发放导出丙档：发放一条后读回，共条数=列表total=数据行', async ({ page }) => {
    test.setTimeout(420_000);
    await ensureTestEntities(page);

    // 发放闸门 3（backend/src/services/color_card_issue_service.rs
    // check_customer_credit_and_status）要求 customers.credit_limit > 0 且 status=active。
    // ensureTestEntities 的 ctx.customerId 借用列表首个活跃客户，其 credit_limit 不保证为正
    // （POST /crm/customers 的 credit_limit 缺省与空串都落 0，customer_handler.rs
    // CreateCustomerRequest），借用即改写在门收紧后必以 BUSINESS_ERROR 拒绝发放，
    // 属前置真值不成立，不是导出腿的问题。造数对齐 flow/110 库存登记腿的官方范式
    // （createCustomer，credit_limit 显式正值数值字符串），本用例自建专属客户。
    const issuedCustomer = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_name: `E2E 色卡发放专属客户 ${Date.now()}`,
      customer_type: 'wholesale',
      status: 'active',
      credit_limit: '1000',
    });
    const customerId = issuedCustomer.data?.id;
    expect(
      customerId,
      `丙档前置专属客户创建失败（发放闸门要求 credit_limit 正值）：${JSON.stringify(issuedCustomer)}`
    ).toBeTruthy();

    // 写：档案色卡（draft 卡 + 色号明细，状态门=可发放词表值）后发放一条
    const seed = await seedColorCardArchive(page, { context: '39c-issue' });
    // 发放闸门要求可用库存不少于发放数量；库存登记入口是 PUT /color-cards/:id 的 stock_quantity，
    // 新卡库存恒为 0，不先登记则会以业务门拒绝（不是导出腿的问题）。
    await apiCall(page, 'PUT', `/color-cards/${seed.cardId}`, { stock_quantity: 10 });
    const issued = await apiCall<{ id?: number }>(page, 'POST', '/color-cards/issues', {
      color_card_id: seed.cardId,
      customer_id: customerId,
      issue_qty: 1,
    });
    expect(issued.data?.id, `丙档前置发放色卡失败：${JSON.stringify(issued)}`).toBeTruthy();

    // 列表与导出腿共用同一归属与筛选构造（admin 视角全量），total 独立读回
    const list = await apiCallRaw<{ items: unknown[]; total: number }>(
      page,
      'GET',
      '/color-cards/issues?page=1&page_size=100'
    );
    expect(
      Array.isArray(list.items),
      `色卡发放列表缺 items 数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);
    expect(list.total, '发放后服务端总数应至少为 1').toBeGreaterThanOrEqual(1);
    console.log(`[39c] 色卡发放列表 total=${list.total}`);

    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/color-cards/issues/export`);
    expect(exportResp.status(), `色卡发放导出应 200，实际 ${exportResp.status()}`).toBe(200);
    const { text, sheetRows } = await extractXlsxText(await exportResp.body());
    const declared = watermarkTotal(text, '色卡发放记录导出');
    console.log(`[39c] 色卡发放：水印共条数=${declared} 文件行=${sheetRows}`);

    expect(declared, '水印共条数应等于列表独立读回 total').toBe(list.total);
    expect(sheetRows - 2, '色卡发放导出数据行应等于服务端 total（分批漏行回潮即红）').toBe(
      list.total
    );
  });

  test('固定资产导出丙档：写唯一编号资产后读回，编号在文件内且共条数自洽', async ({ page }) => {
    test.setTimeout(420_000);
    await ensureTestEntities(page);

    // 写：一张带唯一编号的固定资产
    const assetNo = genCode('E2E-FA39C');
    const created = await apiCall<{ id?: number }>(page, 'POST', '/fixed-assets', {
      asset_name: `E2E 丙档导出资产 ${assetNo}`,
      asset_no: assetNo,
      purchase_date: new Date().toISOString().split('T')[0],
      original_value: 123456,
      useful_life: 60,
      depreciation_method: 'straight_line',
    });
    expect(created.data?.id, `丙档前置建固定资产失败：${JSON.stringify(created)}`).toBeTruthy();

    // 列表出参为 data 数组（handler 返回 Vec，无 total 键），满页判据随之分支
    const list = await apiCallRaw<unknown[]>(page, 'GET', '/fixed-assets?page=1&page_size=100');
    expect(
      Array.isArray(list),
      `固定资产列表应返回数组：${JSON.stringify(list).slice(0, 200)}`
    ).toBe(true);

    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/fixed-assets/export`);
    expect(exportResp.status(), `固定资产导出应 200，实际 ${exportResp.status()}`).toBe(200);
    const { text, sheetRows } = await extractXlsxText(await exportResp.body());
    const declared = watermarkTotal(text, '固定资产导出');
    console.log(`[39c] 固定资产：列表行=${list.length} 水印共条数=${declared} 文件行=${sheetRows}`);

    // 写后读回：刚写入的资产编号必须出现在导出文件（行级真值，非仅按钮可点）
    expect(
      text.includes(assetNo),
      `导出文件应包含刚写入的资产编号 ${assetNo}（写后读回真值）`
    ).toBeTruthy();
    // 水印声明条数与工作表数据行自洽（两数都出自取数腿，旁路截断时同步变小仍成立，
    // 故再加与列表的外部对账）
    expect(declared, '水印共条数应等于文件数据行数（去水印行与标题行）').toBe(sheetRows - 2);
    if (list.length < 100) {
      expect(declared, '列表未满页时导出共条数应等于列表行数（两路全量对账）').toBe(list.length);
    } else {
      expect(
        declared,
        '列表触页宽上限时导出共条数应不小于上限（导出取全、列表被页宽截断）'
      ).toBeGreaterThanOrEqual(list.length);
    }
  });
});
