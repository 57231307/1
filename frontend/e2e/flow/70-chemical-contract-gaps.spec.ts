import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  failureCode,
  deferCleanup,
  flushDeferredCleanups,
  APP_ERROR_CODES,
  BASE_URL,
  type ApiFailureResult,
  type DeferredCleanup,
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
 * 假绿防线：健康位一律改直读信封显式断形（apiCallRaw 非 2xx 即抛，404/403/5xx 判红不弱于
 * 严格健康探测；分页族 readPaged 四键全断、tree 裸数组 requirePlainArray）；写后 GET 回读落库真值、被拒写回读断
 * 无痕/原值未变；rust_decimal 字符串出参一律 Number() 归一；禁 `?? []`/双形状探测/兜底默认值；
 * 数据自建自流转（genCode 唯一码），软删记录用 tryCleanup 幂等兜底清理。
 */

type Row = Record<string, unknown>;

/**
 * 延迟清理队列（范式同 purchase/03 的 CREATED_ORDER_IDS + afterEach、finance/01 的 CLEANUP[]）：
 * 本 spec 大量用例在自建主数据上还留有后续读/判重断言（by-code 回读、"未删重复应被拒"），
 * 若用同步 tryCleanup 当场软删，会把后续断言的前提数据删没（后端过滤 is_deleted=false 后
 * 查不到行 → 回读 404 / 判重返回 200,对后端皆正确,却制造假红并曾被误判成后端缺陷）。
 * 故所有 housekeeping 清理改为 deferCleanup 登记,统一在本文件每条用例结束后（全部断言之后）flush。
 */
const CLEANUP: DeferredCleanup[] = [];
test.afterEach(async ({ page }) => {
  await flushDeferredCleanups(page, CLEANUP);
});

/** rust_decimal 经 JSON 序列化为字符串，比较前归一 */
const toNum = (v: unknown): number => Number(String(v));

const today = (): string => new Date().toISOString().slice(0, 10);
const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/**
 * 断言精确拒绝契约（收紧原「任意 4xx + 三族机器码来者不拒」的假绿）：
 * - HTTP 必须恰为 400（backend/utils/error.rs:356-373：本域拒绝全部映射 BAD_REQUEST 状态）；
 * - 机器码逐条等于调用点期望族。chemical_ops 域写入方真相：
 *   枚举词表校验 = AppError::validation_displayable → VALIDATION_ERROR
 *   （chemical_service.rs::validate_chemical_type、chemical_ops/master.rs 状态词表门）；
 *   引用存在性/唯一性/数值范围/删除守卫/状态门 = AppError::business(_displayable) → BUSINESS_ERROR
 *   （chemical_ops/category.rs::create/delete、master.rs 建改门、lot.rs 检验与更新门、
 *   requisition.rs 各状态门）。门别串号即判红交后端。
 */
function expectRejected(r: ApiFailureResult, what: string, expectedCode: string): void {
  expect(r.status, `${what}：应恰为 HTTP 400，实际 ${r.status} ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：机器码应为 ${expectedCode}，实际 ${failureCode(r)}（${JSON.stringify(r)}）`
  ).toBe(expectedCode);
}

/**
 * 提取器层拒绝（缺必填 JSON 键）：serde 原文只进日志，出参由
 * middleware/trace_context.rs::normalize_extractor_rejection 收进统一信封
 * （HTTP 400 + code=VALIDATION_ERROR）。判状态**且**判机器码，两者都是成文契约。
 */
