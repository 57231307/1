import { request } from './request';
import type { ApiResponse } from '@/types/api';

// 列表/详情出参 = 后端 purchase_inspection::Model（services/purchase_inspection_service 的
// list_inspections / get_inspection 直接序列化实体）。
// 键为实体 snake_case；inspection_status 词表仅 pending/completed
// （models/status/purchase_inventory 的 purchase_inspection 子模块）。
// receipt_no/supplier_name/inspector_name 实体不提供，需后端 JOIN，列已保留（见注释）。
export interface PurchaseInspection {
  id?: number;
  inspection_no: string;
  receipt_id: number | null;
  order_id: number | null;
  supplier_id: number;
  inspection_date: string;
  inspector_id: number | null;
  inspection_type: string | null;
  sample_size: number | null;
  defect_count: number | null;
  pass_quantity: number | null;
  reject_quantity: number | null;
  inspection_status: string | null;
  inspection_result: string | null;
  quality_score: number | null;
  defect_description: string | null;
  attachment_urls: string | null;
  /** 后端 purchase_inspection.notes（备注） */
  notes: string | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  completed_by: number | null;
  /** 需要后端 JOIN：purchase_inspection.receipt_id -> purchase_receipt.receipt_no（Model 无该列） */
  receipt_no?: string | null;
  /** 需要后端 JOIN：purchase_inspection.supplier_id -> suppliers.supplier_name（Model 无该列） */
  supplier_name?: string | null;
  /** 需要后端 JOIN：purchase_inspection.inspector_id -> users 姓名（Model 无该列） */
  inspector_name?: string | null;
  /** get_inspection 仅返回单条 Model、不含明细；明细需另调 /inspections/:id/items，列已保留 */
  items?: PurchaseInspectionItem[];
}

/**
 * 明细出参 = 后端 purchase_inspection_item::Model 原键
 * （services/purchase_inspection_service 的 list_inspection_items 直接序列化实体，
 * 无 JOIN、无别名：产品名/预期数量这类列在明细表里根本不存在）。
 * qualified/unqualified 为 DECIMAL，经 JSON 序列化为字符串（如 "50.0000"），故取并集。
 * 端点信封为 { items, total, inspection_id }（handlers/purchase_inspection_handler 的
 * list_inspection_items）。
 */
export interface PurchaseInspectionItemRecord {
  id: number;
  inspection_id: number;
  product_id: number;
  /** 检验项目名称（purchase_inspection_item.item_name，该表唯一的名称列） */
  item_name: string;
  qualified_quantity: number | string;
  unqualified_quantity: number | string;
  remark: string | null;
  created_at?: string;
  updated_at?: string;
}

export interface PurchaseInspectionItemListResponse {
  items: PurchaseInspectionItemRecord[];
  total: number;
  inspection_id: number;
}

/**
 * 检验单明细的 UI 行模型（新建/编辑表单与详情对话框的 el-table 数据形状）。
 * 注意：这不是 /inspections/{id}/items 的出参类型——后端明细只有 item_name /
 * qualified_quantity / unqualified_quantity / remark 四列业务字段，
 * 读侧请用 `PurchaseInspectionItemRecord` 再映射到本模型（见 usePiProc / usePrRtn）。
 */
export interface PurchaseInspectionItem {
  id?: number;
  inspection_id?: number;
  product_id: number;
  product_name?: string;
  product_code?: string;
  expected_quantity?: number;
  /** UI 编辑态字段（purchase_inspection_item 无此列，读侧映射不填） */
  inspected_quantity?: number;
  passed_quantity: number;
  failed_quantity: number;
  defect_reason?: string;
  remark?: string;
}

/**
 * 明细出参记录 → 检验明细 UI 行（新建/编辑表单与详情对话框表格的数据形状）。
 *
 * purchase_inspection_item 只有 item_name / qualified_quantity / unqualified_quantity / remark
 * 四个业务列：item_name 是该表唯一的名称列，映射到行上的 product_name 供「产品名称」列显示；
 * expected_quantity / inspected_quantity / defect_reason 后端从不提供（明细表无这些列），
 * 映射时如实不写，不用其它列凑数。数量列为 DECIMAL（JSON 里是字符串），统一 Number() 归一。
 */
export function toInspectionUiRow(record: PurchaseInspectionItemRecord): PurchaseInspectionItem {
  return {
    id: record.id,
    inspection_id: record.inspection_id,
    product_id: record.product_id,
    product_name: record.item_name,
    passed_quantity: Number(record.qualified_quantity),
    failed_quantity: Number(record.unqualified_quantity),
    remark: record.remark ?? undefined,
  };
}

export interface PurchaseInspectionQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  supplier_id?: number;
  status?: string;
  result?: string;
  inspection_date_from?: string;
  inspection_date_to?: string;
}

/**
 * 创建质检单请求（严格对齐 backend CreatePurchaseInspectionRequest，
 * services/purchase_inspection_service 内定义）。
 * 注意：后端 supplier_id 是 Option<i32>，但 service 代码在 None 时报错，实际为必填。
 * 不含 items：明细通过 POST /inspections/{id}/items 单独提交。
 * remark 字段：后端键名是 notes（非 remark），历史前端用 remark 字段会被 Axum 丢弃。
 */
