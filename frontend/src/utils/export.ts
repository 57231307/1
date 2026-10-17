import axios from 'axios';
import type { AxiosResponse } from 'axios';
import { ElMessage } from 'element-plus';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';

/**
 * 导出专用 axios 实例（与 request.ts 主实例隔离）
 *
 * 设计要点：
 * - 直接使用 axios，绕过 request.ts 响应拦截器，避免 ApiResponse.code 校验误伤 Blob 响应
 * - 不导入 request.ts，避免触发 router/index.ts 导入链副作用
 * - GET 请求无需 CSRF Token（与 request.ts isCsrfPublicPath 逻辑一致）
 * - withCredentials=true 保证 httpOnly Cookie（access_token）随请求发送
 * - baseURL 与 request.ts 保持一致，避免硬编码
 */
const exportAxios = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL || '/api/v1/erp',
  timeout: 60000,
  withCredentials: true,
  headers: {
    'X-Requested-With': 'XMLHttpRequest',
  },
});

/**
 * 触发浏览器保存 Blob 的唯一下落点（a[download] + createObjectURL + revoke）。
 * downloadName 由调用方决定（Content-Disposition 优先，回退前缀+时间戳）。
 */
function triggerBlobDownload(blob: Blob, downloadName: string): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = downloadName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}

/** 从 Content-Disposition 提取下载文件名；缺失时回退 filename 前缀 + 时间戳 */
function resolveDownloadName(response: AxiosResponse<Blob>, filename: string): string {
  const disposition = response.headers?.['content-disposition'] || '';
  const matched = /filename="?([^";]+)"?/.exec(disposition);
  return matched?.[1] || `${filename}_${new Date().toISOString().replace(/[:.]/g, '')}.xlsx`;
}

/**
 * 从后端下载真 .xlsx（OOXML）文件——唯一合法导出路径
 *
 * 调用后端 GET 端点（如 /crm/customers/export），返回 Blob 流
 * （application/vnd.openxmlformats-officedocument.spreadsheetml.sheet）。
 * 后端已注入水印（操作员/IP/时间戳），前端无需重复添加。
 * 自动从 Content-Disposition 提取文件名；失败时回退到传入的 filename + 时间戳。
 *
 * ⚠ 敏感资源端点已由后端 fail-closed 要求审批令牌，此类下载走
 * downloadExportWithToken；本函数仅用于非敏感资源直连导出。
 *
 * @param apiPath 后端导出 API 路径（如 /crm/customers/export）
 * @param params 查询参数（与 list 接口共用）
 * @param filename 下载文件名前缀（不含扩展名，后端会附加 .xlsx）
 */
export async function exportFromBackend<TParams extends Record<string, unknown>>(
  apiPath: string,
  params: TParams,
  filename: string
): Promise<void> {
  try {
    const response: AxiosResponse<Blob> = await exportAxios.get<Blob>(apiPath, {
      params,
      responseType: 'blob',
    });
    triggerBlobDownload(response.data, resolveDownloadName(response, filename));
    msg.exportOk();
  } catch (err) {
    const errDetail = err instanceof Error ? err.message : msg.translate('exportFailed');
    msg.error('exportFailedReason', { reason: errDetail });
    throw err;
  }
}

/**
 * 解析 blob 请求失败时后端返回的错误体（AppError 失败信封 {code,message,trace_id…}
 * 在 responseType=blob 下同样被 axios 装成 Blob）。取不到 message 原文返回 null，
 * 由调用方回退状态码级提示——绝不静默吞掉真实失败原因。
 */
async function extractBlobErrorDetail(error: unknown): Promise<string | null> {
  const data = (error as { response?: { data?: unknown } } | undefined)?.response?.data;
  if (!(data instanceof Blob)) return null;
  try {
    const parsed = JSON.parse(await data.text()) as { message?: unknown };
    return typeof parsed.message === 'string' && parsed.message.trim() !== ''
      ? parsed.message
      : null;
  } catch (parseError) {
    logger.warn('导出错误响应体不是合法 JSON，无法提取后端 message', parseError);
    return null;
  }
}

/**
 * 凭导出审批令牌下载（审批页集中下载底座，单一入口）
 *
 * 后端敏感导出端点统一以查询参数 download_token 承载一次性令牌
 * （services/export_approval_service.rs enforce_export_download fail-closed 校验，
 * 无/错令牌 403）。端点与参数形状由 constants/export-approval.ts 注册表解析后传入，
 * 调用方不得自行拼路径。失败外显后端信封 message 原文（blob 错误体经
 * extractBlobErrorDetail 还原），取不到才回退导出失败状态码文案，最后原样上抛
 * 供调用方留痕与刷新令牌状态。
 */
export async function downloadExportWithToken(options: {
  apiPath: string;
  params: Record<string, unknown>;
  token: string;
  filenamePrefix: string;
}): Promise<void> {
  try {
    const response: AxiosResponse<Blob> = await exportAxios.get<Blob>(options.apiPath, {
      params: { ...options.params, download_token: options.token },
      responseType: 'blob',
    });
    triggerBlobDownload(response.data, resolveDownloadName(response, options.filenamePrefix));
    msg.exportOk();
  } catch (err) {
    const backendMessage = await extractBlobErrorDetail(err);
    if (backendMessage) {
      ElMessage.error(backendMessage);
    } else {
      const reason = err instanceof Error ? err.message : msg.translate('exportFailed');
      msg.error('exportFailedReason', { reason });
    }
    throw err;
  }
}
