/**
 * 越权负例与拒因归属矩阵（三端共用同一后端门）
 *
 * 功能：
 *   1. 先把“拒因不能由响应文案判定”这条前提坐实成实测事实，而不是写在注释里。
 *      本项目后端的权限拒绝文案永久脱敏为固定出参常量，RBAC 门（无权限码）与
 *      行级数据范围门（权限码已授但归属不在可见集合）给出的 403 出参文案相同。
 *      本文件用两条**不同来源**的 403 去实测它们的 message 逐字相等且都不含资源名与
 *      记录 id，从而把“拿中文整串文案当判据”这条路当场封死。
 *   2. 归因手法照本仓既有的控制组对照：同角色、同端点、只换归属。
 *      先断“读自己名下 200”证权限码已授（排除缺授这一解释），再断“读他人名下 403”，
 *      此时唯一变量是归属，403 才能归到行级数据范围收敛。控制组不成立时用例判红并点名，
 *      因为那条 403 的归因失去了唯一变量。
 *   3. 纵向越权矩阵：低权角色调高权端点必须被拒。端点表不写死角色名单，
 *      持证与无证角色都由后端授权矩阵分组现派生（与 flow/200 同一派生口径），
 *      无键侧必须 403 且机器码归到权限门，持证侧必须不被 RBAC 挡（用不存在的记录 id
 *      发探针，避免真的改到别人的单据）。
 *   4. 横向越权矩阵：改包他人单据 id 必须被拒，读腿与写腿分别判。
 *      报价单按 sales_user_id 归属、成本归集按 created_by 归属，两条腿各自的归属列
 *      从后端源核出并在失败信息里点名。
 *   5. 不相容岗位矩阵：仓管不得改报价、业务员不得直接改库存数量、财务不得改订单明细
 *      逐对断，且每对都带同端点的持证对照组，使“被拒”归到岗位权限而不是环境。
 *      另外把本仓真正承责的互斥表 role_conflicts 读出来核内置默认（钱账分管六对在场的
 *      同时，生产与质量两对必须不在场），并真跑一次写口拒绝：出纳账号改派为会计必须被拒。
 *   6. 三端一致性：同一枚（角色，端点）组合的放行判定与客户端无关。PC 与 H5 与小程序
 *      打的是同一批后端端点，小程序复刻的是后端 matches_permission 那一条判据，
 *      因此本文件把“后端实测结论”与“按该角色登录出参里的权限码派生的结论”逐对全等，
 *      不等即点名——多一个面是越权，少一个面是授权链断。
 *
 * 调用方：Playwright E2E 套件，CI 的 e2e flow 分片按文件哈希收集执行。
 * 入参：API_BASE 取环境变量；角色凭证取 env 优先、回退 role-credentials.json；
 *      持证与无证角色集合、端点派生的资源与动作键全部由 ./role-matrix-support 现读派生。
 * 传给谁：多角色真登录会话与身份钉交给 fixtures/multi-context 的 real 档与
 *      fixtures/relay 的 readRelayIdentity 与 createRelay；请求与拒绝归因交给
 *      flow/helpers 的 apiCallRaw、apiCallExpectFail 与 expectDenied，不另立第二套判据。
 * 存什么：为控制组准备的两张报价单（各挂一个归属人）、两行成本归集（各由不同会话建）、
 *      一张用于互斥写口的出纳账号，以及这些行各自的状态列与审计行。
 * 存哪里：报价单族没有 DELETE 路由，两张单按业务事实留库并在制品里点名；
 *      成本归集有审核腿没有删除腿，两行同样留库并点名；
 *      互斥写口用的账号由 admin 会话当场删除；
 *      会话 cookie 只在各 BrowserContext 内，随 context 关闭消失；
 *      矩阵实测结果写 e2e/.auth/role-visibility-matrix/202-*.json 供人工与小程序侧审阅，
 *      该目录不是基线、不参与判绿。
 *
 * 判红即缺陷的口径：应拒未拒（无键却 2xx）保持判红，应放却拒（持键却 403 FORBIDDEN）
 * 同样判红；控制组的正向腿不成立时负向腿的归因作废，用例点名“控制组不成立”而不是
 * 把负向结论当通过。互斥写口若放行，说明职责分离只剩纸面，保持判红。
 */
import { test, expect, type Page } from '../diagnose-fixture';
import { apiCallExpectFail, apiCallRaw, expectDenied, genCode, getRoleCredential } from './helpers';
import { createIsolatedSession, type IsolatedSession } from '../fixtures/multi-context';
import { readRelayIdentity } from '../fixtures/relay';
import { mkdirSync, writeFileSync } from 'fs';
import {
  backendGrants,
  derivePermissionKey,
  readMatrixGrants,
  readSeededRoles,
} from './role-matrix-support';

type Row = Record<string, unknown>;

const MATRIX_ARTIFACT_DIR = 'e2e/.auth/role-visibility-matrix';

/**
 * 一个不存在的记录 id：越权探针全部打在它上面。
 * 用途：让 RBAC 门的拒绝先于任何业务查找发生，这样“无键角色被拒”判的是权限门，
 * 而“持证角色不被 RBAC 挡”判的也是同一道门，两边都不可能改动真实单据。
 * 传给谁：只进请求路径，不落库。
 */
const ABSENT_ID = 999_999_999;

const seededRoles = readSeededRoles();
const matrixGrants = readMatrixGrants();
const seededRoleCodes = new Set(seededRoles.map(r => r.code));
/** 本车道角色的行级数据范围取值（种子读出的那一枚，不是库里回读） */
const dataScopeByRole = new Map(seededRoles.map(r => [r.code, r.dataScope]));

/** 某角色登录出参里的权限码是否命中该资源动作（与后端 matches_permission 同一条判据） */
function keyGrantedByMatrixRole(role: string, resource: string, action: string): boolean {
  const pairs = matrixGrants.get(role);
  if (!pairs) return false;
  return backendGrants(
    pairs.map(p => `${p.resource}:${p.action}`),
    resource,
    action
  );
}

