import { request } from './request'

export interface Currency {
  id?: number
  code: string
  name: string
  symbol?: string
  isBase: boolean
  precision: number
  isActive: boolean
}

export interface ExchangeRate {
  id?: number
  fromCurrency: string
  toCurrency: string
  rate: number | string
  effectiveDate: string
  source?: string
}

export interface CreateCurrencyRequest {
  code: string
  name: string
  symbol?: string
  isBase: boolean
  precision: number
}

export interface CreateExchangeRateRequest {
  fromCurrency: string
  toCurrency: string
  rate: number
  effectiveDate: string
  source?: string
}

export function createCurrency(data: CreateCurrencyRequest) {
  return request.post('/currencies', data)
}

export function getCurrencyList() {
  return request.get('/currencies')
}

export function getBaseCurrency() {
  return request.get('/currencies/base')
}

// 批次 157d-1 修复：新增设置基础币种 API
export function setBaseCurrency(id: number) {
  return request.post(`/currencies/${id}/set-base`)
}

export function createExchangeRate(data: CreateExchangeRateRequest) {
  return request.post('/exchange-rates', data)
}

export function getExchangeRate(params: {
  fromCurrency: string
  toCurrency: string
  date?: string
}) {
  return request.get('/exchange-rates/query', { params })
}
