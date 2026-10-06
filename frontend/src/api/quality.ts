import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';
import type {
  QualityHandlingMethodValue,
  QualityHandlingStatusValue,
} from '@/constants/quality-unqualified-handling';

export interface QualityStandard {
  id: number;
  standard_code: string;
  standard_name: string;
  version: string;
  /**
   * 类型列真实键名为 standard_type（models/quality_standard.rs:13；
   * DDL standard_type VARCHAR(50) NOT NULL、无 CHECK——migration/src/domain/system/
   * m0005_add_basic_data_and_system_tables.rs:111，故必填、非字面量联合）。
   * 取值词表：写入方文档口径 product/process（quality_standard_handler.rs:40-41）；
   * 服务层缺省落 "general"（quality_standard_service.rs:122-125），存量行可能为该值；
   * 数据库无 CHECK 约束，界面下拉仅 product/process 两项，回填词表外值时原样展示不猜。
   */
  standard_type: string;
  /**
   * 状态列：后端 Model 为 String（models/quality_standard.rs:25），DDL
   * "status" VARCHAR(20) DEFAULT 'draft' 无 CHECK
   * （migration/src/domain/system/m0005_add_basic_data_and_system_tables.rs:122，
   * v15 域尾亦未对该列补 CHECK），且 update_standard 对传入 status 原样落库、
   * 不做词表校验（quality_standard_service.rs:181-183）——故非闭合字面量联合，
   * 类型如实为 string（与同文件 standard_type :11-16 同一判定法）。
   * 权威取值集合（状态流转端点代码实际写入，均出自词表常量）：
   * draft（创建，service:128）、approved（approve，service:314）、
   * rejected（reject，service:351）、active（publish，service:417 落
   * master_data::ACTIVE="active"，models/status/general.rs:52）、
   * archived（archive，service:384 落 master_data::ARCHIVED，general.rs:73）；
   * quality_standard 专属常量见 models/status/quality_dyeing.rs:11-20
   * （draft/approved/rejected）。
   * ⚠ "published" 为幽灵值：handler 文档注释（quality_standard_handler.rs:64）
   * 口径过时，后端全仓源码无任何写入 "published" 的路径（发布落 active），
   * 旧前端以 published 判定发布行导致归档按钮恒不出现——已按代码真相纠正。
   * 词表外存量值（经无校验 update 端点可被写入任意串）界面原样展示、不猜默认
   * （views/quality-standards/index.vue getStatusLabel `|| status` 直出）。
   */
  status: string;
  content: string;
  // 幽灵键 attachments 已删：后端实体无此列（models/quality_standard.rs:8-31 全列核对）、
  // create/update DTO 均不接收（quality_standard_handler.rs:35-52/57-68），
  // 读取恒 undefined、提交被 serde 静默丢弃，属纯假字段（红线：不保留假字段）。
  created_by: number;
  created_by_name: string;
  approved_by: number;
  approved_by_name: string;
  approved_at: string;
  created_at: string;
  updated_at: string;
}

/**
 * 质检记录出参：与后端 models/quality_inspection_record.rs 的 Model 字段一一对应。
 * 后端 Decimal 一律序列化为字符串，数量/比率按 string 取用（渲染前转数字）。
 */