/** 持该键、有角色种子在册、且本车道有其 E2E 凭证的非 admin 角色码 */
function credentialedHolders(resource: string, action: string): string[] {
  const out: string[] = [];
  for (const role of matrixGrants.keys()) {
    if (role === 'admin') continue;
    if (!seededRoleCodes.has(role)) continue;
    if (!keyGrantedByMatrixRole(role, resource, action)) continue;
    if (!getRoleCredential(role)) continue;
    out.push(role);
  }
  return out.sort();
}

/** 不持该键的持证非 admin 角色码（越权侧） */
function credentialedNonHolders(resource: string, action: string): string[] {
  const out: string[] = [];
  for (const role of matrixGrants.keys()) {
    if (role === 'admin') continue;
    if (!seededRoleCodes.has(role)) continue;
    if (keyGrantedByMatrixRole(role, resource, action)) continue;
    if (!getRoleCredential(role)) continue;
    out.push(role);
  }
  return out.sort();
}

/** 一层高权端点：一条方法加一条路径加它的派生键 */
interface PrivilegedEndpoint {
  id: string;
  cn: string;
  method: 'GET' | 'POST' | 'PUT' | 'DELETE';
  /** 请求路径（id 位用 ABSENT_ID，保证不碰真实单据） */
  path: string;
  /** 权限资源段（由后端 extract_resource_info 口径写出，用例内还会二次自证） */
  resource: string;
  /** 权限动作段（由后端 query 与 path 与 method 三级派生口径写出） */
  action: string;
  body?: Record<string, unknown> | unknown[];
  /**
   * RBAC 之外的 handler 层角色门（通过该门所需的角色码集合）。
   * 判据来源是 handler 源文件的明文，例如审计日志查询在 backend src
   * handlers audit_log_handler.rs 里要求 admin 或 auditor。
   * 有这一枚字段时，持键角色的预期是“键放行但角色门按角色码决定”，
   * 不能一律按放行判，否则会把 handler 门的正常拒绝读成授权链断裂。
   */
  handlerRoleGate?: string[];
}

const PRIVILEGED_ENDPOINTS: PrivilegedEndpoint[] = [
  {
    id: 'quotation-approve',
    cn: '报价单批准',
    method: 'POST',
    path: `/quotations/${ABSENT_ID}/approve`,
    resource: 'quotations',
    action: 'approve',
    body: { approval_reason: 'E2E 越权矩阵 探针' },
  },
  {
    id: 'quotation-update',
    cn: '报价单整单改',
    method: 'PUT',
    path: `/quotations/${ABSENT_ID}`,
    resource: 'quotations',
    action: 'update',
    body: { notes: 'E2E 越权矩阵 探针' },
  },
  {
    id: 'adjustment-item-add',
    cn: '库存调整建行（直接改库存数量）',
    method: 'POST',
    path: `/inventory/adjustments/${ABSENT_ID}/items`,
    resource: 'adjustments',
    action: 'create',
    body: { product_id: 1, quantity: '1', unit: '米' },
  },
  {
    id: 'sales-order-update',
    cn: '销售订单整单改（订单与面料明细写腿）',
    method: 'PUT',
    path: `/sales/orders/${ABSENT_ID}`,
    resource: 'orders',
    action: 'update',
    body: { remarks: 'E2E 越权矩阵 探针' },
  },
  {
    id: 'product-delete',
    cn: '产品档案删除',
    method: 'DELETE',
    path: `/products/${ABSENT_ID}`,
    resource: 'products',
    action: 'delete',
  },
  {
    id: 'customer-delete',
    cn: '客户档案删除',
    method: 'DELETE',
    path: `/crm/customers/${ABSENT_ID}`,
    resource: 'customers',
    action: 'delete',
  },
  {
    id: 'role-permission-rewrite',
    cn: '角色授权整批改写',
    method: 'PUT',
    path: `/roles/${ABSENT_ID}/permissions`,
    resource: 'roles',
    action: 'update',
    body: [],
  },
  {
    id: 'user-update',
    cn: '用户账号改派（换角色）',
    method: 'PUT',
    path: `/users/${ABSENT_ID}`,
    resource: 'users',
    action: 'update',
    body: { email: 'e2e-probe@example.com' },
  },
  {
    id: 'purchase-price-approve',
    cn: '采购价批准',
    method: 'POST',
    path: `/purchase/purchase-prices/${ABSENT_ID}/approve`,
    resource: 'purchase-prices',
    action: 'approve',
    body: { approval_reason: 'E2E 越权矩阵 探针' },
  },
  {
    id: 'cost-collection-audit',
    cn: '成本归集审核',
    method: 'POST',
    path: `/production/cost-collections/${ABSENT_ID}/audit`,
    resource: 'cost-collections',
    action: 'audit',
    body: {},
  },
  {
    id: 'audit-log-read',
    cn: '审计日志读',
    method: 'GET',
    path: '/audit-logs?page=1&page_size=1',
    resource: 'audit-logs',
    action: 'read',
    handlerRoleGate: ['admin', 'auditor'],
  },
];

/**
 * 会话缓存：同一条用例内同一角色只真登录一次。
 * 调用方：本文件各用例。入参：browser 与角色码。传给谁：createIsolatedSession 的 real 档。
 * 存什么/存哪里：内存里的角色到会话映射，用例结束时由 closeAll 统一关，不落盘。
 */
const sessionCache = new Map<string, IsolatedSession>();
async function roleSession(
  browser: Parameters<Parameters<typeof test>[2]>[0]['browser'],
  role: string
): Promise<IsolatedSession> {
  const hit = sessionCache.get(role);
  if (hit) return hit;
  const session = await createIsolatedSession(browser, role, {
    authMode: 'real',
    credentialRole: role,
  });
  const identity = await readRelayIdentity(session.page, `角色 ${role} 的越权矩阵会话`);
  const cred = getRoleCredential(role);
  if (!cred) {
    await session.close();
    throw new Error(`角色 ${role} 没有 E2E 凭证却被打开了会话，派生侧与登录侧不一致，判红`);
  }
  expect(
    identity.username,
    `角色 ${role} 登录回来的账号应是凭证里的 ${cred.username}，实际=${identity.username}`
  ).toBe(cred.username);
  identityPermissions.set(role, identity.permissions);
  identityUserId.set(role, identity.userId);
  sessionCache.set(role, session);
  return session;
}

