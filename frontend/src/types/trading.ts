/**
 * 贸易模块业务类型定义
 */

/**
 * 审批状态
 * FE-P2-1 修复（批次 388 v13 复审）：原 status?: string 过于宽泛，收窄为字面量联合类型
 */
export type ApprovalStatus =
  | 'draft'
  | 'pending'
  | 'approved'
  | 'rejected'
  | 'cancelled'
  | 'expired'
  | 'active'
  | 'inactive'

/**
 * 币种
 * FE-P2-1 修复（批次 388 v13 复审）：原 currency: string 过于宽泛，收窄为标准 ISO 4217 子集
 */
export type Currency = 'CNY' | 'USD' | 'EUR' | 'GBP' | 'JPY' | 'HKD'

/**
 * 计量单位
 * FE-P2-1 修复（批次 388 v13 复审）：原 unit: string 过于宽泛，收窄为面料行业常用单位
 */
export type Unit = 'meter' | 'kg' | 'piece' | 'roll' | 'box' | 'yard'

/** 采购合同创建请求 */
export interface PurchaseContractCreateRequest {
  contract_no: string
  supplier_name: string
  contract_date: string
  total_amount: number
  status?: ApprovalStatus
  remark?: string
}

/** 采购合同更新请求 */
export interface PurchaseContractUpdateRequest {
  contract_no?: string
  supplier_name?: string
  contract_date?: string
  total_amount?: number
  status?: ApprovalStatus
  remark?: string
}

/** 采购价格创建请求 */
export interface PurchasePriceCreateRequest {
  product_name: string
  supplier_name: string
  price: number
  currency: Currency
  unit: Unit
  effective_date: string
  expiry_date: string
  status?: ApprovalStatus
}

/** 采购价格更新请求 */
export interface PurchasePriceUpdateRequest {
  product_name?: string
  supplier_name?: string
  price?: number
  currency?: Currency
  unit?: Unit
  effective_date?: string
  expiry_date?: string
  status?: ApprovalStatus
}

/** 销售合同创建请求 */
export interface SalesContractCreateRequest {
  contract_no: string
  customer_name: string
  contract_date: string
  total_amount: number
  status?: ApprovalStatus
  remark?: string
}

/** 销售合同更新请求 */
export interface SalesContractUpdateRequest {
  contract_no?: string
  customer_name?: string
  contract_date?: string
  total_amount?: number
  status?: ApprovalStatus
  remark?: string
}

/** 销售价格创建请求 */
export interface SalesPriceCreateRequest {
  product_name: string
  customer_name: string
  price: number
  currency: Currency
  unit: Unit
  effective_date: string
  status?: ApprovalStatus
}

/** 销售价格更新请求 */
export interface SalesPriceUpdateRequest {
  product_name?: string
  customer_name?: string
  price?: number
  currency?: Currency
  unit?: Unit
  effective_date?: string
  status?: ApprovalStatus
}

/** 销售退货创建请求 */
export interface SalesReturnCreateRequest {
  return_no: string
  customer_name: string
  order_no: string
  return_date: string
  total_amount: number
  reason: string
  status?: ApprovalStatus
}

/** 销售退货更新请求 */
export interface SalesReturnUpdateRequest {
  return_no?: string
  customer_name?: string
  order_no?: string
  return_date?: string
  total_amount?: number
  reason?: string
  status?: ApprovalStatus
}