export interface QualityRecord {
  id: number;
  inspection_no: string;
  inspection_type: string;
  related_type: string | null;
  related_id: number | null;
  product_id: number;
  batch_no: string | null;
  supplier_id: number | null;
  customer_id: number | null;
  inspection_date: string;
  inspector_id: number | null;
  total_qty: string;
  inspected_qty: string;
  qualified_qty: string | null;
  unqualified_qty: string | null;
  qualification_rate: string | null;
  inspection_result: string;
  remark: string | null;
  defect_type: string | null;
  grade: string | null;
  color_no: string | null;
  dye_lot_no: string | null;
  dye_type: string | null;
  auxiliary_type: string | null;
  temperature: string | null;
  fabric_source: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * 新建质检记录请求体：后端 CreateInspectionRecordRequest 里非 Option 的字段都必须提交
 * （inspection_no / inspection_type / product_id / inspection_date / total_qty /
 * inspected_qty / inspection_result），少一个即被反序列化拒绝。
 */
export interface CreateQualityRecordPayload {
  inspection_no: string;
  inspection_type: string;
  product_id: number;
  inspection_date: string;
  total_qty: string | number;
  inspected_qty: string | number;
  inspection_result: string;
  batch_no?: string;
  inspector_id?: number;
  supplier_id?: number;
  customer_id?: number;
  remark?: string;
  defect_type?: string;
  color_no?: string;
  dye_lot_no?: string;
}

// 质检记录更新载荷 —— 对齐后端 UpdateInspectionRecordRequest（handlers/quality_inspection_handler.rs，
// 三态语义 RFC 7386）：键缺席=保持原值、显式 null=清空为 NULL（仅下列可空列）、有值=覆盖。
// 可空列依据 m0005/system 域 DDL（batch_no m0005:162、inspector_id :167、qualified_qty :169、
// unqualified_qty :170 及 system/mod.rs:183-195 补列 color_no/defect_type/dye_lot_no/grade/
// qualification_rate/remark）。
// NOT NULL/模型非 Option 列（inspection_type/inspection_date/total_qty/inspected_qty/
// inspection_result）不声明 null——显式 null 会被后端 business_displayable 拒绝。
export interface UpdateQualityRecordPayload {
  inspection_type?: string;
  batch_no?: string | null;
  inspection_date?: string;
  inspector_id?: number | null;
  total_qty?: string | number;
  inspected_qty?: string | number;
  qualified_qty?: string | number | null;
  unqualified_qty?: string | number | null;
  qualification_rate?: string | number | null;
  inspection_result?: string;
  remark?: string | null;
  defect_type?: string | null;
  grade?: string | null;
  color_no?: string | null;
  dye_lot_no?: string | null;
}

/**
 * 质量标准列表查询参数——严格对齐后端 quality_standard_handler.rs::QualityStandardQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 */
export interface QualityStandardListParams {
  standard_type?: string;
  status?: string;
  page?: number;
  page_size?: number;
}

export function getQualityStandardList(
  params?: QualityStandardListParams
): Promise<ApiResponse<QualityStandard[]>> {
  // 批次 157d-2 修复：后端 quality-standards 已从 /production 域提升到根级
  return request.get('/quality-standards', { params });
}

export function getQualityStandard(id: number): Promise<ApiResponse<QualityStandard>> {
  return request.get(`/quality-standards/${id}`);
}

/**
 * 创建质量标准载荷：逐键对齐后端 CreateQualityStandardRequest
 * （handlers/quality_standard_handler.rs:35-52）。类型键名为 standard_type——此前直传实体
 * （键 type）会被 serde 静默丢弃，服务层落缺省 "general"（quality_standard_service.rs:122-125），
 * 用户所选类型从未入库。id/status/attachments 不在创建 DTO 内，禁止入载荷。
 * standard_code 为 Option：服务层按 is_none() 区分自动生成/手工码（service:116-117），
 * 空字符串会走手工分支落空码，未填时必须省略该键而非传 ""。
 */
export interface CreateQualityStandardPayload {
  standard_code?: string;
  standard_name: string;
  standard_type?: string;
  version?: string;
  content?: string;
  /** 格式 YYYY-MM-DD（handler:46-48），省略时服务端取当天 */
  effective_date?: string;
  expiry_date?: string;
  remark?: string;
}

export function createQualityStandard(
  data: CreateQualityStandardPayload
): Promise<ApiResponse<QualityStandard>> {
  return request.post('/quality-standards', data);
}

/**
 * 更新质量标准载荷：对齐后端 UpdateQualityStandardRequest
 * （handlers/quality_standard_handler.rs:57-68，字段 standard_name/standard_type/content/status/remark 全 Option）。
 * 入参与实体出参的该列键名一致，都是 standard_type（models/quality_standard.rs:13）；
 * id / standard_code / version / attachments 不在更新契约内，提交会被 serde 静默丢弃，禁止放入载荷。
 * 状态流转（审批/驳回/发布/归档）走各自端点，更新接口不提交 status。
 */
export interface UpdateQualityStandardPayload {
  standard_name?: string;
  standard_type?: string;
  content?: string;
  remark?: string;
}

export function updateQualityStandard(
  id: number,
  data: UpdateQualityStandardPayload
): Promise<ApiResponse<QualityStandard>> {
  return request.put(`/quality-standards/${id}`, data);
}

export function deleteQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/quality-standards/${id}`);
}

export function approveQualityStandard(
  id: number,
  data?: { approval_comment?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/approve`, data || {});
}

// 批次 157d-2 新增：驳回质量标准
export function rejectQualityStandard(
  id: number,
  data?: { reject_reason?: string }
): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/reject`, data || {});
}

export function publishQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/publish`);
}

