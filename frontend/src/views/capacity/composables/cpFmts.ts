// capacity 格式化工具集合
// 拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
import { i18n } from '@/i18n';
import { logger } from '@/utils/logger';

/**
 * 工作中心负荷状态词表（产能表「状态」列使用）。
 * 来源为写入方 capacity_service.rs::build_capacity_load_item：
 * load_rate>100⇒OVERLOADED、>80⇒HIGH、>20⇒NORMAL、否则 IDLE。
 * （work-centers 端点的 WorkCenterCapacity.status 是运营态 ACTIVE/INACTIVE，属另一词表，
 *  与本列无关——本列取的是 load-analysis 的负荷态。）
 */
const LOAD_STATUSES = ['IDLE', 'NORMAL', 'HIGH', 'OVERLOADED'] as const;
export type WorkLoadStatus = (typeof LOAD_STATUSES)[number];

const normalize = (status: string | undefined | null): WorkLoadStatus | undefined => {
  if (status == null || status === '') return undefined;
  if ((LOAD_STATUSES as readonly string[]).includes(status)) return status as WorkLoadStatus;
  const message =
    `未识别的工作中心负荷状态「${status}」，` +
    '合法取值见 capacity_service.rs 的 IDLE/NORMAL/HIGH/OVERLOADED';
  logger.error(message, { status });
  throw new Error(message);
};

type TagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 负荷状态 → ElTag 配色 */
export const getStatusType = (status: string | undefined | null): TagType => {
  const map: Record<WorkLoadStatus, TagType> = {
    IDLE: 'success',
    NORMAL: '',
    HIGH: 'warning',
    OVERLOADED: 'danger',
  };
  const normalized = normalize(status);
  return normalized ? map[normalized] : 'info';
};

/** 负荷状态 → 本地化标签（capacityModule.workCenterStatus.*，键名即后端原值） */
export const getStatusLabel = (status: string | undefined | null): string => {
  const normalized = normalize(status);
  return i18n.global.t(
    normalized ? `capacityModule.workCenterStatus.${normalized}` : 'common.statusUnknown'
  );
};

/** 负荷率（百分比，后端已 ×100） → ElTag 配色，阈值与后端状态分级一致 */
export const getLoadRateType = (rate: number): 'success' | 'warning' | 'danger' => {
  if (rate > 100) return 'danger';
  if (rate > 80) return 'warning';
  return 'success';
};
