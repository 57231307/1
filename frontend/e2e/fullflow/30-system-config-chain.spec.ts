// 系统域全流程契约级 E2E — 30 部门树 / 用户 / 单据号查重（配置主数据真实落库与守卫回读）
//
// **声明：本文件未在本地实跑（本机禁跑 Playwright），仅按后端源码契约编写，待 CI/联调验证。**
//
// 本链证明什么（覆盖此前只测"部门页能打开、用户列表能看到行"的系统域）：
//   1) 部门创建：code 缺省时服务端生成 `DEPT_{毫秒时间戳}`（services/department_service.rs:174-178），
//      显式传 code 原样落库（handler DTO CreateDepartmentRequest，department_handler.rs:25-36）；
//      名称唯一性 = 业务族（department_service.rs:147-158）；parent_id 指向不存在部门 = 404
//      （:160-167）；is_active 默认 true（:185）。
//   2) 部门树端点真实父子嵌套（get_department_tree，:387-431）——不是平铺列表。
//   3) 部门更新的**三态语义**（RFC7386 部分更新，department_handler.rs:56-87 +
//      department_service.rs:218-351）：键缺席=保持、显式 null 对 NOT NULL 列（name/code/
//      sort_order/is_active）当场 400 BUSINESS 拒绝（:231-249），对可空列（description/
//      parent_id/manager_id）真实置 NULL（:300-327）；code 改重复值 = BUSINESS（:290-295）。
//   4) 部门删除守卫：有子部门 → 400 BUSINESS（:354-363）；有用户挂靠 → 400 BUSINESS
//      （:365-376，且用户是软删除、department_id 仍在 ⇒ 软删用户同样占用，本链把该事实钉死）；
//      解除引用后可删，再 GET 404。
//   5) 用户契约：用户名唯一=业务族（services/user_service.rs:131-144）、密码强度/邮箱格式
//      =取值族 VALIDATION_ERROR（handlers/user_handler.rs:44-87 自定义校验 + validator）；
//      DELETE /users/{id} 为**软删除**：is_active=false、记录仍在（user_service.rs:470-480）。
//   6) 单据号查重端点（"单号禁手输"契约的读侧）：GET /document-no/check
//      （handlers/document_no_handler.rs:27-41）——真实单号→true、未占用→false、
//      空号→400 VALIDATION_ERROR（no 被 query_params 边界中间件剥键→缺失字段按 IR 判 VALIDATION）、
//      未登记 doc_type→400 BAD_REQUEST（utils/number_generator.rs:582-585 兜底分支）。
//
// CI 测不到（显式声明）：
//   - 部门负责人 manager_name 的 JOIN 富化正确性（依赖 users.real_name 种子，分片账号常为空串）；
//   - 用户 Argon2 哈希/登录锁定策略（login_security 域另链）；
//   - document-no 并发取号 advisory lock 的真实防重（并发时序属压测范畴，此处只钉查重语义）。
import { test, expect, type Page } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  ensureTestEntities,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  failureCode,
  genCode,
  tryCleanup,
  expectBusinessRejection,
  APP_ERROR_CODES,
} from '../flow/helpers';
import { pickListArray } from '../flow/ui-helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.slice().reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

/** AppError 机器码取源 flow/helpers 的 APP_ERROR_CODES（NOT_FOUND 登记依据 utils/error.rs:739） */
const ERR_NOT_FOUND = APP_ERROR_CODES.NOT_FOUND;

function requireNum(v: unknown, label: string): number {
  const n = Number(v);
  if (!Number.isFinite(n) || n <= 0)
    throw new Error(`${label}：无有效数值，raw=${JSON.stringify(v)}`);
  return n;
}

/** 部门列表恒 PaginatedResponse{items,...}（define_crud_handlers → service::list，
 *  与 helpers.ts:469 注释同源）；单形状直读，漂移即红 */
async function listDepartments(page: Page): Promise<Record<string, unknown>[]> {
  const res = await apiCallRaw<unknown>(page, 'GET', '/departments?page=1&page_size=100');
  return pickListArray<Record<string, unknown>>(res, 'items', 'F30 /departments 列表');
}

/** 部门树恒裸数组 Vec<DepartmentTreeNode>（department_handler.rs:98-105） */
async function departmentTree(page: Page): Promise<Record<string, unknown>[]> {
  const res = await apiCallRaw<unknown>(page, 'GET', '/departments/tree');
  return pickListArray<Record<string, unknown>>(res, 'bare', 'F30 /departments/tree');
}

