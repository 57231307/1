import { request } from './request'

export interface ScanToShipRequest {
  barcode: string
  order_id: number
}

export interface ScanToShipResponse {
  message: string
  barcode: string
  piece_no: string
}

export interface ScanResult {
  success: boolean
  message: string
  data?: ScanData
}

export interface ScanData {
  barcode: string
  piece_no: string
  product_id: number
  product_name: string
  batch_no: string
  color_no: string
  grade: string
  quantity_meters: number
  quantity_kg: number
  warehouse_id: number
  warehouse_name: string
  status: string
}

export interface ScanHistory {
  id: number
  barcode: string
  piece_no: string
  scan_type: string
  result: string
  created_at: string
}

export interface ScanHistoryListResponse {
  items: ScanHistory[]
  total: number
  page: number
  page_size: number
}

export function scanToShip(data: ScanToShipRequest) {
  return request.post('/scanner/scan-to-ship', data)
}

export function scanInventory(barcode: string) {
  return request.get('/scanner/scan-inventory', { params: { barcode } })
}

export function getScanHistory(page?: number, pageSize?: number) {
  return request.get('/scanner/history', {
    params: {
      page: page || 0,
      page_size: pageSize || 20,
    },
  })
}

export function getScanStatistics() {
  return request.get('/scanner/scan-statistics')
}
