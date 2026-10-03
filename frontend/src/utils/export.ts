import axios from 'axios';
import type { AxiosResponse } from 'axios';
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
 * 从后端下载真 .xlsx（OOXML）文件——唯一合法导出路径
 *
 * 调用后端 GET 端点（如 /crm/customers/export），返回 Blob 流
 * （application/vnd.openxmlformats-officedocument.spreadsheetml.sheet）。
 * 后端已注入水印（操作员/IP/时间戳），前端无需重复添加。
 * 自动从 Content-Disposition 提取文件名；失败时回退到传入的 filename + 时间戳。
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
    const disposition = response.headers?.['content-disposition'] || '';
    const matched = /filename="?([^";]+)"?/.exec(disposition);
    const downloadName =
      matched?.[1] || `${filename}_${new Date().toISOString().replace(/[:.]/g, '')}.xlsx`;

    const blob = response.data;
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = downloadName;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(link.href);
    msg.exportOk();
  } catch (err) {
    const errDetail = err instanceof Error ? err.message : msg.translate('exportFailed');
    msg.error('exportFailedReason', { reason: errDetail });
    throw err;
  }
}
