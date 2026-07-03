import { request } from './request'

export interface TraceRecord {
  id?: number
  trace_chain_id: string
  five_dimension_id: number
  business_type: string
  business_id: number
  relation_type: string
  created_at?: string
}

// P3 维度 9 修复（批次 87）：移除 [key: string]: any 索引签名，
// 显式声明所有业务字段，提升类型安全性。
// TraceChainResponse 用于 forwardResult/backwardResult 列表展示，
// 动态字段通过 ElTableColumn prop 字符串访问，TS 不检查，无需索引签名。
export interface TraceChainResponse {
  id?: number
  trace_chain_id: string
  five_dimension_id: number
  business_type: string
  business_id: number
  relation_type: string
  created_at?: string
  // 以下字段由后端动态返回，前端列表展示用，显式声明以支持类型推断
  batch_no?: string
  color_no?: string
  grade?: string
  current_stage?: string
  current_bill_no?: string
}

/// 追溯环节信息（FullTraceChainResponse.stages 元素类型）
/// P3 维度 9 修复（批次 87 CI 修复）：新增接口，替代 [key: string]: unknown 索引签名，
/// 使 v-for 遍历 stages 时元素类型可推断，避免 TS2339/TS2322 错误
export interface TraceStage {
  stage_id: number | string
  stage_name: string
  bill_no: string
  stage_type: string
  bill_type?: string
  warehouse_name?: string
  supplier_name?: string
  customer_name?: string
  quantity_meters?: number
  quantity_kg?: number
  created_at?: string
}

export interface FullTraceChainResponse {
  trace_chain_id: string
  five_dimension_id: number
  traces: TraceChainResponse[]
  // 以下字段由后端动态返回，businessTrace/index.vue 模板直接访问，需显式声明
  business_type?: string
  business_id?: number
  relation_type?: string
  created_at?: string
  product_id?: number
  batch_no?: string
  color_no?: string
  grade?: string
  total_stages?: number
  start_time?: string
  end_time?: string
  /// 追溯环节列表（v-for 遍历，需明确类型以支持 stage.stage_id 等属性访问）
  stages?: TraceStage[]
}

// P2-9c 修复（批次 82 v1 复审）：业务追溯查询参数强类型化
export interface TraceQueryParams {
  trace_chain_id?: string
  business_type?: string
  business_id?: number
  five_dimension_id?: number
  supplier_id?: number
  customer_id?: number
  batch_no?: string
}

export const getTraceByFiveDimension = (fiveDimensionId: number | string) =>
  request.get(`/business-trace/five-dimension/${fiveDimensionId}`)

export const forwardTrace = (params?: TraceQueryParams) =>
  request.get('/business-trace/forward', { params })

export const backwardTrace = (params?: TraceQueryParams) =>
  request.get('/business-trace/backward', { params })

// P2-9c 修复（批次 82 v1 复审）：创建追溯快照请求 DTO
export interface TraceSnapshotCreateDto {
  snapshot_type?: string
  remark?: string
  metadata?: unknown
}

export const createTraceSnapshot = (traceChainId: string, data?: TraceSnapshotCreateDto) =>
  request.post(`/business-trace/snapshot/${traceChainId}`, data)
