import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
  genDyeLotNo,
  seedColorCardArchive,
  failureCode,
  verifyEndpointHealthy,
  verifyDownloadEndpointHealthy,
  deferCleanup,
  flushDeferredCleanups,
  APP_ERROR_CODES,
  BASE_URL,
  type ApiFailureResult,
  type DeferredCleanup,
} from './helpers';
import { fillFieldByLabel } from './ui-helpers';

/**
 * 62 染化料台账 → 批次 → 领用 → 消耗/追溯链路（e2e 补齐 A 路）
 *
 * 端点真实性（catalog 域 merge 进 /api/v1/erp 根，routes/mod.rs:417；缸号在 production nest）：
 * - GET/POST     /chemicals、GET /chemicals/by-code/{code}、GET/PUT/DELETE /chemicals/{id}   routes/catalog.rs:151-168
 * - GET/POST     /chemical-lots、GET /chemical-lots/by-no/{no}、GET/PUT/DELETE /chemical-lots/{id}
 *                POST /chemical-lots/{id}/pass-inspection|fail-inspection|consume|scrap      routes/catalog.rs:190-215
 * - GET/POST     /chemical-requisitions、GET /chemical-requisitions/by-no/{no}、
 *                GET/PUT/DELETE /chemical-requisitions/{id}、
 *                POST /{id}/approve|issue|close|cancel|print                                  routes/catalog.rs:218-262
 * - GET/POST     /production/dye-batches、DELETE /production/dye-batches/{id}、
 *                GET /production/dye-batches/export（xlsx 二进制）                            routes/production.rs:27-59（mod.rs:509 nest）
 * - GET/POST     /chemical-categories（tree）                                                  routes/catalog.rs:170-188
 *
 * 状态词表真相（写入方 models/status/wage_energy_chemical_business.rs）：
 * - 领用单：draft→approved→issued→(partial_returned)→closed；类型 production/lab/rd，
 *   production 必须关联 dye_batch_id（services/chemical_ops/requisition.rs:56-62）
 * - 批次：inspection pending→passed/failed；status active→consumed/scrapped（lot.rs:186-264）
 *
 * 追溯（领用↔缸号）：GET /chemical-requisitions?dye_batch_id={id} 必须能按缸号回查到
 * 本用例自建的生产领用单（真实过滤链，非仅 200）。
 *
 * 假绿防线：每次状态流转后 GET 详情回读 status 逐字符判等；分页第二页真值断言
 * （chemical-lots 按 chemical_id 过滤 + page_size=1）；软删除后 by-no 必须 404；
 * 二进制 xlsx/docx 全部走 verifyDownloadEndpointHealthy。
 *
 * 已知前端缺陷（本 spec 不编码、不改测试掩盖，详见交付报告）：
 * ① chemicals/index.vue 批次弹窗提交 chemical_code/lot_date/quantity/status，后端必填
 *    chemical_id（chemical_ops/types.rs:158-176）→ UI 新建批次恒 4xx；
 * ② 分类弹窗提交 {name,parent_id}，后端必填 category_code/category_name/category_type
 *    （types.rs:127-134）→ UI 新建分类恒 4xx；
 * ③ 主数据弹窗类型只有 dye/auxiliary/other，dye 必填 dye_category 而弹窗未采集
 *    （master.rs:52-56），other 不在词表（chemical_service.rs:63-75）→ UI 新建恒 4xx。
 * 因此主数据 UI 只覆盖「列表回读 + 编辑（编辑链路字段与后端对齐，真实可用）」，
 * 创建链路走后端真实契约在 API 层覆盖。
 */

