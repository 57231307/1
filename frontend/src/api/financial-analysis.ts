import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 报告出参 = financial_indicators 表（列表 list_reports 手挑键 name/indicator_code/
 * indicator_type/status/created_at，详情 get_report 追加 formula/unit/remark/updated_at，
 * 见 backend/src/handlers/financial_analysis_handler.rs）。
 * `period` / `executed_at` 仅出现在 execute_report 响应里，列表/详情并不返回，
 * 当前恒缺（需后端把执行期间回写到列表才可显示）。
 */
export interface FinancialReport {
  id: number;
  name: string;
  indicator_code: string;
  indicator_type: string;
  status: string;
  created_at: string;
  formula?: string | null;
  unit?: string | null;
  remark?: string | null;
  updated_at?: string;
  period?: string;
  executed_at?: string;
}

/** 创建报告请求 = `financial_analysis_handler.rs` 的 `CreateReportRequest`（period_start/period_end 必填） */
export interface CreateReportRequest {
  name: string;
  report_type: string;
  period_start: string;
  period_end: string;
  indicators?: number[];
  description?: string;
}

/** 更新报告请求 = `financial_analysis_handler.rs` 的 `UpdateReportRequest` */
export interface UpdateReportRequest {
  name?: string;
  report_type?: string;
  description?: string;
  status?: string;
}

/** 指标出参 = `financial_analysis::Model`（create_indicator 直接 to_value 实体） */
export interface FinancialIndicator {
  id: number;
  indicator_name: string;
  indicator_code: string;
  indicator_type: string;
  formula?: string | null;
  unit?: string | null;
  status: string;
  remark?: string | null;
  created_at: string;
  updated_at: string;
}

/** 新建指标请求 = `financial_analysis_handler.rs` 的 `CreateIndicatorDto`（无 target_value 入参） */
export interface CreateIndicatorRequest {
  name: string;
  code?: string;
  indicator_type?: string;
  formula?: string;
  unit?: string;
  description?: string;
}

/** 趋势出参 = `financial_analysis_result::Model`（get_trends 返回裸模型数组） */
export interface FinancialTrend {
  id: number;
  analysis_type: string;
  period: string;
  indicator_id: number;
  indicator_value: string;
  target_value?: string | null;
  variance?: string | null;
  variance_rate?: string | null;
  trend?: string | null;
  analysis_date?: string | null;
  created_by?: number | null;
  created_at: string;
}

/** 列表查询参数 = list_reports 实际只读 page/page_size */
export interface ReportQueryParams {
  page?: number;
  page_size?: number;
}

/** 趋势查询参数 = `financial_analysis_handler.rs` 的 `TrendQueryParams`（indicator_id 必填，缺失后端 400） */
export interface TrendQueryParams {
  indicator_id?: number;
  start_date?: string;
  end_date?: string;
  period?: string;
  page_size?: number;
}

interface PagedReports {
  items: FinancialReport[];
  total: number;
  page: number;
  page_size: number;
}

export const getReportList = (params?: ReportQueryParams) =>
  request.get<ApiResponse<PagedReports>>('/financial-analysis/reports', { params });

export const createReport = (data: CreateReportRequest) =>
  request.post<ApiResponse<FinancialReport>>('/financial-analysis/reports', data);

export const updateReport = (id: number, data: UpdateReportRequest) =>
  request.put<ApiResponse<FinancialReport>>(`/financial-analysis/reports/${id}`, data);

export const deleteReport = (id: number) => request.delete(`/financial-analysis/reports/${id}`);

export const executeFinancialReport = (id: number) =>
  request.post(`/financial-analysis/reports/${id}/execute`);

export const getFinancialReport = (id: number) =>
  request.get<ApiResponse<FinancialReport>>(`/financial-analysis/reports/${id}`);

/**
 * 执行财务分析报告。后端 financial_analysis_handler.rs 用
 * `Query<ExecuteReportParams { period: Option<String> }>`（period 为 YYYY-MM，
 * 缺失时后端取当前年月），不接收请求体。
 */
export const executeReportWithParams = (reportId: number, period?: string) =>
  request.post<ApiResponse<Record<string, unknown>>>(
    `/financial-analysis/reports/${reportId}/execute`,
    undefined,
    { params: period ? { period } : {} }
  );

export const createFinancialIndicator = (data: CreateIndicatorRequest) =>
  request.post<ApiResponse<FinancialIndicator>>('/financial-analysis/indicators', data);

export const getFinancialTrends = (params?: TrendQueryParams) =>
  request.get<ApiResponse<{ items: FinancialTrend[]; total: number }>>(
    '/financial-analysis/trends',
    {
      params,
    }
  );
