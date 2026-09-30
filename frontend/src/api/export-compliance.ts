import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 出口商检与出口退税：合并自 tax-rebate.ts / export-inspection.ts（统一出口，避免重复实现）
// 商检端点返回裸 JSON（无 ApiResponse 信封），由 export-inspection.ts 的独立 axios 实例处理；
// 退税端点带 ApiResponse 信封，由 tax-rebate.ts 处理。
export {
  getExportInspectionList,
  getExportInspection,
  getExportInspectionPrintUrl,
  getCustomsDeclarationPrintUrl,
  inspectionResultTagMap,
  getExportCertificates,
  getCertificateDetail,
} from './export-inspection';
export {
  createCustomsDeclaration,
  verifyDocumentsCompleteness,
  calculateRefund,
  generateRefundDeclaration,
  getRefundDeclarationList,
  getRefundDeclarationPrintUrl,
} from './tax-rebate';

// 国际贸易术语
export function getPriceComposition(quotationId: number) {
  return request.get(`/incoterms/quotations/${quotationId}/price-composition`);
}

export function calculateIncotermCost(data: Record<string, unknown>) {
  return request.post('/incoterms/cost-calculation', data);
}

export function getIncotermUsageReport() {
  return request.get('/incoterms/usage-report');
}

// 环保税
/** 后端 PeriodQuery 必填两键（environmental_tax_handler.rs:19-22，i32 非 Option） */
export interface DischargePeriodParams {
  period_year: number;
  period_month: number;
}

/** 对齐后端 CreateDischargeRecordRequest（services/environmental_tax_service.rs:23-35）；created_by 由后端从会话注入，前端不发送 */
export interface CreateDischargeRecordPayload {
  /** 词表：wastewater / exhaust / solid_waste（service :179-187） */
  discharge_type: string;
  pollutant_name: string;
  discharge_amount: number;
  discharge_unit?: string;
  concentration?: number;
  concentration_unit?: string;
  period_year: number;
  period_month: number;
  monitoring_point?: string;
  remarks?: string;
}

/** data 为裸数组 Vec<pollutant_discharge_record::Model>（handler :42-46），非分页信封 */
export function getDischargeRecords(params: DischargePeriodParams) {
  return request.get<ApiResponse<Array<Record<string, unknown>>>>(
    '/environmental-tax/discharge-records',
    { params }
  );
}

export function createDischargeRecord(data: CreateDischargeRecordPayload) {
  return request.post<ApiResponse<Record<string, unknown>>>(
    '/environmental-tax/discharge-records',
    data
  );
}

/** data 为裸数组 Vec<EnvironmentalTaxResult>（handler :50-60） */
export function getTaxDeclaration(params: DischargePeriodParams) {
  return request.get<ApiResponse<Array<Record<string, unknown>>>>(
    '/environmental-tax/tax-declarations',
    { params }
  );
}
