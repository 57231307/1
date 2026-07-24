import { request } from './request'
import type { ApiResponse } from '@/types/api'
import type {
  PurchaseContractCreateRequest,
  PurchaseContractUpdateRequest,
  PurchasePriceCreateRequest,
  PurchasePriceUpdateRequest,
  SalesContractCreateRequest,
  SalesContractUpdateRequest,
  SalesPriceCreateRequest,
  SalesPriceUpdateRequest,
  SalesReturnCreateRequest,
  SalesReturnUpdateRequest,
} from '@/types/trading'

// 采购合同
export interface TradingPurchaseContract {
  id: number
  contract_no: string
  supplier_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const getTradingPurchaseContractList = () =>
  request.get<ApiResponse<TradingPurchaseContract[]>>('/trading/purchase-contracts')

export const createTradingPurchaseContract = (data: PurchaseContractCreateRequest) =>
  request.post<ApiResponse<TradingPurchaseContract>>('/trading/purchase-contracts', data)

export const updateTradingPurchaseContract = (id: number, data: PurchaseContractUpdateRequest) =>
  request.put<ApiResponse<TradingPurchaseContract>>(`/trading/purchase-contracts/${id}`, data)

export const deleteTradingPurchaseContract = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/purchase-contracts/${id}`)

export const approveTradingPurchaseContract = (id: number) =>
  request.post<ApiResponse<TradingPurchaseContract>>(`/trading/purchase-contracts/${id}/approve`)

export const executeTradingPurchaseContract = (id: number) =>
  request.post<ApiResponse<TradingPurchaseContract>>(`/trading/purchase-contracts/${id}/execute`)

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

export const getTradingPurchasePriceList = () =>
  request.get<ApiResponse<TradingPurchasePrice[]>>('/trading/purchase-prices')

export const createTradingPurchasePrice = (data: PurchasePriceCreateRequest) =>
  request.post<ApiResponse<TradingPurchasePrice>>('/trading/purchase-prices', data)

export const updateTradingPurchasePrice = (id: number, data: PurchasePriceUpdateRequest) =>
  request.put<ApiResponse<TradingPurchasePrice>>(`/trading/purchase-prices/${id}`, data)

export const deleteTradingPurchasePrice = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/purchase-prices/${id}`)

export const approveTradingPurchasePrice = (id: number) =>
  request.post<ApiResponse<TradingPurchasePrice>>(`/trading/purchase-prices/${id}/approve`)

// 销售合同
export interface TradingSalesContract {
  id: number
  contract_no: string
  customer_name: string
  contract_date: string
  total_amount: number
  status: string
}

export const getTradingSalesContractList = () =>
  request.get<ApiResponse<TradingSalesContract[]>>('/trading/sales-contracts')

export const createTradingSalesContract = (data: SalesContractCreateRequest) =>
  request.post<ApiResponse<TradingSalesContract>>('/trading/sales-contracts', data)

export const updateTradingSalesContract = (id: number, data: SalesContractUpdateRequest) =>
  request.put<ApiResponse<TradingSalesContract>>(`/trading/sales-contracts/${id}`, data)

export const deleteTradingSalesContract = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/sales-contracts/${id}`)

export const approveTradingSalesContract = (id: number) =>
  request.post<ApiResponse<TradingSalesContract>>(`/trading/sales-contracts/${id}/approve`)

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

export const getTradingSalesPriceList = () =>
  request.get<ApiResponse<TradingSalesPrice[]>>('/trading/sales-prices')

export const createTradingSalesPrice = (data: SalesPriceCreateRequest) =>
  request.post<ApiResponse<TradingSalesPrice>>('/trading/sales-prices', data)

export const updateTradingSalesPrice = (id: number, data: SalesPriceUpdateRequest) =>
  request.put<ApiResponse<TradingSalesPrice>>(`/trading/sales-prices/${id}`, data)

export const deleteTradingSalesPrice = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/sales-prices/${id}`)

export const approveTradingSalesPrice = (id: number) =>
  request.post<ApiResponse<TradingSalesPrice>>(`/trading/sales-prices/${id}/approve`)

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

export const getTradingSalesReturnList = () =>
  request.get<ApiResponse<TradingSalesReturn[]>>('/trading/sales-returns')

export const createTradingSalesReturn = (data: SalesReturnCreateRequest) =>
  request.post<ApiResponse<TradingSalesReturn>>('/trading/sales-returns', data)

export const updateTradingSalesReturn = (id: number, data: SalesReturnUpdateRequest) =>
  request.put<ApiResponse<TradingSalesReturn>>(`/trading/sales-returns/${id}`, data)

export const deleteTradingSalesReturn = (id: number) =>
  request.delete<ApiResponse<null>>(`/trading/sales-returns/${id}`)
