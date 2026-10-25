import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 客户信用出参载体：逐键 = backend/src/services/customer_credit_service.rs
 * 的读模型 CustomerCreditView（derive Serialize，无 rename/rename_all）。
 * 该读模型的键名、类型、可空性与 backend/src/models/customer_credit.rs 的实体
 * Model 逐键同源（customer_name 由 customer_id LEFT JOIN customers 富化）；
 * 出参点：handlers/customer_credit_handler.rs 的 list_credits、get_credit、
 * set_credit_rating、create_credit、update_credit 五个出口。
 * 出参键集恒为下面这 15 个 snake_case 键，不多不少。
 *
 * 不属于本载体的键（读写都取不到值：读恒 undefined，写恒被 serde 丢弃）：
 * - credit_rating：等级的真实键是 credit_level；credit_rating 只存在于评估端点出参
 *   CreditEvaluationResult 上，与列表/详情是两个不同载体，不得混用同一个接口；
 * - valid_from / valid_to / remarks：表 customer_credit_ratings 没有这三列
 *   （backend/migration/src/domain/business/m0012_add_ap_ar_finance_analysis.rs），
 *   读模型与 DTO 也都没有对应字段。声明它们只会把"后端没下发"伪装成"值可缺席"。
 *
 * NOT NULL 列不标 ?（标了就是把"后端没下发这个键"合法化成"可能没有"）：
 * id/customer_id/credit_limit/used_credit/available_credit/status/created_at/updated_at。
 * credit_limit/used_credit/available_credit 是 Decimal，backend/Cargo.toml 的
 * rust_decimal 只启 serde feature（未启 serde-float），序列化走 serialize_str，
 * 线上是十进制字符串，故声明为 string（与 api/customer.ts 的 CustomerCreditInfo
 * 同口径；参与运算请用 utils/money.ts 的 decimalWireToNumber，不要在模板里直接和数字比较）。
 *
 * 调用方：views/customer-credit（列表、详情、编辑、评级、调整、占用/释放、停用、评估）
 * 与 api/index.ts 的统一出口。入参：见各函数签名。传给谁：HTTP 请求体/查询串与响应类型标注。
 * 存什么/存哪里：本模块不落盘、不缓存，只做契约声明与请求发送。
 */
