import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  failureCode,
  tryCleanup,
  verifyEndpointHealthy,
  APP_ERROR_CODES,
  BASE_URL,
  type ApiFailureResult,
} from './helpers';

/**
 * 70 染化料链契约缺口补测（分类树 / 主数据状态机与软删 / 批次检验-效期-改价 / 领用更新-取消门控）
 *
 * 与既有覆盖的分工：62 号覆盖「主数据建账+by-code 回读+UI 编辑改名、批次正例链+分页真值、
 * 领用 approve→issue→close 主干+缸号追溯、consume/scrap 终态」；52 号覆盖价格/库存负值。
 * 本 spec 只补这些文件**从未触达**的真实契约面：
 * - 分类域 CRUD/树/删除守卫（62 仅 tree 健康探针）；
 * - 主数据词表必填对（dye→dye_category / auxiliary→auxiliary_category）、状态流转、
 *   非法状态拒绝、软删除后 by-code/详情/list 三读路径一致 404、软删后编码可复用；
 * - 批次 fail-inspection 及其状态机闭包（failed 后再 pass 必须被拒+无痕）、PUT 改单位成本
 *   重算总成本、expiry_before 近效期过滤真实生效、批次软删后 by-no 404、更新不存在批次 404；
 * - 领用单 PUT 的 draft 门控（approved 后更新被拒且原值未变）、cancel 分支（draft→cancelled；
 *   cancelled 不可 approve；closed 不可 cancel；cancelled 不可删但 by-no 仍可读）、缺必填/幽灵引用负例。
 *
 * 端点与信封真相（routes/catalog.rs:151-262，catalog 整体 merge 进 /api/v1/erp 根）：
 * - GET/POST /chemicals、/chemicals/{id}(GET/PUT/DELETE)、/chemicals/by-code/{code}   catalog.rs:152-168
 * - GET/POST /chemical-categories、/chemical-categories/tree、/chemical-categories/{id}(GET/PUT/DELETE)
 *                                                                          catalog.rs:170-188
 * - GET/POST /chemical-lots、/chemical-lots/by-no/{no}、/chemical-lots/{id}(GET/PUT/DELETE)、
 *   POST /chemical-lots/{id}/pass-inspection|fail-inspection|consume|scrap             catalog.rs:190-215
 * - GET/POST /chemical-requisitions、/chemical-requisitions/by-no/{no}、/{id}(GET/PUT/DELETE)、
 *   POST /{id}/approve|issue|close|cancel                                              catalog.rs:218-262
 * 四个列表端点出参 PaginatedResponse{items,total,page,page_size}（utils/response.rs:34-39；
 * handler chemical_handler.rs:150-153/:220-223/:294-297/:410-413）——本 spec 对分页信封
 * **四键全断**；GET /chemical-categories/tree 出参是 Vec 裸数组（handler :263-269），单独钉桩。
 * 单对象端点（by-code/by-no/{id}/动作流转）data=模型本体（handler 直接 ApiResponse::success(model)）。
 *
 * 状态词表真相（写入方 models/status/wage_energy_chemical_business.rs）：
 * chemical_type dye/auxiliary/chemical（:161-170）；chemical_status active/inactive/discontinued
 * （:173-182）；inspection pending/passed/failed/quarantine（:185-197）；
 * lot active/consumed/expired/scrapped（:200-211）；requisition
 * draft/approved/issued/partial_returned/closed/cancelled（:222- 起）。
 * 业务校验点：主数据必填对 services/chemical_ops/master.rs:53-58；状态词表 :376-396；
 * 软删仅查未删（唯一性同口径）:99-115/:409-425；批次检验闭包 lot.rs:186-233
 * （pending/quarantine 才可 pass/fail）；PUT 改价重算 lot.rs:150-168；expiry_before lte lot.rs:308-310；
 * 领用 draft 更新门控 requisition.rs:142-153；delete 门控 :185-192；cancel 分支 :261-274。
 *
 * 不可 CI 化项（如实声明，不造假、不用 optional 版吞绿）：
 * ① quarantine 隔离态：**无任何已注册端点能把 inspection_status 置为 quarantine**
 *    （CreateChemicalLotRequest 固定 pending，lot.rs:90；UpdateChemicalLotRequest 无该键，types.rs:176-188）
 *    → "quarantine→passed/fail 流转"与"quarantine 禁领"CI 不可造，本 spec 不测；
 * ② check_low_stock（chemical_service.rs:148-156 低库存纯函数）**没有对应端点**（全仓 grep 仅
 *    inventory 域的独立同名函数被路由引用）→ 染化料低库存预警 CI 不可测，本 spec 不测；
 * ③ partial_returned 态同样无端点入口（无 return 动作路由）→ 不测。
 * 以上作为"端点缺失/仅单测覆盖"上报主编排。
 *
 * 假绿防线：全部已注册端点严格 verifyEndpointHealthy；写后 GET 回读落库真值、被拒写回读断
 * 无痕/原值未变；rust_decimal 字符串出参一律 Number() 归一；禁 `?? []`/双形状探测/兜底默认值；
 * 数据自建自流转（genCode 唯一码），软删记录用 tryCleanup 幂等兜底清理。
 */

