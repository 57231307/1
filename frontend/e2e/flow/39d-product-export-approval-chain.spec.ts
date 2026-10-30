import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall } from './helpers';

/**
 * 产品导出完整审批链回归（申请人侧流程对应的服务端契约）
 *
 * 流程：manager 创建 product 导出审批申请 至 admin 一级审批通过 至 申请人本人重发取回一次性
 *       下载令牌 至 verify-token 判有效 至 申请人会话持令牌导出 200 且文件体非空 至
 *       同一条令牌二次下载被拒 至 非申请人（审批人）会话消费新鲜令牌被拒。
 *
 * 对照钉死三件事：
 * - 旧路径（无令牌直连 products/export）必 403（fail-closed 生效，缺陷根因）；
 * - 新路径（审批后申请人本人持令牌）必 200 且 blob 非空（申请通道打通，缺陷已修）；
 * - 凭据只能由被批准的申请人本人消费：审批人会话拿同一枚新鲜令牌必 403。
 *
 * 会话主体口径：服务端导出受理门只放行令牌签发给的那位申请人，所以凭据取回与全部下载腿
 * 都走申请人（manager）会话；把下载腿切到审批人会话在这条链里得到的是主体校验的 403，
 * 不是令牌生命周期判据。审批腿走 admin：审批要 export-approvals:approve，manager 不持该键。
 * 两个 403 用会话顺序做控制组区分，不按文案区分（PermissionDenied 出参文案被统一脱敏）。
 *
 * 前置：estimated_rows=10 命中低风险，approval_level=1，一级 approve 即签发令牌。
 * 写请求统一走 helpers.apiCall（CSRF 有界重放），GET 为安全方法不消费 CSRF，直连 page.request。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

interface ApprovalModel {
  id: number;
  status: string;
  resource_type: string;
  file_format: string;
  approval_level: number;
  download_token?: string | null;
}

async function createProductApproval(page: import('@playwright/test').Page): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'product',
    export_params: {},
    estimated_rows: 10,
    file_format: 'xlsx',
  });
  const created = res.data;
  expect(
    created?.id,
    `manager 创建 product 导出审批申请应返回有效 id：${JSON.stringify(created).slice(0, 200)}`
  ).toBeGreaterThan(0);
  // 申请通道打通的直接证据：product 已被敏感清单接受，返回 pending（不再是
  // "资源类型不在敏感清单"的校验拒绝，apiCall 对非 200 信封已判红抛错）
  expect(created?.status, '创建成功后初始状态应为 pending').toBe('pending');
  expect(created?.resource_type, '申请应绑定 product 资源类型').toBe('product');
  expect(created?.file_format, '申请应按提交的文件格式落库').toBe('xlsx');
  console.log(`[39d] manager 已创建 product 导出审批申请 id=${created?.id}`);
  return created.id;
}

async function approveAsAdmin(
  page: import('@playwright/test').Page,
  approvalId: number
): Promise<ApprovalModel> {
  await loginViaUI(page, undefined, undefined, true);
  const res = await apiCall<ApprovalModel>(
    page,
    'POST',
    `/export-approvals/${approvalId}/approve`,
    { comments: 'E2E 产品导出审批链' }
  );
  const approved = res.data;
  expect(
    approved?.status,
    `低风险一级审批通过后状态应为 approved，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toBe('approved');
  // 一次性读取契约：审批出参恒不回显令牌原文（服务端 detail_masked 屏蔽 download_token 与
  // token_hash），凭据只能通过申请人本人的 rotate-token 单独取一次。这里钉"必须为空"，
  // 一旦哪天出参又带出令牌，本断言立刻判红，而不是让下游用例误打误撞继续通过。
  expect(
    approved?.download_token ?? null,
    `审批出参不应回显 download_token，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toBeNull();
  console.log(`[39d] admin 已审批产品导出申请 ${approvalId}`);
  return approved;
}

/// 申请人本人重发并一次性读取下载令牌。
/// 调用方：本文件的完整链用例与跨资源防护用例，审批通过后由 manager 会话执行。
/// 入参：page 为当前 Playwright 页，approvalId 为已 approved 的申请 id。
/// 传给谁：返回的令牌串交给 verify-token 与带令牌导出与二次消费三腿，以及跨资源负例。
/// 判据源：服务端 rotate_token 只允许申请人本人调用（非申请人 403），且要求状态为 approved；
/// 出参含 available 与 token 两键，token 为原文、指纹另列，故这里只回令牌。
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
    `申请人重发令牌应返回 available，实际：${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  const token = rotated.data?.token ?? '';
  expect(
    token.length > 0,
    `重发应下发非空令牌原文，实际：${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  console.warn(`[39d] manager 已重发申请 ${approvalId} 的一次性下载令牌`);
  return token;
}

test.describe('产品导出完整审批链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('对照：无令牌直连产品导出必 403（旧路径 fail-closed）', async ({ page }) => {
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}/products/export`);
    expect(resp.status(), '无 download_token 的产品导出必须被 fail-closed 拒绝 403').toBe(403);
  });

  test('完整链：申请至审批至verify-token有效至持令牌200至二次用token被拒', async ({ page }) => {
    const approvalId = await createProductApproval(page);
    await approveAsAdmin(page, approvalId);
    const token = await rotateTokenAsApplicant(page, approvalId);

    // verify-token：GET 安全方法直连，读审批记录、不消费凭据，故不参与主体口径；
    // 后端出参为完整审批 Model（非 valid 布尔信封），
    // 有效令牌判据是 status approved 且 resource_type product
    await loginViaUI(page, undefined, undefined, true);
    const verified = await apiCall<ApprovalModel>(
      page,
      'GET',
      `/export-approvals/verify-token?token=${token}`
    );
    expect(verified.data?.status, 'verify-token 对已签发令牌应返回 approved').toBe('approved');
    expect(verified.data?.resource_type, 'verify-token 应回显 product 资源绑定').toBe('product');

    // 新路径：申请人本人会话持令牌导出 200 且文件体非空（真实二进制，不是 JSON 错误体）。
    // 凭据签发给申请人，消费腿只能走申请人会话；审批人会话消费属另一条判据，见越权负例。
    await loginAsRole(page, 'manager');
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(
      exportResp.status(),
      `申请人会话持有效令牌导出应 200，approvalId=${approvalId}、会话=manager、令牌已带上且非空` +
        `（长度 ${token.length}）；这一步之前 verify-token 已判该令牌 approved 且绑定 product`
    ).toBe(200);
    const contentType = exportResp.headers()['content-type'] ?? '';
    expect(contentType, `导出响应应为 xlsx 二进制流，实际 content-type=${contentType}`).toContain(
      'spreadsheetml'
    );
    const bytes: Uint8Array = await exportResp.body();
    expect(bytes.byteLength, '导出文件内容必须非空').toBeGreaterThan(0);

    // 一次性语义：同一条令牌在同一个申请人会话二次下载必须 403（download_count 达上限），
    // 钉死防重放。与上一步的唯一差别是"这条令牌已被消费过一次"。
    const replayResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(
      replayResp.status(),
      `同一条令牌在同一个申请人会话二次下载应 403（一次性门），approvalId=${approvalId}；` +
        `本步与上一步唯一变的是这条令牌已被消费，不是主体、不是范围、不是路由`
    ).toBe(403);
  });

  test('越权负例：审批人（非申请人）会话消费申请人新鲜令牌 403，同一条令牌申请人本人 200', async ({
    page,
  }) => {
    const approvalId = await createProductApproval(page);
    await approveAsAdmin(page, approvalId);
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 第一腿走审批人（admin）会话：admin 过 RBAC 与数据域都靠超管短路，
    // 这一步的 403 只能是"令牌的签发对象不是本会话用户"。
    await loginViaUI(page, undefined, undefined, true);
    const outsiderResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(
      outsiderResp.status(),
      `非申请人（审批人 admin）会话消费申请人令牌应 403，approvalId=${approvalId}、` +
        `会话=admin 超管（RBAC 恒放行）、令牌新鲜未消费；此处 403 只能是主体校验命中`
    ).toBe(403);

    // 控制组：同一条令牌交回申请人本人会话，必须 200。服务端把消费登记放在受理门全部放行
    // 之后，所以这一步的 200 反过来证明上一步的拒绝没有消耗令牌（不是消费门），
    // 端点/参数/令牌完全相同，也就不是数据缺失或路由缺失。
    await loginAsRole(page, 'manager');
    const ownerResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(
      ownerResp.status(),
      `同一条未消费令牌在申请人会话应 200，approvalId=${approvalId}；` +
        `与上一步唯一变的是会话主体（admin→manager 申请人本人），端点/参数/令牌完全相同`
    ).toBe(200);
  });

  test('跨资源防护：product 令牌不可用于客户导出', async ({ page }) => {
    const approvalId = await createProductApproval(page);
    await approveAsAdmin(page, approvalId);
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 申请人本人会话拿 product 令牌打客户导出：主体与 RBAC 都放行
    // （manager 在客户资源持通配授权），这里的 403 才能唯一归因到令牌的资源绑定。
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(
      crossResp.status(),
      `product 令牌打 customer 导出应 403，approvalId=${approvalId}、会话=申请人 manager 本人；` +
        `同一条令牌打 product 导出是 200（完整链用例已证），唯一变的是目标端点的资源类型`
    ).toBe(403);
  });
});
