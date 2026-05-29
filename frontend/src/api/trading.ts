import { request } from './request'

// 采购合同
export interface TradingPurchaseContract {
  id: number
  contract_no: string
  supplier_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const listTradingPurchaseContracts = () => request.get('/trading/purchase-contracts')

export const createTradingPurchaseContract = (data: any) =>
  request.post('/trading/purchase-contracts', data)

export const updateTradingPurchaseContract = (id: number, data: any) =>
  request.put(`/trading/purchase-contracts/${id}`, data)

export const deleteTradingPurchaseContract = (id: number) =>
  request.delete(`/trading/purchase-contracts/${id}`)

export const approveTradingPurchaseContract = (id: number) =>
  request.post(`/trading/purchase-contracts/${id}/approve`)

export const executeTradingPurchaseContract = (id: number) =>
  request.post(`/trading/purchase-contracts/${id}/execute`)

// 采购价格
export interface TradingPurchasePrice {
  id: number
  product_name: string
  supplier_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date: string
  status: string
}

export const listTradingPurchasePrices = () => request.get('/trading/purchase-prices')

export const createTradingPurchasePrice = (data: any) =>
  request.post('/trading/purchase-prices', data)

export const updateTradingPurchasePrice = (id: number, data: any) =>
  request.put(`/trading/purchase-prices/${id}`, data)

export const deleteTradingPurchasePrice = (id: number) =>
  request.delete(`/trading/purchase-prices/${id}`)

export const approveTradingPurchasePrice = (id: number) =>
  request.post(`/trading/purchase-prices/${id}/approve`)

// 销售合同
export interface TradingSalesContract {
  id: number
  contract_no: string
  customer_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const listTradingSalesContracts = () => request.get('/trading/sales-contracts')

export const createTradingSalesContract = (data: any) =>
  request.post('/trading/sales-contracts', data)

export const updateTradingSalesContract = (id: number, data: any) =>
  request.put(`/trading/sales-contracts/${id}`, data)

export const deleteTradingSalesContract = (id: number) =>
  request.delete(`/trading/sales-contracts/${id}`)

export const approveTradingSalesContract = (id: number) =>
  request.post(`/trading/sales-contracts/${id}/approve`)

// 销售价格
export interface TradingSalesPrice {
  id: number
  product_name: string
  customer_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  status: string
}

export const listTradingSalesPrices = () => request.get('/trading/sales-prices')

export const createTradingSalesPrice = (data: any) => request.post('/trading/sales-prices', data)

export const updateTradingSalesPrice = (id: number, data: any) =>
  request.put(`/trading/sales-prices/${id}`, data)

export const deleteTradingSalesPrice = (id: number) => request.delete(`/trading/sales-prices/${id}`)

export const approveTradingSalesPrice = (id: number) =>
  request.post(`/trading/sales-prices/${id}/approve`)

// 销售退货
export interface TradingSalesReturn {
  id: number
  return_no: string
  customer_name: string
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status: string
}

export const listTradingSalesReturns = () => request.get('/trading/sales-returns')

export const createTradingSalesReturn = (data: any) => request.post('/trading/sales-returns', data)

export const updateTradingSalesReturn = (id: number, data: any) =>
  request.put(`/trading/sales-returns/${id}`, data)

export const deleteTradingSalesReturn = (id: number) =>
  request.delete(`/trading/sales-returns/${id}`)
