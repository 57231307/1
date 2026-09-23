// trading-contract.ts - 交易合同 API 桩（统一采购/销售合同）
// 来源：拆分原 trading/index.vue 时统一接口调用而创建
// 后端真实路由：
//   采购合同 /purchase/purchase-contracts（purchase.rs purchase_contracts()）
//   销售合同 /sales/sales-contracts（sales.rs sales_contracts()）
//   旧 /trading/* 域仅保留 5 个 GET 列表端点（analytics.rs trading()）
import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface TradingContract {
  id: number;
  contract_no: string;
  supplier_name?: string;
  customer_name?: string;
  contract_date: string;
  total_amount: number;
  status: string;
}

export interface ListTradingContractParams {
  type: 'purchase' | 'sales';
}

export const getTradingContractList = (params: ListTradingContractParams) => {
  if (params.type === 'purchase') {
    return request.get<ApiResponse<TradingContract[]>>('/trading/purchase-contracts');
  }
  return request.get<ApiResponse<TradingContract[]>>('/trading/sales-contracts');
};

export const getTradingContract = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.get<ApiResponse<TradingContract>>(`/purchase/purchase-contracts/${id}`)
    : request.get<ApiResponse<TradingContract>>(`/sales/sales-contracts/${id}`);

export const createTradingContract = (
  data: Partial<TradingContract> & { type: 'purchase' | 'sales' }
) => {
  if (data.type === 'purchase') {
    return request.post<ApiResponse<TradingContract>>('/purchase/purchase-contracts', data);
  }
  return request.post<ApiResponse<TradingContract>>('/sales/sales-contracts', data);
};

export const updateTradingContract = (
  id: number,
  data: Partial<TradingContract>,
  type: 'purchase' | 'sales'
) =>
  type === 'purchase'
    ? request.put<ApiResponse<TradingContract>>(`/purchase/purchase-contracts/${id}`, data)
    : request.put<ApiResponse<TradingContract>>(`/sales/sales-contracts/${id}`, data);

export const deleteTradingContract = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.delete<ApiResponse<null>>(`/purchase/purchase-contracts/${id}`)
    : request.delete<ApiResponse<null>>(`/sales/sales-contracts/${id}`);

export const approveTradingContract = (id: number, type: 'purchase' | 'sales') =>
  type === 'purchase'
    ? request.post<ApiResponse<TradingContract>>(`/purchase/purchase-contracts/${id}/approve`)
    : request.post<ApiResponse<TradingContract>>(`/sales/sales-contracts/${id}/approve`);

// 后端 execute 端点为 PUT，两侧 DTO 不同：
//   采购 purchase_contract_handler::ExecuteContractRequestDto：execution_type/execution_amount/execution_date 必填
//   销售 sales_contract_handler::ExecuteSalesContractRequestDto：execution_type/execution_amount 必填（无日期）
// related_bill_type/related_bill_id/remark 为 Option，本表单不采集即 None；两分支各自只发后端真实读取的键。
export interface ExecuteContractPayload {
  execution_type: string;
  execution_amount: number;
  execution_date?: string;
}

export const executeTradingContract = (
  id: number,
  type: 'purchase' | 'sales',
  data: ExecuteContractPayload
) =>
  type === 'purchase'
    ? request.put<ApiResponse<TradingContract>>(`/purchase/purchase-contracts/${id}/execute`, {
        execution_type: data.execution_type,
        execution_amount: data.execution_amount,
        execution_date: data.execution_date,
      })
    : request.put<ApiResponse<TradingContract>>(`/sales/sales-contracts/${id}/execute`, {
        execution_type: data.execution_type,
        execution_amount: data.execution_amount,
      });
