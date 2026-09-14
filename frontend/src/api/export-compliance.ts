import { request } from './request';

// 出口商检
export function getExportInspectionList(params?: Record<string, unknown>) {
  return request.get('/export-inspections', { params });
}

export function getExportInspectionDetail(id: number) {
  return request.get(`/export-inspections/${id}`);
}

export function getExportCertificates(inspectionId: number) {
  return request.get(`/export-inspections/${inspectionId}/certificates`);
}

export function getCertificateDetail(id: number) {
  return request.get(`/export-inspections/certificates/${id}`);
}

// 出口退税
export function getCustomsDeclarations(params?: Record<string, unknown>) {
  return request.get('/export-refunds/customs-declarations', { params });
}

export function createCustomsDeclaration(data: Record<string, unknown>) {
  return request.post('/export-refunds/customs-declarations', data);
}

export function verifyDocuments(salesOrderId: number, data?: Record<string, unknown>) {
  return request.post(
    `/export-refunds/sales-orders/${salesOrderId}/documents-verification`,
    data ?? {}
  );
}

export function getRefundCalculation(params: Record<string, unknown>) {
  return request.get('/export-refunds/refund-calculation', { params });
}

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
