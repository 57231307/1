import axios from 'axios';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 出口商检域 API
 *
 * 后端路由（backend/src/routes/export_inspection_routes.rs，nest 前缀 /api/v1/erp/export-inspections）：
 * - GET  /                商检单列表（分页）
 * - GET  /{id}            商检单详情
 * - GET  /{id}/print      商检单 DOCX 打印
 * - GET  /{id}/customs-declaration/print  报关单 DOCX 打印
 *
 * 出参约定：
 * - 商检单列表/详情（export_inspection_handler）走全站统一成功信封 `ApiResponse`，
 *   data 分别是 PaginatedResponse{items,total,page,page_size} 与实体对象，需解包取 data；
 * - 产地证端点（certificate_of_origin_handler）目前仍返回裸 JSON，其读取函数直接取 data。
 *
 * 本文件沿用独立 axios 实例（bareApi）：不挂全局响应拦截器，`r.data` 即完整响应体，
 * 由调用方按上述约定自行取载荷（信封端点取 r.data.data，裸 JSON 端点取 r.data）。
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

/**
 * 商检结果值 → el-tag 色的展示层映射（非后端接口字段，键取自实体 result 列的取值域）。
 * 与后端契约 interface 分离声明：本常量是把 `ExportInspection.result` 的取值渲染成
 * 标签颜色，键是 result 的词表值而非出参/入参字段名。
 */
export const inspectionResultTagMap: Record<string, 'info' | 'warning' | 'success' | 'danger'> = {
  pending: 'info',
  pass: 'success',
  qualified: 'success',
  fail: 'danger',
  unqualified: 'danger',
};

/** 获取商检单列表（统一信封 ApiResponse<PaginatedResponse>，解包取 data 的分页载荷） */
export function getExportInspectionList(
  params?: ExportInspectionListQuery
): Promise<ExportInspectionPage> {
  return bareApi
    .get<ApiResponse<ExportInspectionPage>>('/export-inspections', { params })
    .then(r => r.data.data);
}

/** 获取商检单详情（统一信封 ApiResponse，解包取 data 的实体载荷） */
export function getExportInspection(id: number): Promise<ExportInspection> {
  return bareApi
    .get<ApiResponse<ExportInspection>>(`/export-inspections/${id}`)
    .then(r => r.data.data);
}

/** 商检单打印页地址（DOCX 下载，Cookie 随请求自动携带） */
export function getExportInspectionPrintUrl(id: number): string {
  return `${BASE_URL}/export-inspections/${id}/print`;
}

/** 报关单打印页地址（DOCX 下载） */
export function getCustomsDeclarationPrintUrl(id: number): string {
  return `${BASE_URL}/export-inspections/${id}/customs-declaration/print`;
}

/** 产地证列表（GET /{id}/certificates，certificate_of_origin_handler 仍为裸 JSON） */
export function getExportCertificates(
  inspectionId: number
): Promise<{ items: unknown[]; total?: number }> {
  return bareApi.get(`/${inspectionId}/certificates`).then(r => r.data);
}

/** 产地证详情（GET /certificates/{id}，certificate_of_origin_handler 仍为裸 JSON） */
export function getCertificateDetail(id: number): Promise<unknown> {
  return bareApi.get(`/certificates/${id}`).then(r => r.data);
}

/** 商检单分页载荷别名：与后端 PaginatedResponse<export_inspection::Model> 的 data 内层逐字段对齐 */
export type ExportInspectionPage = PaginatedResponse<ExportInspection>;
