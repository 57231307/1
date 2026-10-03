// 权限域全流程契约级 E2E — 29 RBAC 权限键 + 行级数据范围(SELF) + IDOR 403 + 字段脱敏/隐藏
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（覆盖此前"能进页面即绿"的权限域）：
//   1) RBAC 按 URL 段推导的权限键真实生效：权限键 = /api/v1/erp/{seg3}/{seg4} 消歧后的
//      seg4（middleware/permission.rs:259-315 extract_resource_info，'crm/leads'→resource
//      'leads'，path_utils.rs:75 白名单含 'crm'；action=method_to_action :317-327）。
//      新角色零授权 → 一切业务端点 403（check_permission :524-599 查 role_permission 表，
//      admin 短路 :536）；仅授 'leads' 后 /crm/leads 通、/departments 仍 403——
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

  test('29-02 授权 leads 后逐资源粒度生效；SELF 行过滤：他人行不可见、自建行可见且 email 脱敏/address 移除', async ({
    page,
    browser,
  }) => {
    const adminLeadId = await seedLeadAsAdmin(page, 'A2');
    const { roleId, userId, username, password } = await seedRoleAndUser(page, 'Z2');
    await grantPermission(page, roleId, 'leads', 'read');
    await grantPermission(page, roleId, 'leads', 'create');

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
    await grantPermission(page, roleId, 'leads', 'read');
    await grantPermission(page, roleId, 'leads', 'create');
    await grantPermission(page, roleId, 'leads', 'update');
    await grantPermission(page, roleId, 'leads', 'delete');

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
    await grantPermission(page, roleId, 'leads', 'read');
    await grantPermission(page, roleId, 'leads', 'create');

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
});
