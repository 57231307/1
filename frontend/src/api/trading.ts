import request from './request'

// 采购合同
export interface PurchaseContract {
  id: number
  contract_no: string
  supplier_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const listPurchaseContracts = () => 
  request.get('/trading/purchase-contracts')

export const createPurchaseContract = (data: any) => 
  request.post('/trading/purchase-contracts', data)

export const updatePurchaseContract = (id: number, data: any) => 
  request.put(`/trading/purchase-contracts/${id}`, data)

export const deletePurchaseContract = (id: number) => 
  request.delete(`/trading/purchase-contracts/${id}`)

export const approvePurchaseContract = (id: number) => 
  request.post(`/trading/purchase-contracts/${id}/approve`)

export const executePurchaseContract = (id: number) => 
  request.post(`/trading/purchase-contracts/${id}/execute`)

// 采购价格
export interface PurchasePrice {
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

export const listPurchasePrices = () => 
  request.get('/trading/purchase-prices')

export const createPurchasePrice = (data: any) => 
  request.post('/trading/purchase-prices', data)

export const updatePurchasePrice = (id: number, data: any) => 
  request.put(`/trading/purchase-prices/${id}`, data)

export const deletePurchasePrice = (id: number) => 
  request.delete(`/trading/purchase-prices/${id}`)

export const approvePurchasePrice = (id: number) => 
  request.post(`/trading/purchase-prices/${id}/approve`)

// 销售合同
export interface SalesContract {
  id: number
  contract_no: string
  customer_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const listSalesContracts = () => 
  request.get('/trading/sales-contracts')

export const createSalesContract = (data: any) => 
  request.post('/trading/sales-contracts', data)

export const updateSalesContract = (id: number, data: any) => 
  request.put(`/trading/sales-contracts/${id}`, data)

export const deleteSalesContract = (id: number) => 
  request.delete(`/trading/sales-contracts/${id}`)

export const approveSalesContract = (id: number) => 
  request.post(`/trading/sales-contracts/${id}/approve`)

// 销售价格
export interface SalesPrice {
  id: number
  product_name: string
  customer_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  status: string
}

export const listSalesPrices = () => 
  request.get('/trading/sales-prices')

export const createSalesPrice = (data: any) => 
  request.post('/trading/sales-prices', data)

export const updateSalesPrice = (id: number, data: any) => 
  request.put(`/trading/sales-prices/${id}`, data)

export const deleteSalesPrice = (id: number) => 
  request.delete(`/trading/sales-prices/${id}`)

export const approveSalesPrice = (id: number) => 
  request.post(`/trading/sales-prices/${id}/approve`)

// 销售退货
export interface SalesReturn {
  id: number
  return_no: string
  customer_name: string
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status: string
}

export const listSalesReturns = () => 
  request.get('/trading/sales-returns')

export const createSalesReturn = (data: any) => 
  request.post('/trading/sales-returns', data)

export const updateSalesReturn = (id: number, data: any) => 
  request.put(`/trading/sales-returns/${id}`, data)

export const deleteSalesReturn = (id: number) => 
  request.delete(`/trading/sales-returns/${id}`)
