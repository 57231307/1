import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
  loginViaUI,
} from './helpers';

/**
 * 119 权限委托域全生命周期真值链（建委托 双视角列表 有效权限聚合 委托校验 撤销门 过期扫描）
 *
 * 与既有覆盖的分工。permission-delegations 是路由快照在册、e2e 字面量零命中的整域
 * 缺口：集合 建委托 详情读 委托者与被委托者双视角列表 有效权限聚合 权限检查
 * 撤销 过期扫描 七枚端点全部无覆盖。本 spec 把七枚钉成写后读回链，
 * 并把四道业务门 解码门 外键门 状态门的拒绝钉到 HTTP 与机器码两个口径。
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/permission_delegation.rs，静态段先于参数段注册；
 * 前端调用方是 frontend/src/api/system-governance.ts。handler 在
 * backend/src/handlers/permission_delegation_handler.rs，业务规则与落库在
 * backend/src/services/permission_delegation_service.rs，
 * 行落 permission_delegations 表（BIGSERIAL 主键，委托人与被委托人外键指向 users，
 * 状态列 CHECK 词表 pending/active/expired/revoked，另有 chk_no_self_delegation
 * 与 chk_valid_until_after_from 两条数据库级CHECK）。
 *
 * 建委托 POST permission-delegations 入参七键，必填 delegator_id delegatee_id
 * permission_code valid_from valid_until，选填 is_chain_allowed reason。
 * 业务门按固定顺序执行且全部走脱敏 business 出参（HTTP 400 加 BUSINESS_ERROR，
 * 文案不外显）：委托人与被同一人即拒；失效时间不晚于生效时间即拒；生效时间早于
 * 服务端当前时间五分钟容差即拒（防回溯委托）；委托时长超过九十天即拒。
 * 服务层注释宣称的"委托人必须拥有该权限"当前实现未做校验，本 spec 不为其编造
 * 断言；该缺门属现状登记，判责与登记归调度，不在用例里表态"应当放行"。
 * 服务端写死初值 status 为小写 active、is_chain_allowed 缺省补 false、
 * revoked_at revoked_by revoke_reason 三列置空、created_by 取会话用户而非请求体。
 * 委托人不存在的提交会穿过全部业务门撞数据库外键，收口为 500 加 DATABASE_ERROR，
 * 这是该域引用完整性的真实通道，如实钉出。
 *
 * 列表 GET permission-delegations 出参 data 为整行数组（无分页信封），
 * 按 created_at 倒序。user_id 与 as_delegator 组合口径：as_delegator 为 true 时
 * 按委托者列过滤，否则按被委托者列过滤；user_id 缺省不过滤。
 * 详情 GET 按主键取行，主键不存在 404 加 NOT_FOUND。
 *
 * 有效权限聚合 GET active 出参 data 为权限码字符串数组，判据三件同时成立：
 * 状态等于 active、生效时间不晚于当前、失效时间晚于当前。状态等于 active 但
 * 已过失效时间的行不出现在聚合里（读门与状态列分离，这正是过期扫描要存在的理由）。
 * 权限检查 GET check 出参 data 为布尔，判据与聚合同一三件组合。
 *
 * 撤销 POST 对象段 revoke 入参 revoke_reason 选填。门只有两道：主键不存在 404
 * 加 NOT_FOUND；当前状态不是 active 也不是 pending 即 400 加 BUSINESS_ERROR。
 * 服务层注释宣称的"仅委托人或管理员可撤销"当前实现未做操作人身份校验，
 * 会话用户即可撤销。该缺门属现状登记而非契约认可，判责与登记归调度；
 * 本 spec 只钉当前真实可达的行为，不反向锁死，后端补门后相关步骤转红
 * 即预期内的返工信号。
 * 成功撤销写 status 为 revoked、revoked_at 为服务端当前时间、revoked_by 为会话用户、
 * revoke_reason 为提交值，其余列不动。
 *
 * 过期扫描 POST expire-overdue 无入参，把所有"状态 active 且失效时间早于当前"的
 * 行整批改写为 expired 并返回被改写的行数。返回数是全库聚合口径，多会话并发时
 * 本会话无法钉确切值，只钉整数形态；真值判据放在自建行的状态回读上：扫描后
 * 该行状态为 expired 且撤销三列保持空（过期路径不写撤销列），已 expired 的行
 * 不可再被撤销（撞状态门）且二次扫描不回滚状态。
 *
 * 假绿防线。每条写操作都紧跟按主键的独立读回逐列对表；四类业务门与解码门
 * 与外键门一律钉 HTTP 与机器码，拒绝后按被委托者视角回读列表证明专属权限码
 * 零出现；撤销与过期两个终态动作互钉对方入口的拒绝。权限码逐用例专属生成，
 * 域内没有删除端点，撤销与过期即本域数据收口的终态。全程不用 ?? 或 || 兜底，
 * 后端必出的键缺失即形漂判红；造数失败当场红并点名步骤。
 */

