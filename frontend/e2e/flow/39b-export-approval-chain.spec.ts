import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall } from './helpers';

/**
 * 敏感导出完整审批链（customer 资源）
 *
 * 流程：manager 创建审批申请 → admin 一级审批通过 → 申请人本人重发取回一次性下载令牌 →
 *       申请人会话持令牌导出 200 → 同一条令牌二次消费 403 → 跨资源令牌 403 →
 *       无令牌/伪令牌 403（fail-closed）
 *
 * 会话主体口径（唯一判据源：服务端导出受理门只放行令牌签发给的那位申请人）：
 * - 审批腿走 admin：审批要 export-approvals:approve，manager 不持该键，故申请与审批分两人；
 * - 凭据取回与全部令牌消费腿走申请人（manager）本人会话：非申请人会话消费令牌
 *   必须被服务端拒绝，所以"下载腿切到审批人会话"在这条链里是错的判据；
 * - 要同时钉住"一次性消费"和"只能本人使用"两件事，用会话顺序做控制组：
 *   非申请人会话先打一条新鲜未消费令牌得 403，紧接着同一条令牌在申请人会话得 200，
 *   即证明前一步的拒绝不是消费门，也不是数据或路由缺失。
 * - estimated_rows=10 → 低风险 → approval_level=1，一次 approve 即签发令牌。
 *
 * 申请参数形状：export_params 必须与下载腿实际携带的范围参数同形。客户导出端点真实读取
 * 的筛选维度只有 keyword/customer_type/status，分页参数不是该端点的范围维度，写进申请会让
 * 受理门的范围比对判成"批准里有而请求里没有的维度"，与主体校验无关的 403 会掩盖本用例靶心。
 */

const API_BASE = process.env.API_BASE || 'http://localhost:8082';
const API_PREFIX = '/api/v1/erp';

interface ApprovalModel {
  id: number;
  status: string;
  download_token?: string | null;
  resource_type: string;
}

/**
 * manager 创建 customer 导出审批申请，返回申请 id。
 * 调用方：本文件的完整链用例、跨资源用例与越权负例。
 * 入参：page 为当前页；请求体带 resource_type=customer、与下载腿同形的空范围、
 * estimated_rows=10（低风险一级审批）、file_format=xlsx。
 * 传给谁：返回的申请 id 交审批腿与轮换腿。
 * 存什么：导出审批申请行（申请人记为 manager 用户）。
 * 存哪里：export_approval_request 表。
 *
 * 写请求统一走 helpers.apiCall：它先读 csrf_token cookie，命中 CSRF 判据的 403 时用后端
 * x-new-csrf-token 恢复头在有界次数内重放，耗尽仍被拒即抛错判红；GET 为安全方法不消费
 * CSRF，直连 page.request。
 */
async function createApprovalAsManager(page: import('@playwright/test').Page): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'customer',
    export_params: {},
    estimated_rows: 10,
    file_format: 'xlsx',
  });
  const id = res.data?.id;
  // 内容断言不放宽：申请成功必须返回有效正整数 id（HTTP/CSRF/业务失败已由 apiCall 抛红）。
  expect(
    id,
    `manager 创建导出审批申请成功但响应体缺少有效 data.id：${JSON.stringify(res.data).slice(0, 200)}`
  ).toBeGreaterThan(0);
  console.log(`[39b] manager 已创建 customer 导出审批申请 id=${id}`);
  return id as number;
}

/** 切回分片 admin 账号审批该申请，返回审批后的模型（出参按一次性读取契约不回显令牌） */
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
    approved?.status ?? '',
    `审批后状态应含 approved，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toContain('approved');
  // 一次性读取契约：审批出参恒不回显令牌原文（服务端 detail_masked 屏蔽 download_token
  // 与 token_hash），凭据只能由申请人本人经 rotate-token 单独取一次。钉"必须为空"，
  // 哪天出参又漏出令牌，本断言立即判红，而不是让下游用例误打误撞继续绿。
  expect(
    approved?.download_token ?? null,
    `审批出参不应回显 download_token，实际：${JSON.stringify(approved).slice(0, 200)}`
  ).toBeNull();
  console.log(`[39b] admin 已审批申请 ${approvalId}，status=${approved?.status}`);
  return approved as ApprovalModel;
}

/**
 * 申请人本人重发并一次性读取下载令牌，重发后调用方会话保持为申请人（manager）。
 * 调用方：本文件的完整链用例、跨资源用例与越权负例，均在 admin 审批之后执行。
 * 入参：page 为当前页，approvalId 为已 approved 的申请 id。
 * 传给谁：返回的令牌交回调用方用于持令牌导出、二次消费、跨资源与越权四腿。
 * 判据源：服务端 rotate_token 只允许申请人本人调用（非申请人 403）且要求状态为 approved，
 * 出参 available 为真时 token 即一次性凭据原文。
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
    `申请人重发令牌应返回 available，实际：${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  const token = rotated.data?.token ?? '';
  expect(
    token.length > 0,
    `重发应下发非空令牌原文，实际：${JSON.stringify(rotated.data).slice(0, 200)}`
  ).toBe(true);
  console.warn(`[39b] manager 已重发申请 ${approvalId} 的一次性下载令牌`);
  return token;
}