export function getQualityStandardVersions(id: number): Promise<ApiResponse<QualityStandard[]>> {
  return request.get(`/quality-standards/${id}/versions`);
}

// 后端 quality_inspection_handler::list_records 以 success_paginated 返回 PaginatedResponse（items + total + page + page_size）
/**
 * 质检记录列表查询参数——严格对齐后端 quality_inspection_handler.rs::RecordQuery。
 * 全部 Option 字段 → 可选；无 rename_all → 保持 snake_case。
 * batch_no 为批次号（与 sales 侧契约同名）。
 */
export interface QualityRecordListParams {
  product_id?: number;
  batch_no?: string;
  inspection_type?: string;
  inspection_result?: string;
  page?: number;
  page_size?: number;
}

export function getQualityRecordList(
  params?: QualityRecordListParams
): Promise<ApiResponse<PaginatedResponse<QualityRecord>>> {
  return request.get('/production/quality-inspection/records', { params });
}

export function getQualityRecord(id: number): Promise<ApiResponse<QualityRecord>> {
  return request.get(`/production/quality-inspection/records/${id}`);
}

export function createQualityRecord(
  data: CreateQualityRecordPayload
): Promise<ApiResponse<QualityRecord>> {
  return request.post('/production/quality-inspection/records', data);
}

// 批次 94 P2-12 修复：补全质检记录更新接口（原先前端 API 模块缺失，导致 index.vue 更新占位）
export function updateQualityRecord(
  id: number,
  data: UpdateQualityRecordPayload
): Promise<ApiResponse<QualityRecord>> {
  return request.put(`/production/quality-inspection/records/${id}`, data);
}

/**
 * 缺陷列表查询参数——严格对齐后端 quality_inspection_handler.rs::DefectQuery。
 * status 会被服务层直接拿去对 unqualified_products.handling_status 做等值筛选，
 * 取值只能是写入方常量集合（pending / approved / rejected）之一，逐字符相同。
 * record_id（D2 已下推生效）：后端 list_defects 把它透传为服务层 inspection_id，
 * 对实体列 unqualified_products.inspection_id（派生本行的质检记录 id，
 * quality_inspection_records.id）做等值筛选（handler :412、service get_defects_list）；
 * 它是筛选键而不是台账行动作参数——台账行动作的路径 id 一律用行自身 id（见 processDefectRow）。
 */
export interface DefectListParams {
  record_id?: number;
  status?: QualityHandlingStatusValue;
  page?: number;
  page_size?: number;
}

export function getDefectList(
  params?: DefectListParams
): Promise<ApiResponse<UnqualifiedProductRecord[]>> {
  return request.get('/production/quality-inspection/defects', { params });
}

/**
 * 不合格品处理方式取值：与后端 services/quality_inspection_service.rs 的
 * HANDLING_DOWNGRADE_SALE / HANDLING_REWORK / HANDLING_SCRAP 三个常量逐字一致
 * （downgrade_sale 降级销售 / rework 返工 / scrap 报废），词表单一真相源见
 * constants/quality-unqualified-handling.ts。
 * 后端会按质检记录等级校验合法组合（A 级拒绝处理，B 级必须降级销售，C 级必须返工或报废，
 * 见 validate_handling_method_by_grade），非法组合返回业务错误。
 */
export type DefectHandlingMethod = QualityHandlingMethodValue;

/**
 * 处理缺陷请求体：对齐后端 ProcessUnqualifiedRequest
 * （services/quality_inspection_service.rs:179-186）。
 * unqualified_qty / unqualified_reason / handling_method 为非 Option 必填，缺任一项即被 serde 反序列化拒绝（422）；
 * remark / handling_result 为 Option<String>，空值时必须省略该键。
 * unqualified_qty 为 rust_decimal，提交 number 或十进制字符串均可。
 */
export interface ProcessDefectPayload {
  unqualified_qty: string | number;
  unqualified_reason: string;
  handling_method: DefectHandlingMethod;
  remark?: string;
  handling_result?: string;
}

