// 权限域全流程契约级 E2E — 29 RBAC 权限键 + 行级数据范围(SELF) + IDOR 403 + 字段脱敏/隐藏
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（覆盖此前"能进页面即绿"的权限域）：
//   1) RBAC 按 URL 段推导的权限键真实生效：权限键 = /api/v1/erp/{seg3}/{seg4} 消歧后的资源键
//      （middleware/permission.rs:267-300 extract_resource_info：seg3='crm'、seg4='leads' 不构成
//      双层模块前缀（path_utils.rs:113-132 is_nested_module_prefix 未登记 crm/leads），故走
//      resolve_module_prefixed_resource('crm','leads')（path_utils.rs:135-182，:166 消歧分支）
//      ⇒ 运行时权威键是 **`crm-leads`**，与注册表同名（init_service.rs:143 PERMISSION_RESOURCES）；
//      action=method_to_action :325-327）。授未登记的 'leads' 假键不会命中任何派生键（授码端点
//      不按注册表校验 resource_type，role_permission_service.rs:472-487 直插 ⇒ 授权行成死码）。
//      新角色零授权 → 一切业务端点 403（check_permission :532 起查 role_permission 表，
//      admin 短路 :496）；仅授 `crm-leads` 后 /crm/leads 通、/departments 仍 403——
//      逐资源粒度，不是"非黑即白"。
//   2) 行级数据范围 SELF 真实过滤行：新角色默认 data_scope=self
//      （services/role_permission_service.rs:184-185；词表 all/dept/self，
//      utils/data_scope.rs:28-35），列表按 owner_id 过滤
//      （services/crm/lead.rs:141-151 → utils/data_scope.rs:106-109），
//      他人行在列表中真实不可见（不是"前端没渲染"）。
//   3) 详情/改/删的 IDOR 防护：get/update/delete lead 均先以数据范围校验归属
//      （handlers/crm_handler.rs:299-301/:346-348/:361-363 → lead.rs:358-367
//      check_resource_owner(data_scope.rs:149-171)），越权 403 且**被拒资源零漂移**。
//   4) 越权 403 文案永久脱敏：PermissionDenied → code=FORBIDDEN
//      （utils/error.rs:707-745，public_message 固定文案 :763-780）——本链只断 status+code，
//      不断原因文案（用户硬令）。
//   5) 数据权限配置端点契约：scope_type 白名单（handlers/data_permission_handler.rs:143-146，
//      取值 ALL/DEPT/DEPT_AND_BELOW/SELF/CUSTOM）越界 400 VALIDATION；非 admin 写配置 403
//      （require_admin_role :22-32）；配置 hidden_fields 后 U 的线索列表行**真的不含该键**
//      （crm_handler.rs:58-73 filter_fields_batch），而默认分支是脱敏非删除
//      （P1-08-5，crm_handler.rs:75-101 + utils/field_mask.rs mask_phone）。
//   6) 客户收货地址出参的 PII 形态与客户域同一真源
//      （services/crm/cust.rs::mask_customer_pii_defaults，经
//      handlers/customer_address_handler.rs 的出参转换接入）：非 admin 会话 contact_phone
//      为打码值、address 整键不下发；读写出口同形（写响应原文回显即旁路）；admin 同一行
//      原文与本人行的非 PII 列作为正向对照（防"整列恒空"型假绿）。
//
// CI 测不到（显式声明）：
//   - DEPT/DEPT_AND_BELOW 范围：role.data_scope 无 API 写入口（仅迁移 SQL 更新，
//     migration/src/domain/finance/mod.rs:26-35），用例无法自建 dept 范围角色；
//   - RLS 策略层（PostgreSQL app.user_id 会话变量）与应用层过滤的双保险一致性——需 DBA 侧验证；
//   - 权限缓存跨实例失效（Redis pub/sub，permission.rs:400-461）单实例 CI 下不可测；
//   - print/export 角色黑名单（permission.rs:463-478）与字段级写权限（field-permissions）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  tryCleanup,
  loginInIsolatedContext,
  type IsolatedAuthedSession,
  APP_ERROR_CODES,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** AppError 机器码取源 flow/helpers 的 APP_ERROR_CODES（FORBIDDEN 登记依据 utils/error.rs:709 CODE_FORBIDDEN） */
