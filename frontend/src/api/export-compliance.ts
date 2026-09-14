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

// 出口退税（端点方法对齐 routes/export_refund.rs）
export function createCustomsDeclaration(data: Record<string, unknown>) {
  return request.post('/export-refunds/customs-declarations', data);
}

export function verifyDocuments(salesOrderId: number, params?: Record<string, unknown>) {
  return request.get(`/export-refunds/sales-orders/${salesOrderId}/documents-verification`, {
    params,
  });
}

export function calculateRefund(data: Record<string, unknown>) {
  return request.post('/export-refunds/refund-calculation', data);
}

export function generateRefundDeclaration(data: Record<string, unknown>) {
  return request.post('/export-refunds/refund-declarations', data);
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