/**
 * 不合格品（缺陷台账）出参：models/unqualified_product.rs::Model 的整行序列化，
 * 处理端点 process_defect 与列表端点 list_defects 共用同一声明（两处都是
 * serde 直接序列化实体 Vec/单行，服务层不组 DTO、不补键）。
 * - NOT NULL 列（unqualified_no / product_id / unqualified_qty / unqualified_reason /
 *   handling_method / handling_status / created_at / updated_at）不得标 `?`；
 * - 后端 Decimal 一律序列化为字符串（含 scrap_loss_amount），渲染前按字符串直读或先转数字，
 *   不得假定是 number；时间列为 RFC3339 字符串；
 * - handling_method / handling_status / grade 三列在库里是无 CHECK 的 VARCHAR，故类型如实
 *   取 string，写入方实际只会产生 constants/quality-unqualified-handling.ts 登记的词表值，
 *   界面按该词表翻文案、词表外值原样直出不猜。
 */
export interface UnqualifiedProductRecord {
  id: number;
  unqualified_no: string;
  /** 派生本行的质检记录 id（quality_inspection_records.id），非验布记录 id */
  inspection_id: number | null;
  product_id: number;
  batch_no: string | null;
  unqualified_qty: string;
  unqualified_reason: string;
  handling_method: string;
  handling_status: string;
  handling_by: number | null;
  handling_at: string | null;
  remark: string | null;
  grade: string | null;
  handling_result: string | null;
  created_at: string;
  updated_at: string;
  stock_grade_synced: boolean;
  stock_id: number | null;
  scrap_approval_status: string;
  approver_id_fin: number | null;
  approver_id_gm: number | null;
  approved_at_fin: string | null;
  approved_at_gm: string | null;
  scrap_loss_amount: string | null;
}

/**
 * 处理不合格品（从质检记录**开单**，建立不合格品处理单）。
 * ⚠ 路径参数 id 是**质检记录 id**（quality_inspection_records.id）：后端 process_unqualified
 * 拿它 find_by_id 质检记录，再 INSERT 一条 unqualified_products；它不是台账行 id。
 * 同记录幂等守卫（D1② 配套，service process_unqualified :479-493）：该质检记录存在
 * 非终态（handling_status 既非 approved 也非 rejected）的既有行时，再次开单被
 * BUSINESS_ERROR 拒绝——同一记录不会因重复调用累积同号多行。
 * 台账行原地更新处置结果走 processDefectRow，两者 {id} 语义不同，不可互换。
 */
export function processDefect(
  id: number,
  data: ProcessDefectPayload
): Promise<ApiResponse<UnqualifiedProductRecord>> {
  return request.post(`/production/quality-inspection/defects/${id}/process`, data);
}

/**
 * 台账行「处置结果原地更新」请求体：逐键对齐后端 ProcessResultRequest
 * （services/quality_inspection_service.rs:197-206），键集恰为
 * handling_method（必填）+ reason（可选）两键。
 * 身份字段（user_id/handling_by/updated_by/operator_id）在后端 DTO 类型层不存在，
 * 上送只会被 serde 忽略且属伪造身份面，禁止入载荷；操作人由服务端会话派生。
 * reason 为 Option<String>：不填时必须省略该键（提交空白串会被后端
 * VALIDATION 拒绝）；本轮后端仅校验+显式日志、不落库（表无专用理由列，迁移待裁）。
 */
export interface ProcessDefectResultPayload {
  handling_method: DefectHandlingMethod;
  reason?: string;
}

/**
 * 不合格品台账行处置结果**原地更新**（D1②）：不新开行，本行 handling_status
 * 由 pending 推进为 approved 并写 handling_by/handling_at 留痕。
 * 路径参数 id 是**台账行自身 id**（unqualified_products.id，与报废两级审批同一
 * id 语义），不是质检记录 id（后端 routes/production.rs:549-552、handler
 * process_defect_result :447-466）。
 * 状态门与错误族（由后端保证，界面门控须与之一致以免必然 4xx 的假动作）：
 * - 仅 handling_status === pending 可行，其余前驱 ⇒ 4xx BUSINESS_ERROR；
 * - 行处于报废审批流（pending_fin/pending_gm）⇒ 4xx BUSINESS_ERROR；
 * - handling_method === scrap ⇒ 4xx BUSINESS_ERROR（报废终态只能经两级审批达成）；
 * - 词表外 handling_method ⇒ 4xx VALIDATION_ERROR。
 */
export function processDefectRow(
  id: number,
  data: ProcessDefectResultPayload
): Promise<ApiResponse<UnqualifiedProductRecord>> {
  return request.post(`/production/quality-inspection/defects/${id}/process-result`, data);
}

export function archiveQualityStandard(id: number): Promise<ApiResponse<void>> {
  return request.post(`/quality-standards/${id}/archive`);
}