async function seedDepartment(page: Page, nameTag: string, parentId?: number): Promise<number> {
  const body: Record<string, unknown> = {
    name: `E2F30部门${nameTag}${Date.now().toString().slice(-6)}`,
  };
  if (parentId !== undefined) body.parent_id = parentId;
  const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/departments', body);
  const id = requireNum(created.id, '建部门');
  CLEANUP.push({ path: `/departments/${id}`, label: `department#${id}` });
  return id;
}

test.describe('30 系统配置契约链', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await ensureTestEntities(page);
  });

  test('30-01 部门父子链：服务端补码 DEPT_ 前缀 + 树端点真实嵌套；重名 400 / 幽灵父级 404', async ({
    page,
  }) => {
    // 缺 code 创建 → 后端自动生成 DEPT_{ts}（department_service.rs:174-178）
    const parent = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/departments', {
      name: `E2F30父${genCode('P')}`,
    });
    const parentId = requireNum(parent.id, '建父部门');
    CLEANUP.push({ path: `/departments/${parentId}`, label: `department(父)#${parentId}` });
    expect(String(parent.code ?? ''), '缺 code 时服务端补码 DEPT_ 前缀').toMatch(/^DEPT_/);
    expect(parent.is_active, '新部门默认启用（:185）').toBe(true);

    const childId = await seedDepartment(page, '子', parentId);
    const child = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/departments/${childId}`);
    expect(Number(child.parent_id), '父子外键真实落库').toBe(parentId);

    // 树端点：父节点 children 中真实含子部门 id（department_service.rs:387-431 构建）
    const tree = await departmentTree(page);
    const parentNode = tree.find(n => Number(n.id) === parentId);
    expect(parentNode, '树中应含父部门').toBeTruthy();
    const children = Array.isArray(parentNode!.children)
      ? (parentNode!.children as Record<string, unknown>[])
      : [];
    expect(
      children.some(c => Number(c.id) === childId),
      `父节点 children 应含 ${childId}，实际=${JSON.stringify(children).slice(0, 200)}`
    ).toBe(true);

    // 负例 1：名称重复 → 400 BUSINESS（:147-158，business() 脱敏文案，只断码）
    const dup = await apiCallExpectFail(page, 'POST', '/departments', {
      name: String(parent.name),
    });
    expect(dup.status, '部门重名应 400').toBe(400);
    expect(failureCode(dup), '重名机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 负例 2：parent_id 不存在 → 404（:160-167）
    const ghost = await apiCallExpectFail(page, 'POST', '/departments', {
      name: `E2F30幽灵父${genCode('G')}`,
      parent_id: 99999999,
    });
    expect(ghost.status, '不存在父部门应 404').toBe(404);
    expect(failureCode(ghost), '父级引用机器码').toBe(ERR_NOT_FOUND);

    // 列表端点亦真实包含两行（不是"树里假装有"）
    const rows = await listDepartments(page);
    expect(
      rows.some(r => Number(r.id) === parentId && Number(r.parent_id ?? 0) === 0),
      '列表含父'
    ).toBe(true);
    expect(
      rows.some(r => Number(r.id) === childId),
      '列表含子'
    ).toBe(true);
  });

  test('30-02 部门三态更新：NOT NULL 列显式 null 当场 400、可空列显式 null 真置 NULL、改重复 code 400', async ({
    page,
  }) => {
    const deptId = await seedDepartment(page, '三态');
    await apiCall(page, 'PUT', `/departments/${deptId}`, { description: '初始描述' });

    // 键缺席=保持：只改 sort_order，description 不被冲掉
    await apiCall(page, 'PUT', `/departments/${deptId}`, { sort_order: 7 });
    let back = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/departments/${deptId}`);
    expect(back.description, '键缺席不更新（三态语义）').toBe('初始描述');
    expect(Number(back.sort_order), 'sort_order 覆盖').toBe(7);

    // NOT NULL 列显式 null → 400 BUSINESS（department_service.rs:231-249，displayable 文案直传）
    const nullName = await apiCallExpectFail(page, 'PUT', `/departments/${deptId}`, {
      name: null,
    });
    expect(nullName.status, 'name 显式 null 应 400').toBe(400);
    expect(failureCode(nullName), 'NOT NULL 清空机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const nullActive = await apiCallExpectFail(page, 'PUT', `/departments/${deptId}`, {
      is_active: null,
    });
    expect(nullActive.status, 'is_active 显式 null 应 400').toBe(400);
    expect(failureCode(nullActive), 'NOT NULL 清空机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    // 被拒后零漂移
    back = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/departments/${deptId}`);
    expect(String(back.name).startsWith('E2F30部门三态'), '被拒后 name 保持').toBe(true);
    expect(back.is_active, '被拒后 is_active 保持 true').toBe(true);

    // 可空列显式 null → 真置 NULL（:300-303）
    await apiCall(page, 'PUT', `/departments/${deptId}`, { description: null });
    back = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/departments/${deptId}`);
    expect(back.description ?? null, 'description 显式 null 应真实置 NULL').toBeNull();

    // 改 code 撞重复 → 400 BUSINESS（:282-298）
    const otherCode = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/departments/${await seedDepartment(page, '撞码')}`
    );
    const clash = await apiCallExpectFail(page, 'PUT', `/departments/${deptId}`, {
      code: String(otherCode.code),
    });
    expectBusinessRejection(clash, '部门 code 撞重复应 400 业务拒绝');
  });

  test('30-03 部门删除守卫：子部门占位/用户挂靠均 400 业务拒绝；解除引用后删除成功、回读 404', async ({
    page,
  }) => {
    const parentId = await seedDepartment(page, '守父');
    const childId = await seedDepartment(page, '守子', parentId);

    // 有子 → BUSINESS（department_service.rs:354-363）
    const blockedByChild = await apiCallExpectFail(page, 'DELETE', `/departments/${parentId}`);
    expectBusinessRejection(blockedByChild, `父部门 ${parentId} 有子部门 ${childId}，删除应 400`);
    const still = await apiCallRaw<Record<string, unknown>>(
      page,
      'GET',
      `/departments/${parentId}`
    );
    expect(Number(still.id), '被拒后父部门仍在').toBe(parentId);

    // 用户挂靠 → BUSINESS（:365-376）。用户软删除后 department_id 仍在（user_service.rs:470-480
    // 只翻 is_active），守卫按行计数不放行——本钉揭示"删了用户却删不掉部门"是既有契约而非 bug。
    const user = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/users', {
      username: `f30u${Date.now().toString().slice(-6)}`,
      password: `E30p!${Date.now().toString().slice(-6)}Qz`,
      department_id: parentId,
    });
    const userId = requireNum(user.id, '建用户');
    CLEANUP.push({ path: `/users/${userId}`, label: `user#${userId}` });
    await apiCall(page, 'DELETE', `/users/${userId}`); // 先软删，验证软删用户仍占用
    const blockedByUser = await apiCallExpectFail(page, 'DELETE', `/departments/${parentId}`);
    expectBusinessRejection(blockedByUser, `部门 ${parentId} 有（软删）用户挂靠，删除应 400`);

    // 无引用路径：子部门可直接删 → 删成功后父部门仍被软删用户挡（现场不变），
    // 而另一个全新部门删子后删除成功 → GET 404（守卫可解除的正向半支）
    const p2 = await seedDepartment(page, '守父2');
    const c2 = await seedDepartment(page, '守子2', p2);
    await apiCall(page, 'DELETE', `/departments/${c2}`);
    await apiCall(page, 'DELETE', `/departments/${p2}`);
    const gone = await apiCallExpectFail(page, 'GET', `/departments/${p2}`);
    expect(gone.status, '解除引用删除后应 404').toBe(404);
    expect(failureCode(gone), '404 机器码').toBe(ERR_NOT_FOUND);
  });

  test('30-04 用户契约：重名 400 BUSINESS、弱密码/坏邮箱 400 VALIDATION、软删除 is_active=false 记录仍在', async ({
    page,
  }) => {
    const ts = Date.now().toString().slice(-6);
    const username = `f30dup${ts}`;
    const created = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/users', {
      username,
      password: `E30ok!${ts}Zq`,
      phone: '13800000304',
    });
    const userId = requireNum(created.id, '建用户');
    CLEANUP.push({ path: `/users/${userId}`, label: `user#${userId}` });
    expect(created.username, '用户名回读').toBe(username);
    expect(created.is_active, '默认激活').toBe(true);
    expect(
      created.password ?? undefined,
      '出参绝不携带密码哈希（UserResponse 无该键）'
    ).toBeUndefined();

    // 重名 → 业务族（user_service.rs:131-144）
    // 密码必须是「强度合规」的强密码（含 ASCII 小写——password_validator 的 require_lowercase，
    // 且大小写+数字+特殊齐全），否则请求会先被强度校验门以 VALIDATION_ERROR 拒，根本到不了
    // 唯一性门（BUSINESS），本负例就验不到重名语义。旧值缺 ASCII 小写字母
    // （别的 为中文、W 大写、余皆数字）即卡此门——形状对齐首个建单 E30ok!${ts}Zq（line 238）。
    const dup = await apiCallExpectFail(page, 'POST', '/users', {
      username,
      password: `E30dup!${ts}Zq`,
    });
    expect(dup.status, '重复用户名应 400').toBe(400);
    expect(failureCode(dup), '重名机器码').toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 弱密码（<8 位）→ 取值族（user_handler.rs:44-72 custom validator）
    const weak = await apiCallExpectFail(page, 'POST', '/users', {
      username: `f30weak${ts}`,
      password: 'abc123',
    });
    expect(weak.status, '弱密码应 400').toBe(400);
    expect(failureCode(weak), '弱密码机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 坏邮箱 → 取值族（CreateUserRequest #[validate(email)] :81）
    const badEmail = await apiCallExpectFail(page, 'POST', '/users', {
      username: `f30mail${ts}`,
      password: `E30mail!${ts}Qz`,
      email: 'not-an-email',
    });
    expect(badEmail.status, '坏邮箱应 400').toBe(400);
    expect(failureCode(badEmail), '坏邮箱机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 软删除契约（user_service.rs:470-480）：DELETE 后记录仍在、is_active=false
    await apiCall(page, 'DELETE', `/users/${userId}`);
    const after = await apiCallRaw<Record<string, unknown>>(page, 'GET', `/users/${userId}`);
    expect(Number(after.id), '软删除后记录仍可查').toBe(userId);
    expect(after.is_active, '软删除写 is_active=false').toBe(false);
  });

  test('30-05 单据号查重：真实号 true / 未占用 false / 空号与未登记类型 400 BAD_REQUEST', async ({
    page,
  }) => {
    // 用服务端取号生成的真实线索号做"已占用"样本（不自造号段）
    const lead = await apiCallRaw<Record<string, unknown>>(page, 'POST', '/crm/leads', {
      company_name: `E2F30查重${Date.now().toString().slice(-6)}`,
      contact_name: '查重样本',
    });
    const leadId = requireNum(lead.id, '建查重线索');
    CLEANUP.push({ path: `/crm/leads/${leadId}`, label: `crm_lead(查重)#${leadId}` });
    const leadNo = String(lead.lead_no);
    expect(leadNo, '样本单号存在').toMatch(/^LD/);

    const taken = await apiCallRaw<boolean>(
      page,
      'GET',
      `/document-no/check?doc_type=crm_lead&no=${encodeURIComponent(leadNo)}`
    );
    expect(taken, '真实已占用单号应返回 true（document_no_handler.rs:27-41）').toBe(true);

    const free = await apiCallRaw<boolean>(
      page,
      'GET',
      `/document-no/check?doc_type=crm_lead&no=LD-UNUSED-${encodeURIComponent(leadNo)}-X`
    );
    expect(free, '未占用单号应返回 false').toBe(false);

    // 空白单号 no=%20：normalize_empty_query_params 在 handler 之前按 trim 剔除空值 query 键
    // （query_params 第 25-38、53-74 行），CheckDocNoQuery.no 随之缺失，走 Query 反序列化——本仓既定 IR
    // 「字段/缺失校验 = VALIDATION_ERROR」（error 第 358、794 行，HTTP 仍 400）。旧用例误设 BAD_REQUEST，
    // 以为命中 handler :34-36 的空号分支，但剥键后该分支不可达。纯空白是否应改判 BAD_REQUEST 属后端
    // 契约待判，本任务只按 IR 对齐判据、不动后端。
    const empty = await apiCallExpectFail(
      page,
      'GET',
      `/document-no/check?doc_type=crm_lead&no=%20`
    );
    expect(empty.status, '空单号应 400').toBe(400);
    expect(failureCode(empty), '空单号机器码').toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 未登记 doc_type → 400 BAD_REQUEST（number_generator 未登记兜底分支）
    const unknown = await apiCallExpectFail(
      page,
      'GET',
      `/document-no/check?doc_type=no_such_doc_type&no=X1`
    );
    expect(unknown.status, '未登记类型应 400').toBe(400);
    expect(failureCode(unknown), '未登记类型机器码').toBe(APP_ERROR_CODES.BAD_REQUEST);
  });
});