const identityPermissions = new Map<string, string[]>();
const identityUserId = new Map<string, number>();

async function closeAllSessions(): Promise<void> {
  for (const session of sessionCache.values()) await session.close().catch(() => undefined);
  sessionCache.clear();
}

/** 该角色对一条端点的后端实测结论：是否被权限门挡掉 */
async function deniedByPermissionGate(
  page: Page,
  ep: PrivilegedEndpoint
): Promise<{ blocked: boolean; status: number; code?: string | number; message?: unknown }> {
  const result = await apiCallExpectFail(page, ep.method, ep.path, ep.body);
  return {
    blocked: result.status === 403 && result.code === 'FORBIDDEN',
    status: result.status,
    code: result.code,
    message: result.message,
  };
}

test.describe('越权负例与拒因归属矩阵', () => {
  let admin: IsolatedSession;

  test.beforeAll(async ({ browser }) => {
    admin = await createIsolatedSession(browser, 'admin', { authMode: 'real' });
  });

  test.afterEach(async () => {
    await closeAllSessions();
  });

  // ==================== 一 拒因归属的前提自证 ====================

  test('前提自证 RBAC 门与行级数据范围门的 403 出参文案相同，故禁止拿文案判拒因', async ({
    browser,
  }) => {
    // 第一步：由 admin 建一张挂他人名下的报价单草稿（归属人＝销售经理），
    // 同时留一张归属人为 admin 自己的同族单据，供两类门各打一次同一个端点。
    const product = await apiCallRaw<Row>(admin.page, 'POST', '/products', {
      code: genCode('E2E-PRIV-P'),
      name: 'E2E 越权矩阵产品',
      unit: '米',
      product_type: '辅料',
    });
    const productId = Number(product?.id);
    if (!Number.isFinite(productId) || productId <= 0) {
      throw new Error(
        `前置失败：建探针产品未返回有效 id，实际出参=${JSON.stringify(product)?.slice(0, 240)}`
      );
    }
    const customer = await apiCallRaw<Row>(admin.page, 'POST', '/crm/customers', {
      customer_name: genCode('E2E-PRIV-C'),
    });
    const customerId = Number(customer?.id);
    if (!Number.isFinite(customerId) || customerId <= 0) {
      throw new Error(
        `前置失败：建探针客户未返回有效 id，实际出参=${JSON.stringify(customer)?.slice(0, 240)}`
      );
    }

    const mgrCred = getRoleCredential('sales_manager');
    if (!mgrCred) {
      throw new Error(
        '前置失败：sales_manager 无 E2E 凭证（env 与 role-credentials.json 两处都无该角色码），控制组归属人无从确定'
      );
    }
    const mgrUser = await resolveUserId(admin.page, mgrCred.username);

    const ownQuotationId = await createQuotationDraft(admin.page, {
      customerId,
      salesUserId: mgrUser,
      productId,
      notes: 'E2E 越权矩阵 经理名下件',
    });
    const otherQuotationId = await createQuotationDraft(admin.page, {
      customerId,
      salesUserId: 1,
      productId,
      notes: 'E2E 越权矩阵 他人名下件',
    });

    // 第二步：两类门各打同一个端点 GET /quotations/{id}
    // 甲 权限门：warehouse_keeper 没有 quotations:read，必须 403
    const keeper = await roleSession(browser, 'warehouse_keeper');
    const rbacDeny = await apiCallExpectFail(keeper.page, 'GET', `/quotations/${ownQuotationId}`);
    expectDenied(rbacDeny, '权限门侧：仓管无报价读键却读经理名下报价单');

    // 乙 行级门：sales_manager 有 quotations:read，但那张单归属不是它，必须 403
    const manager = await roleSession(browser, 'sales_manager');
    expect(
      identityPermissions.get('sales_manager') ?? [],
      '前置坐实：销售经理登录出参的权限码里必须含 quotations:read，否则下面的 403 归不到行级数据范围'
    ).toEqual(expect.arrayContaining([expect.stringMatching(/^quotations:/)]));
    const scopeDeny = await apiCallExpectFail(
      manager.page,
      'GET',
      `/quotations/${otherQuotationId}`
    );
    expectDenied(scopeDeny, '行级门侧：销售经理读他人名下报价单');

    // 丙 事实核：两条 403 的出参文案逐字相等，且都不含资源名与记录 id
    expect(rbacDeny.status, `两类门必须同为 403，权限门侧实测 ${rbacDeny.status}`).toBe(
      scopeDeny.status
    );
    const rbacMessage = String(rbacDeny.message ?? '');
    const scopeMessage = String(scopeDeny.message ?? '');
    expect(
      rbacMessage,
      '权限门侧的 403 出参文案缺失（后端统一信封应带 message），无从证明文案不可判'
    ).not.toBe('');
    expect(
      rbacMessage,
      `两类门的 403 出参文案必须相同（本项目权限拒绝文案永久脱敏为固定常量）：` +
        `权限门侧=${JSON.stringify(rbacMessage)} 行级门侧=${JSON.stringify(scopeMessage)}；` +
        '两者一旦不同，说明后端开始外显拒因，本文件的控制组对照口径需重判'
    ).toBe(scopeMessage);
    expect(
      rbacMessage,
      `403 文案里出现了资源名或记录 id（${JSON.stringify(rbacMessage)}），` +
        '说明后端在往出参泄露归属线索，越权探测面可被反推'
    ).not.toMatch(/quotation|报价|9\d{8}/i);

    mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
    writeFileSync(
      `${MATRIX_ARTIFACT_DIR}/202-denial-message-equality.json`,
      JSON.stringify(
        {
          note: '两类门的 403 出参实测同文案，拒因只能由控制组对照归因，禁止拿中文整串当判据',
          rbacGate: { role: 'warehouse_keeper', status: rbacDeny.status, code: rbacDeny.code },
          dataScopeGate: {
            role: 'sales_manager',
            dataScope: dataScopeByRole.get('sales_manager'),
            status: scopeDeny.status,
            code: scopeDeny.code,
          },
          messageIdentical: rbacMessage === scopeMessage,
        },
        null,
        2
      ) + '\n',
      'utf-8'
    );
  });

  // ==================== 二 控制组对照的归属归因 ====================

  test('控制组 同角色同端点先读自己名下 200 再读他人名下 403，唯一变量是归属（报价单腿）', async ({
    browser,
  }) => {
    const mgrCred = getRoleCredential('sales_manager');
    if (!mgrCred) throw new Error('前置失败：sales_manager 无 E2E 凭证，控制组归属人无从确定');
    const mgrUserId = await resolveUserId(admin.page, mgrCred.username);
    const manager = await roleSession(browser, 'sales_manager');

    const product = await apiCallRaw<Row>(admin.page, 'POST', '/products', {
      code: genCode('E2E-CTRL-P'),
      name: 'E2E 控制组产品',
      unit: '米',
      product_type: '辅料',
    });
    const customer = await apiCallRaw<Row>(admin.page, 'POST', '/crm/customers', {
      customer_name: genCode('E2E-CTRL-C'),
    });
    const productId = Number(product?.id);
    const customerId = Number(customer?.id);
    if (!Number.isFinite(productId) || !Number.isFinite(customerId)) {
      throw new Error('前置失败：控制组的探针产品或客户未返回 id，两张对照单无从建');
    }

    // 正向腿的单据：归属列 sales_user_id 就是本棒登录的经理
    const ownId = await createQuotationDraft(admin.page, {
      customerId,
      salesUserId: mgrUserId,
      productId,
      notes: 'E2E 越权矩阵 控制组 本人名下件',
    });
    // 负向腿的单据：归属列取另一个真实存在的用户（admin 自己），其余变量与正向腿全同
    const otherId = await createQuotationDraft(admin.page, {
      customerId,
      salesUserId: 1,
      productId,
      notes: 'E2E 越权矩阵 控制组 他人名下件',
    });

    const own = await apiCallExpectFail(manager.page, 'GET', `/quotations/${ownId}`);
    expect(
      own.status,
      `控制组正向腿不成立：销售经理 ${mgrCred.username}(id=${mgrUserId}) 读自己名下报价单 ${ownId} ` +
        `必须 200 以证 quotations:read 已授；实际 status=${own.status} code=${JSON.stringify(own.code)}。` +
        '此处若也 403，说明是权限码缺授（RBAC 门）而非行级数据范围，负向腿的归因即失去唯一变量'
    ).toBe(200);

    const ownRow = await apiCallRaw<Row>(manager.page, 'GET', `/quotations/${ownId}`);
    expect(
      Number(ownRow.id),
      `控制组正向腿必须真读到本名下单据内容，返回 id=${JSON.stringify(ownRow.id)} 应为 ${ownId}`
    ).toBe(ownId);

    const denied = await apiCallExpectFail(manager.page, 'GET', `/quotations/${otherId}`);
    expectDenied(
      denied,
      `控制组负向腿：销售经理读他人名下报价单 ${otherId} 应被行级数据范围拒（同端点读自己名下 ${ownId} ` +
        '已 200，两处唯一变量是归属人；报价单归属列 sales_user_id 与列表 apply_data_scope 同源）'
    );

    // 归属列取值自证：正向腿读到的行，其归属确实等于登录者，负向腿确实不等
    expect(
      Number(ownRow.sales_user_id),
      `正向腿单据的归属列 sales_user_id 应等于本棒 user_id=${mgrUserId}，实际=${JSON.stringify(ownRow.sales_user_id)}`
    ).toBe(mgrUserId);
    const otherRow = await apiCallRaw<Row>(admin.page, 'GET', `/quotations/${otherId}`);
    expect(
      Number(otherRow.sales_user_id),
      `负向腿单据的归属列应不等于本棒 user_id=${mgrUserId}，否则两处就不是只差归属这一个变量`
    ).not.toBe(mgrUserId);
  });

  test('控制组 成本归集写腿的归属归因（created_by 为唯一归属列，读腿与写腿都要拒）', async ({
    browser,
  }) => {
    const ccCred = getRoleCredential('cost_accountant');
    if (!ccCred) throw new Error('前置失败：cost_accountant 无 E2E 凭证，成本归集控制组无从开棒');
    const accountant = await roleSession(browser, 'cost_accountant');
    expect(
      dataScopeByRole.get('cost_accountant'),
      `前置坐实：cost_accountant 的行级数据范围应是自持档（种子值 self），实际=${dataScopeByRole.get(
        'cost_accountant'
      )}；范围不是 self 或 dept 退化形态时下面的归属对照不成立`
    ).toBe('self');

    const day = new Date().toISOString().slice(0, 10);
    const ownLot = genCode('E2E-SC-OWN');
    const otherLot = genCode('E2E-SC-OTHR');

    // 本人名下：由本棒会话自己建，created_by 落本棒 user_id
    const own = await apiCallRaw<Row>(accountant.page, 'POST', '/production/cost-collections', {
      collection_date: day,
      cost_object_type: 'dye_lot',
      dye_lot_no: ownLot,
      batch_no: ownLot,
      color_no: 'E2ESC01',
      direct_material: '10.00',
      direct_labor: '1.00',
      manufacturing_overhead: '1.00',
      processing_fee: '1.00',
      dyeing_fee: '1.00',
    });
    const ownId = Number(Array.isArray(own) ? own[0]?.id : (own as Row)?.id);
    if (!Number.isFinite(ownId) || ownId <= 0) {
      throw new Error(
        `前置失败：本棒建成本归集未返回 id，实际出参=${JSON.stringify(own)?.slice(0, 240)}`
      );
    }
    // 他人名下：由 admin 会话建，created_by 落 admin
    const other = await apiCallRaw<Row>(admin.page, 'POST', '/production/cost-collections', {
      collection_date: day,
      cost_object_type: 'dye_lot',
      dye_lot_no: otherLot,
      batch_no: otherLot,
      color_no: 'E2ESC02',
      direct_material: '10.00',
      direct_labor: '1.00',
      manufacturing_overhead: '1.00',
      processing_fee: '1.00',
      dyeing_fee: '1.00',
    });
    const otherId = Number(Array.isArray(other) ? other[0]?.id : (other as Row)?.id);
    if (!Number.isFinite(otherId) || otherId <= 0) {
      throw new Error(
        `前置失败：admin 建对照成本归集未返回 id，实际出参=${JSON.stringify(other)?.slice(0, 240)}`
      );
    }

    const ownRead = await apiCallRaw<Row>(
      accountant.page,
      'GET',
      `/production/cost-collections/${ownId}`
    );
    expect(
      Number(ownRead.id),
      `控制组正向腿不成立：成本会计读自己建的成本归集 ${ownId} 应 200 并读回该行，实际 id=${JSON.stringify(ownRead.id)}`
    ).toBe(ownId);
    expect(
      Number(ownRead.created_by),
      `正向腿行的归属列 created_by 应等于本棒 user_id=${identityUserId.get('cost_accountant')}，` +
        `实际=${JSON.stringify(ownRead.created_by)}；本仓成本归集的唯一归属列是 created_by，不是审计列以外的别的列`
    ).toBe(identityUserId.get('cost_accountant'));

    const deniedRead = await apiCallExpectFail(
      accountant.page,
      'GET',
      `/production/cost-collections/${otherId}`
    );
    expectDenied(
      deniedRead,
      `越权读腿：成本会计读他人名下成本归集 ${otherId} 必须被行级数据范围拒（同端点读自己名下 ${ownId} 已 200，` +
        '唯一变量是 created_by 归属）'
    );

    const deniedWrite = await apiCallExpectFail(
      accountant.page,
      'PUT',
      `/production/cost-collections/${otherId}`,
      { dyeing_fee: '9999.00' }
    );
    expectDenied(
      deniedWrite,
      `越权写腿：成本会计改他人名下成本归集 ${otherId} 的染厂加工费必须被拒（读腿已证同族单据可读、` +
        '写键已授，此处唯一变量仍是归属）'
    );

    // 拒后回读必须还是原值：拒绝不能是"改了但报错"
    const after = await apiCallRaw<Row>(
      admin.page,
      'GET',
      `/production/cost-collections/${otherId}`
    );
    expect(
      Number(after.dyeing_fee),
      `越权写被拒后，他人名下行 ${otherId} 的 dyeing_fee 必须仍是建单值 1.00，实际=${JSON.stringify(after.dyeing_fee)}`
    ).toBe(1);
  });

  // ==================== 三 纵向越权矩阵 ====================

  for (const ep of PRIVILEGED_ENDPOINTS) {
    test(`纵向越权 ${ep.id}（${ep.cn}）无键角色必须被权限门拒、持证角色不得被 RBAC 挡`, async ({
      browser,
    }) => {
      // 派生自证：本文件声明的键必须等于后端 extract_route_info 对同一条路径的派生结果，
      // 否则测的是另一枚键，越权结论与授权面都对不上。
      const derived = derivePermissionKey(ep.method, `/api/v1/erp${ep.path.split('?')[0]}`);
      expect(
        `${derived.resource}:${derived.action}`,
        `端点 ${ep.id} 的方法与路径派生出的键应等于本文件声明，` +
          `实际派生=${derived.resource}:${derived.action}（${derived.basis}），声明=${ep.resource}:${ep.action}`
      ).toBe(`${ep.resource}:${ep.action}`);

      const holders = credentialedHolders(ep.resource, ep.action);
      const blockers = credentialedNonHolders(ep.resource, ep.action);
      expect(
        blockers.length,
        `端点 ${ep.id} 找不到“无 ${ep.resource}:${ep.action} 且有凭证”的角色，越权侧无从判定，判红而非空过`
      ).toBeGreaterThan(0);

      // 持证侧为空分两种情形，必须分清：
      // 甲 该键在整个授权矩阵里从未授予任何分组，是运行期死键（除超管旁路外无人可用），判红；
      // 乙 该键授给了某个分组，但那个分组在后端角色种子里不建角、本车道也没有它的凭证，
      //   这是“测不到”而不是“没授权”，属 E2E 覆盖缺口，由 flow/200 的底数一判红，
      //   本文件如实登记，不在此处重复计红也不静默当成通过。
      const grantedGroups = [...matrixGrants.entries()]
        .filter(([, pairs]) =>
          backendGrants(
            pairs.map(p => `${p.resource}:${p.action}`),
            ep.resource,
            ep.action
          )
        )
        .map(([role]) => role);
      if (holders.length === 0) {
        expect(
          grantedGroups.length,
          `端点 ${ep.id} 的键 ${ep.resource}:${ep.action} 在整个后端授权矩阵里从未被授予，` +
            '该端点对所有业务岗位都是死键（要么键名派生与授权面不同、要么授权链断），点名交人工'
        ).toBeGreaterThan(0);
      }

      const observed: string[] = [];
      for (const role of blockers.slice(0, 3)) {
        const session = await roleSession(browser, role);
        const outcome = await deniedByPermissionGate(session.page, ep);
        expectDenied(
          { status: outcome.status, code: outcome.code },
          `越权侧：角色 ${role}（无 ${ep.resource}:${ep.action}）调端点 ${ep.method} ${ep.path}`
        );
        observed.push(`${role}|无键|403 权限门`);
      }
      for (const role of holders.slice(0, 3)) {
        const session = await roleSession(browser, role);
        const outcome = await deniedByPermissionGate(session.page, ep);
        // RBAC 之外的 handler 角色门是第二道门：角色不在门内时，被拒是应然，不是授权链断裂
        const expectedBlocked = ep.handlerRoleGate ? !ep.handlerRoleGate.includes(role) : false;
        expect(
          outcome.blocked,
          `持证侧：角色 ${role} 持 ${ep.resource}:${ep.action}` +
            `${ep.handlerRoleGate ? `，handler 角色门要求 ${ep.handlerRoleGate.join(' 或 ')}` : ''}，` +
            `预期被权限面拒=${expectedBlocked}，实际 status=${outcome.status} code=${JSON.stringify(outcome.code)}` +
            '（应放却拒属授权链或门控缺陷，应拒却放属越权缺陷，两向都保持判红）'
        ).toBe(expectedBlocked);
        observed.push(
          `${role}|持键|${expectedBlocked ? '按 handler 角色门拒' : '未被 RBAC 挡'} status=${outcome.status}`
        );
      }
      if (holders.length === 0) {
        observed.push(`持证侧无凭证会话可跑，该键的授权分组=${grantedGroups.join(',') || '无'}`);
      }

      mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
      writeFileSync(
        `${MATRIX_ARTIFACT_DIR}/202-vertical-${ep.id}.json`,
        JSON.stringify(
          {
            endpoint: `${ep.method} ${ep.path}`,
            derivedKey: `${derived.resource}:${derived.action}`,
            holderRoles: holders,
            blockerRolesSample: blockers.slice(0, 3),
            observed,
            note: '探针全部打在不存在记录 id 上，持证侧只判 RBAC 门是否放行，不改动任何真实单据',
          },
          null,
          2
        ) + '\n',
        'utf-8'
      );
    });
  }

  // ==================== 四 不相容岗位 ====================

  test('不相容岗位 仓管不得改报价（批准与整单改两腿都拒，销售经理同端点不被 RBAC 挡）', async ({
    browser,
  }) => {
    const warehouseRoles = ['warehouse_keeper', 'warehouse_manager'].filter(r =>
      getRoleCredential(r)
    );
    expect(
      warehouseRoles.length,
      '仓管侧岗位在 E2E 凭证里一个都没有，职责分离无从断，判红而非空过'
    ).toBeGreaterThan(0);
    for (const role of warehouseRoles) {
      const session = await roleSession(browser, role);
      for (const ep of PRIVILEGED_ENDPOINTS.filter(e => e.resource === 'quotations')) {
        expect(
          keyGrantedByMatrixRole(role, ep.resource, ep.action),
          `后端授权矩阵把 ${ep.resource}:${ep.action} 授给了仓管岗 ${role}，职责分离定义已被改动`
        ).toBe(false);
        const outcome = await deniedByPermissionGate(session.page, ep);
        expectDenied(
          { status: outcome.status, code: outcome.code },
          `仓管 ${role} 调 ${ep.cn} 端点`
        );
      }
    }
    // 同端点对照：批准权在销售经理手里（唯一变量是岗位权限码）
    const mgr = await roleSession(browser, 'sales_manager');
    const approve = PRIVILEGED_ENDPOINTS.find(e => e.id === 'quotation-approve')!;
    const mgrOutcome = await deniedByPermissionGate(mgr.page, approve);
    expect(
      mgrOutcome.blocked,
      `对照侧失守：销售经理持 quotations:approve 却被 RBAC 挡（status=${mgrOutcome.status} code=${JSON.stringify(
        mgrOutcome.code
      )}），仓管被拒的结论就失去同端点对照`
    ).toBe(false);
  });

  test('不相容岗位 业务员侧不得直接改库存数量（调整建行拒，仓储经理同端点不被 RBAC 挡）', async ({
    browser,
  }) => {
    const salesSide = [
      'sales_manager',
      'salesperson',
      'after_sales_manager',
      'customer_service',
    ].filter(r => getRoleCredential(r) && matrixGrants.has(r));
    expect(
      salesSide.length,
      '业务员侧岗位在凭证与授权矩阵里都取不到，库存数量写腿的职责分离无从断，判红'
    ).toBeGreaterThan(0);
    const ep = PRIVILEGED_ENDPOINTS.find(e => e.id === 'adjustment-item-add')!;
    const derived = derivePermissionKey(ep.method, `/api/v1/erp${ep.path}`);
    expect(`${derived.resource}:${derived.action}`).toBe('adjustments:create');

    for (const role of salesSide) {
      const session = await roleSession(browser, role);
      const outcome = await deniedByPermissionGate(session.page, ep);
      expectDenied(
        { status: outcome.status, code: outcome.code },
        `业务侧岗位 ${role} 直接给库存调整单建行（改库存数量）`
      );
    }
    const keeper = await roleSession(browser, 'inventory_manager');
    const holderOutcome = await deniedByPermissionGate(keeper.page, ep);
    expect(
      holderOutcome.blocked,
      `对照侧失守：库存经理持 adjustments:create 却被 RBAC 挡（status=${holderOutcome.status}），` +
        '业务侧被拒的结论失去同端点对照'
    ).toBe(false);
  });

  test('不相容岗位 财务侧不得改订单与面料明细（销售订单写腿拒，销售管理侧不被 RBAC 挡）', async ({
    browser,
  }) => {
    const financeSide = [
      'finance_manager',
      'cost_accountant',
      'accountant',
      'cashier',
      'ar_accountant',
      'ap_accountant',
    ].filter(r => getRoleCredential(r));
    expect(
      financeSide.length,
      '财务侧岗位一个凭证都取不到，订单写腿的职责分离无从断，判红'
    ).toBeGreaterThan(0);
    const ep = PRIVILEGED_ENDPOINTS.find(e => e.id === 'sales-order-update')!;
    for (const role of financeSide.slice(0, 4)) {
      const session = await roleSession(browser, role);
      const outcome = await deniedByPermissionGate(session.page, ep);
      expectDenied(
        { status: outcome.status, code: outcome.code },
        `财务侧岗位 ${role} 改销售订单（连带其面料明细行）`
      );
    }
    const mgr = await roleSession(browser, 'sales_manager');
    const mgrOutcome = await deniedByPermissionGate(mgr.page, ep);
    expect(
      mgrOutcome.blocked,
      `对照侧失守：销售经理持 orders:update 却被 RBAC 挡（status=${mgrOutcome.status}），` +
        '财务侧被拒的结论失去同端点对照'
    ).toBe(false);
  });

  test('互斥岗位权威表 role_conflicts 的内置默认可测：钱账六对在场、生产与质量两对不在场', async () => {
    const rows = await apiCallRaw<unknown>(admin.page, 'GET', '/roles/conflicts');
    if (!Array.isArray(rows)) {
      throw new Error(
        `GET /roles/conflicts 出参不是数组，实际键=${
          rows && typeof rows === 'object' ? Object.keys(rows).join(',') : String(rows)
        }`
      );
    }
    const pairs = (rows as Row[]).map(r =>
      [String(r.role_a_code), String(r.role_b_code)].sort().join('-')
    );
    const KEEP_FINANCE_PAIRS = [
      ['accountant', 'finance_manager'],
      ['accountant', 'cashier'],
      ['cashier', 'finance_manager'],
      ['cashier', 'purchase_manager'],
      ['cashier', 'purchase_clerk'],
      ['cashier', 'sales_manager'],
    ].map(p => p.slice().sort().join('-'));
    const DROPPED_PRODUCTION_PAIRS = [
      ['production_manager', 'qc_manager'],
      ['dyeing_master', 'quality_inspector'],
    ].map(p => p.slice().sort().join('-'));

    const lost = KEEP_FINANCE_PAIRS.filter(p => !pairs.includes(p));
    expect(
      lost,
      `库里的 role_conflicts 缺少内置的钱账分管互斥对 ${lost.join(' 与 ')}——` +
        '职责分离只剩纸面，换角色的写口校验对这些岗位不再生效'
    ).toEqual([]);
    const resurrected = DROPPED_PRODUCTION_PAIRS.filter(p => pairs.includes(p));
    expect(
      resurrected,
      `库里的 role_conflicts 又出现已被撤掉的“生产-质量”内置互斥对 ${resurrected.join(' 与 ')}——` +
        '纺织厂一人多岗是常态，这两对不该由系统内置硬互斥'
    ).toEqual([]);
    expect(
      pairs.length,
      `role_conflicts 实测 ${pairs.length} 对，为零说明整张互斥表没被播种，写口校验恒通过`
    ).toBeGreaterThan(0);
  });

  test('互斥写口真跑一次 出纳账号改派为会计必须被业务门拒（不是权限门拒）', async () => {
    const roles = await apiCallRaw<{ roles?: Row[] }>(admin.page, 'GET', '/roles');
    const list = roles.roles ?? [];
    const roleIdOf = (code: string): number => {
      const hit = list.find(r => String(r.code) === code);
      const id = Number(hit?.id);
      if (!Number.isFinite(id) || id <= 0) {
        throw new Error(
          `库里读不到角色码 ${code} 的主键，互斥写口用例的前置不成立（底数面由 flow/200 判）`
        );
      }
      return id;
    };
    const cashierRoleId = roleIdOf('cashier');
    const accountantRoleId = roleIdOf('accountant');

    const username = genCode('e2e_sod_cashier').toLowerCase();
    const created = await apiCallRaw<Row>(admin.page, 'POST', '/users', {
      username,
      password: 'E2eSod!Cashier2026',
      role_id: cashierRoleId,
    });
    const userId = Number(created?.id);
    if (!Number.isFinite(userId) || userId <= 0) {
      throw new Error(
        `前置失败：建出纳探针账号未返回 id，实际出参=${JSON.stringify(created)?.slice(0, 240)}`
      );
    }
    try {
      const failed = await apiCallExpectFail(admin.page, 'PUT', `/users/${userId}`, {
        role_id: accountantRoleId,
      });
      expect(
        failed.status,
        `互斥写口失守：出纳账号 ${userId} 被改派为会计（role_conflicts 在场互斥对）却得到 ` +
          `HTTP ${failed.status}，职责分离的写口校验没生效`
      ).not.toBe(200);
      // 归因到业务门而不是权限门：拒因必须是状态机或校验族机器码，不许是 FORBIDDEN
      expect(
        failed.code,
        `互斥改派的拒绝必须归到业务或校验门机器码（VALIDATION_ERROR 或 BUSINESS_ERROR 或 BAD_REQUEST），` +
          `实际 status=${failed.status} code=${JSON.stringify(failed.code)}`
      ).toEqual(expect.stringMatching(/^(VALIDATION_ERROR|BUSINESS_ERROR|BAD_REQUEST)$/));
    } finally {
      await apiCallExpectFail(admin.page, 'DELETE', `/users/${userId}`).catch(() => undefined);
    }
  });

  // ==================== 五 三端一致性（放行判定与客户端无关） ====================

  test('三端一致 后端实测放行结论必须等于按该角色登录出参权限码派生的结论（PC 与 H5 与小程序同一道门）', async ({
    browser,
  }) => {
    const roles = [
      ...new Set(
        PRIVILEGED_ENDPOINTS.flatMap(ep => [
          ...credentialedHolders(ep.resource, ep.action).slice(0, 1),
          ...credentialedNonHolders(ep.resource, ep.action).slice(0, 1),
        ])
      ),
    ].slice(0, 6);
    expect(
      roles.length,
      '越权矩阵挑不出任何可测角色（凭证缺失或授权矩阵分组改名），跨端一致性判据无从导出，判红'
    ).toBeGreaterThan(0);

    const mismatches: string[] = [];
    const matrixRows: string[] = [];
    for (const ep of PRIVILEGED_ENDPOINTS) {
      for (const role of roles) {
        const session = await roleSession(browser, role);
        const permissions = identityPermissions.get(role) ?? [];
        const rbacAllow = backendGrants(permissions, ep.resource, ep.action);
        // handler 层角色门是 RBAC 之后的第二道门，预期结论必须把两道门叠起来算，
        // 否则会把“审计面按角色码拒掉非审计岗”这一正确形态读成跨端不一致。
        const gateAllow = ep.handlerRoleGate ? ep.handlerRoleGate.includes(role) : true;
        const derived = rbacAllow && gateAllow;
        const outcome = await deniedByPermissionGate(session.page, ep);
        const allowed = !outcome.blocked;
        matrixRows.push(
          `${role}|${ep.method} ${ep.path}|RBAC=${rbacAllow}|handler角色门=${gateAllow}|派生放行=${derived}|实测放行=${allowed}`
        );
        if (derived !== allowed) {
          mismatches.push(
            `角色 ${role} 对端点 ${ep.method} ${ep.path}：按后端两道门叠算应${derived ? '放行' : '被拒'}（` +
              `RBAC=${rbacAllow} 依据键 ${ep.resource}:${ep.action}，handler 角色门=${gateAllow}` +
              `${ep.handlerRoleGate ? ` 要求 ${ep.handlerRoleGate.join(' 或 ')}` : ' 无'}），` +
              `后端实测${allowed ? '放行' : `拒（status=${outcome.status} code=${JSON.stringify(outcome.code)}）`}。` +
              '三端复刻的都是后端这一条判据，两者不等说明运行期的键派生与授权面已漂移'
          );
        }
      }
    }
    mkdirSync(MATRIX_ARTIFACT_DIR, { recursive: true });
    writeFileSync(
      `${MATRIX_ARTIFACT_DIR}/202-cross-client-gate.json`,
      JSON.stringify(
        {
          note: '每行是（角色，端点）的后端实测放行与按权限码派生结论。小程序沿用同一枚权限码口径，故这张表就是小程序侧的对账基准。',
          roles,
          rows: matrixRows,
        },
        null,
        2
      ) + '\n',
      'utf-8'
    );
    expect(
      mismatches,
      `跨端一致性不平共 ${mismatches.length} 项：\n  ${mismatches.join('\n  ')}`
    ).toEqual([]);
  });

  test('越权拒绝在列表形态与单条形态两条读腿上一致（同一枚权限码不因请求形状而变）', async ({
    browser,
  }) => {
    const blockers = credentialedNonHolders('purchase-prices', 'read').slice(0, 2);
    expect(
      blockers.length,
      '找不到无 purchase-prices:read 的持证角色，进价面的越权一致性无从断，判红'
    ).toBeGreaterThan(0);
    for (const role of blockers) {
      const session = await roleSession(browser, role);
      const listOutcome = await apiCallExpectFail(
        session.page,
        'GET',
        '/purchase/purchase-prices?page=1&page_size=5'
      );
      expectDenied(
        { status: listOutcome.status, code: listOutcome.code },
        `角色 ${role} 读采购价列表腿`
      );
      const singleOutcome = await apiCallExpectFail(
        session.page,
        'GET',
        `/purchase/purchase-prices/1`
      );
      expectDenied(
        { status: singleOutcome.status, code: singleOutcome.code },
        `角色 ${role} 读采购价单条腿`
      );
    }
    const holders = credentialedHolders('purchase-prices', 'read').slice(0, 1);
    for (const role of holders) {
      const session = await roleSession(browser, role);
      const listOutcome = await apiCallExpectFail(
        session.page,
        'GET',
        '/purchase/purchase-prices?page=1&page_size=5'
      );
      expect(
        `${listOutcome.status}:${String(listOutcome.code ?? '')}`,
        `持证角色 ${role} 的采购价列表腿不应是权限门拒绝，实际 status=${listOutcome.status} code=${JSON.stringify(
          listOutcome.code
        )}`
      ).not.toBe('403:FORBIDDEN');
    }
  });
});