/**
 * 断言精确拒绝契约（收紧原「任意 4xx + 三族机器码来者不拒」的假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373：ValidationError/BusinessError/
 *   BadRequest 全部映射 BAD_REQUEST 状态，本域拒绝不存在其它 4xx 分支）；
 * - 机器码逐条等于调用点期望族。chemical_ops 域写入方真相：
 *   唯一性/引用存在性/数值范围/状态门 = AppError::business(_displayable) → BUSINESS_ERROR
 *   （chemical_ops/lot.rs::create/consume/scrap、chemical_ops/requisition.rs 各门）；
 *   枚举词表校验 = AppError::validation_displayable → VALIDATION_ERROR
 *   （chemical_service.rs::validate_requisition_type）。
 * 门别串号即判红交后端；永不读取文案（business 族出参脱敏，不可判）。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${r.status} ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

const today = (): string => new Date().toISOString().slice(0, 10);

/**
 * 延迟清理队列（范式同 purchase/03 的 CREATED_ORDER_IDS + afterEach、finance/01 的 CLEANUP[]）：
 * 本 spec 大量用例在自建主数据上还留有后续读断言（by-code 回读、by-no 回读、分页真值、
 * 生产领用按缸号追溯等）。同步 tryCleanup 当场软删会让后端按 is_deleted=false 过滤后查不到行
 * → 回读 404 / 关联校验被拒 → 用例假红、曾被误判成后端缺陷（见 #4669/#4671）。
 * 故所有 housekeeping 清理改为 deferCleanup 登记，统一在每条用例（全部断言之后）flush。
 */
const CLEANUP: DeferredCleanup[] = [];
test.afterEach(async ({ page }) => {
  await flushDeferredCleanups(page, CLEANUP);
});

/** 后端词表合法、且无需附加必填分类字段的化料类型只有 chemical（dye/auxiliary 需附加字段） */
async function createChemicalApi(
  page: import('@playwright/test').Page,
  overrides: Record<string, unknown> = {}
): Promise<{ model: Record<string, unknown>; code: string }> {
  const code = genCode('E2E-CH');
  const res = await apiCall<Record<string, unknown>>(page, 'POST', '/chemicals', {
    chemical_code: code,
    chemical_name: `62号化料${code}`,
    chemical_type: 'chemical',
    unit: 'kg',
    standard_price: '10.50',
    ...overrides,
  });
  const model = res.data;
  expect(model?.id, `化料创建应返回 id：${JSON.stringify(res)}`).toBeTruthy();
  // 62-02 稍后还要 by-code/详情/UI 列表回读并改名；62-03/05 还要以本化料 id 建批——当场软删会 404，
  // 登记到断言之后再清理
  deferCleanup(CLEANUP, 'DELETE', `/chemicals/${Number(model?.id)}`, '[62] chemical');
  return { model, code };
}

/** 列表信封显式钉桩（chemical_handler.rs list_* → data={items,total,page,page_size}）：
 *  先断 items 为数组再取数，禁止 `?? []` 吞缺键。 */