export interface CreatePurchaseInspectionPayload {
  receipt_id?: number;
  order_id?: number;
  /** 后端声明为 Option<i32>，但 create_inspection 在 None 时直接 validation 拒绝，故按必填建模 */
  supplier_id: number;
  inspection_date?: string;
  inspector_id?: number;
  inspection_type?: string;
  sample_size?: number;
  /** 后端键名是 notes（非 remark） */
  notes?: string;
}

/**
 * 更新质检单请求（严格对齐 backend UpdatePurchaseInspectionRequest，
 * services/purchase_inspection_service 内定义）。
 * 仅 sample_size/defect_description/notes 三字段可更新。
 */
export interface UpdatePurchaseInspectionPayload {
  sample_size?: number;
  defect_description?: string;
  notes?: string;
}

/**
 * 创建质检明细请求（严格对齐 backend CreateInspectionItemDto，
 * handlers/purchase_inspection_handler 内定义）。
 */
export interface CreateInspectionItemPayload {
  product_id: number;
  item_name: string;
  qualified_quantity: number;
  unqualified_quantity: number;
  remark?: string;
}

/**
 * 更新质检明细请求（严格对齐 backend UpdateInspectionItemDto，
 * handlers/purchase_inspection_handler 内定义）。
 */
export interface UpdateInspectionItemPayload {
  qualified_quantity?: number;
  unqualified_quantity?: number;
  remark?: string;
}

// 检验单列表
export const getPurchaseInspectionList = (params?: PurchaseInspectionQueryParams) =>
  request.get<ApiResponse<{ items: PurchaseInspection[]; total: number }>>(
    '/purchase/inspections',
    { params }
  );

/**
 * 统计卡聚合出参 = 后端 PurchaseInspectionStats（models/purchase_inspection 内定义，
 * Serialize 无 rename_all，snake 原键）。
 * 四键均为 u64 整数计数、可空性为必填（NOT 可空——序列化侧无数值缺省，缺键即契约漂移）。
 * 分桶与列表同一套筛选参数、同一条件构造点（services/purchase_inspection_service
 * ::base_filtered_query）；partial 归入 failed（同 service 侧 inspection_stats 分桶文书，
 * 词表常量同源）。
 * 恒等式 pending+passed+failed===total 当前写入规则下成立但无 DB 约束兜底：
 * 词表外异常行只进 total，四卡之和可能小于总数，属如实呈现而非前端兜底对象。
 */
export interface PurchaseInspectionStats {
  total: number;
  pending: number;
  passed: number;
  failed: number;
}

// 统计卡聚合：入参与列表完全同一 PurchaseInspectionQueryParams（后端同一 InspectionQueryParams，
// handlers/purchase_inspection_handler::get_inspection_stats 只取筛选条件、page/page_size
// 在该端点被忽略、不参与条件构造，
// 直发同参数对象以保证"卡片与表格同参数"在前端侧不留第二份参数装配逻辑）。
export const getPurchaseInspectionStats = (params?: PurchaseInspectionQueryParams) =>
  request.get<ApiResponse<PurchaseInspectionStats>>('/purchase/inspections/stats', { params });

// 创建检验单：请求体严格为 CreatePurchaseInspectionPayload（对齐后端 CreatePurchaseInspectionRequest）。
export const createPurchaseInspection = (data: CreatePurchaseInspectionPayload) =>
  request.post<ApiResponse<PurchaseInspection>>('/purchase/inspections', data);

// 获取检验单详情
export const getPurchaseInspectionById = (id: number) =>
  request.get<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`);

// 更新检验单：请求体严格为 UpdatePurchaseInspectionPayload（对齐后端 UpdatePurchaseInspectionRequest）。
export const updatePurchaseInspection = (id: number, data: UpdatePurchaseInspectionPayload) =>
  request.put<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}`, data);

// 后端 purchase_inspection_handler::complete_inspection 的 Json<CompleteInspectionRequest>
// 必填 pass_quantity / reject_quantity / inspection_result（snake_case，无 rename_all）。
export interface CompleteInspectionPayload {
  pass_quantity: number;
  reject_quantity: number;
  inspection_result: string;
}

export const completePurchaseInspection = (id: number, data: CompleteInspectionPayload) =>
  request.post<ApiResponse<PurchaseInspection>>(`/purchase/inspections/${id}/complete`, data);

// 获取检验明细；后端 list_inspection_items 返回 {items:[], total, inspection_id} 嵌套对象，
// items = purchase_inspection_item::Model 原键（见 PurchaseInspectionItemRecord）
export const getPurchaseInspectionItemList = (id: number) =>
  request.get<ApiResponse<PurchaseInspectionItemListResponse>>(`/purchase/inspections/${id}/items`);

// 创建检验明细：请求体严格为 CreateInspectionItemPayload（对齐后端 CreateInspectionItemDto）。
export const createPurchaseInspectionItem = (id: number, data: CreateInspectionItemPayload) =>
  request.post<ApiResponse<PurchaseInspectionItem>>(`/purchase/inspections/${id}/items`, data);

// 更新检验明细：请求体严格为 UpdateInspectionItemPayload（对齐后端 UpdateInspectionItemDto）。
export const updatePurchaseInspectionItem = (
  id: number,
  itemId: number,
  data: UpdateInspectionItemPayload
) =>
  request.put<ApiResponse<PurchaseInspectionItem>>(
    `/purchase/inspections/${id}/items/${itemId}`,
    data
  );

// 删除检验明细
export const deletePurchaseInspectionItem = (id: number, itemId: number) =>
  request.delete<ApiResponse<void>>(`/purchase/inspections/${id}/items/${itemId}`);