const ERR_FORBIDDEN = APP_ERROR_CODES.FORBIDDEN;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/** 403 断言口径：只钉 status+机器码，不断原因文案（权限文案永久脱敏是产品硬令） */
function expectForbidden(
  res: { status: number; code?: string | number; message?: string | null },
  context: string
): void {
  expect(
    res.status,
    `${context}：应 403，实际 status=${res.status} code=${res.code ?? '(none)'}`
  ).toBe(403);
  expect(failureCode(res), `${context}：403 机器码应为 FORBIDDEN`).toBe(ERR_FORBIDDEN);
}

/** 本用例专属：自建 角色 + 用户（不消耗全局种子），返回 id 与登录凭证 */
async function seedRoleAndUser(
  page: Page,
  tag: string
): Promise<{ roleId: number; userId: number; username: string; password: string }> {
  const ts = Date.now().toString().slice(-6);
  const role = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/roles', {
    name: `E2F29角色${ts}${tag}`,
    // 角色编码：小写字母/数字/下划线 3-50（role_permission_service.rs:151-158）
    code: `e2f29_${tag.toLowerCase()}_${ts}`,
  });
  const roleId = requireNum(role.id, '建角色');
  CLEANUP.push({ path: `/roles/${roleId}`, label: `role#${roleId}` });

  const username = `f29${tag.toLowerCase()}u${ts}`;
  // 密码强度：>=8 + 混合类型 + 非常见 + 不含用户名子串（user_handler.rs:44-72）
  const password = `E2p!${ts}${tag}9qX`;
  const user = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/users', {
    username,
    password,
    role_id: roleId,
  });
  const userId = requireNum(user.id, '建用户');
  CLEANUP.push({ path: `/users/${userId}`, label: `user#${userId}` });
  // 回读：角色/归属真实落库（GET /users/{id} admin 可查）
  const back = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/users/${userId}`);
  expect(back.username, '用户名落库').toBe(username);
  expect(Number(back.role_id), 'role_id 外键落库').toBe(roleId);
  expect(back.is_active, '新用户默认激活').toBe(true);
  return { roleId, userId, username, password };
}

/** 授权并登记清理（记录 role_permission id，防跨用例残留授权） */
async function grantPermission(
  page: Page,
  roleId: number,
  resourceType: string,
  action: string
): Promise<void> {
  const perm = await apiCallRaw<Record<string, unknown>>(
    page,
    'POST',
    `/roles/${roleId}/permissions`,
    {
      resource_type: resourceType,
      action,
      allowed: true,
    }
  );
  const permId = requireNum(perm.id, `授权 ${resourceType}:${action}`);
  CLEANUP.push({ path: `/roles/permissions/${permId}`, label: `role_permission#${permId}` });
}

/** admin 会话建一条线索（本用例自用），返回 id */
async function seedLeadAsAdmin(page: Page, tag: string): Promise<number> {
  const lead = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/leads', {
    company_name: `E2F29Admin线索${tag}${Date.now().toString().slice(-6)}`,
    contact_name: `归属人甲${tag}`,
    mobile_phone: '13800002901',
  });
  const id = requireNum(lead.id, 'admin 建线索');
  CLEANUP.push({ path: `/crm/leads/${id}`, label: `crm_lead(admin)#${id}` });
  return id;
}

/** 读 /crm/leads 的 data 数组（形状：lead.rs:163-168 手搓 {data,total,page,page_size}） */
async function listLeadsRaw(
  page: Page,
  params = '?page=1&page_size=100'
): Promise<Record<string, unknown>[]> {
  const res = await apiCallRaw<unknown>(page, 'GET', `/crm/leads${params}`);
  return pickListArray<Record<string, unknown>>(res, 'data', 'F29 /crm/leads 列表');
}

