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

export function listCurrencies() {
  return request.get('/currencies')
}

export function getBaseCurrency() {
  return request.get('/currencies/base')
}

export function createExchangeRate(data: CreateExchangeRateRequest) {
  return request.post('/exchange-rates', data)
}

export function getExchangeRate(params: { fromCurrency: string; toCurrency: string; date?: string }) {
  return request.get('/exchange-rates', { params })
}