type Row = Record<string, unknown>;

/** rust_decimal 经 JSON 序列化为字符串，比较前归一 */
const toNum = (v: unknown): number => Number(String(v));

const today = (): string => new Date().toISOString().slice(0, 10);
const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

const APP_REJECT_CODES: string[] = [
  APP_ERROR_CODES.VALIDATION_ERROR,
  APP_ERROR_CODES.BUSINESS_ERROR,
  APP_ERROR_CODES.BAD_REQUEST,
];

function expectRejected(r: ApiFailureResult, what: string): void {
  expect(
    r.status,
    `${what}：应被 4xx 拒绝，实际 ${r.status} ${JSON.stringify(r)}`
  ).toBeGreaterThanOrEqual(400);
  expect(r.status, `${what}：不得以 5xx 冒充拒绝（${JSON.stringify(r)}）`).toBeLessThan(500);
  expect(
    APP_REJECT_CODES,
    `${what}：机器码应为校验/业务/请求类，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toContain(failureCode(r));
}

/** 提取器层拒绝（缺必填 JSON 键）走 axum 默认纯文本 400，非统一 JSON 信封——只断真实 status */
function expectExtractorReject(r: ApiFailureResult, what: string): void {
  expect(r.status, `${what}：应被 400 拒绝（提取器层），实际 ${JSON.stringify(r)}`).toBe(400);
}

/** 分页信封四键全断（PaginatedResponse{items,total,page,page_size}，utils/response.rs:34-39） */
async function readPaged(page: Page, path: string, endpoint: string): Promise<Row[]> {
  const data = await apiCallRaw<{
    items?: unknown;
    total?: unknown;
    page?: unknown;
    page_size?: unknown;
  }>(page, 'GET', path);
  expect(
    Array.isArray(data?.items),
    `${endpoint} data.items 必须为数组，实际 ${JSON.stringify(data)}`
  ).toBe(true);
  expect(typeof data.total, `${endpoint} total 必须为数字（禁 ?? 0 兜底）`).toBe('number');
  expect(typeof data.page, `${endpoint} page 必须回显（PaginatedResponse 四键齐全）`).toBe(
    'number'
  );
  expect(typeof data.page_size, `${endpoint} page_size 必须回显`).toBe('number');
  return data.items as Row[];
}

/** 裸数组信封钉桩（GET /chemical-categories/tree → Vec<CategoryModel>） */
function requirePlainArray(data: unknown, endpoint: string): Row[] {
  expect(
    Array.isArray(data),
    `${endpoint} 出参 data 必须为数组（handler 直接 to_value(Vec)），实际 ${JSON.stringify(data)}`
  ).toBe(true);
  return data as Row[];
}

function findRowById(items: Row[], id: number, endpoint: string): Row {
  const row = items.find(it => Number(it.id) === id);
  expect(
    row,
    `${endpoint}：应回读到自建记录 id=${id}，实际 ${JSON.stringify(items).slice(0, 500)}`
  ).toBeTruthy();
  return row as Row;
}

/** 自建化料主数据（type=chemical 无附加必填，master.rs:49-96 真相） */
async function createChemical(
  page: Page,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; code: string }> {
  const code = genCode('E2E70CH');
  const res = await apiCall<Row>(page, 'POST', '/chemicals', {
    chemical_code: code,
    chemical_name: `70号化料${code}`,
    chemical_type: 'chemical',
    unit: 'kg',
    standard_price: '10.00',
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `化料创建应返回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  await tryCleanup(page, 'DELETE', `/chemicals/${id}`, `[70] chemical ${code}`);
  return { id, code };
}

/** 自建批次 */
async function createLot(
  page: Page,
  chemicalId: number,
  overrides: Record<string, unknown> = {}
): Promise<{ id: number; lotNo: string }> {
  const lotNo = genCode('E2E70LOT');
  const res = await apiCall<Row>(page, 'POST', '/chemical-lots', {
    lot_no: lotNo,
    chemical_id: chemicalId,
    quantity_received: '10',
    unit_cost: '1.00',
    ...overrides,
  });
  const id = Number(res.data?.id);
  expect(id, `批次创建应返回 id：${JSON.stringify(res)}`).toBeGreaterThan(0);
  await tryCleanup(page, 'DELETE', `/chemical-lots/${id}`, `[70] lot ${lotNo}`);
  return { id, lotNo };
}

test.describe.serial('70 染化料契约缺口：分类/状态机/检验闭包/效期/更新与取消门控', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('70-01 分类树：建根+子→tree 真值→停用出树→删除守卫→软删 404→软删后编码复用；负例', async ({
    page,
  }) => {
    await verifyEndpointHealthy(page, '/chemical-categories?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/chemical-categories/tree');

    // 根分类（category_type 复用 chemical_type 词表：validate_chemical_type，category.rs:35）
    const rootCode = genCode('E2E70CRT');
    const childCode = genCode('E2E70CRC');
    const root = await apiCall<Row>(page, 'POST', '/chemical-categories', {
      category_code: rootCode,
      category_name: `70根分类${rootCode}`,
      category_type: 'dye',
      sort_order: 10,
    });
    const rootId = Number(root.data?.id);
    expect(rootId, `根分类创建应返回 id：${JSON.stringify(root)}`).toBeGreaterThan(0);
    expect(root.data?.is_active, '新建分类默认启用（category.rs:72）').toBe(true);
    expect(root.data?.parent_id, '根分类 parent_id 应落 NULL').toBeNull();
    expect(Number(root.data?.sort_order), 'sort_order 应逐字落库').toBe(10);

    const child = await apiCall<Row>(page, 'POST', '/chemical-categories', {
      category_code: childCode,
      category_name: `70子分类${childCode}`,
      category_type: 'dye',
      parent_id: rootId,
    });
    const childId = Number(child.data?.id);
    expect(childId, `子分类创建应返回 id：${JSON.stringify(child)}`).toBeGreaterThan(0);
    expect(Number(child.data?.parent_id), '父子挂接应落库 parent_id').toBe(rootId);
    expect(Number(child.data?.sort_order), 'sort_order 缺省落 0（category.rs:71）').toBe(0);

    // tree 裸数组：根集合含 root；?parent_id=root 集合含 child
    const rootTree = requirePlainArray(
      await apiCallRaw<unknown>(page, 'GET', '/chemical-categories/tree'),
      'GET /chemical-categories/tree'
    );
    expect(
      rootTree.some(it => Number(it.id) === rootId),
      `tree（parent_id IS NULL）应含自建根分类 id=${rootId}`
    ).toBe(true);
    const childTree = requirePlainArray(
      await apiCallRaw<unknown>(page, 'GET', `/chemical-categories/tree?parent_id=${rootId}`),
      `GET /chemical-categories/tree?parent_id=${rootId}`
    );
    const childInTree = findRowById(childTree, childId, 'tree?parent_id 子集合');
    expect(childInTree.category_type, '树节点类型应逐字落库 dye').toBe('dye');

    // 停用子分类：tree 只回 is_active=true（category.rs:185）→ 出树；列表按 is_active=false 可回读
    await apiCall(page, 'PUT', `/chemical-categories/${childId}`, { is_active: false });
    const childTreeAfter = requirePlainArray(
      await apiCallRaw<unknown>(page, 'GET', `/chemical-categories/tree?parent_id=${rootId}`),
      'GET tree?parent_id（停用后）'
    );
    expect(
      childTreeAfter.some(it => Number(it.id) === childId),
      '停用分类不得再出现在 tree（tree 过滤 is_active=true）'
    ).toBe(false);
    const inactiveList = await readPaged(
      page,
      `/chemical-categories?parent_id=${rootId}&is_active=false&page=1&page_size=50`,
      'GET /chemical-categories?is_active=false'
    );
    expect(
      findRowById(inactiveList, childId, '停用列表回读').is_active,
      '停用应真实落库 is_active=false'
    ).toBe(false);

    // 删除守卫：父有未删子分类必拒（category.rs:124-133），且被拒无痕（GET 父仍 200）
    const delParent = await apiCallExpectFail(page, 'DELETE', `/chemical-categories/${rootId}`);
    expectRejected(delParent, '存在子分类时删除父分类');
    const parentStill = await apiCallRaw<Row>(page, 'GET', `/chemical-categories/${rootId}`);
    expect(Number(parentStill.id), '被拒删除后父分类应仍可读').toBe(rootId);

    // 软删子分类 → 详情 404 NOT_FOUND；随后同编码可重建（唯一性仅查未删，category.rs:50-60）
    await apiCall(page, 'DELETE', `/chemical-categories/${childId}`);
    const gone = await apiCallExpectFail(page, 'GET', `/chemical-categories/${childId}`);
    expect(gone.status, '软删后分类详情应 404').toBe(404);
    expect(failureCode(gone), '软删后机器码 NOT_FOUND').toBe('NOT_FOUND');
    const recreated = await apiCall<Row>(page, 'POST', '/chemical-categories', {
      category_code: childCode,
      category_name: `70重建${childCode}`,
      category_type: 'dye',
      parent_id: rootId,
    });
    const newChildId = Number(recreated.data?.id);
    expect(
      newChildId > 0 && newChildId !== childId,
      `软删后同编码应可重建（新 id）：${JSON.stringify(recreated)}`
    ).toBe(true);
    await tryCleanup(page, 'DELETE', `/chemical-categories/${newChildId}`, '[70] category 重建');
    await tryCleanup(page, 'DELETE', `/chemical-categories/${rootId}`, '[70] category 根');

    // 负例：词表外类型 / 父不存在 / 编码重复（均为 service 层 AppError，统一信封）
    const badType = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: genCode('E2E70CBT'),
      category_name: '词表外类型',
      category_type: 'other',
    });
    expectRejected(badType, "category_type='other' 不在 dye/auxiliary/chemical 词表");
    const ghostParent = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: genCode('E2E70CGP'),
      category_name: '幽灵父',
      category_type: 'dye',
      parent_id: 999999999,
    });
    expectRejected(ghostParent, '父分类不存在应被拒（category.rs:38-47）');
    const dupCode = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: rootCode,
      category_name: '重复编码',
      category_type: 'dye',
    });
    expectRejected(dupCode, `未删记录中重复编码 ${rootCode} 应被拒`);
    // 缺必填 category_code → 提取器 400（types.rs:121-129 必填非 Option）
    const missingKey = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_name: '缺编码键',
      category_type: 'dye',
    });
    expectExtractorReject(missingKey, '缺 category_code 必填应 400');
  });

  test('70-02 主数据：必填对词表/状态流转/非法状态原值不变/软删三读路径 404/编码复用', async ({
    page,
  }) => {
    // dye 缺 dye_category、auxiliary 缺 auxiliary_category 必拒（master.rs:53-58）
    const dyeMiss = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70DM'),
      chemical_name: '缺染料类别',
      chemical_type: 'dye',
    });
    expectRejected(dyeMiss, 'dye 缺 dye_category（master.rs:53-55）');
    const auxMiss = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70AM'),
      chemical_name: '缺助剂类别',
      chemical_type: 'auxiliary',
    });
    expectRejected(auxMiss, 'auxiliary 缺 auxiliary_category（master.rs:56-58）');
    // 词表外类型 'other'（UI 弹窗第三项，62 号头部缺陷③）→ 必须被拒
    const otherType = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70OT'),
      chemical_name: '词表外 other',
      chemical_type: 'other',
    });
    expectRejected(otherType, "chemical_type='other' 不在词表（chemical_service.rs:63-76）");

    // 合法 dye 正例（GHS/MSDS 字段链：msds_url 提供则 msds_updated_at 落值，master.rs:81-85）
    const { id, code } = await createChemical(page, {
      chemical_type: 'dye',
      dye_category: 'reactive',
      color_index: 'C.I. Reactive Blue 19',
      ghs_classification: 'Eye Irrit. 2',
      signal_word: 'warning',
      msds_url: '/msds/e2e-70.pdf',
      safety_stock: '5',
      reorder_point: '8',
    });
    const detail = await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`);
    expect(detail.dye_category, 'dye_category 应落库 reactive').toBe('reactive');
    expect(detail.color_index, 'color_index 应逐字落库').toBe('C.I. Reactive Blue 19');
    expect(detail.signal_word, 'signal_word 应落库 warning').toBe('warning');
    expect(
      detail.msds_updated_at,
      '提交 msds_url 时 msds_updated_at 必须由后端写入（非请求键）'
    ).toBeTruthy();
    expect(toNum(detail.safety_stock), 'safety_stock 应落库 5').toBe(5);
    expect(toNum(detail.reorder_point), 'reorder_point 应落库 8').toBe(8);
    expect(detail.status, '新建主数据初始状态 active（master.rs:171）').toBe('active');

    // 状态流转 active→inactive→discontinued 每步回读 + list?status 过滤真值
    await apiCall(page, 'PUT', `/chemicals/${id}`, { status: 'inactive' });
    expect(
      (await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).status,
      '第一次流转应落库 inactive'
    ).toBe('inactive');
    await apiCall(page, 'PUT', `/chemicals/${id}`, { status: 'discontinued' });
    const discList = await readPaged(
      page,
      '/chemicals?status=discontinued&page=1&page_size=100',
      'GET /chemicals?status=discontinued'
    );
    expect(
      findRowById(discList, id, 'list?status=discontinued 过滤').status,
      '状态过滤应命中自建 discontinued 记录'
    ).toBe('discontinued');

    // 非法状态词表外（master.rs:376-396）→ 拒绝且回读原值未变
    const badStatus = await apiCallExpectFail(page, 'PUT', `/chemicals/${id}`, {
      status: 'archived',
    });
    expectRejected(badStatus, "status='archived' 不在 active/inactive/discontinued 词表");
    expect(
      (await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).status,
      '被拒状态更新后落库值应仍为 discontinued（无痕）'
    ).toBe('discontinued');
    // 负价 PUT（master.rs:263-267）→ 拒绝且标准价未变
    const priceBefore = toNum(
      (await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).standard_price
    );
    const badPrice = await apiCallExpectFail(page, 'PUT', `/chemicals/${id}`, {
      standard_price: '-1.00',
    });
    expectRejected(badPrice, 'PUT 负标准价应被拒');
    expect(
      toNum((await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).standard_price),
      '被拒改价后应仍为原值（无痕）'
    ).toBe(priceBefore);

    // 软删：by-code / 详情 / list 三读路径一致不可见（master.rs:399-425 过滤 is_deleted=false）
    await apiCall(page, 'DELETE', `/chemicals/${id}`);
    const goneByCode = await apiCallExpectFail(
      page,
      'GET',
      `/chemicals/by-code/${encodeURIComponent(code)}`
    );
    expect(goneByCode.status, '软删后 by-code 应 404').toBe(404);
    expect(failureCode(goneByCode), 'by-code 404 机器码 NOT_FOUND').toBe('NOT_FOUND');
    const goneById = await apiCallExpectFail(page, 'GET', `/chemicals/${id}`);
    expect(goneById.status, '软删后详情应 404').toBe(404);
    const kwList = await readPaged(
      page,
      `/chemicals?keyword=${encodeURIComponent(code)}&page=1&page_size=50`,
      'GET /chemicals?keyword（软删后）'
    );
    expect(
      kwList.filter(it => it.chemical_code === code).length,
      '软删记录不得再出现在列表（keyword 命中集）'
    ).toBe(0);
    // 软删后编码可复用（唯一性只查未删，master.rs:99-115）
    const revived = await apiCall<Row>(page, 'POST', '/chemicals', {
      chemical_code: code,
      chemical_name: `70重建${code}`,
      chemical_type: 'chemical',
    });
    const newId = Number(revived.data?.id);
    expect(newId > 0 && newId !== id, `软删后同编码应可重建：${JSON.stringify(revived)}`).toBe(
      true
    );
    await tryCleanup(page, 'DELETE', `/chemicals/${newId}`, '[70] chemical 重建');

    // UI 页面与端点严格健康（不重复 62 的 UI 编辑链路）
    await verifyEndpointHealthy(page, `/chemicals/by-code/${encodeURIComponent(code)}`);
    await page.goto(`${BASE_URL}/chemicals`);
    await expect(page.getByRole('tab', { name: '染化料', exact: true })).toBeVisible();
  });

  test('70-03 批次：fail-inspection 闭包/改价重算总成本/expiry_before 近效期/软删 404/负例', async ({
    page,
  }) => {
    const chem = await createChemical(page);

    // 不合格检验链：pending → failed，报告 URL 同体落库；failed 后 pass 必须被拒且无痕
    //（检验闭包只允许 pending/quarantine 出发，lot.rs:186-233；quarantine 无端点入口见头部声明①）
    const failLot = await createLot(page, chem.id, {
      quantity_received: '10',
      unit_cost: '1.00',
    });
    await apiCall(page, 'POST', `/chemical-lots/${failLot.id}/fail-inspection`, {
      inspection_report_url: '/reports/e2e-70-fail.pdf',
    });
    const failed = await apiCallRaw<Row>(page, 'GET', `/chemical-lots/${failLot.id}`);
    expect(failed.inspection_status, 'fail-inspection 应落库 failed').toBe('failed');
    expect(failed.inspection_report_url, '不合格报告地址应落库').toBe('/reports/e2e-70-fail.pdf');
    const revivePass = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-lots/${failLot.id}/pass-inspection`,
      {}
    );
    expectRejected(revivePass, 'failed 终态再 pass-inspection 应被状态机拒绝');
    expect(
      (await apiCallRaw<Row>(page, 'GET', `/chemical-lots/${failLot.id}`)).inspection_status,
      '被拒 pass 后应仍为 failed（无痕）'
    ).toBe('failed');
    const failFilter = await readPaged(
      page,
      `/chemical-lots?chemical_id=${chem.id}&inspection_status=failed&page=1&page_size=50`,
      'GET /chemical-lots?inspection_status=failed'
    );
    expect(
      findRowById(failFilter, failLot.id, 'failed 过滤回读').inspection_status,
      'failed 过滤应命中自建批次'
    ).toBe('failed');

    // PUT 改单位成本 → total_cost 按 quantity_received 重算（lot.rs:150-168），量字段不动
    const costLot = await createLot(page, chem.id, {
      quantity_received: '500',
      unit_cost: '2.50',
    });
    const before = await apiCallRaw<Row>(page, 'GET', `/chemical-lots/${costLot.id}`);
    expect(toNum(before.total_cost), '初始总成本 500×2.5=1250').toBe(1250);
    await apiCall(page, 'PUT', `/chemical-lots/${costLot.id}`, {
      unit_cost: '3.00',
      remarks: 'E2E70 改价',
    });
    const after = await apiCallRaw<Row>(page, 'GET', `/chemical-lots/${costLot.id}`);
    expect(toNum(after.unit_cost), '新单位成本应落库 3').toBe(3);
    expect(toNum(after.total_cost), '总成本应重算为 500×3=1500').toBe(1500);
    expect(toNum(after.quantity_received), '改价不得动接收量').toBe(500);
    expect(after.remarks, '同体 remarks 应落库').toBe('E2E70 改价');
    // 负单位成本 → 拒绝且原重算值未变（无痕）
    const negCost = await apiCallExpectFail(page, 'PUT', `/chemical-lots/${costLot.id}`, {
      unit_cost: '-1',
    });
    expectRejected(negCost, 'PUT 负单位成本应被拒（lot.rs:151-153）');
    expect(
      toNum((await apiCallRaw<Row>(page, 'GET', `/chemical-lots/${costLot.id}`)).total_cost),
      '被拒改价后总成本应仍为 1500（无痕）'
    ).toBe(1500);
    // 不存在批次 PUT → 404 NOT_FOUND（lot.rs:269-275）
    const ghostPut = await apiCallExpectFail(page, 'PUT', '/chemical-lots/999999999', {
      unit_cost: '1',
    });
    expect(ghostPut.status, '不存在批次更新应 404').toBe(404);
    expect(failureCode(ghostPut), '404 机器码 NOT_FOUND').toBe('NOT_FOUND');

    // expiry_before 近效期过滤（lte，lot.rs:308-310）：+5d 命中、+40d 不命中，且结果集逐条满足阈值
    const nearLot = await createLot(page, chem.id, { expiry_date: ymd(5) });
    const farLot = await createLot(page, chem.id, { expiry_date: ymd(40) });
    const cutoff = ymd(10);
    const nearList = await readPaged(
      page,
      `/chemical-lots?chemical_id=${chem.id}&expiry_before=${cutoff}&page=1&page_size=50`,
      'GET /chemical-lots?expiry_before'
    );
    expect(
      nearList.some(it => Number(it.id) === nearLot.id),
      `近效期批次（+5d ≤ ${cutoff}）应命中`
    ).toBe(true);
    expect(
      nearList.some(it => Number(it.id) === farLot.id),
      '远效期批次（+40d）不得混入'
    ).toBe(false);
    for (const it of nearList) {
      expect(
        String(it.expiry_date) <= cutoff,
        `expiry_before 结果行应逐条 expiry_date ≤ ${cutoff}：${JSON.stringify(it)}`
      ).toBe(true);
    }

    // 批次软删 → by-no 404 NOT_FOUND（62 仅断过领用单软删路径，批次读路径此处补齐）
    const delLotNo = nearLot.lotNo;
    await apiCall(page, 'DELETE', `/chemical-lots/${nearLot.id}`);
    const lotGone = await apiCallExpectFail(
      page,
      'GET',
      `/chemical-lots/by-no/${encodeURIComponent(delLotNo)}`
    );
    expect(lotGone.status, '软删后 by-no 应 404').toBe(404);
    expect(failureCode(lotGone), 'by-no 404 机器码 NOT_FOUND').toBe('NOT_FOUND');
  });

  test('70-04 领用单：draft 更新门控/cancel 分支闭包/closed 不可取消/负例无痕', async ({
    page,
  }) => {
    // draft 更新正例：remarks/required_date 落库回读（requisition.rs:157-177）
    const mk = await apiCall<Row>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      remarks: 'E2E70-R1',
    });
    const id = Number(mk.data?.id);
    const no = String(mk.data?.requisition_no);
    expect(id, `领用单创建应返回 id：${JSON.stringify(mk)}`).toBeGreaterThan(0);
    expect(mk.data?.status, '新建领用单初始 draft（requisition.rs:122）').toBe('draft');
    await tryCleanup(page, 'DELETE', `/chemical-requisitions/${id}`, `[70] req ${no}`);

    const requiredDate = ymd(7);
    await apiCall(page, 'PUT', `/chemical-requisitions/${id}`, {
      remarks: 'E2E70-R2',
      required_date: requiredDate,
    });
    const updated = await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id}`);
    expect(updated.remarks, 'draft PUT remarks 应落库 R2').toBe('E2E70-R2');
    expect(String(updated.required_date), 'required_date 应逐字落库').toBe(requiredDate);

    // approve 后 PUT 必拒（仅 draft 可更新，:148-153），且被拒更新无痕
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/approve`);
    const latePut = await apiCallExpectFail(page, 'PUT', `/chemical-requisitions/${id}`, {
      remarks: 'E2E70-应被拒',
    });
    expectRejected(latePut, 'approved 后 PUT 更新');
    const afterReject = await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id}`);
    expect(afterReject.remarks, '被拒 PUT 后 remarks 应仍为 R2（原值未变）').toBe('E2E70-R2');
    expect(afterReject.status, '被拒 PUT 后状态应仍为 approved').toBe('approved');

    // closed 终态不可 cancel（:263-265），且状态未变
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/issue`);
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/close`);
    const cancelClosed = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-requisitions/${id}/cancel`
    );
    expectRejected(cancelClosed, 'closed 后 cancel 应被拒绝');
    expect(
      (await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id}`)).status,
      '被拒 cancel 后应仍为 closed（无痕）'
    ).toBe('closed');

    // draft → cancelled 正例 + 闭包负例：cancelled 不可 approve/delete，by-no 仍可读（软删未发生）
    const mk2 = await apiCall<Row>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'rd',
      requisition_date: today(),
      remarks: 'E2E70-cancel',
    });
    const id2 = Number(mk2.data?.id);
    const no2 = String(mk2.data?.requisition_no);
    expect(id2, `第二张领用单应创建成功：${JSON.stringify(mk2)}`).toBeGreaterThan(0);
    await apiCall(page, 'POST', `/chemical-requisitions/${id2}/cancel`);
    const cancelled = await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id2}`);
    expect(cancelled.status, 'cancel 后应落库 cancelled').toBe('cancelled');
    const approveCancelled = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-requisitions/${id2}/approve`
    );
    expectRejected(approveCancelled, 'cancelled 再 approve 应被状态机拒绝');
    const reCancel = await apiCallExpectFail(page, 'POST', `/chemical-requisitions/${id2}/cancel`);
    expectRejected(reCancel, 'cancelled 重复 cancel 应被拒（:266-267）');
    const delCancelled = await apiCallExpectFail(page, 'DELETE', `/chemical-requisitions/${id2}`);
    expectRejected(delCancelled, 'cancelled 删除应被拒（仅 draft 可删，:185-192）');
    const stillThere = await apiCallRaw<Row>(
      page,
      'GET',
      `/chemical-requisitions/by-no/${encodeURIComponent(no2)}`
    );
    expect(Number(stillThere.id), '被拒删除不得软删记录（by-no 仍可读）').toBe(id2);
    expect(stillThere.status, 'by-no 回读状态应仍 cancelled（无痕）').toBe('cancelled');

    // 创建负例：缺 requisition_date（提取器 400）/ 幽灵生产订单（业务 4xx）/ 负总金额
    const missDate = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
    });
    expectExtractorReject(missDate, '缺 requisition_date 必填应 400（types.rs:210-220）');
    const ghostOrder = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      production_order_id: 999999999,
    });
    expectRejected(ghostOrder, '不存在生产订单应被拒（requisition.rs:74-82）');
    const negAmount = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      total_amount: '-1',
    });
    expectRejected(negAmount, '负总金额应被拒（:84-87）');
    // PUT 负总金额（draft 态）同样被拒且原值未变
    const mk3 = await apiCall<Row>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      total_amount: '10.00',
    });
    const id3 = Number(mk3.data?.id);
    expect(id3, '第三张（draft 负金额实验）应创建成功').toBeGreaterThan(0);
    await tryCleanup(page, 'DELETE', `/chemical-requisitions/${id3}`, '[70] req3');
    const negPut = await apiCallExpectFail(page, 'PUT', `/chemical-requisitions/${id3}`, {
      total_amount: '-0.01',
    });
    expectRejected(negPut, 'draft PUT 负总金额应被拒（:169-172）');
    expect(
      toNum((await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id3}`)).total_amount),
      '被拒改额后应仍为 10（无痕）'
    ).toBe(10);
  });
});