/**
 * 按用户名精确解析 user_id。
 * 调用方：本文件的控制组用例（确定报价单归属人）。
 * 入参：admin 会话 page 与用户名。传给谁：GET /users 按 keyword 过滤后逐字比对 username。
 * 存什么/存哪里：只读请求，不持久化。
 * 命中数不等于 1 即判红：归属人不可判时，控制组的两条腿就不是只差归属。
 */
async function resolveUserId(page: Page, username: string): Promise<number> {
  const listed = await apiCallRaw<{ users?: Row[] }>(
    page,
    'GET',
    `/users?page=1&page_size=20&keyword=${encodeURIComponent(username)}`
  );
  const hits = (listed.users ?? []).filter(u => String(u.username ?? '') === username);
  expect(
    hits.length,
    `按用户名精确匹配 ${username} 应命中 1 行，实际 ${hits.length} 行——控制组的归属人 userId 不可判`
  ).toBe(1);
  const id = Number(hits[0].id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(
      `用户 ${username} 的 /users 出参 id 非有效数值，实际=${JSON.stringify(hits[0].id)}`
    );
  }
  return id;
}

/**
 * 由 admin 会话建一张报价单草稿并指定归属人。
 * 调用方：拒因前提自证与报价单控制组。
 * 入参：page、客户 id、归属业务员 user_id、产品 id、备注。
 * 传给谁：POST /quotations（归属列 sales_user_id 取入参，后端 create 不校验它等于创建人）。
 * 存什么/存哪里：报价单头与明细行落业务表；报价单族没有 DELETE 路由，按业务事实留库。
 */
