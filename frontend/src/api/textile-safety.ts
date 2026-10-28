import axios from 'axios';

/**
 * 纺织品安全合规域 API
 *
 * 后端路由（backend/src/routes/textile_safety.rs，nest 前缀 /api/v1/erp）：
 * - /textile-submissions        送检单（列表 详情 建单 修改 删草稿 状态机 清单推导 童装核查 整单判定）
 * - /textile-reports            第三方报告（登记 修正 完整性门 结果录入 报告判定）
 * - /textile-test-items         检测项目主数据
 * - /textile-limit-rules        限值判定表维护面
 * - /textile-export-requirements 出口市场要求登记面
 *
 * 出参走全站统一成功信封 ApiResponse{code,data}，读取函数一律解包 data；
 * 分页载荷 PaginatedResponse{items,total,page,page_size}。
 * 类型键与后端 SeaORM 实体逐字段对齐（check-api-keys 棘轮），不得手添后端不存在的键。
 */
const BASE_URL = import.meta.env.VITE_API_BASE_URL || '/api/v1/erp';

const api = axios.create({ baseURL: BASE_URL, timeout: 30000, withCredentials: true });

async function unwrap<T>(p: Promise<{ data: { data?: T } }>): Promise<T> {
  const body = (await p).data;
  if (body.data === undefined || body.data === null) {
    throw new Error('纺织品安全域接口返回缺少 data 载荷');
  }
  return body.data as T;
}

// ===== 实体类型（与 backend/src/models/textile_*.rs 对齐） =====

