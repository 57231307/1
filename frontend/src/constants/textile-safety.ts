/**
 * 纺织品安全合规域词表（与 backend/src/models/status/textile_safety.rs 同源的展示换算表）
 *
 * 存储 token 一律是后端稳定码（带 GB18401_ 前缀、内销 dom 等），展示文本只在本文件换算；
 * 新增取值必须先改后端词表与迁移 CHECK，再随动本表，两处集合相等由后端纸面用例锁死。
 */

export const SAFETY_CATEGORY_LABELS: Record<string, string> = {
  GB18401_A: 'A类（婴幼儿）',
  GB18401_B: 'B类（直接接触皮肤）',
  GB18401_C: 'C类（非直接接触）',
};

export const END_USER_GROUP_LABELS: Record<string, string> = {
  infant: '婴幼儿',
  child: '儿童',
  adult: '成人',
};

export const CONTACT_TYPE_LABELS: Record<string, string> = {
  direct: '直接接触皮肤',
  indirect: '非直接接触皮肤',
  infant_use: '婴幼儿用品（三岁以下）',
};

export const PRODUCT_FORM_LABELS: Record<string, string> = {
  final: '最终产品',
  intermediate: '中间产品',
};

export const TARGET_MARKET_LABELS: Record<string, string> = {
  dom: '内销',
  eu: '欧盟',
  us: '美国',
  jp: '日本',
  other: '其它市场',
};

export const COLOR_ROLE_LABELS: Record<string, string> = {
  full: '全色考核',
  representative: '代表色考核',
  customer_specified: '客户指定色',
};

export const EXEMPTION_TAG_LABELS: Record<string, string> = {
  wet_processing: '后续湿处理（中间产品 pH 放宽档）',
  laundering_fade_non_final: '需洗涤会褪色的非最终产品（色牢度不考核）',
  plain_or_bleached: '本色或漂白（色牢度不考核）',
  traditional_hand_dyed: '扎染蜡染传统手工着色（色牢度不考核）',
};

export const SUBMISSION_STATUS_LABELS: Record<string, string> = {
  draft: '草稿',
  sampling: '待取样',
  sent: '已送检',
  reported: '已收报告',
  judged: '已判定',
  closed: '已关闭',
  withdrawn: '已撤回',
};

export const VERDICT_LABELS: Record<string, string> = {
  pass: '合格',
  fail: '不合格',
  na: '不考核',
  pending: '待判定',
};

export const ACCEPTANCE_STATUS_LABELS: Record<string, string> = {
  pending: '待核对',
  accepted: '已受理',
  rejected: '已退回',
};

export const ELEMENT_CODE_LABELS: Record<string, string> = {
  cma: '缺 CMA 资质',
  cnas: '缺 CNAS 资质',
  sample_name: '缺样品名称',
  material_composition: '缺材质成分',
  color_no: '缺色号',
  test_basis: '检验依据缺失或格式不全',
  item_results: '缺检测项目与结果',
  conclusion: '缺判定结论',
  expired: '报告过期或缺有效期',
  not_yet_valid: '报告出具日尚未到达',
  batch_mismatch: '批次口径不一致',
};

/** 多份报告采信排除原因（后端 models::status::report_adoption_reason 同字典，单源消费） */
export const ADOPTION_REASON_LABELS: Record<string, string> = {
  adopted: '本次采信',
  superseded: '已被出具日更新的报告替代',
  expired: '已过有效期',
  valid_window_missing: '缺有效期无法核验',
  not_yet_valid: '出具日尚未到达',
  not_accepted: '完整性门未放行',
};

export const BASIS_STANDARD_LABELS: Record<string, string> = {
  GB18401: 'GB 18401-2010 基本安全',
  GB31701: 'GB 31701-2015 婴童加审',
  EXPORT: '出口客户要求',
};

export const ITEM_KIND_LABELS: Record<string, string> = {
  numeric: '数值型',
  grade: '等级型',
  sensory: '感官型',
  boolean: '证据型',
};

export const CHECK_KIND_LABELS: Record<string, string> = {
  measurable: '可测值',
  manual: '人工核查',
};

export const COMPARATOR_LABELS: Record<string, string> = {
  lte: '不大于上限',
  gte: '不低于下限',
  between: '区间内',
  forbidden: '禁用',
  must_none: '必须为无',
  na: '不考核',
};

export const OEKO_CLASS_LABELS: Record<string, string> = {
  I: 'I 级（婴幼儿）',
  II: 'II 级（直接接触皮肤）',
  III: 'III 级（非直接接触）',
  IV: 'IV 级（装饰材料）',
};

export const REQUIREMENT_TYPE_LABELS: Record<string, string> = {
  oeko_tex: 'OEKO-TEX 认证分级',
  reach_svhc: 'REACH 高度关注物质',
  cpsia_lead: 'CPSIA 铅',
  cpsia_phthalate: 'CPSIA 邻苯二甲酸酯',
  cpsia_cert: 'CPSIA 儿童产品证书',
  jp_formaldehyde: '日本甲醛管控',
};

/** 送检项目清单归因标签（后端 models::status::textile_item_source 同字典） */
export const ITEM_SOURCE_LABELS: Record<string, string> = {
  GB18401: '法定基本安全',
  GB31701: '婴童加考',
  EXPORT: '出口市场推荐',
  CUSTOMER: '客户点名',
};

/** 归因标签到 el-tag type 的映射：法定与婴童为 mandatory，出口为 recommended，客户为 optional */
export const ITEM_SOURCE_TAG_TYPE: Record<string, string> = {
  GB18401: 'danger',
  GB31701: 'warning',
  EXPORT: 'primary',
  CUSTOMER: 'info',
};

/** 等级 token 九档（半级含 3-4），与后端 textile_fastness_grade::ALL 同源 */
export const FASTNESS_GRADE_TOKENS = ['1', '1-2', '2', '2-3', '3', '3-4', '4', '4-5', '5'];

export const FLAG_VALUE_LABELS: Record<string, string> = {
  none: '无/未检出',
  detected: '检出',
};

/** 送检单状态机可编辑态（与后端 submission_status::EDITABLE 同源） */
export const EDITABLE_STATUSES = ['draft', 'sampling', 'sent'];

/** 可人工推进的状态迁移表（与后端 ALLOWED_TRANSITIONS 同源，UI 只呈现合法按钮） */
export const STATUS_TRANSITIONS: Record<string, string[]> = {
  draft: ['sampling', 'withdrawn'],
  sampling: ['sent', 'withdrawn'],
  sent: ['withdrawn'],
  reported: ['withdrawn'],
  judged: ['closed'],
  closed: [],
  withdrawn: [],
};

export function optionList(map: Record<string, string>) {
  return Object.entries(map).map(([value, label]) => ({ value, label }));
}