type Row = Record<string, unknown>;

/** 失败机器码：DATABASE_ERROR 是提取器与约束违例统一信封的数据库族通道，
 *  helpers 的常量集未收录，此处按后端出参原文局部声明，不改动共享夹具 */
const DATABASE_ERROR_CODE = 'DATABASE_ERROR';

const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 时间列只断非空且可解析为时刻，比对提交值时用 epoch 毫秒而非字符串原样 */
function assertParseableTime(value: unknown, label: string): number {
  expect(
    typeof value === 'string' && value.length > 0,
    `${label} 必须是非空字符串，实际 ${JSON.stringify(value)}`
  ).toBe(true);
  const parsed = Date.parse(String(value));
  expect(Number.isNaN(parsed), `${label} 必须能被解析为时间，实际 ${JSON.stringify(value)}`).toBe(
    false
  );
  return parsed;
}

/** 会话用户主键：created_by revoked_by 等身份列的真值来源，取不到当场红 */
async function selfUserId(page: Page): Promise<number> {
  const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
  const id = num(me.id);
  expect(Number.isInteger(id) && id > 0, `119 会话用户主键缺失，实际 ${JSON.stringify(me)}`).toBe(
    true
  );
  return id;
}

/** 从用户列表取一个不等于会话用户的主键作委托人（外键要求真实用户），取不到当场红 */
async function otherUserId(page: Page, selfId: number): Promise<number> {
  const list = await apiCallRaw<Row>(page, 'GET', '/users?page=1&page_size=50');
  expect(
    Array.isArray(list.users),
    `119 用户列表必须出 users 数组，实际 ${JSON.stringify(list)}`
  ).toBe(true);
  const users = list.users as Row[];
  const other = users.find(u => num(u.id) !== selfId);
  expect(
    other !== undefined,
    `119 用户列表里找不到第二个用户可当委托人，实际 ${JSON.stringify(users.map(u => u.id))}`
  ).toBe(true);
  return num(other!.id);
}

type DelegationBody = {
  delegator_id: number;
  delegatee_id: number;
  permission_code: string;
  valid_from: string;
  valid_until: string;
  is_chain_allowed?: boolean;
  reason?: string;
};

