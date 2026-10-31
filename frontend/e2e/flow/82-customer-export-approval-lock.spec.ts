import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall } from './helpers';

/**
 * 敏感数据(customer)导出审批流端到端锁测 — flow/82
 *
 * 锁定的根因行为链(逐点对着后端源码核实,非编造):
 * - 创建申请:POST /export-approvals,敏感清单含 customer;新建落库 status=pending
 *   且 download_token 恒为 null(令牌只在最终 approve 时签发);
 *   export_params 落空对象,与下载腿实际携带的范围参数同形 — 客户导出端点真实读取的筛选
 *   维度只有 keyword/customer_type/status,分页参数不是它的范围维度,写进申请会让受理门
 *   的范围比对判成"批准里有而请求里没有的维度",与主体校验无关的 403 会掩盖本用例靶心。
 *   estimated_rows 低于 1000 命中 risk_level=low,approval_level=1,一次审批即终结。
 * - 审批权限门:能否审批(含申请人自批)由 RBAC 权限键 export-approvals:approve 决定,
 *   service 层不再对"审批人=申请人"做无条件硬拒;持键者自批同样 2xx 通过,
 *   并如实记录 approver_user_id(自批时 approver_user_id 等于 applicant_user_id)。
 * - 令牌一次性读取:低风险一级 approve 后出参 status=approved 但 download_token 恒为 null
 *   （服务端 detail_masked 屏蔽凭据原文），凭据由申请人本人 POST /export-approvals/{id}
 *   /rotate-token 单独取一次，形如 exp_ + 32 位小写十六进制；
 *   GET /export-approvals/verify-token 可复核同一记录。
 * - 带令牌下载:申请人本人会话 GET /crm/customers/export?download_token=... 必须 200,
 *   Content-Type 为 xlsx MIME,响应体以 ZIP 魔数 PK 开头且非空
 *   (CI 内唯一能真实验证"审批到令牌到下载"全通的证据面)。
 * - 主体门:令牌只能由被批准的申请人本人消费,审批人(非申请人)会话带同一枚新鲜令牌必 403。
 * - 一次性:同一条令牌在申请人会话二次消费必 403(download_count 达上限)。
 * - fail-closed:不带令牌直接下载 403;customer 令牌跨资源用于 products 导出 403。
 *
 * 会话主体口径:服务端导出受理门只放行令牌签发给的那位申请人,所以凭据取回与全部下载腿
 * 一律走申请人会话,不得出现审批人会话消费令牌。两个 403(主体门与一次性门)不按文案区分 —
 * PermissionDenied 出参文案被统一脱敏成同一个值,靠文案区分等于把后端脱敏加固当测试桩。
 * 判据用会话顺序做控制组:非申请人会话先打一条新鲜未消费令牌得 403,紧接着同一条令牌在
 * 申请人会话得 200,即证前一步的拒绝不是消费门,也不是数据或路由缺失。
 *
 * 角色安排:
 * - 申请人:manager 角色账号(loginAsRole,凭证由 global-setup 角色账号写入);
 *   该岗在产品与客户资源持通配授权,申请人会话能过 RBAC 真实到达 handler 的受理门;
 * - 他人审批:分片默认 admin 账号(持 export-approvals:approve,同时也是越权负例里的
 *   "非申请人"会话 — 超管过 RBAC,它的 403 只能来自主体门);
 * - 跨资源负例走申请人会话:申请人过得了目标端点 RBAC,403 才能唯一归因到令牌资源绑定;
 * - 无令牌与伪造令牌的 403 走 admin 会话:权限中间件对超管短路,
 *   因此这些腿的 403 必然来自 handler 的审批门,不会与权限层混淆;
 * - 自批正向用 admin 本人申请 + admin 本人审批:admin 持 export-approvals:approve,
 *   当前契约下自批应 2xx 通过、状态转 approved 并签发令牌,审批人如实记为申请人本人。
 *
 * 写请求统一走 helpers.apiCall(内置 CSRF 有界重放,CSRF 竞败不会被误读成业务拒绝,
 * 业务 4xx/5xx 会抛出真实原因);GET 为安全方法不消费 CSRF,下载直连 page.request。
 * 注释遵循受限字符集,不引用文件行号与批次历史。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** 导出审批记录(后端 Model 脱敏直出,snake_case 键名) */
