import axios from 'axios';

/**
 * 出口商检域 API
 *
 * 后端路由（backend/src/routes/export_inspection_routes.rs，nest 前缀 /api/v1/erp/export-inspections）：
 * - GET  /                商检单列表（分页）
 * - GET  /{id}            商检单详情
 * - GET  /{id}/print      商检单 DOCX 打印
 * - GET  /{id}/customs-declaration/print  报关单 DOCX 打印
 *
 * 注意：该域 handler 直接返回裸 JSON（{ items, total } / 模型对象），
 * 无 ApiResponse code 信封，故使用独立 axios 实例绕过全局响应拦截器的 code 校验。
 */
const BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api/v1/erp';

const bareApi = axios.create({
  baseURL: BASE_URL,
  timeout: 30000,
  withCredentials: true,
});

/** 出口商检记录（对齐 backend/src/models/export_inspection.rs） */
export interface ExportInspection {
  id: number;
  inspection_no: string;
  sales_order_id: number;
  delivery_id: number | null;
  product_name: string;
  hs_code: string;
  inspection_type: string;
  inspection_agency: string;
  inspection_date: string;
  result: string;
  report_url: string | null;
  certificate_no: string | null;
  certificate_expiry: string | null;
  remarks: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface ExportInspectionListQuery {
  sales_order_id?: number;
  inspection_no?: string;
  result?: string;
  page?: number;
  page_size?: number;
}

export interface ExportInspectionPage {
  items: ExportInspection[];
  total: number;
}

/** 商检单结果 → 标签类型映射 */
export const inspectionResultTagMap: Record<string, 'info' | 'warning' | 'success' | 'danger'> = {
  pending: 'info',
  pass: 'success',
  qualified: 'success',
  fail: 'danger',
  unqualified: 'danger',
};

/** 获取商检单列表 */
export function getExportInspectionList(
  params?: ExportInspectionListQuery
): Promise<ExportInspectionPage> {
  return bareApi.get<ExportInspectionPage>('/export-inspections', { params }).then(r => r.data);
}

/** 获取商检单详情 */
export function getExportInspection(id: number): Promise<ExportInspection> {
  return bareApi.get<ExportInspection>(`/export-inspections/${id}`).then(r => r.data);
}

/** 商检单打印页地址（DOCX 下载，Cookie 随请求自动携带） */
export function getExportInspectionPrintUrl(id: number): string {
  return `${BASE_URL}/export-inspections/${id}/print`;
}

/** 报关单打印页地址（DOCX 下载） */
export function getCustomsDeclarationPrintUrl(id: number): string {
  return `${BASE_URL}/export-inspections/${id}/customs-declaration/print`;
}
