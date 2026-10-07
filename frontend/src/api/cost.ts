import { request } from './request';

export interface CostCollection {
  id?: number;
  collection_no: string;
  collection_date: string;
  batch_no?: string;
  color_no?: string;
  /**
   * 六项金额列后端均为 rust_decimal::Decimal（models/cost_collection.rs:24-30，
   * NOT NULL 无 Option），且本仓 rust_decimal 仅启用 `serde` 特性
   * （backend/Cargo.toml:60 `features = ["serde"]`，未开 serde-with-float /
   * serialize-bigdecimal-as-plain 之外的数值化特性）→ JSON 序列化为**十进制字符串**。
   * 此前声明为 number 属谎报（直接 .toFixed/参与 + 运算会崩溃或变字符串拼接）。
   * 按本仓 H1/H2/H3 先例（commit a08731f7 / 1fd1355c / 4e670fe2）定型为
   * string | number，真实数值消费点在调用侧 Number() 归一
   * （views/cost/tabs/CostCollectionTab.vue:93-117 列表展示、:506-520 编辑回填）。
   * processing_fee/dyeing_fee 为后端模型 :27-28 非 Option 列（创建 DTO
   * cost_collection_handler.rs:45-62 亦必填、服务层五项求和落 total_cost），
   * 此前实体声明缺失，读取形状与后端真实出参不符——补入。
   */
  direct_material: string | number;
  direct_labor: string | number;
  manufacturing_overhead: string | number;
  processing_fee: string | number;
  dyeing_fee: string | number;
  total_cost?: string | number;
  status: string;
  type?: string;
  period?: string;
  department_id?: number;
  remark?: string;
  warehouse_id?: number;
  notes?: string;
  created_at?: string;
  updated_at?: string;
}

// P2-9c 修复（批次 82 v1 复审）：成本归集列表查询参数强类型化
export interface CostCollectionQueryParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  // 归集单号 / 颜色：后端 DTO 真实接收并下推到列（collection_no 模糊、color_no 等值）
  collection_no?: string;
  color_no?: string;
  batch_no?: string;
  status?: string;
  period?: string;
  type?: string;
}

// v11 批次 159 P2-4 修复：已被 CostCollectionTab.vue 接入使用，移除过时 TODO 注释
export const deleteCollection = (id: number) =>
  request.delete(`/production/cost-collections/${id}`);

// v11 批次 159 P2-4 修复：已被 CostCollectionTab.vue 接入使用，移除过时 TODO 注释
export const auditCollection = (id: number, approved: boolean, comment?: string) =>
  request.post(`/production/cost-collections/${id}/audit`, { approved, comment });

// 成本归集列表查询（重命名自 listCollections）
export const getCostCollectionList = (params?: CostCollectionQueryParams) =>
  request.get('/production/cost-collections', { params });

// 创建成本归集入参：对齐后端 handlers/cost_collection_handler.rs:45-62 CreateCostCollectionRequestDto。
// collection_date 与 direct_material/direct_labor/manufacturing_overhead/processing_fee/dyeing_fee
// 五项金额为后端非 Option 必填（models/cost_collection.rs:25-28 NOT NULL 列，服务层
// cost_collection_service.rs:82-86 用五者求和落 total_cost）——缺 processing_fee/dyeing_fee
// 会被 serde 反序列化直接 400。此前以实体出参形状 Partial<CostCollection> 兼作入参：
// id/collection_no/total_cost/status/type/period/department_id/remark/warehouse_id/notes
// 等键后端不读（serde 静默丢弃），真正必填的两项费用反而从未提交。
export interface CreateCostCollectionInput {
  collection_date: string;
  cost_object_type?: string;
  cost_object_id?: number;
  cost_object_no?: string;
  batch_no?: string;
  color_no?: string;
  /* * 按缸号核算 */
  dye_lot_no?: string;
  workshop?: string;
  direct_material: number;
  direct_labor: number;
  manufacturing_overhead: number;
  processing_fee: number;
  dyeing_fee: number;
  output_quantity_meters?: number;
  output_quantity_kg?: number;
}

// 创建成本归集（重命名自 createCollection）
export const createCostCollection = (data: CreateCostCollectionInput) =>
  request.post('/production/cost-collections', data);

// 更新成本归集（重命名自 updateCollection）
export const updateCostCollection = (id: number, data: Partial<CostCollection>) =>
  request.put(`/production/cost-collections/${id}`, data);

export const deleteCostCollection = deleteCollection;
export const auditCostCollection = auditCollection;

export const COST_STATUS = {
  DRAFT: 'draft',
  PENDING: 'pending',
  APPROVED: 'approved',
  REJECTED: 'rejected',
};