function requireItems(data: { items?: unknown }, endpoint: string): Array<Record<string, unknown>> {
  expect(
    Array.isArray(data?.items),
    `${endpoint} 响应 data.items 必须为数组（真实信封 {items,total}），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data.items as Array<Record<string, unknown>>;
}

/** 缸号创建接口回查用「全局最大 id」（dye_batch_handler.rs:212-217）并发分片下可能返回别人的记录，
 *  因此本用例一律按 batch_no 从列表回查定位自己的缸号。
 *
 * 四维门控前置（dye_batch_handler.rs:203-254 resolve_dye_color_identity 函数体）：
 * color_no 非空 ⇒ 染色布 ⇒ dye_lot_no 必填，且 color_no 必须在色卡档案（color_card_item）
 * 中按 color_code 唯一可查，否则 400「色号…在色卡档案中不存在」。原 seed 提交未入档的
 * 编造色号 `E2E-CN62xxxx` 且缺 dye_lot_no，被正当门控拒绝——按本仓既有 seed 范式
 * （10a-extended-inventory-approval.spec.ts:175-181 / fabric/02-dye.spec.ts:43-49）补前置：
 * 先建专属色卡+唯一色号（值真实可追溯），再带统一取号器生成的缸号建批次。 */
async function seedDyeBatch(page: import('@playwright/test').Page): Promise<number> {
  const batchNo = genCode('E2E-DB62');
  const archive = await seedColorCardArchive(page, { context: '62 缸号链色卡前置' });
  await apiCall(page, 'POST', '/production/dye-batches', {
    batch_no: batchNo,
    color_no: archive.colorCode,
    dye_lot_no: genDyeLotNo(),
    planned_quantity: 100,
  });
  const listed = await apiCallRaw<{ items?: unknown }>(
    page,
    'GET',
    `/production/dye-batches?batch_no=${encodeURIComponent(batchNo)}&page=1&page_size=50`
  );
  const mine = requireItems(listed, 'GET /production/dye-batches').find(
    it => it.batch_no === batchNo
  );
  const id = Number(mine?.id);
  expect(id, `按 batch_no=${batchNo} 应回查到自己创建的缸号`).toBeGreaterThan(0);
  // 62-04 用本缸号 id 建生产领用单并按缸号追溯回读——当场软删会让关联校验/追溯断言失败，登记之后再清理
  deferCleanup(CLEANUP, 'DELETE', `/production/dye-batches/${id}`, '[62] dye-batch');
  return id;
}

test.describe.serial('62 染化料台账→批次→领用→消耗/追溯链路', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  test('62-01 页面可达 + 台账/批次/领用/分类端点严格健康', async ({ page }) => {
    await page.goto(`${BASE_URL}/chemicals`);
    await expect(page.getByRole('tab', { name: '染化料', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '批次管理', exact: true })).toBeVisible();
    await expect(page.getByRole('tab', { name: '分类管理', exact: true })).toBeVisible();

    await verifyEndpointHealthy(page, '/chemicals?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/chemical-lots?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/chemical-requisitions?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/chemical-categories/tree');
  });

  test('62-02 台账主数据：API 建账 → by-code 落库真值 → UI 列表回读 → UI 改名 → 详情回读', async ({
    page,
  }) => {
    const created = await createChemicalApi(page);
    const id = Number(created.model.id);
    const code = created.code;

    // by-code 回读逐字段（真实响应结构：chemical_master::Model snake_case）
    const byCode = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemicals/by-code/${encodeURIComponent(code)}`
    );
    expect(Number(byCode.id), 'by-code 应回读到同一记录').toBe(id);
    expect(byCode.chemical_code, '编码逐字落库').toBe(code);
    expect(byCode.chemical_type, '类型应落库 chemical').toBe('chemical');
    expect(Number(byCode.standard_price), '标准价应落库 10.50').toBe(10.5);
    expect(byCode.unit, '单位应落库 kg').toBe('kg');

    // UI 列表回读（页面 mount 即 GET /chemicals，列表按 id 倒序，新行在首页）
    await page.goto(`${BASE_URL}/chemicals`);
    const row = page.locator('.el-table__row').filter({ hasText: code }).first();
    await expect(row, `染化料列表应回读 ${code}`).toBeVisible({ timeout: 15_000 });

    // UI 编辑改名（编辑链路 payload 与 UpdateChemicalMasterRequest 对齐，是页面真实可用的写路径）
    await row.getByRole('button', { name: '编辑' }).click();
    const dialog = page.getByRole('dialog', { name: '编辑染化料' });
    await expect(dialog).toBeVisible();
    const newName = `62改名${code.slice(-6)}`;
    await fillFieldByLabel(dialog, page, '名称', newName);
    await dialog.getByRole('button', { name: '保存' }).click();
    await expect(page.getByText('已更新')).toBeVisible({ timeout: 15_000 });

    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/chemicals/${id}`);
    expect(after.chemical_name, 'UI 改名必须真实落库').toBe(newName);
    expect(after.chemical_code, '编辑不得丢编码').toBe(code);
  });

  test('62-03 批次链路：建批→检验合格→分页第二页真值→批号回读；负例', async ({ page }) => {
    const chem = await createChemicalApi(page);
    const chemId = Number(chem.model.id);
    const lotNo1 = genCode('E2E-LOT1');
    const lotNo2 = genCode('E2E-LOT2');

    const c1 = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-lots', {
      lot_no: lotNo1,
      chemical_id: chemId,
      quantity_received: '500',
      unit_cost: '2.50',
      supplier_lot_no: 'SUP-A1',
      received_date: today(),
    });
    const lot1 = Number(c1.data?.id);
    expect(lot1, `批次1创建应返回 id：${JSON.stringify(c1)}`).toBeGreaterThan(0);
    expect(Number(c1.data?.quantity_received), '接收数量 500 落库').toBe(500);
    expect(Number(c1.data?.quantity_available), '可用量初始应等于接收量 500').toBe(500);
    expect(Number(c1.data?.total_cost), '总成本 = 500 × 2.5 = 1250 落库').toBe(1250);
    expect(c1.data?.inspection_status, '新建批次检验状态应为 pending').toBe('pending');
    expect(c1.data?.status, '新建批次库存状态应为 active').toBe('active');
    // 批次1 稍后还要 by-no 回读、分页第二页真值、pass-inspection 流转断言——当场软删会 404，登记之后再清理
    deferCleanup(CLEANUP, 'DELETE', `/chemical-lots/${lot1}`, '[62] lot1');

    await apiCall(page, 'POST', '/chemical-lots', {
      lot_no: lotNo2,
      chemical_id: chemId,
      quantity_received: '100',
      unit_cost: '3.00',
    });

    // by-no 回读（列表外的第二读路径）
    const byNo = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemical-lots/by-no/${encodeURIComponent(lotNo1)}`
    );
    expect(Number(byNo.id), 'by-no 应命中批次1').toBe(lot1);
    expect(Number(byNo.chemical_id), '批次须挂在自建化料下').toBe(chemId);

    // 分页真值：chemical_id 精确过滤 + page_size=1，page2 必须返回较早的批次1（id 倒序，
    // 排序真相：chemical_ops/lot.rs list → order_by_desc(Id)）
    const p1 = await apiCallRaw<{ items: Array<Record<string, unknown>>; total: number }>(
      page,
      'GET',
      `/chemical-lots?chemical_id=${chemId}&page=1&page_size=1`
    );
    expect(Number(p1.total), '该化料下批次数应为 2').toBe(2);
    expect(
      Array.isArray(p1.items) && p1.items.length,
      `第一页应真实返回 1 行，实际 ${JSON.stringify(p1)}`
    ).toBe(1);
    const p2 = await apiCallRaw<{ items: Array<Record<string, unknown>> }>(
      page,
      'GET',
      `/chemical-lots?chemical_id=${chemId}&page=2&page_size=1`
    );
    expect(
      Array.isArray(p2.items) && p2.items.length,
      `第二页应真实返回 1 行，实际 ${JSON.stringify(p2)}`
    ).toBe(1);
    expect(p2.items[0].lot_no, '第二页（id 倒序末位）应为先建的批次1').toBe(lotNo1);
    expect(p1.items[0].lot_no, '第一页应为后建的批次2').toBe(lotNo2);

    // 来料检验合格 → 落库回读
    await apiCall(page, 'POST', `/chemical-lots/${lot1}/pass-inspection`, {
      inspection_report_url: '/reports/e2e-62.pdf',
    });
    const passed = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/chemical-lots/${lot1}`);
    expect(passed.inspection_status, '检验后状态应落库 passed').toBe('passed');
    expect(passed.inspection_report_url, '报告地址应落库').toBe('/reports/e2e-62.pdf');

    // 负例：批号重复 / 不存在的化料 id / 负数量
    const dup = await apiCallExpectFail(page, 'POST', '/chemical-lots', {
      lot_no: lotNo1,
      chemical_id: chemId,
      quantity_received: '1',
    });
    expectRejected(dup, `重复批号 ${lotNo1}`, APP_ERROR_CODES.BUSINESS_ERROR);
    const ghost = await apiCallExpectFail(page, 'POST', '/chemical-lots', {
      lot_no: genCode('E2E-LOTG'),
      chemical_id: 999999999,
      quantity_received: '1',
    });
    expectRejected(ghost, '不存在化料 id 建批', APP_ERROR_CODES.BUSINESS_ERROR);
    const neg = await apiCallExpectFail(page, 'POST', '/chemical-lots', {
      lot_no: genCode('E2E-LOTN'),
      chemical_id: chemId,
      quantity_received: '-1',
    });
    expectRejected(neg, '负接收数量', APP_ERROR_CODES.BUSINESS_ERROR);
  });

  test('62-04 领用链路：生产领用→审批→发料→关闭逐态回读 + 按缸号追溯；负例与删除门控', async ({
    page,
  }) => {
    const ctx = getCtx();
    const dyeBatchId = await seedDyeBatch(page);
    const deptId = ctx.departmentIds[0];

    const mk = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'production',
      department_id: deptId ?? null,
      requisition_date: today(),
      dye_batch_id: dyeBatchId,
      total_amount: '100.00',
      remarks: 'E2E-62 生产领用',
    });
    const id = Number(mk.data?.id);
    const no = String(mk.data?.requisition_no);
    expect(id, `领用单创建应返回 id：${JSON.stringify(mk)}`).toBeGreaterThan(0);
    expect(no, '领用单号应服务端生成为 CR 前缀').toMatch(/^CR\d{8}\d{3}$/);
    expect(mk.data?.status, '新建领用单应为 draft').toBe('draft');
    expect(Number(mk.data?.dye_batch_id), '领用单应绑定自建缸号').toBe(dyeBatchId);

    // 逐态流转 + 每步回读（流转即断言，不事后补看）
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/approve`);
    const approved = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemical-requisitions/${id}`
    );
    expect(approved.status, '审批后应落库 approved').toBe('approved');

    await apiCall(page, 'POST', `/chemical-requisitions/${id}/issue`);
    const issued = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemical-requisitions/${id}`
    );
    expect(issued.status, '发料后应落库 issued').toBe('issued');

    await apiCall(page, 'POST', `/chemical-requisitions/${id}/close`);
    const closed = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemical-requisitions/${id}`
    );
    expect(closed.status, '关闭后应落库 closed').toBe('closed');

    // by-no 读路径与详情一致
    const byNo = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/chemical-requisitions/by-no/${encodeURIComponent(no)}`
    );
    expect(Number(byNo.id), 'by-no 应命中同一领用单').toBe(id);
    expect(byNo.status, 'by-no 回读状态一致').toBe('closed');

    // 追溯：按缸号过滤领用列表，必须命中本用例的生产领用单
    const trace = await apiCallRaw<{ items?: unknown }>(
      page,
      'GET',
      `/chemical-requisitions?dye_batch_id=${dyeBatchId}&page=1&page_size=50`
    );
    const hit = requireItems(trace, 'GET /chemical-requisitions?dye_batch_id').find(
      it => it.requisition_no === no
    );
    expect(hit, `缸号 ${dyeBatchId} 追溯应命中领用单 ${no}`).toBeTruthy();
    expect(hit?.requisition_type, '追溯命中记录类型应为 production').toBe('production');
    expect(Number(hit?.total_amount), '追溯命中记录金额应为 100').toBe(100);

    // 负例（自建自流转，不复用上单）：
    // ① 非法类型词表值（chemical_service.rs::validate_requisition_type → validation_displayable）
    const badType = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'warehouse',
      requisition_date: today(),
    });
    expectRejected(badType, '非法领用类型 warehouse', APP_ERROR_CODES.VALIDATION_ERROR);
    // ② 生产领用缺缸号（chemical_ops/requisition.rs::create → AppError::business 内控门）
    const missBatch = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'production',
      requisition_date: today(),
    });
    expectRejected(missBatch, '生产领用缺 dye_batch_id', APP_ERROR_CODES.BUSINESS_ERROR);
    // ③ 缸号不存在（同函数引用存在性门 → AppError::business）
    const ghostBatch = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'production',
      requisition_date: today(),
      dye_batch_id: 999999999,
    });
    expectRejected(ghostBatch, '不存在的缸号', APP_ERROR_CODES.BUSINESS_ERROR);
    // ④ draft 直接发料必须被状态机拒绝（requisition.rs::issue 仅 approved 可发料）
    const d2 = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
    });
    const id2 = Number(d2.data?.id);
    expect(id2, '化验室领用应自建成功').toBeGreaterThan(0);
    const illegalIssue = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-requisitions/${id2}/issue`
    );
    expectRejected(illegalIssue, 'draft 直接 issue', APP_ERROR_CODES.BUSINESS_ERROR);
    // ⑤ 终态后重复审批拒绝（requisition.rs::approve 仅 draft 可审批）
    const reApprove = await apiCallExpectFail(page, 'POST', `/chemical-requisitions/${id}/approve`);
    expectRejected(reApprove, 'closed 再 approve', APP_ERROR_CODES.BUSINESS_ERROR);
    // ⑥ 删除门控：closed 不可删、draft 可删（删后 by-no 404）（requisition.rs::delete 仅 draft）
    const delClosed = await apiCallExpectFail(page, 'DELETE', `/chemical-requisitions/${id}`);
    expectRejected(delClosed, 'closed 领用单删除', APP_ERROR_CODES.BUSINESS_ERROR);
    await apiCall(page, 'DELETE', `/chemical-requisitions/${id2}`);
    const gone = await apiCallExpectFail(
      page,
      'GET',
      `/chemical-requisitions/by-no/${String(d2.data?.requisition_no)}`
    );
    expect(gone.status, '软删除后 by-no 应 404').toBe(404);
    expect(failureCode(gone), '软删除后机器码 NOT_FOUND').toBe('NOT_FOUND');
  });

  test('62-05 批次消耗/报废终态回读 + 负例', async ({ page }) => {
    const chem = await createChemicalApi(page);
    const mk = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-lots', {
      lot_no: genCode('E2E-LOTC'),
      chemical_id: Number(chem.model.id),
      quantity_received: '200',
      unit_cost: '1.25',
    });
    const id = Number(mk.data?.id);
    expect(id, '待消耗批次应创建成功').toBeGreaterThan(0);

    const consumed = await apiCall<Record<string, unknown>>(
      page,
      'POST',
      `/chemical-lots/${id}/consume`
    );
    expect(consumed.data?.status, 'consume 后应落库 consumed').toBe('consumed');
    expect(Number(consumed.data?.quantity_available), 'consume 后可用量应清零').toBe(0);
    const detail = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/chemical-lots/${id}`);
    expect(detail.status, '详情回读同为 consumed').toBe('consumed');

    // 非 active 终态再 consume / scrap 均被拒（chemical_ops/lot.rs::consume/scrap 状态门）
    const again = await apiCallExpectFail(page, 'POST', `/chemical-lots/${id}/consume`);
    expectRejected(again, 'consumed 再 consume', APP_ERROR_CODES.BUSINESS_ERROR);
    const scrapAfter = await apiCallExpectFail(page, 'POST', `/chemical-lots/${id}/scrap`);
    expectRejected(scrapAfter, 'consumed 后 scrap', APP_ERROR_CODES.BUSINESS_ERROR);

    // 报废正例（自建新批次，不依赖上单）
    const mk2 = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-lots', {
      lot_no: genCode('E2E-LOTS'),
      chemical_id: Number(chem.model.id),
      quantity_received: '10',
    });
    const scrapped = await apiCall<Record<string, unknown>>(
      page,
      'POST',
      `/chemical-lots/${Number(mk2.data?.id)}/scrap`
    );
    expect(scrapped.data?.status, 'scrap 后应落库 scrapped').toBe('scrapped');
  });

  test('62-06 单据打印与缸号 xlsx 导出：二进制端点走 download 严格健康', async ({ page }) => {
    const mk = await apiCall<Record<string, unknown>>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'rd',
      requisition_date: today(),
    });
    const id = Number(mk.data?.id);
    expect(id, '研发领用单应创建成功').toBeGreaterThan(0);
    await verifyDownloadEndpointHealthy(page, `/chemical-requisitions/${id}/print`);
    // 缸号导出为 xlsx（dye_batch_handler.rs:404-426 build_xlsx_response）——
    // JSON helper 会对其 JSON.parse 假红，必须走 download helper
    await verifyDownloadEndpointHealthy(page, '/production/dye-batches/export');
    await apiCall(page, 'DELETE', `/chemical-requisitions/${id}`);
  });

  test('62-07 批次管理/分类管理 Tab 容器渲染（列表自动加载缺失属前端缺陷，见报告）', async ({
    page,
  }) => {
    await page.goto(`${BASE_URL}/chemicals`);
    await page.getByRole('tab', { name: '批次管理', exact: true }).click();
    await expect(
      page.locator('.el-tabs__content .el-table').nth(1),
      '批次管理 Tab 表格容器应渲染'
    ).toBeVisible({ timeout: 15_000 });
    await page.getByRole('tab', { name: '分类管理', exact: true }).click();
    await expect(
      page.getByRole('button', { name: '新建分类' }),
      '分类管理 Tab 工具栏应渲染'
    ).toBeVisible({ timeout: 15_000 });
    // 说明：切 Tab 不触发数据加载（chemicals/index.vue:269 onMounted 仅 load()，
    // loadLots/loadCategories 只在保存/删除回调里被调用，无 activeTab watch）。
    // 不在这里断言列表行数（会恒 0），该缺陷作为源码问题上报，不靠断言空表掩盖。
  });
});