async function createQuotationDraft(
  page: Page,
  args: { customerId: number; salesUserId: number; productId: number; notes: string }
): Promise<number> {
  const detail = await apiCallRaw<Row>(page, 'GET', `/products/${args.productId}`);
  const unit = String(detail?.unit ?? '米');
  const today = new Date().toISOString().slice(0, 10);
  const validUntil = new Date(Date.now() + 30 * 86400000).toISOString().slice(0, 10);
  const res = await apiCallRaw<Row>(page, 'POST', '/quotations', {
    customer_id: args.customerId,
    sales_user_id: args.salesUserId,
    quotation_date: today,
    valid_until: validUntil,
    currency: 'CNY',
    exchange_rate: '1',
    base_currency: 'CNY',
    price_terms: 'FOB',
    tax_inclusive: true,
    tax_rate: '0',
    items: [
      {
        product_id: args.productId,
        unit,
        quantity: '100',
        unit_price: '1',
        unit_price_with_tax: '1',
      },
    ],
    notes: args.notes,
  });
  const id = Number(res?.id);
  if (!Number.isFinite(id) || id <= 0) {
    throw new Error(
      `前置失败：建对照报价单 ${args.notes} 未返回 id，实际出参=${JSON.stringify(res)?.slice(0, 240)}`
    );
  }
  return id;
}