test.describe('P5.9b 敏感导出完整审批链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('客户导出审批链：创建→审批→令牌导出→二次消费拒', async ({ page }) => {
    // 1-2. manager 创建申请 + admin 审批（审批键 manager 不持，必须换人办理）
    const approvalId = await createApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 审批链测试');

    // 3. 一次性读取凭据：无凭据 = 链断在这里，不能降级为可选分支；轮换后停在申请人会话
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 4. 申请人本人持令牌导出：凭据签发给申请人，消费腿只能走申请人会话
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(
      exportResp.status(),
      `申请人会话持有效令牌导出应 200，approvalId=${approvalId}、会话=manager、令牌已带上且非空（长度 ${token.length}）`
    ).toBe(200);

    // 5. 令牌二次消费拒绝（download_count 达上限）：变的只有"这条令牌已被消费过一次"
    const secondResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(
      secondResp.status(),
      `同一条令牌在同一个申请人会话二次消费应 403（一次性门），approvalId=${approvalId}；` +
        `本步与会话上一步的唯一差别是这条令牌已被消费，不是主体、不是范围、不是路由`
    ).toBe(403);
  });

  test('越权负例：审批人（非申请人）会话消费申请人令牌 403，同一条未消费令牌申请人本人 200', async ({
    page,
  }) => {
    const approvalId = await createApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 主体校验负例');
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 第一腿：切到审批人（admin）会话。admin 过 RBAC 与数据域都靠超管短路，
    // 因此这一步若被拒，唯一可能的拒绝来源是"令牌的签发对象不是本会话用户"。
    await loginViaUI(page, undefined, undefined, true);
    const outsiderResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(
      outsiderResp.status(),
      `非申请人（审批人 admin）会话消费申请人令牌应 403，approvalId=${approvalId}、` +
        `会话=admin 超管（RBAC 恒放行）、令牌新鲜未消费；此处 403 只能是主体校验命中`
    ).toBe(403);

    // 控制组：同一条令牌交回申请人本人会话，必须 200。
    // 若上一步的 403 其实来自消费/次数门，令牌已被判消费，这一步只能是 403；
    // 服务端把消费登记放在受理门全部放行之后，所以这一步的 200 反过来证明
    // 上一步的拒绝没有消耗令牌，也就不是消费门，更不是数据缺失或路由缺失（同 URL 同参数）。
    await loginAsRole(page, 'manager');
    const ownerResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(
      ownerResp.status(),
      `同一条未消费令牌在申请人会话应 200，approvalId=${approvalId}；` +
        `与上一步唯一变的是会话主体（admin→manager 申请人本人），端点/参数/令牌完全相同`
    ).toBe(200);
  });

  test('跨资源令牌拒绝：customer 令牌用于 product 导出', async ({ page }) => {
    // 创建 customer 审批并拿到令牌（manager 申请 + admin 审批，凭据取回停在申请人会话）
    const approvalId = await createApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 跨资源测试');
    const token = await rotateTokenAsApplicant(page, approvalId);

    // 申请人本人会话拿 customer 令牌打 product 导出：主体与 RBAC 都放行
    // （manager 在产品资源持通配授权），这里的 403 只能归因到令牌的资源绑定
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(
      crossResp.status(),
      `customer 令牌打 product 导出应 403，approvalId=${approvalId}、会话=申请人 manager 本人；` +
        `同一条令牌打 customer 导出是 200（完整链用例已证），唯一变的是目标端点的资源类型`
    ).toBe(403);
  });

  test('审批状态机：未审批申请直接导出 403（fail-closed 兜底）', async ({ page }) => {
    // pending 状态的申请其 token 为 null——直接带空/伪 token 导出被拒
    const resp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=not-a-real-token`
    );
    expect(resp.status()).toBe(403);
  });
});
