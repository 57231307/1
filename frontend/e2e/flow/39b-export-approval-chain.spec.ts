import { test, expect } from '../diagnose-fixture';
import { loginViaUI, loginAsRole, apiCall } from './helpers';

/**
 * P5.9b 敏感导出完整审批链
 *
 * 流程：创建审批申请 → 一级审批（approve）→ 生成下载令牌 →
 *       持令牌导出 200 → 令牌二次消费拒绝 → 跨资源令牌拒绝
 *
 * 前置：export_approval_service.rs:224 明确禁止申请人自审
 *       （"审批人不能是申请人本人（防自审批）"），因此完整链必须双人：
 *       由 manager 角色账号创建申请，再切回分片 admin 账号审批。
 *       estimated_rows=10 → 风险等级 low → approval_level=1，一次 approve 即出令牌。
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
 * 写请求（POST /export-approvals、POST /export-approvals/{id}/approve）统一走 helpers.apiCall：
 * 它先读 csrf_token cookie、命中 403 且判据为 CSRF（csrf.rs 直出体 MISSING/INVALID）时用后端
 * x-new-csrf-token 恢复头在 CSRF_RECOVERY_MAX_ATTEMPTS 上限内有界重放，耗尽仍被拒即抛错判红。
 * 本用例原先自持 csrfJsonHeaders 直连 page.request.post，只取一次 cookie 里的 token、无任何重放
 * ——一次性 token 被并发/UI 抢先消费后即 403 CSRF_TOKEN_INVALID（新红）。改用 apiCall 复用
 * 同一有界重放原语，不 skip、不放宽 CSRF 断言。GET 导出为安全方法不消费 CSRF，仍直连。
 */
async function createApprovalAsManager(page: import('@playwright/test').Page): Promise<number> {
  await loginAsRole(page, 'manager');
  const res = await apiCall<ApprovalModel>(page, 'POST', '/export-approvals', {
    resource_type: 'customer',
    export_params: { page: 1, page_size: 10 },
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
 * 申请人本人重发并一次性读取下载令牌。
 * 调用方：本文件的完整链用例与跨资源用例，均在 admin 审批之后执行。
 * 入参：page 为当前页，approvalId 为已 approved 的申请 id。
 * 传给谁：返回的令牌交回调用方用于持令牌导出、二次消费与跨资源三腿。
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
    // 1-2. manager 创建申请 + admin 审批（自审被后端禁止，必须双人）
    const approvalId = await createApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 审批链测试');

    // 3. 一次性读取凭据：无凭据 = 链断在这里，不能降级为可选分支
    const token = await rotateTokenAsApplicant(page, approvalId);
    // 取凭据走的是申请人会话；导出腿切回 admin，让 200 与随后的二次消费 403
    // 只归因到令牌自身，而不是调用方角色。
    await loginViaUI(page, undefined, undefined, true);

    // 4. 持令牌导出
    const exportResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(exportResp.status(), '持有效令牌导出应 200').toBe(200);

    // 5. 令牌二次消费拒绝（download_count 上限/一次性）
    const secondResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=${token}`
    );
    expect(secondResp.status(), '令牌二次消费应被拒（403）').toBe(403);
  });

  test('跨资源令牌拒绝：customer 令牌用于 product 导出', async ({ page }) => {
    // 创建 customer 审批并拿到令牌（manager 申请 + admin 审批）
    const approvalId = await createApprovalAsManager(page);
    await approveAsAdmin(page, approvalId, 'E2E 跨资源测试');
    const token = await rotateTokenAsApplicant(page, approvalId);
    // 同完整链用例：切回 admin，让 403 只能归因到令牌的资源绑定
    await loginViaUI(page, undefined, undefined, true);

    // customer 令牌用于 product 导出 → resource_type 不匹配 403
    const crossResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/products/export?download_token=${token}`
    );
    expect(crossResp.status(), '跨资源令牌应 403').toBe(403);
  });

  test('审批状态机：未审批申请直接导出 403（fail-closed 兜底）', async ({ page }) => {
    // pending 状态的申请其 token 为 null——直接带空/伪 token 导出被拒
    const resp = await page.request.get(
      `${API_BASE}${API_PREFIX}/crm/customers/export?download_token=not-a-real-token`
    );
    expect(resp.status()).toBe(403);
  });
});