test.describe('29 权限键与数据范围契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('29-01 零授权角色：一切业务端点 403（只断 FORBIDDEN 码）；admin 对照可达', async ({
    page,
    browser,
  }) => {
    const adminLeadId = await seedLeadAsAdmin(page, 'A1');
    const { username, password } = await seedRoleAndUser(page, 'Z1');

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);
      // 未登记的权限键：读列表被拒（permission.rs:524 check_permission 无 role_permission 行）
      const deniedList = await apiCallExpectFail(
        session.page,
        'GET',
        '/crm/leads?page=1&page_size=10'
      );
      expectForbidden(deniedList, `零授权用户 ${username} GET /crm/leads`);
      // 对照：同端点 admin 会话可达且能看见自己的线索（证明 403 是权限判定，不是端点坏了）
      const adminList = await listLeadsRaw(page);
      expect(
        adminList.some(l => Number(l.id) === adminLeadId),
        `admin 列表应含线索 ${adminLeadId}`
      ).toBe(true);
    } finally {
      await session?.close();
    }
  });

  test('29-02 授权 crm-leads 后逐资源粒度生效；SELF 行过滤：他人行不可见、自建行可见且 email 脱敏/address 移除', async ({
    page,
    browser,
  }) => {
    const adminLeadId = await seedLeadAsAdmin(page, 'A2');
    const { roleId, userId, username, password } = await seedRoleAndUser(page, 'Z2');
    await grantPermission(page, roleId, 'crm-leads', 'read');
    await grantPermission(page, roleId, 'crm-leads', 'create');

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);

      // 授权生效：U 可读列表（200）
      const beforeCreate = await listLeadsRaw(session.page);
      // SELF 行过滤（data_scope.rs:106-109 + lead.rs:141-151）：admin 归属的行对 U 不可见
      expect(
        beforeCreate.some(l => Number(l.id) === adminLeadId),
        `SELF 范围用户不应看到 admin 归属线索 ${adminLeadId}，实际列表=${JSON.stringify(
          beforeCreate.map(l => ({ id: l.id, owner_id: l.owner_id }))
        ).slice(0, 300)}`
      ).toBe(false);

      // U 自建线索（create 已授权；owner 落 U 自己，lead.rs:53）
      // 注：crm_lead 的联系方式列名是 mobile_phone/email，P1-08-5 默认脱敏分支实际命中的是
      // email（mask）与 address（remove）（crm_handler.rs:86-98 逐键取 contact_phone/email）
      const own = await apiCallRaw<Record<string, unknown>>(session.page, 'POST', '/crm/leads', {
        company_name: `E2F29U线索${Date.now().toString().slice(-6)}`,
        contact_name: '归属人乙',
        mobile_phone: '13800002902',
        email: 'e2f29own@example.com',
        address: 'E2F29地址不应外显',
      });
      const ownId = requireNum(own.id, 'U 建线索');
      CLEANUP.push({ path: `/crm/leads/${ownId}`, label: `crm_lead(U)#${ownId}` });

      const afterCreate = await listLeadsRaw(session.page);
      const ownRow = afterCreate.find(l => Number(l.id) === ownId);
      expect(ownRow, 'U 应看到自己归属的线索（SELF 不误伤本人）').toBeTruthy();
      expect(Number(ownRow!.owner_id), '行 owner_id=U 用户 id').toBe(userId);
      // 默认脱敏分支（无 data_permission 配置且非 admin：crm_handler.rs:75-101）——
      // email 被打码（field_mask.rs:16-25 首字母+***@域），address 键被移除；
      // 而不是"行没返回"
      expect(
        typeof ownRow!.email === 'string' &&
          String(ownRow!.email).includes('***@') &&
          ownRow!.email !== 'e2f29own@example.com',
        `非 admin 列表 email 应脱敏，实际=${JSON.stringify(ownRow!.email)}`
      ).toBe(true);
      expect(
        ownRow && !('address' in ownRow),
        `非 admin 列表 address 键应被移除（crm_handler.rs:98），实际行=${JSON.stringify(
          ownRow
        ).slice(0, 300)}`
      ).toBe(true);

      // 逐资源粒度：departments 键未授权 → 仍 403（不是"授权一个域全部放行"）
      const deniedDept = await apiCallExpectFail(
        session.page,
        'GET',
        '/departments?page=1&page_size=10'
      );
      expectForbidden(deniedDept, `仅授 leads 的用户 GET /departments`);
    } finally {
      await session?.close();
    }
  });

  test('29-03 IDOR 门：越权读/改/删他人线索 403 且被拒资源零漂移；本人线索对照可达', async ({
    page,
    browser,
  }) => {
    const adminLeadId = await seedLeadAsAdmin(page, 'A3');
    const { roleId, username, password } = await seedRoleAndUser(page, 'Z3');
    await grantPermission(page, roleId, 'crm-leads', 'read');
    await grantPermission(page, roleId, 'crm-leads', 'create');
    await grantPermission(page, roleId, 'crm-leads', 'update');
    await grantPermission(page, roleId, 'crm-leads', 'delete');

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);

      // 中间件放行（已授权）但数据范围拒绝（lead.rs:358-367 check_resource_owner）→ 403
      const deniedGet = await apiCallExpectFail(session.page, 'GET', `/crm/leads/${adminLeadId}`);
      expectForbidden(deniedGet, `SELF 用户 GET 他人线索详情`);
      const deniedPut = await apiCallExpectFail(session.page, 'PUT', `/crm/leads/${adminLeadId}`, {
        contact_name: 'E2F29篡改',
      });
      expectForbidden(deniedPut, `SELF 用户 PUT 他人线索`);
      const deniedDel = await apiCallExpectFail(
        session.page,
        'DELETE',
        `/crm/leads/${adminLeadId}`
      );
      expectForbidden(deniedDel, `SELF 用户 DELETE 他人线索`);

      // 零漂移复证：admin 回读，被篡改/删除目标的业务字段一字未变
      const intact = await apiCallRaw<Record<string, unknown>>(
        page,
        'GET',
        `/crm/leads/${adminLeadId}`
      );
      expect(intact.contact_name, '越权 PUT 被拒后 contact_name 零漂移').not.toBe('E2F29篡改');
      expect(intact.lead_status ?? null, '线索状态不被越权请求改写').toBeNull();

      // 对照：U 自己的线索 GET 200
      const own = await apiCallRaw<Record<string, unknown>>(session.page, 'POST', '/crm/leads', {
        company_name: `E2F29对照${Date.now().toString().slice(-6)}`,
        contact_name: '本人可达',
      });
      const ownId = requireNum(own.id, 'U 建对照线索');
      CLEANUP.push({ path: `/crm/leads/${ownId}`, label: `crm_lead(U-对照)#${ownId}` });
      const ownGet = await apiCallRaw<Record<string, unknown>>(
        session.page,
        'GET',
        `/crm/leads/${ownId}`
      );
      expect(Number(ownGet.id), '本人线索可达（403 不是"一律拒绝"）').toBe(ownId);
    } finally {
      await session?.close();
    }
  });

  test('29-04 数据权限配置契约：非法 scope 400 / 非 admin 403 / hidden_fields 配置后列表行真实丢键', async ({
    page,
    browser,
  }) => {
    const { roleId, username, password } = await seedRoleAndUser(page, 'Z4');
    await grantPermission(page, roleId, 'crm-leads', 'read');
    await grantPermission(page, roleId, 'crm-leads', 'create');

    // 非法 scope_type → 400 VALIDATION（data_permission_handler.rs:143-146 白名单）
    const badScope = await apiCallExpectFail(page, 'POST', '/data-permissions', {
      role_id: roleId,
      resource_type: 'crm_lead',
      scope_type: 'EVERYTHING',
    });
    expect(badScope.status, '非法 scope_type 应 400').toBe(400);
    expect(failureCode(badScope), '非法 scope 机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);
      // U 写数据权限 → 403（未登记 data-permissions 键，中间件先行拒绝；handler 内还有
      // require_admin_role 二次防线 data_permission_handler.rs:22-32）
      const deniedWrite = await apiCallExpectFail(session.page, 'POST', '/data-permissions', {
        role_id: roleId,
        resource_type: 'crm_lead',
        scope_type: 'ALL',
      });
      expectForbidden(deniedWrite, '非授权用户写数据权限');

      // U 先建一条自己的线索：配置前 email"存在但被打码"
      const own = await apiCallRaw<Record<string, unknown>>(session.page, 'POST', '/crm/leads', {
        company_name: `E2F29脱敏对照${Date.now().toString().slice(-6)}`,
        contact_name: '脱敏实验',
        email: 'e2f29hide@example.com',
      });
      const ownId = requireNum(own.id, 'U 建脱敏线索');
      CLEANUP.push({ path: `/crm/leads/${ownId}`, label: `crm_lead(U-脱敏)#${ownId}` });
      const before = (await listLeadsRaw(session.page)).find(l => Number(l.id) === ownId);
      expect(before, '配置前 U 应看到自建线索').toBeTruthy();
      expect(
        'email' in (before as object) && String(before!.email).includes('***@'),
        `配置前 email 应为脱敏态（键在、值打码），实际=${JSON.stringify(before!.email)}`
      ).toBe(true);

      // admin 配置 hidden_fields=["email"]（POST /data-permissions，
      // DataPermissionService 落库；读取应用点 crm_handler.rs:58-73）
      const cfg = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/data-permissions', {
        role_id: roleId,
        resource_type: 'crm_lead',
        scope_type: 'SELF',
        hidden_fields: ['email'],
      });
      const cfgId = requireNum(cfg.id, '数据权限配置 id');
      CLEANUP.push({ path: `/data-permissions/${cfgId}`, label: `data_permission#${cfgId}` });
      expect(cfg.scope_type, '配置回读 scope=SELF（白名单值 data_permission_handler.rs:143）').toBe(
        'SELF'
      );

      // 配置列表端点回读：真实登记
      const cfgList = await apiCallRaw<unknown>(page, 'GET', `/data-permissions/roles/${roleId}`);
      const cfgRows = pickListArray<Record<string, unknown>>(
        cfgList,
        'bare',
        'F29 /data-permissions/roles'
      );
      const row = cfgRows.find(r => r.resource_type === 'crm_lead');
      expect(row, `角色 ${roleId} 的 crm_lead 配置应可回读`).toBeTruthy();
      expect(row!.scope_type, '列表回读 SELF').toBe('SELF');

      // U 再读列表：行仍在（SELF 本人）、但 email 键真实消失（隐藏 ≠ 打码）
      const after = (await listLeadsRaw(session.page, '?page=1&page_size=100')).find(
        l => Number(l.id) === ownId
      );
      expect(after, '配置后本人线索仍应在列表（隐藏字段不隐藏行）').toBeTruthy();
      expect(
        after && !('email' in after),
        `hidden_fields 生效后列表行不应含 email 键，实际=${JSON.stringify(after).slice(0, 300)}`
      ).toBe(true);
    } finally {
      await session?.close();
    }
  });

  test('29-05 客户收货地址出参 PII：非 admin 电话打码且 address 键不下发；admin 同一行原文对照', async ({
    page,
    browser,
  }) => {
    const { roleId, username, password } = await seedRoleAndUser(page, 'Z5');
    // 地址端点的权限键按 URL 段推导（seg3='crm' 是模块前缀，seg4='customers' 未登记消歧
    // 映射 ⇒ path_utils.rs resolve_module_prefixed_resource 走默认分支）⇒ 运行时键 'customers'
    await grantPermission(page, roleId, 'customers', 'read');
    await grantPermission(page, roleId, 'customers', 'create');
    await grantPermission(page, roleId, 'customers', 'update');

    const PLAIN_PHONE = '13800002905';
    const PLAIN_ADDRESS = 'E2F29明文详细地址不应外显';

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);

      // 地址行的可见性经父客户归属继承（SELF 范围 ⇒ 必须 U 本人的客户），
      // 故先由 U 自建客户（customer_handler.rs 把 owner 落创建人本人）
      const cust = await apiCallRaw<Record<string, unknown>>(
        session.page,
        'POST',
        '/crm/customers',
        {
          customer_name: `E2F29地址脱敏客户${Date.now().toString().slice(-6)}`,
        }
      );
      const customerId = requireNum(cust.id, 'U 建客户');
      CLEANUP.push({ path: `/crm/customers/${customerId}`, label: `customer(U)#${customerId}` });

      // admin 写入明文电话与详细地址；写响应即第一重正向对照（admin 拿原文，不是空列）
      const created = await apiCallRaw<Record<string, unknown>>(
        page,
        'POST',
        `/crm/customers/${customerId}/addresses`,
        {
          contact_name: '收货人戊',
          contact_phone: PLAIN_PHONE,
          address: PLAIN_ADDRESS,
          province: '广东省',
        }
      );
      const addressId = requireNum(created.id, 'admin 建地址');
      CLEANUP.push({
        path: `/crm/customers/${customerId}/addresses/${addressId}`,
        label: `customer_address#${addressId}`,
      });
      expect(String(created.contact_phone), 'admin 写响应 contact_phone 应为原文').toBe(
        PLAIN_PHONE
      );
      expect(String(created.address), 'admin 写响应 address 应为原文').toBe(PLAIN_ADDRESS);

      // admin 读列表：同一行仍是原文（读出口未被"为脱敏而清空"）
      const adminRow = pickListArray<Record<string, unknown>>(
        await apiCallRaw<unknown>(page, 'GET', `/crm/customers/${customerId}/addresses`),
        'bare',
        'F29 admin 地址列表'
      ).find(r => Number(r.id) === addressId);
      expect(adminRow, 'admin 列表应含该地址行').toBeTruthy();
      expect(String(adminRow!.contact_phone), 'admin 列表电话应为原文').toBe(PLAIN_PHONE);
      expect(String(adminRow!.address), 'admin 列表 address 应为原文').toBe(PLAIN_ADDRESS);

      // 非 admin 读列表：同一门（本人客户）放行、同一行，但出参形态按客户域真源改写
      const uRow = pickListArray<Record<string, unknown>>(
        await apiCallRaw<unknown>(session.page, 'GET', `/crm/customers/${customerId}/addresses`),
        'bare',
        'F29 非 admin 地址列表'
      ).find(r => Number(r.id) === addressId);
      expect(uRow, 'SELF 用户应看到本人客户的地址行（脱敏只改形态，不隐藏行）').toBeTruthy();
      // ① 电话：键保留、值打码，且必须 ≠ 原文
      expect(
        typeof uRow!.contact_phone === 'string' &&
          String(uRow!.contact_phone).includes('****') &&
          uRow!.contact_phone !== PLAIN_PHONE,
        `非 admin 列表 contact_phone 应为打码值，实际=${JSON.stringify(uRow!.contact_phone)}`
      ).toBe(true);
      // ② 详细地址：整键不下发（既不是空串也不是 null——那是"值确实为空"，两回事）
      expect(
        !('address' in uRow!),
        `非 admin 列表不应含 address 键，实际行=${JSON.stringify(uRow).slice(0, 300)}`
      ).toBe(true);
      // 正向对照②：本人行的非 PII 列照常下发（防"整行被清空"型假绿）
      expect(String(uRow!.contact_name), '非 admin 收货人列仍下发').toBe('收货人戊');
      expect(String(uRow!.province), '非 admin 省份列仍下发').toBe('广东省');
      expect(uRow!.is_default, '非 admin 默认标记仍下发').toBe(false);

      // 写响应旁路：非 admin 只改一个非 PII 列（remark），响应不得回显原文电话/地址
      const updated = await apiCallRaw<Record<string, unknown>>(
        session.page,
        'PUT',
        `/crm/customers/${customerId}/addresses/${addressId}`,
        { remark: 'E2F29仅改备注' }
      );
      expect(
        typeof updated.contact_phone === 'string' &&
          String(updated.contact_phone).includes('****') &&
          updated.contact_phone !== PLAIN_PHONE,
        `非 admin 写响应 contact_phone 应为打码值，实际=${JSON.stringify(updated.contact_phone)}`
      ).toBe(true);
      expect(
        !('address' in updated),
        `非 admin 写响应不应含 address 键，实际=${JSON.stringify(updated).slice(0, 300)}`
      ).toBe(true);
      expect(String(updated.remark), '写响应应回读本次真正改动的列').toBe('E2F29仅改备注');

      // 库侧零污染复证：U 的一次编辑不能把打码值/空地址写回真实列（admin 原文回读）
      const afterWrite = pickListArray<Record<string, unknown>>(
        await apiCallRaw<unknown>(page, 'GET', `/crm/customers/${customerId}/addresses`),
        'bare',
        'F29 admin 地址列表（写后复证）'
      ).find(r => Number(r.id) === addressId);
      expect(afterWrite, '写后 admin 仍能读回该地址行').toBeTruthy();
      expect(String(afterWrite!.contact_phone), '库内电话应仍是原文（打码值未被写回）').toBe(
        PLAIN_PHONE
      );
      expect(String(afterWrite!.address), '库内详细地址应仍是原文').toBe(PLAIN_ADDRESS);
    } finally {
      await session?.close();
    }
  });

  test('29-06 客户 360 的 shipping_addresses 出参 PII：非 admin 电话打码且 address 键不下发；admin 同一行原文对照', async ({
    page,
    browser,
  }) => {
    const { roleId, username, password } = await seedRoleAndUser(page, 'Z6');
    // 360 端点权限键按 URL 段推导（seg3='crm' 模块前缀、seg4='customers' ⇒ 运行时键 'customers'，
    // 与地址列表端点同源）；读 360 需 customers:read，SELF 用户自建本人客户需 customers:create
    await grantPermission(page, roleId, 'customers', 'read');
    await grantPermission(page, roleId, 'customers', 'create');

    const PLAIN_PHONE = '13800002906';
    const PLAIN_ADDRESS = 'E2F29明文收货地址不应经360外显';

    let session: IsolatedAuthedSession | undefined;
    try {
      session = await loginInIsolatedContext(browser, username, password);

      // SELF 范围 ⇒ 地址行可见性经父客户归属继承（get_customer_360 行门 check_resource_owner
      // 按 customers.owner_id），故先由 U 自建客户（owner 落本人）
      const cust = await apiCallRaw<Record<string, unknown>>(
        session.page,
        'POST',
        '/crm/customers',
        { customer_name: `E2F29地址360客户${Date.now().toString().slice(-6)}` }
      );
      const customerId = requireNum(cust.id, 'U 建客户');
      CLEANUP.push({ path: `/crm/customers/${customerId}`, label: `customer(U)#${customerId}` });

      // admin 写入明文电话与详细地址，作为两种会话共同的数据基线
      const created = await apiCallRaw<Record<string, unknown>>(
        page,
        'POST',
        `/crm/customers/${customerId}/addresses`,
        {
          contact_name: '收货人己',
          contact_phone: PLAIN_PHONE,
          address: PLAIN_ADDRESS,
          province: '广东省',
        }
      );
      const addressId = requireNum(created.id, 'admin 建地址');
      CLEANUP.push({
        path: `/crm/customers/${customerId}/addresses/${addressId}`,
        label: `customer_address#${addressId}`,
      });

      // 正向对照（admin）：360 出参 shipping_addresses 行是原文，address 键存在、
      // contact_phone 是原文 ⇒ 证明"脱敏没把整列清空"，且 admin 既有放行口径未收紧
      const adminData = await apiCallRaw<{ shipping_addresses: Record<string, unknown>[] }>(
        page,
        'GET',
        `/crm/customers/${customerId}/360`
      );
      expect(
        Array.isArray(adminData.shipping_addresses),
        `360 出参应含 shipping_addresses 数组，实际=${JSON.stringify(adminData).slice(0, 300)}`
      ).toBe(true);
      const adminRow = adminData.shipping_addresses.find(r => Number(r.id) === addressId);
      expect(adminRow, 'admin 360 应含该地址行').toBeTruthy();
      expect(String(adminRow!.contact_phone), 'admin 360 contact_phone 应为原文').toBe(PLAIN_PHONE);
      expect(
        'address' in adminRow! && String(adminRow!.address),
        'admin 360 address 键应存在且为原文'
      ).toBe(PLAIN_ADDRESS);
      // 非 PII 列（收货人/省份/默认标记）admin 侧照常下发
      expect(String(adminRow!.contact_name), 'admin 360 收货人列仍下发').toBe('收货人己');

      // 非 admin（SELF 本人客户，行门放行同一行）：出参形态按客户域真源逐行改写
      const uData = await apiCallRaw<{ shipping_addresses: Record<string, unknown>[] }>(
        session.page,
        'GET',
        `/crm/customers/${customerId}/360`
      );
      expect(
        Array.isArray(uData.shipping_addresses),
        `非 admin 360 出参应含 shipping_addresses 数组，实际=${JSON.stringify(uData).slice(0, 300)}`
      ).toBe(true);
      const uRow = uData.shipping_addresses.find(r => Number(r.id) === addressId);
      expect(uRow, 'SELF 用户应看到本人客户的地址行（脱敏只改形态，不隐藏行）').toBeTruthy();
      // ① 电话：键保留、值打码，且必须 ≠ 原文
      expect(
        typeof uRow!.contact_phone === 'string' &&
          String(uRow!.contact_phone).includes('****') &&
          uRow!.contact_phone !== PLAIN_PHONE,
        `非 admin 360 contact_phone 应为打码值，实际=${JSON.stringify(uRow!.contact_phone)}`
      ).toBe(true);
      // ② 详细地址：整键不下发（既不是空串也不是 null——那是"值确实为空"，两回事）
      expect(
        !('address' in uRow!),
        `非 admin 360 不应含 address 键，实际行=${JSON.stringify(uRow).slice(0, 300)}`
      ).toBe(true);
      // 正向对照②：本人行的非 PII 列照常下发（防"整行被清空"型假绿）
      expect(String(uRow!.contact_name), '非 admin 360 收货人列仍下发').toBe('收货人己');
      expect(String(uRow!.province), '非 admin 360 省份列仍下发').toBe('广东省');
      expect(uRow!.is_default, '非 admin 360 默认标记仍下发').toBe(false);
      // 响应体任何位置都不得出现明文电话/地址（无论键名、无论嵌套）
      const raw = JSON.stringify(uData);
      expect(
        raw.includes(PLAIN_PHONE),
        `非 admin 360 响应不得含明文电话: ${raw.slice(0, 300)}`
      ).toBe(false);
      expect(
        raw.includes(PLAIN_ADDRESS),
        `非 admin 360 响应不得含明文地址: ${raw.slice(0, 300)}`
      ).toBe(false);
    } finally {
      await session?.close();
    }
  });
});
