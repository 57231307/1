import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface SalesContract {
  id: number;
  contract_no: string;
  contract_name: string;
  customer_id: number;
  /** 后端 sales_contract::Model.customer_name（Option，左连接可空，models/sales_contract.rs:16） */
  customer_name: string | null;
  contract_type?: string;
  contract_date: string;
  signed_date?: string;
  start_date: string;
  end_date: string;
  effective_date?: string;
  expiry_date?: string;
  /** 后端 sales_contract::Model.total_amount 为 Option<Decimal>（models/sales_contract.rs:17） */
  total_amount: number | null;
  currency: string;
  payment_terms?: string;
  payment_method?: string;
  delivery_date?: string;
  delivery_location?: string;
  /** 词表出自后端 status/bpm_crm_contract.rs 的 contract 模块：仅 draft/active/cancelled，无 pending/completed */
  status: 'draft' | 'active' | 'cancelled';
  items: ContractItem[];
  return_items?: ContractItem[];
  delivery_terms: string;
  /** 后端 sales_contract::Model 无 remarks 列（models/sales_contract.rs 全字段核对），需后端补 remarks */
  remarks?: string;
  created_by: number;
  /** 后端 sales_contract::Model 无创建人名称列（models/sales_contract.rs:26 仅 created_by id），需后端 JOIN users 补 created_by_name */
  created_by_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface ContractItem {
  id: number;
  contract_id: number;
  product_id: number;
  product_name: string;
  product_code: string;
  quantity: number;
  unit: string;
  price: number;
  amount: number;
  remark: string;
  /**
   * 后端 sales_contract_items.quantity_tolerance_pct（可空，NULL=用品类/全局默认）。
   * 出参专用（创建入参另用 CreateContractItemInput，那边是数值），rust_decimal 序列化为字符串，NULL→null。
   * 消费点：useSc.prepareEdit 回显需 Number() 归一后绑定 el-input-number。
   */
  quantity_tolerance_pct: string | null;
}

// 后端 sales_contract_handler::SalesContractQuery（list_contracts 的 Query<T>，全字段 Option、snake_case）
export interface SalesContractQuery {
  keyword?: string;
  status?: string;
  customer_id?: number;
  page?: number;
  page_size?: number;
}

export function getSalesContractList(
  params?: SalesContractQuery
): Promise<ApiResponse<SalesContract[]>> {
  return request.get('/sales/sales-contracts', { params });
}

export function getSalesContract(id: number): Promise<ApiResponse<SalesContract>> {
  return request.get(`/sales/sales-contracts/${id}`);
}

/** 创建合同明细行入参（对齐后端 CreateContractItemDto） */
export interface CreateContractItemInput {
  product_id?: number;
  product_name: string;
  product_spec?: string;
  unit: string;
  quantity: number;
  quantity_tolerance_pct?: number | null;
  unit_price: number;
  delivery_date?: string;
  remarks?: string;
}

/** 创建销售合同入参（对齐后端 CreateSalesContractRequestDto） */
export interface CreateSalesContractPayload {
  contract_no: string;
  contract_name: string;
  customer_id: number;
  total_amount: number;
  contract_type?: string;
  payment_terms?: string;
  delivery_date?: string;
  remark?: string;
  items?: CreateContractItemInput[];
}

export function createSalesContract(
  data: CreateSalesContractPayload
): Promise<ApiResponse<SalesContract>> {
  return request.post('/sales/sales-contracts', data);
}

export function updateSalesContract(
  id: number,
  data: Partial<SalesContract>
): Promise<ApiResponse<SalesContract>> {
  return request.put(`/sales/sales-contracts/${id}`, data);
}

export function deleteSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/sales/sales-contracts/${id}`);
}

export function approveSalesContract(id: number): Promise<ApiResponse<void>> {
  return request.post(`/sales/sales-contracts/${id}/approve`);
}

// 执行销售合同请求体：对齐后端 sales_contract_handler::ExecuteSalesContractRequestDto。
// execution_type / execution_amount 必填（后端仅接受 delivery=出库 / payment=收款）。
// related_bill_type/related_bill_id 为可选关联单据，本表单不采集，避免留死字段。
export interface ExecuteSalesContractRequest {
  execution_type: string;
  execution_amount: number;
  remark?: string;
}

export function executeSalesContract(
  id: number,
  data: ExecuteSalesContractRequest
): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/execute`, data);
}

// 后端 sales_contract_handler::CancelSalesContractRequest 必填 reason（取消原因）
export function cancelSalesContract(id: number, reason: string): Promise<ApiResponse<void>> {
  return request.put(`/sales/sales-contracts/${id}/cancel`, { reason });
}
