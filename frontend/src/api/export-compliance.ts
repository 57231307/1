import { request } from './request';

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

export function calculatePriceComposition(quotationId: number, data: Record<string, unknown>) {
  return request.post(`/incoterms/quotations/${quotationId}/price-composition`, data);
}

export function calculateIncotermCost(data: Record<string, unknown>) {
  return request.post('/incoterms/cost-calculation', data);
}

export function getIncotermUsageReport() {
  return request.get('/incoterms/usage-report');
}

// 环保税
export function getDischargeRecords(params?: Record<string, unknown>) {
  return request.get('/environmental-tax/discharge-records', { params });
}

export function createDischargeRecord(data: Record<string, unknown>) {
  return request.post('/environmental-tax/discharge-records', data);
}

export function getTaxDeclaration(params: Record<string, unknown>) {
  return request.get('/environmental-tax/tax-declarations', { params });
}
