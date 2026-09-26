/**
 * 端点矩阵可达性判定工具（traversal 专用，不改公共 flow/helpers）
 *
 * 背景：37/39/41 三张端点矩阵此前对 404/400 一律 `test.skip()`，导致
 * "端点未注册 / 配置路径写错 / 打印导出整族坏掉" 永远不红（doto iter31 判责）。
 * 本模块把"响应形态"变成真实判定：
 *   - 5xx                                   → 处理器崩溃，判红
 *   - <400                                  → 交由调用方按内容断言（200 + zip）
 *   - 4xx 且为后端标准错误体                 → 处理器已注册，仅业务/数据原因拒绝
 *   - 4xx 且非标准错误体（裸 404/405，无 JSON） → 路由未注册 / 路径写错，判红
 *
 * 判定依据（后端契约，只读引用，不改后端）：
 *   - 所有 AppError 经 IntoResponse 出参恒为标准错误体
 *     `{code,message,trace_id,timestamp}`（backend/src/utils/error.rs:429-436）；
 *   - 根 Router 未挂全局 fallback（backend/src/routes/mod.rs:534 仅挂
 *     sql_injection_audit 中间件），未匹配路由由 axum 返回空体 404/405，
 *     因此"注册但数据缺失"（有 trace_id 的标准错误体）与"路径写错"（空体）可区分。
 */
import { expect } from '@playwright/test';

export type ProbeOutcome = 'ok' | 'registered-error' | 'unregistered' | 'server-error';

/** 解析后端标准错误体；非标准（空体/HTML/二进制）返回 null。 */
export function parseAppError(bodyText: string): { code: string } | null {
  try {
    const j = JSON.parse(bodyText) as Record<string, unknown>;
    if (
      j &&
      typeof j === 'object' &&
      typeof j.trace_id === 'string' &&
      'code' in j &&
      'message' in j
    ) {
      return { code: String(j.code) };
    }
  } catch {
    // 非 JSON（裸 404/405）→ 非标准错误体
  }
  return null;
}

/** 基于已取到的 status + bodyText 判定矩阵响应形态。 */
export function classifyStatus(status: number, bodyText: string): ProbeOutcome {
  if (status >= 500) return 'server-error';
  if (status < 400) return 'ok';
  return parseAppError(bodyText) ? 'registered-error' : 'unregistered';
}

/**
 * 断言端点"已注册且未崩溃"：5xx 与裸 4xx（未注册）一律判红。
 * 返回 'ok' | 'registered-error' 供调用方进一步处理。
 */
export function assertEndpointRegistered(
  label: string,
  outcome: ProbeOutcome,
  detail: { status: number; bodyText: string }
): void {
  if (outcome === 'server-error') {
    throw new Error(
      `[matrix] ${label} 崩溃：HTTP ${detail.status}，响应片段=${detail.bodyText.slice(0, 160)}`
    );
  }
  if (outcome === 'unregistered') {
    throw new Error(
      `[matrix] ${label} 未被后端注册：HTTP ${detail.status} 返回裸错误（无标准错误体 ` +
        `{code,message,trace_id}）。路径拼写错误或后端路由缺失——确实不适用的端点须在配置里显式` +
        `标注（noPrint/noExport）并给出 backend 路由依据，不能当作"数据缺失"静默跳过。`
    );
  }
}

/**
 * 断言 xlsx/docx 导出/打印成功响应：必须是真实 zip 容器（PK magic）且体积合理，
 * Content-Type 为 OOXML/octet-stream。把"200 但内容坏了"也变成红色。
 */
export function assertZipDocxResponse(
  label: string,
  body: Buffer,
  contentType: string,
  minBytes = 1024
): void {
  const isZip = body.length > 4 && body[0] === 0x50 && body[1] === 0x4b;
  expect(
    isZip,
    `${label} 200 响应应为 zip 容器（PK\\x03\\x04），实际首字节=${body[0]}/${body[1]}、长度=${body.length}B`
  ).toBe(true);
  expect(body.length, `${label} 响应体应 >${minBytes}B，实际 ${body.length}B`).toBeGreaterThan(
    minBytes
  );
  expect(
    contentType.includes('openxmlformats') ||
      contentType.includes('octet-stream') ||
      contentType.includes('spreadsheetml') ||
      contentType.includes('wordprocessingml'),
    `${label} Content-Type 应为 OOXML/octet-stream，实际 ${contentType}`
  ).toBeTruthy();
}

/**
 * 断言 admin 账号未被权限拒绝：标准错误体里出现 FORBIDDEN/UNAUTHORIZED 即真实缺陷
 * （admin 持 `*:*`，被拒说明该端点权限模型或路由有问题）。
 */
export function assertNotPermissionDenied(label: string, code: string): void {
  expect(
    ['FORBIDDEN', 'UNAUTHORIZED', 'PERMISSION_DENIED', '403', '401'],
    `${label} 被权限/认证拒绝（${code}），admin 持 * 不应出现——真实缺陷`
  ).not.toContain(code);
}
