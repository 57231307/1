// 导入模板配套下载与列说明取数。调用方 各域导入对话框（SKU 对照表、CRM 线索）。
// 入参 importType 为后端 ImportExportService::get_import_template 登记的类型名。
// 传给谁 downloadImportTemplate 落浏览器文件，getImportTemplateMeta 供界面渲染列清单与必填标记。
// 返回的列集合来自后端权威解析表，界面不得另抄一份列名。
import { request } from './request';
import type { ApiResponse } from '@/types/api-response';

/// 导入模板单列的元数据，字段与后端 ImportColumnDef 序列化同名。
export interface ImportTemplateColumn {
  field: string;
  title: string;
  data_type: string;
  required: boolean;
  example: string | null;
}

/// 导入模板元数据信封，字段与后端 describe_import_template 返回同名。
export interface ImportTemplateMeta {
  import_type: string;
  name: string;
  description: string;
  columns: ImportTemplateColumn[];
}

/** 下载指定类型的导入模板，返回真实 xlsx 文件流 */
export function downloadImportTemplate(importType: string): Promise<Blob> {
  return request.get<Blob>(`/templates/download/${importType}`, { responseType: 'blob' });
}

/** 取指定类型导入模板的列清单与必填标记，用于界面列说明 */
export function getImportTemplateMeta(
  importType: string
): Promise<ApiResponse<ImportTemplateMeta>> {
  return request.get<ApiResponse<ImportTemplateMeta>>(`/templates/meta/${importType}`);
}

/** 触发浏览器保存模板文件。入参 blob 与文件名，传给谁 浏览器下载管理器 */
export function saveTemplateBlob(blob: Blob, fileName: string): void {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = fileName;
  document.body.appendChild(link);
  link.click();
  document.body.removeChild(link);
  URL.revokeObjectURL(url);
}