function expectExtractorReject(r: ApiFailureResult, what: string): void {
  expect(r.status, `${what}：应被 400 拒绝（提取器层），实际 ${JSON.stringify(r)}`).toBe(400);
  expect(
    failureCode(r),
    `${what}：提取器拒绝归一后机器码应为 VALIDATION_ERROR，实际 ${JSON.stringify(r)}`
  ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
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
  // 化料主数据在调用方（70-02 等）还要 by-code/详情回读，绝不能当场软删——登记到断言之后清理
  deferCleanup(CLEANUP, 'DELETE', `/chemicals/${id}`, `[70] chemical ${code}`);
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
  // 批次在调用方（70-03 等）还要详情/by-no 回读，绝不能当场软删——登记到断言之后清理
  deferCleanup(CLEANUP, 'DELETE', `/chemical-lots/${id}`, `[70] lot ${lotNo}`);
  return { id, lotNo };
}

test.describe.serial('70 染化料契约缺口：分类/状态机/检验闭包/效期/更新与取消门控', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('70-01 分类树：建根+子→tree 真值→停用出树→删除守卫→软删 404→软删后编码复用；负例', async ({
    page,
  }) => {
    // 判据：除 2xx 外还显式断 PaginatedResponse 四键与 tree
    // 裸数组契约漂移抓不到。readPaged/requirePlainArray 走 apiCallRaw（非 2xx 即抛，
    // 404/403 判红不变）+ 信封显式断形；本用例后续（tree 含根/子、停用出树、软删 404）
    // 的真值断言不在此重复。
    const catProbe = await readPaged(
      page,
      '/chemical-categories?page=1&page_size=5',
      'GET /chemical-categories?page=1&page_size=5'
    );
    expect(
      catProbe.length,
      `/chemical-categories 应遵守 page_size=5 上限，实际 ${catProbe.length}`
    ).toBeLessThanOrEqual(5);
    requirePlainArray(
      await apiCallRaw<unknown>(page, 'GET', '/chemical-categories/tree'),
      'GET /chemical-categories/tree'
    );

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

    // 删除守卫：父有未删子分类必拒（chemical_ops/category.rs::delete → business），且被拒无痕（GET 父仍 200）
    const delParent = await apiCallExpectFail(page, 'DELETE', `/chemical-categories/${rootId}`);
    expectRejected(delParent, '存在子分类时删除父分类', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // 登记顺序：先父后子（flushDeferredCleanups 逆序执行 ⇒ 实际删除为子先父后，
    // 与原即时清理「child→parent」的可达顺序一致，避免父因尚有未删子被引用守卫拒删而泄漏）
    deferCleanup(CLEANUP, 'DELETE', `/chemical-categories/${rootId}`, '[70] category 根');
    deferCleanup(CLEANUP, 'DELETE', `/chemical-categories/${newChildId}`, '[70] category 重建');

    // 负例：词表外类型（validate_chemical_type → validation_displayable）/
    // 父不存在（category.rs::create → business），均为 service 层 AppError，统一信封。
    // 注意：**重复编码不在本用例断言**——判重只查未删行（chemical_ops/category.rs::create，
    // 本表既定语义"软删后同码可复用"），而本用例上文已把根分类软删（:tryCleanup），
    // 此处对已软删的 rootCode 再断"重复应拒"与同用例的"软删可复用"断言语义互斥，
    // 后端不可两全。"未删状态下重复必拒"由紧随的 70-01b 独立用例锁定，
    // 其数据全程保持未删，两条用例语义各自干净。
    const badType = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: genCode('E2E70CBT'),
      category_name: '词表外类型',
      category_type: 'other',
    });
    expectRejected(
      badType,
      "category_type='other' 不在 dye/auxiliary/chemical 词表",
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    const ghostParent = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: genCode('E2E70CGP'),
      category_name: '幽灵父',
      category_type: 'dye',
      parent_id: 999999999,
    });
    expectRejected(ghostParent, '父分类不存在应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
    // 缺必填 category_code → 400（types.rs:121-129 必填非 Option；提取器拒绝已被
    // trace_context.rs normalize_extractor_rejection 收进 400+VALIDATION_ERROR 信封，
    // 状态码判定不变）
    const missingKey = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_name: '缺编码键',
      category_type: 'dye',
    });
    expectExtractorReject(missingKey, '缺 category_code 必填应 400');
  });

  test('70-01b 分类编码判重：未删状态下同码 POST 必被拒（400/BUSINESS_ERROR + 真实文案外显 + 零落库无痕）', async ({
    page,
  }) => {
    // 判重口径真相（category.rs:50-60）：查重**只过滤未删行**，本表族既定语义
    // "软删后同码可复用"（70-01 childCode、70-02 chemical_code 同源钉桩）。
    // 本用例主体记录**全程保持未删**，只锁"未删重复必拒"这半边语义，与 70-01
    // 的"软删可复用"互不掺杂，不再出现改前 70-01 先删根、后断根码重复的自序矛盾。
    // 判据：readPaged 直读四键信封 + page_size 上限，不只判 2xx。
    const dupProbe = await readPaged(
      page,
      '/chemical-categories?page=1&page_size=5',
      'GET /chemical-categories?page=1&page_size=5（70-01b）'
    );
    expect(
      dupProbe.length,
      `/chemical-categories 应遵守 page_size=5 上限，实际 ${dupProbe.length}`
    ).toBeLessThanOrEqual(5);

    const dupCode = genCode('E2E70CRD');
    const originName = `70判重主体${dupCode}`;
    const origin = await apiCall<Row>(page, 'POST', '/chemical-categories', {
      category_code: dupCode,
      category_name: originName,
      category_type: 'dye',
    });
    const originId = Number(origin.data?.id);
    expect(originId, `判重主体分类应创建成功：${JSON.stringify(origin)}`).toBeGreaterThan(0);
    // 判重前提：主体分类必须**保持未删**——查重只过滤未删行（category.rs:54-68）。
    // 清理绝不能当场发起（同步 tryCleanup 会立即软删，使下方"未删重复应被拒"的 POST 查不到未删行、
    // 后端正当返回 200 → 用例假红、曾被误判后端缺陷）；改为登记，断言全部结束后再 flush 清理。
    deferCleanup(CLEANUP, 'DELETE', `/chemical-categories/${originId}`, `[70] category ${dupCode}`);

    // 未删状态下同码再 POST → 必拒。本仓刚把该族拒绝从脱敏常量改为可外显
    // （AppError::business_displayable），故收紧为精确判定：400 + BUSINESS_ERROR，
    // 不放宽成"任意 4xx"。
    const dup = await apiCallExpectFail(page, 'POST', '/chemical-categories', {
      category_code: dupCode,
      category_name: '重复编码应被拒',
      category_type: 'dye',
    });
    expect(dup.status, `未删同码应被 400 拒绝，实际 ${JSON.stringify(dup)}`).toBe(400);
    expect(
      failureCode(dup),
      `未删同码机器码应为 BUSINESS_ERROR，实际 ${failureCode(dup)}（${JSON.stringify(dup)}）`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 真实文案外显（正向钉）：不得是脱敏常量「业务处理失败」，须含公开规则词
    // "编码"且回显用户自己提交的编码（用户改码即可通过 ⇒ 可外显族，
    // 判据同 wave-i 报告 §②）。
    const dupMsg = String(dup.message ?? '');
    expect(dupMsg, `拒绝文案应外显真实规则而非脱敏常量：${JSON.stringify(dup)}`).not.toBe(
      '业务处理失败'
    );
    expect(dupMsg, `拒绝文案应含公开规则词「编码」：${dupMsg}`).toContain('编码');
    expect(dupMsg, `拒绝文案应回显用户提交的编码 ${dupCode}：${dupMsg}`).toContain(dupCode);

    // 双向钉（泄漏红线，同 contract_wave7 §⑤-1）：不得含表名/SQL/竞态错误码等
    // 内部详情；剔除用户自己提交的编码后，文案中不应再有任何数字（内部 ID/
    // 其他记录标识一律不外进出参）。
    const leakBlacklist = ['chemical_category', 'insert', 'select', 'unique', '23505', 'sql'];
    for (const leak of leakBlacklist) {
      expect(dupMsg.toLowerCase(), `拒绝文案不得泄漏内部详情「${leak}」：${dupMsg}`).not.toContain(
        leak
      );
    }
    expect(
      dupMsg.split(dupCode).join(''),
      `拒绝文案剔除自身编码后不应残留数字（内部 ID 泄漏）：${dupMsg}`
    ).not.toMatch(/\d/);

    // 回读确认零落库：被拒同码在未删行中仍恰 1 条，且就是原行（id/名称未被覆盖，
    // 无痕）。列表无编码过滤参数（types.rs:144-150），按分页信封全量翻页扫描，
    // 上限护栏防 total 异常时死循环。
    const activeRows: Row[] = [];
    let pageNo = 1;
    for (;;) {
      const data = await apiCallRaw<{
        items?: unknown;
        total?: unknown;
        page?: unknown;
        page_size?: unknown;
      }>(page, 'GET', `/chemical-categories?page=${pageNo}&page_size=200`);
      expect(
        Array.isArray(data?.items),
        `/chemical-categories 第 ${pageNo} 页 data.items 必须为数组`
      ).toBe(true);
      expect(typeof data.total, `/chemical-categories 第 ${pageNo} 页 total 必须为数字`).toBe(
        'number'
      );
      expect(typeof data.page, `/chemical-categories 第 ${pageNo} 页 page 必须回显`).toBe('number');
      expect(typeof data.page_size, `/chemical-categories 第 ${pageNo} 页 page_size 必须回显`).toBe(
        'number'
      );
      activeRows.push(...(data.items as Row[]));
      if (activeRows.length >= Number(data.total)) break;
      pageNo += 1;
      expect(
        pageNo,
        '未删分类分页扫描超出护栏上限（total 与实际行数不一致？）'
      ).toBeLessThanOrEqual(20);
    }
    const sameCode = activeRows.filter(it => it.category_code === dupCode);
    expect(
      sameCode.length,
      `被拒重复 POST 后同码未删行应恰为 1 条（零落库），实际 ${JSON.stringify(sameCode)}`
    ).toBe(1);
    expect(Number(sameCode[0].id), '同码唯一未删行应是原行 id').toBe(originId);
    expect(sameCode[0].category_name, '被拒重复不得覆盖原行名称（无痕）').toBe(originName);
    // 原行详情仍可读（拒绝未引发任何软删/变更）
    const originStill = await apiCallRaw<Row>(page, 'GET', `/chemical-categories/${originId}`);
    expect(Number(originStill.id), '被拒重复后原分类应仍按 id 可读').toBe(originId);
  });

  test('70-02 主数据：必填对词表/状态流转/非法状态原值不变/软删三读路径 404/编码复用', async ({
    page,
  }) => {
    // dye 缺 dye_category、auxiliary 缺 auxiliary_category 必拒（chemical_ops/master.rs 必填对门 → business）
    const dyeMiss = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70DM'),
      chemical_name: '缺染料类别',
      chemical_type: 'dye',
    });
    expectRejected(dyeMiss, 'dye 缺 dye_category', APP_ERROR_CODES.BUSINESS_ERROR);
    const auxMiss = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70AM'),
      chemical_name: '缺助剂类别',
      chemical_type: 'auxiliary',
    });
    expectRejected(auxMiss, 'auxiliary 缺 auxiliary_category', APP_ERROR_CODES.BUSINESS_ERROR);
    // 词表外类型 'other'（UI 弹窗第三项，62 号头部缺陷③）→ 必须被拒
    //（chemical_service.rs::validate_chemical_type → validation_displayable）
    const otherType = await apiCallExpectFail(page, 'POST', '/chemicals', {
      chemical_code: genCode('E2E70OT'),
      chemical_name: '词表外 other',
      chemical_type: 'other',
    });
    expectRejected(otherType, "chemical_type='other' 不在词表", APP_ERROR_CODES.VALIDATION_ERROR);

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

    // 非法状态词表外（chemical_ops/master.rs 更新门 → validation_displayable）→ 拒绝且回读原值未变
    const badStatus = await apiCallExpectFail(page, 'PUT', `/chemicals/${id}`, {
      status: 'archived',
    });
    expectRejected(
      badStatus,
      "status='archived' 不在 active/inactive/discontinued 词表",
      APP_ERROR_CODES.VALIDATION_ERROR
    );
    expect(
      (await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).status,
      '被拒状态更新后落库值应仍为 discontinued（无痕）'
    ).toBe('discontinued');
    // 负价 PUT（chemical_ops/master.rs 更新门 → business）→ 拒绝且标准价未变
    const priceBefore = toNum(
      (await apiCallRaw<Row>(page, 'GET', `/chemicals/${id}`)).standard_price
    );
    const badPrice = await apiCallExpectFail(page, 'PUT', `/chemicals/${id}`, {
      standard_price: '-1.00',
    });
    expectRejected(badPrice, 'PUT 负标准价应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // 重建行 newId 在末尾还要按 code 走 by-code 回读断形——
    // 当场软删会让该读路径 404，故登记到本用例断言之后再清理
    deferCleanup(CLEANUP, 'DELETE', `/chemicals/${newId}`, '[70] chemical 重建');

    // UI 页面可达 + by-code 读路径真值回读（不重复 62 的 UI 编辑链路）。
    // 判据：不只判 2xx——软删重建后 by-code 若错误命中
    // 已软删旧行、或编码键漂移，健康探针全察觉不到；改为钉 by-code 返回体逐键：
    // chemical_code 逐字等于复用编码，且 id 必须是重建新行 newId（软删旧行不可见）。
    const byCodeAfterRebuild = await apiCallRaw<Row>(
      page,
      'GET',
      `/chemicals/by-code/${encodeURIComponent(code)}`
    );
    expect(
      byCodeAfterRebuild.chemical_code,
      `by-code 回读编码应逐字等于 ${code}，实际 ${JSON.stringify(byCodeAfterRebuild)}`
    ).toBe(code);
    expect(
      Number(byCodeAfterRebuild.id),
      `by-code 应命中软删后重建的新行 id=${newId}（已软删旧 id=${id} 不可见）`
    ).toBe(newId);
    await page.goto(`${BASE_URL}/chemicals`);
    await expect(page.getByRole('tab', { name: '染化料', exact: true })).toBeVisible();
  });

  test('70-03 批次：fail-inspection 闭包/改价重算总成本/expiry_before 近效期/软删 404/负例', async ({
    page,
  }) => {
    const chem = await createChemical(page);

    // 不合格检验链：pending → failed，报告 URL 同体落库；failed 后 pass 必须被拒且无痕
    //（检验闭包只允许 pending/quarantine 出发，chemical_ops/lot.rs::pass_inspection → business）
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
    expectRejected(
      revivePass,
      'failed 终态再 pass-inspection 应被状态机拒绝',
      APP_ERROR_CODES.BUSINESS_ERROR
    );
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
    // 负单位成本 → 拒绝且原重算值未变（无痕）（chemical_ops/lot.rs::update → business）
    const negCost = await apiCallExpectFail(page, 'PUT', `/chemical-lots/${costLot.id}`, {
      unit_cost: '-1',
    });
    expectRejected(negCost, 'PUT 负单位成本应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // 该领用单 id 在下方还要 PUT 更新/approve/issue/close（draft 门控等断言）——当场软删会让这些写读全 404，
    // 登记到断言之后再清理
    deferCleanup(CLEANUP, 'DELETE', `/chemical-requisitions/${id}`, `[70] req ${no}`);

    const requiredDate = ymd(7);
    await apiCall(page, 'PUT', `/chemical-requisitions/${id}`, {
      remarks: 'E2E70-R2',
      required_date: requiredDate,
    });
    const updated = await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id}`);
    expect(updated.remarks, 'draft PUT remarks 应落库 R2').toBe('E2E70-R2');
    expect(String(updated.required_date), 'required_date 应逐字落库').toBe(requiredDate);

    // approve 后 PUT 必拒（chemical_ops/requisition.rs::update 仅 draft 可更新 → business），且被拒更新无痕
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/approve`);
    const latePut = await apiCallExpectFail(page, 'PUT', `/chemical-requisitions/${id}`, {
      remarks: 'E2E70-应被拒',
    });
    expectRejected(latePut, 'approved 后 PUT 更新', APP_ERROR_CODES.BUSINESS_ERROR);
    const afterReject = await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id}`);
    expect(afterReject.remarks, '被拒 PUT 后 remarks 应仍为 R2（原值未变）').toBe('E2E70-R2');
    expect(afterReject.status, '被拒 PUT 后状态应仍为 approved').toBe('approved');

    // closed 终态不可 cancel（requisition.rs::cancel → business），且状态未变
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/issue`);
    await apiCall(page, 'POST', `/chemical-requisitions/${id}/close`);
    const cancelClosed = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-requisitions/${id}/cancel`
    );
    expectRejected(cancelClosed, 'closed 后 cancel 应被拒绝', APP_ERROR_CODES.BUSINESS_ERROR);
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
    // cancelled 各门（requisition.rs::approve/cancel/delete → business）
    const approveCancelled = await apiCallExpectFail(
      page,
      'POST',
      `/chemical-requisitions/${id2}/approve`
    );
    expectRejected(
      approveCancelled,
      'cancelled 再 approve 应被状态机拒绝',
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    const reCancel = await apiCallExpectFail(page, 'POST', `/chemical-requisitions/${id2}/cancel`);
    expectRejected(reCancel, 'cancelled 重复 cancel 应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
    const delCancelled = await apiCallExpectFail(page, 'DELETE', `/chemical-requisitions/${id2}`);
    expectRejected(
      delCancelled,
      'cancelled 删除应被拒（仅 draft 可删）',
      APP_ERROR_CODES.BUSINESS_ERROR
    );
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
    expectExtractorReject(missDate, '缺 requisition_date 必填应 400');
    const ghostOrder = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      production_order_id: 999999999,
    });
    // 生产订单存在性门（chemical_ops/requisition.rs::create → business）
    expectRejected(ghostOrder, '不存在生产订单应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
    const negAmount = await apiCallExpectFail(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      total_amount: '-1',
    });
    // 总金额范围门（同函数 → business）
    expectRejected(negAmount, '负总金额应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
    // PUT 负总金额（draft 态）同样被拒且原值未变
    const mk3 = await apiCall<Row>(page, 'POST', '/chemical-requisitions', {
      requisition_type: 'lab',
      requisition_date: today(),
      total_amount: '10.00',
    });
    const id3 = Number(mk3.data?.id);
    expect(id3, '第三张（draft 负金额实验）应创建成功').toBeGreaterThan(0);
    // 该单 id3 在下方还要 PUT 负金额 + 详情回读原值（无痕断言）——当场软删会让其 404，登记到断言之后清理
    deferCleanup(CLEANUP, 'DELETE', `/chemical-requisitions/${id3}`, '[70] req3');
    // draft PUT 负总金额（requisition.rs::update → business）
    const negPut = await apiCallExpectFail(page, 'PUT', `/chemical-requisitions/${id3}`, {
      total_amount: '-0.01',
    });
    expectRejected(negPut, 'draft PUT 负总金额应被拒', APP_ERROR_CODES.BUSINESS_ERROR);
    expect(
      toNum((await apiCallRaw<Row>(page, 'GET', `/chemical-requisitions/${id3}`)).total_amount),
      '被拒改额后应仍为 10（无痕）'
    ).toBe(10);
  });
});
