/**
 * 统一 API 响应结构
 */
export interface ApiResponse<T = unknown> {
  code: number
  message: string
  data: T
  timestamp: string
}

/**
 * 分页响应结构
 */
export interface PaginatedResponse<T = unknown> {
  code: number
  message: string
  data: {
    items: T[]
    total: number
    page: number
    page_size: number
  }
  timestamp: string
}

/**
 * 错误响应结构
 */
export interface ErrorResponse {
  code: number
  message: string
  error?: string
  details?: Record<string, unknown>
  timestamp: string
}

/**
 * 空响应（仅返回状态）
 */
export interface EmptyResponse {
  code: number
  message: string
  timestamp: string
}
