/**
 * 统一 API 响应结构
 */
export interface ApiResponse<T = unknown> {
  code: number
  message: string
  data: T
  timestamp?: string
  /**
   * 分页总数（可选，仅列表分页接口由后端顶层返回）。
   * 后端 PaginatedResponse<T> 的 total 字段在 data 内部，
   * 部分历史接口也可能在顶层冗余返回 total 用于前端快速读取。
   */
  total?: number
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
