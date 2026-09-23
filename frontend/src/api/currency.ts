import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 币种出参 = `currency_handler.rs` 的 `CurrencyResponse`（list/base/set-base 均返回它，
 * 无 rename_all）：id / code / name / symbol / is_base / precision / is_active，均 snake_case。
 */
export interface Currency {
  id: number;
  code: string;
  name: string;
  symbol?: string | null;
  is_base?: boolean | null;
  precision?: number | null;
  is_active?: boolean | null;
}

/**
 * 汇率出参 = `currency_handler.rs` 的 `ExchangeRateResponse`：id / from_currency /
 * to_currency / rate（字符串）/ effective_date（字符串）。DTO 未透出 source 列。
 */
export interface ExchangeRate {
  id: number;
  from_currency: string;
  to_currency: string;
  rate: string;
  effective_date: string;
}

/** 新建币种表单字段（与 CurrencyResponse / currency 表列同名） */
export interface CreateCurrencyRequest {
  code: string;
  name: string;
  symbol?: string;
  is_base: boolean;
  precision: number;
}

/** 新建汇率：字段对齐 `currency_handler.rs` 的 `CreateExchangeRateApiRequest`（无 source 入参） */
export interface CreateExchangeRateRequest {
  from_currency: string;
  to_currency: string;
  rate: number;
  effective_date: string;
}

export function createCurrency(data: CreateCurrencyRequest) {
  return request.post('/currencies', data);
}

export function getCurrencyList(): Promise<ApiResponse<Currency[]>> {
  return request.get('/currencies');
}

export function getBaseCurrency(): Promise<ApiResponse<Currency>> {
  return request.get('/currencies/base');
}

export function setBaseCurrency(id: number): Promise<ApiResponse<Currency>> {
  return request.post(`/currencies/${id}/set-base`);
}

export function createExchangeRate(
  data: CreateExchangeRateRequest
): Promise<ApiResponse<ExchangeRate>> {
  return request.post('/exchange-rates', data);
}

/** 汇率查询：字段对齐 `currency_handler.rs` 的 `GetExchangeRateQuery`（from_currency / to_currency） */
export function getExchangeRate(params: {
  from_currency: string;
  to_currency: string;
}): Promise<ApiResponse<ExchangeRate>> {
  return request.get('/exchange-rates/query', { params });
}
