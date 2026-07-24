import { request } from './request'

export interface CostCollection {
  id?: number
  collection_no: string
  collection_date: string
  batch_no?: string
  color_no?: string
  direct_material: number
  direct_labor: number
  manufacturing_overhead: number
  total_cost?: number
  status: string
  type?: string
  period?: string
  department_id?: number
  remark?: string
  warehouse_id?: number
  notes?: string
  created_at?: string
  updated_at?: string
}

// P2-9c 修复（批次 82 v1 复审）：成本归集列表查询参数强类型化
export interface CostCollectionQueryParams {
  page?: number
  page_size?: number
  keyword?: string
  batch_no?: string
  status?: string
  period?: string
  type?: string
}

// v11 批次 159 P2-4 修复：已被 CostCollectionTab.vue 接入使用，移除过时 TODO 注释
export const deleteCollection = (id: number) => request.delete(`/production/cost-collections/${id}`)

// v11 批次 159 P2-4 修复：已被 CostCollectionTab.vue 接入使用，移除过时 TODO 注释
export const auditCollection = (id: number, approved: boolean, comment?: string) =>
  request.post(`/production/cost-collections/${id}/audit`, { approved, comment })

// 成本归集列表查询（重命名自 listCollections）
export const getCostCollectionList = (params?: CostCollectionQueryParams) =>
  request.get('/production/cost-collections', { params })

// 创建成本归集（重命名自 createCollection）
export const createCostCollection = (data: Partial<CostCollection>) =>
  request.post('/production/cost-collections', data)

// 更新成本归集（重命名自 updateCollection）
export const updateCostCollection = (id: number, data: Partial<CostCollection>) =>
  request.put(`/production/cost-collections/${id}`, data)

export const deleteCostCollection = deleteCollection
export const auditCostCollection = auditCollection

export const COST_STATUS = {
  DRAFT: 'draft',
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
}
