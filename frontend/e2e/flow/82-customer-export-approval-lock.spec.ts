import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall, apiCallExpectFail } from './helpers';

/**
 * 敏感数据(customer)导出审批流端到端锁测 — flow/82
 *
 * 锁定的根因行为链(逐点对着后端源码核实,非编造):
 * - 创建申请:POST /export-approvals,敏感清单含 customer;新建落库 status=pending
 *   且 download_token 恒为 null(令牌只在最终 approve 时签发);
 *   estimated_rows 低于 1000 命中 risk_level=low,approval_level=1,一次审批即终结。
 * - 审批分离:approve 服务门禁止申请人==审批人(防自批),违反时 403,
 *   统一失败信封 code=FORBIDDEN(脱敏文案,故只钉 status 与 code,不断 message)。
 * - 令牌下发:低风险一级 approve 后返回 status=approved 且 download_token 形如
 *   exp_ + 32 位小写十六进制;GET /export-approvals/verify-token 可复核同一记录。
 * - 带令牌下载:GET /crm/customers/export?download_token=... 必须 200,
 *   Content-Type 为 xlsx MIME,响应体以 ZIP 魔数 PK 开头且非空
 *   (CI 内唯一能真实验证"审批到令牌到下载"全通的证据面)。
 * - fail-closed:不带令牌直接下载 403;customer 令牌跨资源用于 products 导出 403。
 *
 * 角色安排(复用 39b/39d 已在 CI 验证可用的账号面):
 * - 申请人:manager 角色账号(loginAsRole,凭证由 global-setup 角色账号写入);
 * - 审批人与下载人:分片默认 admin 账号(权限中间件对 admin 短路,
 *   因此无令牌/跨资源的 403 必然来自 handler 的审批门,不会与权限层混淆);
 * - 自批负例改用 admin 本人申请 + admin 本人审批:admin 权限全通,
 *   403 只能来自服务层"申请人==审批人"这一道门,判据唯一。
 *
 * 写请求统一走 helpers.apiCall/apiCallExpectFail(内置 CSRF 有界重放,
 * CSRF 竞败不会被误读成业务拒绝);GET 为安全方法不消费 CSRF,下载直连 page.request。
 * 注释遵循受限字符集,不引用文件行号与批次历史。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

/** 导出审批记录(后端 Model 直出,snake_case 键名) */
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
}

/** 统一失败信封(utils/error.rs ErrorResponse):code 字符串机器码 + trace_id */
interface FailureEnvelope {
  code?: unknown;
  message?: unknown;
  trace_id?: unknown;
  timestamp?: unknown;
}

/** 低风险 customer 导出申请:manager 创建,返回申请 id */
async function createCustomerApprovalAsManager(
  page: import('@playwright/test').Page
): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'customer',
    export_params: { page: 1, page_size: 10 },
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
    const token = approved.download_token;
    expect(
      token,
      `审批通过后应签发 download_token,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBeTruthy();
    expect(
      String(token),
      `download_token 应为 exp_ 前缀加 32 位小写十六进制,实际 ${token}`
    ).toMatch(/^exp_[0-9a-f]{32}$/);

    // 3. verify-token 复核同一记录(admin 数据域为 All,归属门必然放行)
    const verified = await apiCall<ApprovalModel>(
      page,
      'GET',
      `/export-approvals/verify-token?token=${encodeURIComponent(String(token))}`
    );
    expect(verified.data?.id, 'verify-token 应命中同一申请记录').toBe(approvalId);
    expect(verified.data?.status, 'verify-token 对已签发令牌应返回 approved').toBe('approved');
    expect(verified.data?.resource_type, 'verify-token 应回显 customer 资源绑定').toBe('customer');

    // 4. 带令牌下载:200 且响应体是真实 xlsx 二进制(x-zip 魔数 PK 开头,非空)
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${encodeURIComponent(String(token))}`
    );
    expect(exportResp.status(), '持有效令牌导出应 200').toBe(200);
    const contentType = exportResp.headers()['content-type'] ?? '';
    expect(
      contentType,
      `导出响应 Content-Type 应为 xlsx MIME,实际 content-type=${contentType}`
    ).toContain('application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
    const bytes: Uint8Array = await exportResp.body();
    expect(bytes.byteLength, '导出文件内容必须非空').toBeGreaterThan(0);
    expect(bytes[0], 'xlsx 为 zip 容器,魔数首字节应为 P(0x50)').toBe(0x50);
    expect(bytes[1], 'xlsx 为 zip 容器,魔数次字节应为 K(0x4b)').toBe(0x4b);
  });

  test('负例:申请人自批被拒(403 FORBIDDEN,非 5xx)', async ({ page }) => {
    // 以 admin 本人申请再由本人审批:admin 在权限中间件短路,
    // 该 403 唯一来源是服务层"审批人不能是申请人本人"门,判据不与其他层混淆
    const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
      resource_type: 'customer',
      export_params: {},
      estimated_rows: 5,
      file_format: 'xlsx',
    });
    const approvalId = res.data?.id;
    expect(
      approvalId,
      `admin 创建导出审批申请应返回有效 id:${JSON.stringify(res.data).slice(0, 200)}`
    ).toBeGreaterThan(0);

    const fail = await apiCallExpectFail(page, 'POST', `/export-approvals/${approvalId}/approve`, {
      comments: 'E2E 自批负例',
    });
    expect(fail.status, `自批应恰为 403,实际 status=${fail.status}`).toBe(403);
    expect(
      fail.code,
      `自批拒绝码应为 FORBIDDEN(权限族),实际 code=${fail.code} message=${fail.message}`
    ).toBe('FORBIDDEN');
    expect(fail.status, '自批拒绝不得退化为 5xx 裸崩').toBeLessThan(500);

    // 尽力取消该 pending 申请,不留悬挂待办;取消失败仅告警不掩盖(记录日志)
    await apiCall(page, 'POST', `/export-approvals/${approvalId}/cancel`).catch(e => {
      console.warn(
        `[82] 取消自批负例遗留申请 ${approvalId} 失败(不阻断断言): ${(e as Error).message}`
      );
    });
  });

  test('负例:不带令牌直接下载 customer 导出 403(fail-closed)', async ({ page }) => {
    // admin 权限全通,此 403 只能来自 handler 审批门的缺令牌拒绝
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/crm/customers/export`);
    await expectForbiddenEnvelope(resp, '无令牌直接导出 customer');
  });

  test('负例:customer 令牌跨资源用于 product 导出 403(资源绑定不匹配)', async ({ page }) => {
    const approvalId = await createCustomerApprovalAsManager(page);
    const approved = await approveAsAdmin(page, approvalId, 'E2E 跨资源锁测');
    const token = approved.download_token;
    expect(
      token,
      `前置:审批后应签发令牌,实际:${JSON.stringify(approved).slice(0, 200)}`
    ).toBeTruthy();

    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${encodeURIComponent(String(token))}`
    );
    await expectForbiddenEnvelope(crossResp, 'customer 令牌跨资源下载 product');
  });
});