export interface CustomerCredit {
  id: number;
  customer_id: number;
  /**
   * 读模型里该键恒出，唯一取值来源是 services/customer_credit_service.rs 的
   * column_as(customer::Column::CustomerName, "customer_name") JOIN 投影；客户行
   * 缺失时如实为 null（键必出）。前端只按真实键绑定，不得用 customer_id 拼假名、
   * 也不得兜底显示 id。
   */
  customer_name: string | null;
  /** 信用等级（字母档，如 B）；null = 未评，界面须显式区分，不得塌成 0 */
  credit_level: string | null;
  credit_score: number | null;
  credit_limit: string;
  used_credit: string;
  available_credit: string;
  credit_days: number | null;
  last_assessment_date: string | null;
  next_assessment_date: string | null;
  status: string;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * POST /crm/customer-credits/{id}/rating 与 POST /crm/customer-credits（创建）的请求体：
 * 严格对齐后端 handlers/customer_credit_handler.rs 的 CreditRatingRequestDto。
 * 该评分 handler 无 Path 形参，customer_id 只从请求体读取（i32 非 Option 必填），
 * 路径 {id} 仅为路由占位（见 routes/crm.rs 的挂载）。
 * 表单采集的 credit_level/credit_score/credit_days/credit_limit/remark 与必填
 * customer_id 必须一次全量上送；rating/reason 这类后端不认的键会被 serde 丢弃，
 * 缺 customer_id 则必然 400。
 *
 * 各键类型如实反映 Option<T>：null 与"键缺席"在该 DTO 上等价（serde 都解成 None）。
 * 后端 None 语义见 services/customer_credit_limit.rs 的 set_credit_rating（等级兜 B、
 * 分数兜 60、账期兜 30、更新时 credit_limit 保持原值）——这些默认是后端的产品口径，
 * 前端不得代替用户预填 0/B/60/30 去"对齐"它（详见 views/customer-credit/tabs/RatingDialogTab.vue）。
 *
 * 注意：remark 在该 DTO 上合法（serde 收下），但 services/customer_credit_service.rs
 * 的 CreditRatingRequest.remark 挂着 allow(dead_code, reason = "预留")，且
 * set_credit_rating 的 update/insert 两条分支都没把它写进 ActiveModel、表也没有
 * 对应列，提交的备注当前不会落库。这是后端/DB 侧缺口，不由前端删字段掩盖。
 */
export interface SetCreditRatingInput {
  customer_id: number;
  /** 信用等级（后端校验 ≤20 字符） */
  credit_level?: string | null;
  credit_score?: number | null;
  /** 信用额度：后端 Option<Decimal>，None=保持原值、Some(v)=显式设置（含 0） */
  /** 额度线形态：控件侧是 number，提交前转十进制字符串；后端 Decimal 两种形态都接受 */
  credit_limit?: string | number | null;
  credit_days?: number | null;
  /** 备注（后端校验 ≤500 字符）；后端键名是单数 remark，实体出参里根本没有备注键 */
  remark?: string | null;
}

/**
 * PUT /crm/customer-credits/{id} 的请求体：严格对齐后端
 * handlers/customer_credit_handler.rs 的 UpdateCreditDto（全部 Option，只更新提交的字段）。
 *
 * 显式不声明的键（不得出现在载荷里）：
 * - status：UpdateCreditDto 没有该字段，PUT 带了会被 serde 丢弃；改状态的真实入口
 *   是 POST /{id}/deactivate（软停用），编辑对话框因此不提供状态控件，避免
 *   摆一个静默无效的假控件。
 * - id/customer_id：路径参数已给出客户，DTO 无对应字段。
 * - used_credit/available_credit：由占用/释放/调整端点按业务动作推导，不接受直接赋值。
 * - valid_from/valid_to/remarks：既非 DTO 字段也非表列。
 */
export interface UpdateCustomerCreditInput {
  credit_level?: string | null;
  credit_score?: number | null;
  /** 额度线形态：控件侧是 number，提交前转十进制字符串；后端 Decimal 两种形态都接受 */
  credit_limit?: string | number | null;
  credit_days?: number | null;
  remark?: string | null;
}

export interface CreditAdjustment {
  amount: number;
  reason: string;
  adjustment_type: 'increase' | 'decrease';
}

/**
 * 信用占用/释放入参：严格对齐后端 CreditAmountRequest
 * （handlers/customer_credit_handler.rs，仅一个 amount 字段，
 * 且绑定 validate_amount_range → 必须 >0）。后端不接收 business_type/business_id，
 * 多传只会造成"前端以为记了、后端没落库"的契约谎言，故不声明。
 */
export interface CreditOccupation {
  amount: number;
}

/** 后端 services/customer_credit_service.rs 的 EvaluationFactor */
export interface CreditEvaluationFactor {
  factor_name: string;
  weight: number;
  score: number;
  description: string;
}

/**
 * POST /crm/customer-credits/evaluate 的出参载体 =
 * services/customer_credit_service.rs 的 CreditEvaluationResult（handler 的 evaluate
 * 出口序列化它，不是 customer_credit::Model 或 CustomerCreditView）。
 * 这是本域唯一真实带 credit_rating 键的载体（credit_rating: String，
 * 由 customer_credit_evaluate.rs 按分数换算出的字母档）；它与列表/详情载体
 * （CustomerCredit）必须各自独立声明——混用同一个接口就是 credit_rating 被错绑
 * 到列表列的通道。
 * recommended_limit 是 Decimal，线上为字符串；credit_score/weight 是真数值（i32/f64）。
 */
export interface CreditEvaluationResult {
  customer_id: number;
  customer_name: string;
  credit_score: number;
  credit_rating: string;
  recommended_limit: string;
  evaluation_factors: CreditEvaluationFactor[];
  evaluation_date: string;
}

/**
 * 评估入参 = handlers/customer_credit_handler.rs 的 CreditEvaluationRequest，
 * 只有 customer_id + evaluation_date 两个字段；该端点无路径参数（routes/crm.rs），
 * 载荷不得携带 id——该键不在 DTO 上，传了只会被 serde 丢弃（幽灵写键）。
 */
export interface CreditEvaluationRequest {
  customer_id: number;
  evaluation_date: string;
}

/**
 * 列表筛选入参 = handlers/customer_credit_handler.rs 的 CreditQuery
 * （customer_id/credit_level/status/page/page_size）。
 * 后端没有 keyword 字段：查询串里带 keyword 只会被整键丢弃，形成"前端以为筛了、
 * 后端没筛"的幽灵查询键，故不声明；等级筛选用真实的 credit_level 键。
 */
export interface CustomerCreditQueryParams {
  page?: number;
  page_size?: number;
  credit_level?: string;
  status?: string;
  customer_id?: number;
}

/**
 * 客户信用列表出参声明，分页信封形态。
 * 功能，声明该列表端点的返回载荷是 items total page page_size 四键的分页信封；
 * items 的行载体是本文件顶部的 CustomerCredit（= 后端 CustomerCreditView 富化视图），
 * 信封只包裹分页元数据，不改变行键集。
 * 调用方，客户信用列表页与一切按分页消费该端点的视图。
 * 入参，可选的 page page_size credit_level status customer_id 查询参数
 * （后端 CreditQuery 没有 keyword 键，筛等级用 credit_level）。
 * 传给谁，后端 CRM 域 customer-credits 资源的 list_credits handler。
 * 存什么，只描述出参形状，不写库。
 * 存哪里，无。
 * 依据，后端 list_credits 的返回类型是 ApiResponse 包 PaginatedResponse，
 * 信封四键在前端类型 api-response 与后端 utils response 两侧同源，
 * 由 check-api-envelope 门禁与客户信用动作权限与信封锁共同钉住；
 * 此前该端点出参是裸数组、服务层算出的 total 以 _total 丢弃，属契约缺口，本形态已补齐。
 */
export const getCustomerCreditList = (
  params?: CustomerCreditQueryParams
): Promise<ApiResponse<PaginatedResponse<CustomerCredit>>> =>
  request.get('/crm/customer-credits', { params });

/**
 * 详情：路由写的是 {id}，但后端 get_credit(Path(customer_id)) 与
 * service.get_view_by_customer_id 都按 customer_id 查（Column::CustomerId.eq）——
 * 该 {id} 是客户自然键（= customers.id），不是行主键 customer_credit_ratings.id；
 * 传信用行的主键 id 会读到错行或 404。故形参命名为 customerId。
 */
export const getCustomerCredit = (customerId: number): Promise<ApiResponse<CustomerCredit>> =>
  request.get(`/crm/customer-credits/${customerId}`);

/** POST /crm/customer-credits → create_credit，收 CreditRatingRequestDto（与评分端点同 DTO） */
export const createCustomerCredit = (
  data: SetCreditRatingInput
): Promise<ApiResponse<CustomerCredit>> => request.post('/crm/customer-credits', data);

/** PUT /crm/customer-credits/{id}：路径 {id} 与详情端点同语义，都是 customer_id 自然键 */
export const updateCustomerCredit = (
  customerId: number,
  data: UpdateCustomerCreditInput
): Promise<ApiResponse<CustomerCredit>> => request.put(`/crm/customer-credits/${customerId}`, data);

/** DELETE /crm/customer-credits/{id}：后端 delete_credit 实际执行 deactivate 软停用（与 POST /{id}/deactivate 等价） */
export const deleteCustomerCredit = (customerId: number): Promise<ApiResponse<void>> =>
  request.delete(`/crm/customer-credits/${customerId}`);

/** 评分端点不读路径 id（customer_id 只从请求体取），形参 customerId 仅拼路由占位段 */
export const setCreditRating = (
  customerId: number,
  data: SetCreditRatingInput
): Promise<ApiResponse<CustomerCredit>> =>
  request.post(`/crm/customer-credits/${customerId}/rating`, data);

/** 占用额度：路径 {id} 是客户自然键（与详情端点同口径），载荷见 CreditOccupation */
export const occupyCredit = (
  customerId: number,
  data: CreditOccupation
): Promise<ApiResponse<void>> => request.post(`/crm/customer-credits/${customerId}/occupy`, data);

/**
 * 释放信用额度：后端 release_credit 收 CreditAmountRequest
 * （handlers/customer_credit_handler.rs，字段只有 amount），
 * 服务层按 amount 递减 used_credit，不存在"占用记录 ID"这一入参；
 * 请求体缺 amount 必然 serde 反序列化失败报参数错误。
 * 路径 {id} 是客户自然键（与详情端点同口径）。
 */
export const releaseCredit = (
  customerId: number,
  data: CreditOccupation
): Promise<ApiResponse<void>> => request.post(`/crm/customer-credits/${customerId}/release`, data);

/** 调整额度：后端 adjust_credit_limit(Path(customer_id))，路径键是客户自然键 */
export const adjustCreditLimit = (
  customerId: number,
  data: CreditAdjustment
): Promise<ApiResponse<CustomerCredit>> =>
  request.post(`/crm/customer-credits/${customerId}/adjust`, data);

/** 停用：后端 deactivate_credit(Path(customer_id))，service.deactivate 按 CustomerId 过滤（路径键是客户自然键） */
export const deactivateCredit = (customerId: number): Promise<ApiResponse<void>> =>
  request.post(`/crm/customer-credits/${customerId}/deactivate`);

/** 评估：无路径参数（routes/crm.rs），出参是 CreditEvaluationResult 而非信用实体 */
export const evaluateCustomerCredit = (
  data: CreditEvaluationRequest
): Promise<ApiResponse<CreditEvaluationResult>> =>
  request.post('/crm/customer-credits/evaluate', data);