interface ApprovalModel {
  id: number;
  status: string;
  resource_type: string;
  file_format: string;
  risk_level: string;
  approval_level: number;
  applicant_user_id: number;
  approver_user_id: number | null;
  download_token: string | null;
  download_count: number;
  max_downloads: number;
  token_consumed: boolean;
}

/** 统一失败信封(utils/error.rs ErrorResponse):code 字符串机器码 + trace_id */
interface FailureEnvelope {
  code?: unknown;
  message?: unknown;
  trace_id?: unknown;
  timestamp?: unknown;
}

/**
 * 低风险 customer 导出申请:manager 创建,返回申请 id。
 * export_params 落空对象,与本文件全部下载腿实际携带的范围参数同形(受理门逐项比对)。
 */
async function createCustomerApprovalAsManager(
  page: import('@playwright/test').Page
): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'customer',
    export_params: {},
    estimated_rows: 10,
    file_format: 'xlsx',
  });
  const created = res.data;
  expect(
    created?.id,
    `manager 创建 customer 导出审批申请应返回有效 id:${JSON.stringify(created).slice(0, 200)}`
  ).toBeGreaterThan(0);
  expect(created?.status, '创建成功后初始状态应为 pending').toBe('pending');
  expect(created?.resource_type, '申请应绑定 customer 资源类型').toBe('customer');
  return created.id;
}

/** admin 审批该申请并返回审批后的模型(含签发令牌) */
async function approveAsAdmin(
  page: import('@playwright/test').Page,
  approvalId: number,
  comments: string
): Promise<ApprovalModel> {
  await loginViaUI(page, undefined, undefined, true);
  const res = await apiCall<ApprovalModel>(
    page,
    'POST',
    `/export-approvals/${approvalId}/approve`,
    { comments }
  );
  const approved = res.data;
  expect(
    approved?.status,
    `低风险一级审批通过后状态应为 approved,实际:${JSON.stringify(approved).slice(0, 200)}`
  ).toBe('approved');
  return approved;
}

/**
 * 申请人本人重发并一次性取回下载令牌,返回后调用方会话停在申请人(manager)。
 * 调用方:本文件的正向全链用例、跨资源负例与越权负例,均在 admin 审批之后执行。
 * 入参:page 为当前页,approvalId 为已 approved 的申请 id。
 * 传给谁:返回的令牌交回调用方用于带令牌下载、二次消费、跨资源与越权四腿。
 * 判据源:服务端 rotate_token 只允许申请人本人调用(非申请人 403)且要求状态为 approved;
 * 出参 available 为真时 token 即一次性凭据原文,形如 exp_ + 32 位小写十六进制。
 */
