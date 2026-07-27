import axios from 'axios';
import type { AxiosResponse } from 'axios';
import { msg } from '@/utils/message';

/** 导出列定义，使用泛型支持类型安全的字段访问 */
export interface ExportColumn<T extends Record<string, unknown> = Record<string, unknown>> {
  key: keyof T & string;
  title: string;
  formatter?: (value: unknown, row: T) => string;
}

/** 导出选项，使用泛型约束数据类型 */
export interface ExportOptions<T extends Record<string, unknown> = Record<string, unknown>> {
  filename: string;
  columns: ExportColumn<T>[];
  data: T[];
  /** 导出格式，默认 excel（规则 3：禁止 CSV 作为最终交付） */
  format?: 'excel';
  /** V15 P1-4-3：资源类型标识（用于永久禁止导出黑名单校验，可选） */
  resourceType?: string;
}

/**
 * V15 P1-4-3：永久禁止导出的资源黑名单
 *
 * 以下资源为企业核心技术机密，永久禁止通过前端本地导出：
 * - lab_dip：化验室 OK 样配方
 * - production_recipe：大货处方
 * - flow_card：流转卡条码
 *
 * 若需导出上述资源，必须走后端二级审批流程（approval_token），
 * 前端 exportToExcel 在调用时校验资源类型，命中黑名单直接拒绝。
 */
const EXPORT_BLOCKED_RESOURCE_TYPES: readonly string[] = [
  'lab_dip',
  'production_recipe',
  'flow_card',
];

/**
 * V15 P1-4-3：检查资源类型是否在永久禁止导出黑名单中
 *
 * @param resourceType 资源类型标识（与后端权限码 resource_type 对应）
 * @returns true 表示禁止导出，false 表示允许
 */
export function isExportBlocked(resourceType: string): boolean {
  return EXPORT_BLOCKED_RESOURCE_TYPES.includes(resourceType);
}

function generateExcelHTML<T extends Record<string, unknown>>(
  columns: ExportColumn<T>[],
  data: T[]
): string {
  const headers = columns.map(col => `<th>${col.title}</th>`).join('');
  const rows = data
    .map(row => {
      const cells = columns
        .map(col => {
          const value = row[col.key];
          const formatted = col.formatter ? col.formatter(value, row) : String(value ?? '');
          return `<td>${formatted}</td>`;
        })
        .join('');
      return `<tr>${cells}</tr>`;
    })
    .join('');
  return `
    <html xmlns:o="urn:schemas-microsoft-com:office:office"
          xmlns:x="urn:schemas-microsoft-com:office:excel"
          xmlns="http://www.w3.org/TR/REC-html40">
    <head>
      <meta charset="utf-8">
      <!--[if gte mso 9]>
      <xml>
        <x:ExcelWorkbook>
          <x:ExcelWorksheets>
            <x:ExcelWorksheet>
              <x:Name>Sheet1</x:Name>
              <x:WorksheetOptions>
                <x:DisplayGridlines/>
              </x:WorksheetOptions>
            </x:ExcelWorksheet>
          </x:ExcelWorksheets>
        </x:ExcelWorkbook>
      </xml>
      <![endif]-->
    </head>
    <body>
      <table>
        <thead><tr>${headers}</tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </body>
  `;
}

function downloadFile(content: string, filename: string, mimeType: string) {
  const BOM = '\uFEFF';
  const blob = new Blob([BOM + content], { type: mimeType });
  const link = document.createElement('a');
  link.href = URL.createObjectURL(blob);
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(link.href);
}

/**
 * V15 P0-S12 修复（Batch 474）：本地 HTML 导出保留为兼容方案（资源尚未接入后端 export 时降级使用）
 *
 * 历史背景：原 exportToExcel 生成 .xls HTML 格式，无水印、无审计、无合规保障，
 * 已被 P0-S12 列为 P0 阻塞级问题。新接入后端的资源应改用 exportFromBackend。
 *
 * V15 P1-4-3：新增 resourceType 参数，命中永久禁止导出黑名单时直接拒绝。
 */
export function exportToExcel<T extends Record<string, unknown>>(options: ExportOptions<T>) {
  const { filename, columns, data, resourceType } = options;
  if (!data || data.length === 0) {
    msg.warning('noDataToExport');
    return;
  }
  // V15 P1-4-3：永久禁止导出资源黑名单校验（lab_dip/production_recipe/flow_card）
  if (resourceType && isExportBlocked(resourceType)) {
    msg.error('exportBlockedResource', { resource: resourceType });
    console.warn(`[P1-4-3] 资源 ${resourceType} 已被永久禁止导出（核心技术机密）`);
    return;
  }
  const htmlContent = generateExcelHTML(columns, data);
  const date = new Date().toISOString().split('T')[0];
  downloadFile(htmlContent, `${filename}_${date}.xls`, 'application/vnd.ms-excel;charset=utf-8;');
  msg.exportOk();
}

export function exportData<T extends Record<string, unknown>>(options: ExportOptions<T>) {
  exportToExcel(options);
}

/**
 * V15 P0-S12 修复（Batch 474）：导出专用 axios 实例
 *
 * 设计要点（与 request.ts 主实例隔离）：
 * - 直接使用 axios，绕过 request.ts 响应拦截器，避免 ApiResponse.code 校验误伤 Blob 响应
 *   （request.ts 拦截器 return res as unknown as AxiosResponse，使得 get<T>() 返回 Promise<T>，
 *   对 Blob 类型丢失 .headers/.data，导致 TS2339）
 * - 不导入 request.ts，避免触发 router/index.ts 导入链副作用（router 顶层 beforeEach
 *   在测试环境外调用，导致 tests/unit/utils.test.ts TypeError）
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
 * V15 P0-S12 + P0-S15 修复（Batch 474）：从后端下载带水印的 xlsx 文件
 *
 * 设计要点：
 * - 调用后端 GET API（如 `/crm/customers/export`），返回 Blob 流（application/vnd.openxmlformats-officedocument.spreadsheetml.sheet）
 * - 后端已注入水印（操作员/IP/时间戳），前端无需重复添加
 * - 自动从 Content-Disposition 提取文件名；失败时回退到传入的 filename + 时间戳
 * - 保留本地 exportToExcel 作为兼容方案（资源尚未接入后端 export 时降级使用）
 *
 * @param apiPath 后端导出 API 路径（如 `/crm/customers/export`）
 * @param params 查询参数（与 list 接口共用）
 * @param filename 下载文件名前缀（不含扩展名，后端会附加 .xlsx）
 */
export async function exportFromBackend<TParams extends Record<string, unknown>>(
  apiPath: string,
  params: TParams,
  filename: string
): Promise<void> {
  try {
    // 使用独立的 exportAxios 实例，返回完整 AxiosResponse<Blob>
    const response: AxiosResponse<Blob> = await exportAxios.get<Blob>(apiPath, {
      params,
      responseType: 'blob',
    });
    // V15 P0-S12：从 Content-Disposition 提取文件名（后端返回 filename="customers_export_xxx.xlsx"）
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
    // V15 P0-S12：错误用 msg 表达（与 exportToExcel 行为一致）
    const errDetail = err instanceof Error ? err.message : msg.translate('exportFailed');
    msg.error('exportFailedReason', { reason: errDetail });
    throw err;
  }
}
