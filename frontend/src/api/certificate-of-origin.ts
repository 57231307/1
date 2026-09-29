import axios from 'axios';

/**
 * 出口产地证域 API
 *
 * 后端路由（backend/src/routes/export_inspection_routes.rs，nest 前缀 /api/v1/erp/export-inspections）：
 * - GET  /{inspection_id}/certificates  按商检单查询产地证列表（分页）
 * - GET  /certificates/{id}             产地证详情
 * - GET  /certificates/{id}/print       产地证 DOCX 打印
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

/** 出口产地证（对齐 backend/src/models/certificate_of_origin.rs） */
export interface CertificateOfOrigin {
  id: number;
  certificate_no: string;
  inspection_id: number | null;
  product_name: string;
  hs_code: string;
  origin_country: string;
  destination_country: string;
  quantity: string | number;
  unit: string;
  invoice_amount: string | number | null;
  certificate_type: string;
  issue_date: string;
  expiry_date: string | null;
  status: string;
  remarks: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface CertificateOfOriginListQuery {
  inspection_id?: number;
  status?: string;
  page?: number;
  page_size?: number;
}

export interface CertificateOfOriginPage {
  items: CertificateOfOrigin[];
  total: number;
}

/** 产地证状态 → 标签类型映射 */
export const certificateStatusTagMap: Record<string, 'info' | 'warning' | 'success' | 'danger'> = {
  active: 'success',
  revoked: 'danger',
  expired: 'warning',
};

/** 获取产地证列表 */
export function getCertificateOfOriginList(
  params?: CertificateOfOriginListQuery
): Promise<CertificateOfOriginPage> {
  return bareApi
    .get<CertificateOfOriginPage>('/export-inspections/certificates', { params })
    .then(r => r.data);
}

/** 获取产地证详情 */
export function getCertificateOfOrigin(id: number): Promise<CertificateOfOrigin> {
  return bareApi
    .get<CertificateOfOrigin>(`/export-inspections/certificates/${id}`)
    .then(r => r.data);
}

/** 获取某商检单下的产地证列表 */
export function getCertificatesByInspection(
  inspectionId: number,
  params?: CertificateOfOriginListQuery
): Promise<CertificateOfOriginPage> {
  return bareApi
    .get<CertificateOfOriginPage>(`/export-inspections/${inspectionId}/certificates`, { params })
    .then(r => r.data);
}

/** 产地证打印页地址（DOCX 下载） */
export function getCertificateOfOriginPrintUrl(id: number): string {
  return `${BASE_URL}/export-inspections/certificates/${id}/print`;
}