async function rotateTokenAsApplicant(
  page: import('@playwright/test').Page,
  approvalId: number
): Promise<string> {
  await loginAsRole(page, 'manager');
  const rotated = await apiCall<{ available?: boolean; token?: string | null }>(
    page,
    'POST',
    `/export-approvals/${approvalId}/rotate-token`
  );
  expect(
    rotated.data?.available === true,
    `申请人重发令牌应 available,实际:${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  const token = rotated.data?.token ?? '';
  expect(
    token.length > 0,
    `重发应下发非空令牌原文,实际:${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  expect(token, `下发的令牌应为 exp_ 前缀加 32 位小写十六进制,实际 ${token}`).toMatch(
    /^exp_[0-9a-f]{32}$/
  );
  console.warn(`[82] manager 已重发申请 ${approvalId} 的一次性下载令牌`);
  return token;
}

/** 断言 403 响应为统一失败信封且 code=FORBIDDEN(禁止只判状态码的存在性) */
async function expectForbiddenEnvelope(
  resp: import('@playwright/test').APIResponse,
  scene: string
): Promise<void> {
  expect(resp.status(), `${scene}:应被拒绝且 HTTP 状态恰为 403`).toBe(403);
  let body: FailureEnvelope;
  try {
    body = (await resp.json()) as FailureEnvelope;
  } catch (e) {
    throw new Error(
      `${scene}:403 响应体不是统一 JSON 信封(${(e as Error).message}),拒绝面形状漂移判红`
    );
  }
  expect(body.code, `${scene}:失败信封 code 应为 FORBIDDEN,实际 ${JSON.stringify(body)}`).toBe(
    'FORBIDDEN'
  );
  expect(
    typeof body.trace_id === 'string' && body.trace_id.length > 0,
    `${scene}:失败信封应携带非空 trace_id(AppError 统一出参),实际 ${JSON.stringify(body)}`
  ).toBe(true);
}

test.describe('customer 敏感导出审批链锁测', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('正向全链:申请(pending 无令牌)至他人审批(签发 exp 令牌)至带令牌下载 200 xlsx 字节', async ({
    page,
  }) => {
    // 1. manager 创建申请,立即以申请人身份回读详情:pending 且无令牌
    const approvalId = await createCustomerApprovalAsManager(page);
    const detail = await apiCall<ApprovalModel>(page, 'GET', `/export-approvals/${approvalId}`);
    const row = detail.data;
    expect(row?.status, '回读初始状态应为 pending').toBe('pending');
    expect(
      row?.download_token,
      `pending 阶段不得下发 download_token,实际:${JSON.stringify(row).slice(0, 200)}`
    ).toBeNull();
    expect(row?.risk_level, 'estimated_rows=10 应评估为 low 风险').toBe('low');
    expect(row?.approval_level, 'low 风险对应一级审批').toBe(1);

    // 2. 切回 admin(非申请人)审批:approved + 签发符合格式的令牌,且审批人与申请人分离
    const approved = await approveAsAdmin(page, approvalId, 'E2E 导出审批链锁测');
    expect(
      approved?.approver_user_id,
      `审批后应落 approver_user_id,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).not.toBeNull();
    expect(approved.approver_user_id, '审批人与申请人必须为不同用户(严格分离)').not.toBe(
      approved.applicant_user_id
    );
    const maskedToken = approved.download_token;
    expect(
      maskedToken ?? null,
      `一次性读取契约下审批出参不应回显 download_token,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBeNull();
    // 凭据由申请人本人经 rotate-token 单独取一次(服务端只允许申请人轮换),取回后停在申请人会话
    const token = await rotateTokenAsApplicant(page, approvalId);
    // verify-token 是读审批记录、不消费凭据,留在 admin 会话(数据域 All,归属门必然放行);
    // 它不参与主体口径,主体口径只约束真正消费令牌的下载腿。
    await loginViaUI(page, undefined, undefined, true);

    // 3. verify-token 复核同一记录(admin 数据域为 All,归属门必然放行)
    const verified = await apiCall<ApprovalModel>(
      page,
      'GET',
      `/export-approvals/verify-token?token=${encodeURIComponent(String(token))}`
    );
    expect(verified.data?.id, 'verify-token 应命中同一申请记录').toBe(approvalId);
    expect(verified.data?.status, 'verify-token 对已签发令牌应返回 approved').toBe('approved');
    expect(verified.data?.resource_type, 'verify-token 应回显 customer 资源绑定').toBe('customer');

    // 4. 申请人本人会话带令牌下载:200 且响应体是真实 xlsx 二进制(x-zip 魔数 PK 开头,非空)
    await loginAsRole(page, 'manager');
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${encodeURIComponent(String(token))}`
    );
    expect(
      exportResp.status(),
      `申请人会话持有效令牌导出应 200,approvalId=${approvalId}、会话=manager、令牌已带上且非空` +
        `(长度 ${token.length});这一步之前 verify-token 已判该令牌 approved 且绑定 customer`
    ).toBe(200);
    const contentType = exportResp.headers()['content-type'] ?? '';
    expect(
      contentType,
      `导出响应 Content-Type 应为 xlsx MIME,实际 content-type=${contentType}`
    ).toContain('application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
    const bytes: Uint8Array = await exportResp.body();
    expect(bytes.byteLength, '导出文件内容必须非空').toBeGreaterThan(0);
    expect(bytes[0], 'xlsx 为 zip 容器,魔数首字节应为 P(0x50)').toBe(0x50);
    expect(bytes[1], 'xlsx 为 zip 容器,魔数次字节应为 K(0x4b)').toBe(0x4b);

    // 5. 一次性语义:同一条令牌在同一个申请人会话二次消费必 403(download_count 达上限)。
    // 与上一步唯一变的是"这条令牌已被消费过一次",主体/端点/参数完全相同。
    const replayResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${encodeURIComponent(String(token))}`
    );
    expect(
      replayResp.status(),
      `同一条令牌在同一个申请人会话二次消费应 403(一次性门),approvalId=${approvalId};` +
        `上一步已 200 并登记消费,本步与上步唯一变的是这条令牌的消费状态`
    ).toBe(403);
  });

  test('正向:持审批权限的申请人自批通过(approve 接受 approver==applicant 并签发令牌)', async ({
    page,
  }) => {
    // admin 权限全通且持 export-approvals:approve。按当前契约,自批由 RBAC 权限键决定、
    // service 层不再无条件硬拒"审批人=申请人",故 admin 本人申请 + 本人审批应 2xx 成功。
    // 若后端误恢复旧严格分离门,apiCall 会在业务 403 时抛错、真实暴露判红,不会假绿。
    const created = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
      resource_type: 'customer',
      export_params: {},
      estimated_rows: 5,
      file_format: 'xlsx',
    });
    const approvalId = created.data?.id;
    expect(
      approvalId,
      `admin 创建导出审批申请应返回有效 id:${JSON.stringify(created.data).slice(0, 200)}`
    ).toBeGreaterThan(0);
    expect(created.data?.status, '创建后初始状态应为 pending').toBe('pending');
    const applicantId = created.data?.applicant_user_id;
    expect(
      applicantId,
      `创建响应应落 applicant_user_id,实际:${JSON.stringify(created.data).slice(0, 200)}`
    ).not.toBeNull();

    // 同一身份(admin)审批:apiCall 仅在业务失败时抛错,能正常返回即证明 approve 未被拒
    const approved = (
      await apiCall<ApprovalModel>(page, 'POST', `/export-approvals/${approvalId}/approve`, {
        comments: 'E2E 自批正向锁测',
      })
    ).data;
    expect(
      approved?.status,
      `持权自批通过后状态应转 approved,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBe('approved');
    expect(
      approved?.approver_user_id,
      `自批应如实记录 approver_user_id,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).not.toBeNull();
    expect(approved?.approver_user_id, '自批时审批人应等于申请人(持权自批被接受且如实记录)').toBe(
      applicantId
    );

    // 一次性读取契约：审批出参不回显凭据，由申请人本人（本例即持权自批者）重发取回
    expect(
      approved.download_token ?? null,
      `自批出参不应回显 download_token,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBeNull();
    const rotatedSelf = await apiCall<{ available?: boolean; token?: string | null }>(
      page,
      'POST',
      `/export-approvals/${approvalId}/rotate-token`
    );
    const token = rotatedSelf.data?.token ?? '';
    expect(
      token.length > 0,
      `申请人自批后重发应下发令牌,实际:${JSON.stringify(rotatedSelf.data).slice(0, 200)}`
    ).toBe(true);
    expect(token, `下发的令牌应为 exp_ 前缀加 32 位小写十六进制,实际 ${token}`).toMatch(
      /^exp_[0-9a-f]{32}$/
    );

    // verify-token 复核,证明自批签发的令牌已真实落库并可解析同一记录
    const verified = await apiCall<ApprovalModel>(
      page,
      'GET',
      `/export-approvals/verify-token?token=${encodeURIComponent(String(token))}`
    );
    expect(verified.data?.id, 'verify-token 应命中同一自批申请记录').toBe(approvalId);
    expect(verified.data?.status, 'verify-token 对自批已签发令牌应返回 approved').toBe('approved');
    expect(verified.data?.resource_type, 'verify-token 应回显 customer 资源绑定').toBe('customer');
  });

  test('负例:不带令牌直接下载 customer 导出 403(fail-closed)', async ({ page }) => {
    // admin 权限全通,此 403 只能来自 handler 审批门的缺令牌拒绝
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/crm/customers/export`);
    await expectForbiddenEnvelope(resp, '无令牌直接导出 customer');
  });

  test('越权负例:审批人(非申请人)会话消费申请人新鲜令牌 403,同一条未消费令牌申请人本人 200', async ({
    page,
  }) => {
    const approvalId = await createCustomerApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 主体校验锁测');
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 第一腿走审批人(admin)会话:超管过 RBAC 与数据域都靠短路放行,
    // 令牌也确实有效(申请人刚轮换拿到、格式已核),所以这一步的 403 只能是主体校验命中。
    await loginViaUI(page, undefined, undefined, true);
    const outsiderResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${encodeURIComponent(String(token))}`
    );
    expect(
      outsiderResp.status(),
      `非申请人(审批人 admin)会话消费申请人令牌应 403,approvalId=${approvalId}、` +
        `会话=admin 超管(RBAC 恒放行)、令牌新鲜未消费;此处 403 只能是主体校验命中`
    ).toBe(403);

    // 控制组:同一条令牌交回申请人本人会话,必须 200。服务端把消费登记放在受理门全部放行
    // 之后,所以这一步的 200 反过来证明上一步的拒绝没有消耗令牌(不是消费门),
    // 端点/参数/令牌完全相同,也就不是数据缺失或路由缺失。
    await loginAsRole(page, 'manager');
    const ownerResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${encodeURIComponent(String(token))}`
    );
    expect(
      ownerResp.status(),
      `同一条未消费令牌在申请人会话应 200,approvalId=${approvalId};` +
        `与上一步唯一变的是会话主体(admin→manager 申请人本人),端点/参数/令牌完全相同`
    ).toBe(200);

    // 状态回读:控制组兑现的这一次就是唯一一次消费,计数与终态与"只消费一次"一致
    const detail = await apiCall<ApprovalModel>(page, 'GET', `/export-approvals/${approvalId}`);
    expect(
      detail.data?.download_count,
      `回读消费计数应为 1(越权那一腿没计进),实际:${JSON.stringify(detail.data).slice(0, 200)}`
    ).toBe(1);
    expect(detail.data?.token_consumed, '申请人本人这一次消费后应置 token_consumed').toBe(true);
  });

  test('负例:customer 令牌跨资源用于 product 导出 403(资源绑定不匹配)', async ({ page }) => {
    const approvalId = await createCustomerApprovalAsManager(page);
    const approved = await approveAsAdmin(page, approvalId, 'E2E 跨资源锁测');
    expect(
      approved.download_token ?? null,
      `一次性读取契约下审批出参不应回显令牌,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBeNull();
    // 凭据由申请人本人重发取回,取回后停在申请人会话
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 跨资源下载腿走申请人本人会话:申请人过得了 RBAC(该岗在产品资源持通配授权)、
    // 主体也是令牌签发对象,所以这里的 403 才能唯一归因到令牌的资源绑定。
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${encodeURIComponent(String(token))}`
    );
    await expectForbiddenEnvelope(
      crossResp,
      `customer 令牌跨资源下载 product(approvalId=${approvalId}、会话=申请人 manager 本人,` +
        `同一条令牌打 customer 导出是 200——正向全链已证,唯一变的是目标端点的资源类型)`
    );
  });
});
