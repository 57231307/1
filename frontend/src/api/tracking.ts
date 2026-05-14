import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface PageVisit {
  id: number
  userId: number
  userName: string
  pagePath: string
  pageName: string
  duration: number
  referrer?: string
  ipAddress: string
  userAgent: string
  createdAt: string
}

export interface PageStatistics {
  pagePath: string
  pageName: string
  visitCount: number
  uniqueUsers: number
  avgDuration: number
}

export interface PageVisitQueryParams extends QueryParams {
  userId?: number
  pagePath?: string
  startDate?: string
  endDate?: string
}

export interface PageStatisticsParams extends QueryParams {
  startDate: string
  endDate: string
  limit?: number
}

export function trackPageVisit(data: {
  pagePath: string
  pageName: string
  duration: number
  referrer?: string
}): Promise<ApiResponse<void>> {
  return request.post('/tracking/page-visit', data)
}

export function getPageVisits(params?: PageVisitQueryParams): Promise<ApiResponse<{ list: PageVisit[]; total: number }>> {
  return request.get('/tracking/page-visits', { params })
}

export function getPageStatistics(params: PageStatisticsParams): Promise<ApiResponse<{ list: PageStatistics[]; total: number }>> {
  return request.get('/tracking/page-statistics', { params })
}

export function getUserActivity(userId: number, params?: QueryParams): Promise<ApiResponse<{ list: PageVisit[]; total: number }>> {
  return request.get(`/tracking/user/${userId}/activity`, { params })
}

export function getRealTimeVisitors(): Promise<ApiResponse<{ count: number; users: { userId: number; userName: string; pagePath: string; visitedAt: string }[] }>> {
  return request.get('/tracking/realtime')
}