/** 自建本 worker 专属委托，返回整行；造数失败当场红并点名是建委托这一步 */
async function createDelegation(page: Page, body: DelegationBody): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/permission-delegations', body);
  const id = num(created.id);
  expect(
    Number.isInteger(id) && id > 0,
    `119 建委托造数失败，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

/** 以被委托者视角回读列表，返回权限码到状态映射前的原始行数组 */
async function listAsDelegatee(page: Page, userId: number): Promise<Row[]> {
  const list = await apiCallRaw<Row[]>(
    page,
    'GET',
    `/permission-delegations?user_id=${userId}&as_delegator=false`
  );
  expect(Array.isArray(list), `被委托者视角列表必须出数组，实际 ${JSON.stringify(list)}`).toBe(
    true
  );
  return list;
}

test.describe.serial('119 权限委托域全生命周期真值链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 建委托服务端初值逐列回显 详情与双视角列表互证 聚合与检查正向放行', async ({ page }) => {
    const selfId = await selfUserId(page);
    const delegatorId = await otherUserId(page, selfId);
    const code = genCode('E119A');
    const fromMs = Date.now() - 60_000;
    const untilMs = Date.now() + 86_400_000;
    const created = await createDelegation(page, {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: code,
      valid_from: new Date(fromMs).toISOString(),
      valid_until: new Date(untilMs).toISOString(),
      reason: `119建委托${code}`,
    });
    const delegationId = num(created.id);

    // 服务端写死的初值与身份列来源
    expect(str(created.status), '建行状态必须为小写 active').toBe('active');
    expect(created.is_chain_allowed, '缺省提交时链式委托位必须补 false').toBe(false);
    expect(num(created.created_by), '创建人必须是会话用户而不是请求体携带').toBe(selfId);
    expect(created.revoked_at, '建行撤销时间必须为空').toBeNull();
    expect(created.revoked_by, '建行撤销人必须为空').toBeNull();
    expect(created.revoke_reason, '建行撤销原因必须为空').toBeNull();
    expect(num(created.delegator_id), '委托人必须回显提交值').toBe(delegatorId);
    expect(num(created.delegatee_id), '被委托人必须回显提交值').toBe(selfId);
    expect(str(created.permission_code), '权限码必须回显提交值').toBe(code);
    expect(str(created.reason), '委托原因必须回显提交值').toBe(`119建委托${code}`);
    expect(
      assertParseableTime(created.valid_from, '生效时间'),
      '生效时间 epoch 必须等于提交值'
    ).toBe(fromMs);
    expect(
      assertParseableTime(created.valid_until, '失效时间'),
      '失效时间 epoch 必须等于提交值'
    ).toBe(untilMs);

    // 独立读口互证，防"响应回显但没落库"
    const detail = await apiCallRaw<Row>(page, 'GET', `/permission-delegations/${delegationId}`);
    expect(num(detail.id), '详情主键必须等于建行主键').toBe(delegationId);
    expect(str(detail.status), '详情状态必须为 active').toBe('active');
    expect(str(detail.permission_code), '详情权限码必须等于提交值').toBe(code);
    expect(num(detail.created_by), '详情创建人必须是会话用户').toBe(selfId);

    // 双视角列表：被委托者视角与委托者视角都必须包含这一行
    const asDelegatee = await listAsDelegatee(page, selfId);
    const mine1 = asDelegatee.find(r => num(r.id) === delegationId);
    expect(mine1 !== undefined, `被委托者视角列表必须包含自建行 ${delegationId}`).toBe(true);
    expect(num(mine1!.delegatee_id), '被委托者视角每行被委托列都必须等于过滤值').toBe(selfId);
    const asDelegator = await apiCallRaw<Row[]>(
      page,
      'GET',
      `/permission-delegations?user_id=${delegatorId}&as_delegator=true`
    );
    expect(Array.isArray(asDelegator), '委托者视角列表必须出数组').toBe(true);
    const mine2 = (asDelegator as Row[]).find(r => num(r.id) === delegationId);
    expect(mine2 !== undefined, `委托者视角列表必须包含自建行 ${delegationId}`).toBe(true);
    expect(num(mine2!.delegator_id), '委托者视角命中行委托列必须等于过滤值').toBe(delegatorId);

    // 有效权限聚合与检查：三件判据同时成立即正向放行
    const active = await apiCallRaw<unknown[]>(
      page,
      'GET',
      `/permission-delegations/active/${selfId}`
    );
    expect(Array.isArray(active), '有效权限聚合必须出数组').toBe(true);
    expect((active as string[]).includes(code), `聚合必须包含自建权限码 ${code}`).toBe(true);
    const checkOn = await apiCallRaw<boolean>(
      page,
      'GET',
      `/permission-delegations/check?delegatee_id=${selfId}&permission_code=${encodeURIComponent(code)}`
    );
    expect(checkOn, '有效期内 active 委托的检查必须为 true').toBe(true);
    const checkOff = await apiCallRaw<boolean>(
      page,
      'GET',
      `/permission-delegations/check?delegatee_id=${selfId}&permission_code=${genCode('E119NONE')}`
    );
    expect(checkOff, '从未委托过的权限码检查必须为 false').toBe(false);

    // 收口：本域没有删除端点，自建行以撤销终态离场，不留可生效的悬挂委托
    await apiCall<unknown>(page, 'POST', `/permission-delegations/${delegationId}/revoke`, {
      revoke_reason: '119建委托读链用例收口撤销',
    });
    const closed = await apiCallRaw<Row>(page, 'GET', `/permission-delegations/${delegationId}`);
    expect(str(closed.status), '收口撤销后状态必须为 revoked').toBe('revoked');
  });

  test('T2 四道业务门与解码门外键门全拒 专属权限码零出现', async ({ page }) => {
    const selfId = await selfUserId(page);
    const delegatorId = await otherUserId(page, selfId);
    const now = Date.now();

    // 一 委托人与被委托人同一人：business 脱敏门
    const samePerson = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: selfId,
      delegatee_id: selfId,
      permission_code: genCode('E119B1'),
      valid_from: new Date(now).toISOString(),
      valid_until: new Date(now + 86_400_000).toISOString(),
    });
    expect(samePerson.status, `同一人委托必须 400，实际 ${samePerson.status}`).toBe(400);
    expect(
      failureCode(samePerson),
      `同一人委托必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(samePerson.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 二 失效不晚于生效：同样走 business 门（数据库 CHECK 之外的服务层前置）
    const badRange = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: genCode('E119B2'),
      valid_from: new Date(now).toISOString(),
      valid_until: new Date(now).toISOString(),
    });
    expect(badRange.status, `失效等于生效必须 400，实际 ${badRange.status}`).toBe(400);
    expect(
      failureCode(badRange),
      `时段非法必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(badRange.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 三 回溯委托：生效时间早于服务端当前五分钟容差
    const backdated = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: genCode('E119B3'),
      valid_from: new Date(now - 600_000).toISOString(),
      valid_until: new Date(now + 86_400_000).toISOString(),
    });
    expect(backdated.status, `回溯委托必须 400，实际 ${backdated.status}`).toBe(400);
    expect(
      failureCode(backdated),
      `回溯委托必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(backdated.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 四 时长超九十天上限
    const tooLong = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: genCode('E119B4'),
      valid_from: new Date(now).toISOString(),
      valid_until: new Date(now + 91 * 86_400_000).toISOString(),
    });
    expect(tooLong.status, `九十天以上委托必须 400，实际 ${tooLong.status}`).toBe(400);
    expect(
      failureCode(tooLong),
      `时长越界必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(tooLong.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 五 必填键缺失：权限码不落即解码门，400 加 VALIDATION_ERROR
    const missingKey = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      valid_from: new Date(now).toISOString(),
      valid_until: new Date(now + 86_400_000).toISOString(),
    });
    expect(missingKey.status, `缺权限码必须 400，实际 ${missingKey.status}`).toBe(400);
    expect(
      failureCode(missingKey),
      `缺必填键必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(missingKey.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 六 委托人外键缺行：穿过业务门后撞数据库外键，收口为 500 加 DATABASE_ERROR
    const fkMissing = await apiCallExpectFail(page, 'POST', '/permission-delegations', {
      delegator_id: 999999999,
      delegatee_id: selfId,
      permission_code: genCode('E119B5'),
      valid_from: new Date(now).toISOString(),
      valid_until: new Date(now + 86_400_000).toISOString(),
    });
    expect(
      fkMissing.status,
      `委托人不存在的提交必须走外键通道 500，实际 ${fkMissing.status} code=${JSON.stringify(
        fkMissing.code
      )}`
    ).toBe(500);
    expect(
      failureCode(fkMissing),
      `外键违例必须归因 DATABASE_ERROR，实际 ${JSON.stringify(fkMissing.code)}`
    ).toBe(DATABASE_ERROR_CODE);

    // 七 检查端点必填查询参数缺失：提取器拒绝收口为 400 加 VALIDATION_ERROR
    const badQuery = await apiCallExpectFail(
      page,
      'GET',
      `/permission-delegations/check?permission_code=${encodeURIComponent(genCode('E119B6'))}`
    );
    expect(badQuery.status, `检查缺 delegatee_id 必须 400，实际 ${badQuery.status}`).toBe(400);
    expect(
      failureCode(badQuery),
      `查询参数缺失必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badQuery.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 零推进总证：七次被拒的提交都不许在被委托者视角留下专属权限码的行
    const rows = await listAsDelegatee(page, selfId);
    const leaked = rows.filter(r => str(r.permission_code).startsWith('E119B'));
    expect(leaked, `被拒提交不许建行，实际混入 ${JSON.stringify(leaked)}`).toHaveLength(0);

    // 详情读口对不存在主键的归因通道：404 加 NOT_FOUND
    const detailMissing = await apiCallExpectFail(page, 'GET', '/permission-delegations/99999999');
    expect(detailMissing.status, `不存在主键的详情必须 404，实际 ${detailMissing.status}`).toBe(
      404
    );
    expect(failureCode(detailMissing), '不存在主键的详情必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('T3 撤销写终态列 聚合与检查即刻翻面 重复撤销撞状态门 零漂移', async ({ page }) => {
    const selfId = await selfUserId(page);
    const delegatorId = await otherUserId(page, selfId);
    const code = genCode('E119C');
    const now = Date.now();
    const created = await createDelegation(page, {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: code,
      valid_from: new Date(now - 60_000).toISOString(),
      valid_until: new Date(now + 86_400_000).toISOString(),
      is_chain_allowed: true,
    });
    const delegationId = num(created.id);
    expect(created.is_chain_allowed, '显式提交 true 时链式委托位必须回显 true').toBe(true);

    // 撤销成功：出参数据载荷为空，真值判据全部放在按主键回读
    const revoked = await apiCall<unknown>(
      page,
      'POST',
      `/permission-delegations/${delegationId}/revoke`,
      { revoke_reason: `119撤销${code}` }
    );
    expect(
      revoked.data === null,
      `撤销成功不出业务载荷，实际 ${JSON.stringify(revoked.data)}`
    ).toBe(true);
    const after = await apiCallRaw<Row>(page, 'GET', `/permission-delegations/${delegationId}`);
    expect(str(after.status), '撤销后状态必须为 revoked').toBe('revoked');
    expect(num(after.revoked_by), '撤销人必须是会话用户').toBe(selfId);
    expect(str(after.revoke_reason), '撤销原因必须回显提交值').toBe(`119撤销${code}`);
    assertParseableTime(after.revoked_at, '撤销时间');
    // 撤销只动终态列：身份列与权限码与时段列原样
    expect(num(after.delegatee_id), '撤销不许改动被委托人列').toBe(selfId);
    expect(str(after.permission_code), '撤销不许改动权限码列').toBe(code);
    expect(
      assertParseableTime(after.valid_until, '撤销后的失效时间'),
      '失效时间 epoch 必须原样'
    ).toBe(now + 86_400_000);

    // 聚合与检查即刻翻面
    const active = await apiCallRaw<unknown[]>(
      page,
      'GET',
      `/permission-delegations/active/${selfId}`
    );
    expect((active as string[]).includes(code), `撤销后聚合必须不再包含 ${code}`).toBe(false);
    const checkAfter = await apiCallRaw<boolean>(
      page,
      'GET',
      `/permission-delegations/check?delegatee_id=${selfId}&permission_code=${encodeURIComponent(code)}`
    );
    expect(checkAfter, '撤销后检查必须为 false').toBe(false);

    // 重复撤销撞状态门：revoked 不在 active/pending 白名单
    const revokeAgain = await apiCallExpectFail(
      page,
      'POST',
      `/permission-delegations/${delegationId}/revoke`,
      { revoke_reason: '二次撤销' }
    );
    expect(revokeAgain.status, `重复撤销必须 400，实际 ${revokeAgain.status}`).toBe(400);
    expect(
      failureCode(revokeAgain),
      `重复撤销必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(revokeAgain.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterDouble = await apiCallRaw<Row>(
      page,
      'GET',
      `/permission-delegations/${delegationId}`
    );
    expect(str(afterDouble.status), '被拒的二次撤销不许改动状态').toBe('revoked');
    expect(
      assertParseableTime(afterDouble.revoked_at, '被拒二次撤销后的撤销时间'),
      '被拒的二次撤销不许改动撤销时间'
    ).toBe(assertParseableTime(after.revoked_at, '首次撤销时间'));
    expect(str(afterDouble.revoke_reason), '被拒的二次撤销不许改写撤销原因').toBe(`119撤销${code}`);

    // 不存在主键的撤销：404 加 NOT_FOUND
    const revokeMissing = await apiCallExpectFail(
      page,
      'POST',
      '/permission-delegations/99999999/revoke',
      {}
    );
    expect(revokeMissing.status, `不存在主键的撤销必须 404，实际 ${revokeMissing.status}`).toBe(
      404
    );
    expect(failureCode(revokeMissing), '不存在主键的撤销必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
  });

  test('T4 过期行读门先行拦截 扫描改写终态 过期不写撤销列且不可再撤销', async ({ page }) => {
    const selfId = await selfUserId(page);
    const delegatorId = await otherUserId(page, selfId);
    const code = genCode('E119D');
    const now = Date.now();
    // 生效取五分钟容差内、失效取过去的窗口：行以 active 落库但已过失效时间，
    // 这是本域唯一能从 API 造出"active 且过期"行的合法通道
    const created = await createDelegation(page, {
      delegator_id: delegatorId,
      delegatee_id: selfId,
      permission_code: code,
      valid_from: new Date(now - 120_000).toISOString(),
      valid_until: new Date(now - 60_000).toISOString(),
      reason: `119过期扫描${code}`,
    });
    const delegationId = num(created.id);
    expect(str(created.status), '过期窗口建行初值仍为 active').toBe('active');

    // 读门先行：状态 active 但已过失效时间的行不进聚合、检查为 false
    const activeBefore = await apiCallRaw<unknown[]>(
      page,
      'GET',
      `/permission-delegations/active/${selfId}`
    );
    expect(
      (activeBefore as string[]).includes(code),
      `未扫描的过期 active 行不许进聚合，实际 ${JSON.stringify(activeBefore)}`
    ).toBe(false);
    const checkBefore = await apiCallRaw<boolean>(
      page,
      'GET',
      `/permission-delegations/check?delegatee_id=${selfId}&permission_code=${encodeURIComponent(code)}`
    );
    expect(checkBefore, '未扫描的过期 active 行检查必须为 false').toBe(false);

    // 扫描返回数是全库聚合口径，只钉整数形态；真值判据在下面按主键回读
    const scan = await apiCallRaw<unknown>(page, 'POST', '/permission-delegations/expire-overdue');
    expect(
      Number.isInteger(num(scan)) && num(scan) >= 0,
      `过期扫描出参必须是非负整数，实际 ${JSON.stringify(scan)}`
    ).toBe(true);

    const afterScan = await apiCallRaw<Row>(page, 'GET', `/permission-delegations/${delegationId}`);
    expect(str(afterScan.status), '扫描后自建行状态必须为 expired').toBe('expired');
    expect(afterScan.revoked_at, '过期改写不许写撤销时间列').toBeNull();
    expect(afterScan.revoked_by, '过期改写不许写撤销人列').toBeNull();
    expect(afterScan.revoke_reason, '过期改写不许写撤销原因列').toBeNull();
    expect(str(afterScan.permission_code), '过期改写不许改动权限码列').toBe(code);

    // 终态互钉：expired 不在 active/pending 白名单，撤销入口对过期行撞状态门
    const revokeExpired = await apiCallExpectFail(
      page,
      'POST',
      `/permission-delegations/${delegationId}/revoke`,
      { revoke_reason: '过期行撤销' }
    );
    expect(revokeExpired.status, `过期行撤销必须 400，实际 ${revokeExpired.status}`).toBe(400);
    expect(
      failureCode(revokeExpired),
      `过期行撤销必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(revokeExpired.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const afterBadRevoke = await apiCallRaw<Row>(
      page,
      'GET',
      `/permission-delegations/${delegationId}`
    );
    expect(str(afterBadRevoke.status), '被拒的撤销不许把 expired 改回其它状态').toBe('expired');

    // 二次扫描幂等：已 expired 的行不被重复计数改写，状态不回滚
    const scanAgain = await apiCallRaw<unknown>(
      page,
      'POST',
      '/permission-delegations/expire-overdue'
    );
    expect(Number.isInteger(num(scanAgain)), '二次扫描出参仍为整数').toBe(true);
    const finalRow = await apiCallRaw<Row>(page, 'GET', `/permission-delegations/${delegationId}`);
    expect(str(finalRow.status), '二次扫描不许改动已过期行的状态').toBe('expired');
  });
});
