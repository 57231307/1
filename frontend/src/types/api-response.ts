/**
 * 统一 API 响应结构
 */
export interface ApiResponse<T = unknown> {
  code: number;
  message: string;
  data: T;
  timestamp?: string;
  /**
   * 分页总数（可选，仅列表分页接口由后端顶层返回）。
   * 后端 PaginatedResponse<T> 的 total 字段在 data 内部，
   * 部分历史接口也可能在顶层冗余返回 total 用于前端快速读取。
   */
  total?: number;
}

/**
 * 分页响应结构（对应后端 PaginatedResponse<T>，作为 ApiResponse.data 字段的内容）
 */
export interface PaginatedResponse<T = unknown> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

/**
 * 错误响应结构 —— 全站唯一的 HTTP 失败信封，对应后端
 * `backend/src/utils/error.rs` 的 `ErrorResponse`（`AppError::into_response` 出参）：
 * `{ code: "<字符串机器码>", message: "<脱敏常量或可外显文案>", trace_id: "<uuid>", timestamp: <秒级 i64> }`。
 *
 * - `code` 是字符串机器码（NOT_FOUND / VALIDATION_ERROR / BUSINESS_ERROR / UNAUTHORIZED /
 *   FORBIDDEN / BAD_REQUEST / INTERNAL_ERROR / DATABASE_ERROR / NOT_IMPLEMENTED /
 *   TOO_MANY_REQUESTS，见 `AppError::error_code()`），不是 HTTP 状态码数字。
 * - 失败信封只有这四个键：字段级校验详情、`errors` 数组都不在其中（`errors` 仅存在于
 *   导入/批量类 DTO 的 data 里）。
 * - HTTP 失败一律以非 2xx 状态码返回，不会出现在 2xx 响应体中。
 */
export interface ErrorResponse {
  code: string;
  message: string;
  trace_id: string;
  timestamp: number;
}

/**
 * 空响应（仅返回状态）
 */
export interface EmptyResponse {
  code: number;
  message: string;
  timestamp: string;
}
