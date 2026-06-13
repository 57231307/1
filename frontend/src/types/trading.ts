/**
 * 贸易模块业务类型定义
 */

/** 采购合同创建请求 */
export interface PurchaseContractCreateRequest {
  contract_no: string
  supplier_name: string
  contract_date: string
  total_amount: number
  status?: string
  remark?: string
}

/** 采购合同更新请求 */
export interface PurchaseContractUpdateRequest {
  contract_no?: string
  supplier_name?: string
  contract_date?: string
  total_amount?: number
  status?: string
  remark?: string
}

/** 采购价格创建请求 */
export interface PurchasePriceCreateRequest {
  product_name: string
  supplier_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  expiry_date: string
  status?: string
}

/** 采购价格更新请求 */
export interface PurchasePriceUpdateRequest {
  product_name?: string
  supplier_name?: string
  price?: number
  currency?: string
  unit?: string
  effective_date?: string
  expiry_date?: string
  status?: string
}

/** 销售合同创建请求 */
export interface SalesContractCreateRequest {
  contract_no: string
  customer_name: string
  contract_date: string
  total_amount: number
  status?: string
  remark?: string
}

/** 销售合同更新请求 */
export interface SalesContractUpdateRequest {
  contract_no?: string
  customer_name?: string
  contract_date?: string
  total_amount?: number
  status?: string
  remark?: string
}

/** 销售价格创建请求 */
export interface SalesPriceCreateRequest {
  product_name: string
  customer_name: string
  price: number
  currency: string
  unit: string
  effective_date: string
  status?: string
}

/** 销售价格更新请求 */
export interface SalesPriceUpdateRequest {
  product_name?: string
  customer_name?: string
  price?: number
  currency?: string
  unit?: string
  effective_date?: string
  status?: string
}

/** 销售退货创建请求 */
export interface SalesReturnCreateRequest {
  return_no: string
  customer_name: string
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status?: string
}

/** 销售退货更新请求 */
export interface SalesReturnUpdateRequest {
  return_no?: string
  customer_name?: string
  order_no?: string
  return_date?: string
  total_amount?: number
  reason?: string
  status?: string
}