export interface TextileSubmission {
  id: number;
  submission_no: string;
  product_id: number;
  batch_no: string | null;
  color_no: string;
  process_no: string;
  color_role: string;
  end_user_group: string;
  contact_type: string;
  safety_category: string;
  product_form: string;
  target_market: string;
  exemption_tags: string | null;
  status: string;
  overall_verdict: string | null;
  remarks: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface TextileReport {
  id: number;
  report_no: string;
  submission_id: number;
  inspection_agency: string;
  has_cma: boolean;
  has_cnas: boolean;
  sample_name: string | null;
  material_composition: string | null;
  color_no: string | null;
  process_no: string | null;
  test_basis: string | null;
  conclusion: string | null;
  oeko_tex_class: string | null;
  issue_date: string;
  valid_until: string | null;
  acceptance_status: string;
  rejection_detail: string;
  report_file_url: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export interface TextileReportResult {
  id: number;
  report_id: number;
  item_id: number;
  measured_value: number | null;
  measured_text: string | null;
  measured_unit: string | null;
  test_method: string | null;
  verdict: string;
  judged_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface TextileChildCheck {
  id: number;
  submission_id: number;
  item_id: number;
  check_kind: string;
  measured_value: number | null;
  measured_unit: string | null;
  measured_flag: string | null;
  check_desc: string | null;
  manual_conclusion: string | null;
  verdict: string;
  judged_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface TextileSubmissionItemRow {
  id: number;
  source: string;
  item_id: number;
  item_code: string;
  item_name: string;
  item_kind?: string;
  basis_standard: string;
  check_kind?: string;
  unit?: string | null;
}

export interface TextileTestItem {
  id: number;
  item_code: string;
  item_name: string;
  item_kind: string;
  basis_standard: string;
  check_kind: string;
  method_standard: string | null;
  unit: string | null;
  description: string;
  created_at: string;
  updated_at: string;
}

export interface TextileLimitRule {
  id: number;
  item_id: number;
  safety_category: string;
  product_form: string;
  condition_tag: string;
  comparator: string;
  limit_low: number | null;
  limit_high: number | null;
  clause_ref: string;
  remark: string;
  created_at: string;
  updated_at: string;
}

export interface TextileExportRequirement {
  id: number;
  requirement_code: string;
  market: string;
  requirement_type: string;
  oeko_tex_class: string | null;
  standard_ref: string;
  detail: string;
  mandatory: boolean;
  created_at: string;
  updated_at: string;
}

/** 库存联动回显（backend StockLinkResult，序列化键名一致） */
export interface StockLinkResult {
  matched: number;
  updated_stock_ids: number[];
  scope_note: string;
}

export interface SubmissionPage {
  items: TextileSubmission[];
  total: number;
  page: number;
  page_size: number;
}

export interface SubmissionDetail {
  submission: TextileSubmission;
  items: TextileSubmissionItemRow[];
  child_checks: TextileChildCheck[];
  reports: TextileReport[];
  adoption: AdoptionDecision;
  market_requirements: TextileExportRequirement[];
}

/** 采信决策的最小报告快照（后端 ReportCandidate 同形，采信原因消费本结构点名） */
export interface AdoptionCandidate {
  id: number;
  report_no: string;
  acceptance_status: string;
  issue_date: string;
  valid_until: string | null;
  created_at: string;
}

export interface ReportAdoptionPoint {
  report_no: string;
  /** 排除原因，取值域见 constants/textile-safety.ts 的 ADOPTION_REASON_LABELS */
  reason: string;
}

/** 同批次多份报告的采信结论（后端至多采信一份，其余逐枚带原因排除） */
export interface AdoptionDecision {
  adopted: AdoptionCandidate | null;
  excluded: ReportAdoptionPoint[];
}

export interface StatusResult {
  submission: TextileSubmission;
  stock_link: StockLinkResult;
}

export interface JudgeResult {
  submission: TextileSubmission;
  stock_link: StockLinkResult;
  adoption: AdoptionDecision;
  expired_reports_named: string[];
}

export interface AcceptanceResult {
  report: TextileReport;
  missing_elements: string[];
}

export interface ReportDetail {
  report: TextileReport;
  results: TextileReportResult[];
}

// ===== 入参类型（与 handler 请求体对齐） =====

export interface CreateSubmissionBody {
  submission_no: string;
  product_id: number;
  batch_no: string | null;
  color_no: string;
  process_no: string;
  color_role?: string;
  end_user_group: string;
  contact_type: string;
  safety_category: string;
  product_form: string;
  target_market: string;
  exemption_tags: string[];
  remarks: string | null;
}

export interface RegisterReportBody {
  report_no: string;
  submission_id: number;
  inspection_agency: string;
  has_cma: boolean;
  has_cnas: boolean;
  sample_name: string | null;
  material_composition: string | null;
  color_no: string | null;
  process_no: string | null;
  test_basis: string | null;
  conclusion: string | null;
  oeko_tex_class: string | null;
  issue_date: string;
  valid_until: string | null;
  report_file_url: string | null;
}

export interface ResultRowBody {
  item_code: string;
  measured_value?: string | number | null;
  measured_token?: string | null;
  measured_text?: string | null;
  measured_unit?: string | null;
  test_method?: string | null;
}

export interface ChildCheckRowBody {
  item_code: string;
  measured_value?: string | number | null;
  measured_unit?: string | null;
  measured_flag?: string | null;
  check_desc?: string | null;
  manual_conclusion?: string | null;
}

// ===== 送检单面 =====

export const listSubmissions = (params: {
  product_id?: number;
  status?: string;
  target_market?: string;
  page?: number;
  page_size?: number;
}) => unwrap<SubmissionPage>(api.get('/textile-submissions', { params }));

export const getSubmission = (id: number) =>
  unwrap<SubmissionDetail>(api.get(`/textile-submissions/${id}`));

export const createSubmission = (body: CreateSubmissionBody) =>
  unwrap<TextileSubmission>(api.post('/textile-submissions', body));

export const updateSubmission = (id: number, body: CreateSubmissionBody) =>
  unwrap<TextileSubmission>(api.put(`/textile-submissions/${id}`, body));

export const deleteSubmission = (id: number) =>
  unwrap<{ deleted: number }>(api.delete(`/textile-submissions/${id}`));

export const setSubmissionStatus = (id: number, status: string) =>
  unwrap<StatusResult>(api.put(`/textile-submissions/${id}/status`, { status }));

export const generateItems = (id: number, extraItemCodes: string[] = []) =>
  unwrap<{
    items: TextileSubmissionItemRow[];
    scope_note: string;
    needs_attention: boolean;
  }>(api.post(`/textile-submissions/${id}/items`, { extra_item_codes: extraItemCodes }));

export const upsertChildChecks = (id: number, rows: ChildCheckRowBody[]) =>
  unwrap<TextileChildCheck[]>(api.put(`/textile-submissions/${id}/child-checks`, { rows }));

export const judgeSubmission = (id: number) =>
  unwrap<JudgeResult>(api.put(`/textile-submissions/${id}/judgement`, {}));

// ===== 第三方报告面 =====

export const listReports = (submissionId: number) =>
  unwrap<{ reports: TextileReport[] }>(
    api.get('/textile-reports', { params: { submission_id: submissionId } })
  );

export const getReport = (id: number) => unwrap<ReportDetail>(api.get(`/textile-reports/${id}`));

export const createReport = (body: RegisterReportBody) =>
  unwrap<TextileReport>(api.post('/textile-reports', body));

export const updateReport = (id: number, body: RegisterReportBody) =>
  unwrap<TextileReport>(api.put(`/textile-reports/${id}`, body));

export const acceptReport = (id: number) =>
  unwrap<AcceptanceResult>(api.put(`/textile-reports/${id}/acceptance`, {}));

export const upsertResults = (id: number, rows: ResultRowBody[]) =>
  unwrap<TextileReportResult[]>(api.post(`/textile-reports/${id}/results`, { rows }));

export const judgeReport = (id: number) =>
  unwrap<TextileReportResult[]>(api.put(`/textile-reports/${id}/judgement`, {}));

// ===== 主数据面 =====

export const listTestItems = (basis?: string) =>
  unwrap<{ items: TextileTestItem[] }>(api.get('/textile-test-items', { params: { basis } }));

export const createTestItem = (body: Record<string, unknown>) =>
  unwrap<TextileTestItem>(api.post('/textile-test-items', body));

export const updateTestItem = (id: number, body: Record<string, unknown>) =>
  unwrap<TextileTestItem>(api.put(`/textile-test-items/${id}`, body));

export const deleteTestItem = (id: number) =>
  unwrap<{ deleted: number }>(api.delete(`/textile-test-items/${id}`));

export const listLimitRules = (params: { item_id?: number; safety_category?: string }) =>
  unwrap<{ rules: TextileLimitRule[] }>(api.get('/textile-limit-rules', { params }));

export const createLimitRule = (body: Record<string, unknown>) =>
  unwrap<TextileLimitRule>(api.post('/textile-limit-rules', body));

export const updateLimitRule = (id: number, body: Record<string, unknown>) =>
  unwrap<TextileLimitRule>(api.put(`/textile-limit-rules/${id}`, body));

export const deleteLimitRule = (id: number) =>
  unwrap<{ deleted: number }>(api.delete(`/textile-limit-rules/${id}`));

export const listExportRequirements = (market?: string) =>
  unwrap<{ requirements: TextileExportRequirement[] }>(
    api.get('/textile-export-requirements', { params: { market } })
  );

export const createExportRequirement = (body: Record<string, unknown>) =>
  unwrap<TextileExportRequirement>(api.post('/textile-export-requirements', body));

export const updateExportRequirement = (id: number, body: Record<string, unknown>) =>
  unwrap<TextileExportRequirement>(api.put(`/textile-export-requirements/${id}`, body));

export const deleteExportRequirement = (id: number) =>
  unwrap<{ deleted: number }>(api.delete(`/textile-export-requirements/${id}`));
