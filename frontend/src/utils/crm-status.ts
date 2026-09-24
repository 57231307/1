import { logger } from '@/utils/logger';

/**
 * CRM 线索状态（crm_lead.lead_status）的单一映射源。
 *
 * 取值与后端权威值逐字一致（全小写）：
 * new / assigned / contacted / qualified / converted / lost / pool
 *
 * Record<LeadStatus, …> 让「后端词表新增状态必须补全映射」由编译器保证。
 */
export const LEAD_STATUS = {
  NEW: 'new',
  ASSIGNED: 'assigned',
  CONTACTED: 'contacted',
  QUALIFIED: 'qualified',
  CONVERTED: 'converted',
  LOST: 'lost',
  POOL: 'pool',
} as const;

export type LeadStatus = (typeof LEAD_STATUS)[keyof typeof LEAD_STATUS];

/** 全部合法线索状态值（按业务流程排序）：筛选下拉的取值来源 */
export const LEAD_STATUSES: LeadStatus[] = Object.values(LEAD_STATUS);

/** 线索状态 → i18n 文案键 */
export const LEAD_STATUS_LABEL_KEYS: Record<LeadStatus, string> = {
  new: 'crmLeads.leadStatus.new',
  assigned: 'crmLeads.leadStatus.assigned',
  contacted: 'crmLeads.leadStatus.contacted',
  qualified: 'crmLeads.leadStatus.qualified',
  converted: 'crmLeads.leadStatus.converted',
  lost: 'crmLeads.leadStatus.lost',
  pool: 'crmLeads.leadStatus.pool',
};

/** 线索状态 → el-tag 配色 */
export type CrmTagType = '' | 'success' | 'warning' | 'info' | 'danger' | 'primary';

export const LEAD_STATUS_TAG_TYPES: Record<LeadStatus, CrmTagType> = {
  new: 'info',
  assigned: 'info',
  contacted: 'warning',
  qualified: 'primary',
  converted: 'success',
  lost: 'danger',
  pool: 'info',
};

/**
 * 把后端返回的线索状态归一到已知枚举。
 * - null/undefined/空串（字段缺失或尚无状态）：返回 undefined，由调用方渲染中性占位；
 * - 词表外的非空取值（脏数据）：记错误日志并抛错，映射缺口必须让页面和日志同时报错。
 */
export function normalizeLeadStatus(status: string | undefined | null): LeadStatus | undefined {
  if (status == null || status === '') return undefined;
  if ((LEAD_STATUSES as readonly string[]).includes(status)) {
    return status as LeadStatus;
  }
  const message =
    `未识别的线索状态「${status}」，` + '合法取值见后端 crm_lead.lead_status 权威词表（全小写）';
  logger.error(message, { status });
  throw new Error(message);
}

export function leadStatusLabelKey(status: string | undefined | null): string {
  const normalized = normalizeLeadStatus(status);
  return normalized ? LEAD_STATUS_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function leadStatusTagType(status: string | undefined | null): CrmTagType {
  const normalized = normalizeLeadStatus(status);
  return normalized ? LEAD_STATUS_TAG_TYPES[normalized] : 'info';
}

/**
 * CRM 商机阶段（crm_opportunity.opportunity_stage）的单一映射源。
 *
 * 取值与后端权威值逐字一致（全大写）：
 * QUALIFICATION / NEEDS_ANALYSIS / PROPOSAL / NEGOTIATION / CLOSED_WON / CLOSED_LOST
 *
 * Record<OpportunityStage, …> 让「后端词表新增阶段必须补全映射」由编译器保证。
 */
export const OPPORTUNITY_STAGE = {
  QUALIFICATION: 'QUALIFICATION',
  NEEDS_ANALYSIS: 'NEEDS_ANALYSIS',
  PROPOSAL: 'PROPOSAL',
  NEGOTIATION: 'NEGOTIATION',
  CLOSED_WON: 'CLOSED_WON',
  CLOSED_LOST: 'CLOSED_LOST',
} as const;

export type OpportunityStage = (typeof OPPORTUNITY_STAGE)[keyof typeof OPPORTUNITY_STAGE];

/** 全部合法商机阶段值（按业务流程排序）：筛选下拉的取值来源 */
export const OPPORTUNITY_STAGES: OpportunityStage[] = Object.values(OPPORTUNITY_STAGE);

/** 商机阶段 → i18n 文案键 */
export const OPPORTUNITY_STAGE_LABEL_KEYS: Record<OpportunityStage, string> = {
  QUALIFICATION: 'crmOpportunities.stageLabels.QUALIFICATION',
  NEEDS_ANALYSIS: 'crmOpportunities.stageLabels.NEEDS_ANALYSIS',
  PROPOSAL: 'crmOpportunities.stageLabels.PROPOSAL',
  NEGOTIATION: 'crmOpportunities.stageLabels.NEGOTIATION',
  CLOSED_WON: 'crmOpportunities.stageLabels.CLOSED_WON',
  CLOSED_LOST: 'crmOpportunities.stageLabels.CLOSED_LOST',
};

/** 商机阶段 → el-tag 配色 */
export const OPPORTUNITY_STAGE_TAG_TYPES: Record<OpportunityStage, CrmTagType> = {
  QUALIFICATION: 'info',
  NEEDS_ANALYSIS: '',
  PROPOSAL: 'warning',
  NEGOTIATION: 'primary',
  CLOSED_WON: 'success',
  CLOSED_LOST: 'danger',
};

/**
 * 把后端返回的商机阶段归一到已知枚举。
 * - null/undefined/空串：返回 undefined；
 * - 词表外的非空取值（脏数据）：记错误日志并抛错。
 */
export function normalizeOpportunityStage(
  stage: string | undefined | null
): OpportunityStage | undefined {
  if (stage == null || stage === '') return undefined;
  if ((OPPORTUNITY_STAGES as readonly string[]).includes(stage)) {
    return stage as OpportunityStage;
  }
  const message =
    `未识别的商机阶段「${stage}」，` +
    '合法取值见后端 crm_opportunity.opportunity_stage 权威词表（全大写）';
  logger.error(message, { stage });
  throw new Error(message);
}

export function opportunityStageLabelKey(stage: string | undefined | null): string {
  const normalized = normalizeOpportunityStage(stage);
  return normalized ? OPPORTUNITY_STAGE_LABEL_KEYS[normalized] : 'common.statusUnknown';
}

export function opportunityStageTagType(stage: string | undefined | null): CrmTagType {
  const normalized = normalizeOpportunityStage(stage);
  return normalized ? OPPORTUNITY_STAGE_TAG_TYPES[normalized] : 'info';
}
