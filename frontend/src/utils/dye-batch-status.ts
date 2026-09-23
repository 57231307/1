import { logger } from '@/utils/logger';

/**
 * 缸号（dye_batch.status）全生命周期状态的单一映射源。
 *
 * 取值与后端 models/status/quality_dyeing.rs 的 dye_batch_lifecycle_status 模块逐字一致
 * （全小写下划线，共 16 态），亦是 services/dye_batch_state_machine_validation.rs 的
 * validate_lifecycle_status / is_valid_status 白名单。列表接口用
 * dye_batch::Column::Status.eq(status) 精确匹配、不做大小写归一也不做取值校验
 * （handlers/dye_batch_handler.rs::list_dye_batches），因此「状态筛选下拉的提交值」
 * 「行内操作按钮的状态门槛」「状态标签文案/配色键」三处都必须使用本模块的原值，任何其它
 * 形态（如已废弃的 ACTIVE/COMPLETED、或大小写改写）在后端既筛不出数据也对应不上文案。
 */
export const DYE_BATCH_LIFECYCLE_STATUS = {
  PENDING_SCHEDULE: 'pending_schedule',
  SCHEDULED: 'scheduled',
  PREPARING: 'preparing',
  DYEING: 'dyeing',
  WASHING: 'washing',
  FIXING: 'fixing',
  DEHYDRATING: 'dehydrating',
  DRYING: 'drying',
  INSPECTING: 'inspecting',
  STORED: 'stored',
  SHIPPED: 'shipped',
  CANCELLED: 'cancelled',
  TERMINATED: 'terminated',
  REWORK: 'rework',
  ON_HOLD: 'on_hold',
  FAILED: 'failed',
} as const;

export type DyeBatchLifecycleStatus =
  (typeof DYE_BATCH_LIFECYCLE_STATUS)[keyof typeof DYE_BATCH_LIFECYCLE_STATUS];

/** 全部合法状态值（按生命周期顺序）：筛选下拉的取值来源 */
export const DYE_BATCH_LIFECYCLE_STATUSES: DyeBatchLifecycleStatus[] = Object.values(
  DYE_BATCH_LIFECYCLE_STATUS
);

/** el-tag 配色类型（与 element-plus Tag 的 type 属性对齐，'' 为默认主题色） */
export type DyeBatchTagType = '' | 'success' | 'warning' | 'info' | 'danger';

/** 状态 → i18n 文案键（locales 的 dyeBatch.statusLabels.*，键名即后端原值） */
export const DYE_BATCH_STATUS_LABEL_KEYS: Record<DyeBatchLifecycleStatus, string> = {
  pending_schedule: 'dyeBatch.statusLabels.pending_schedule',
  scheduled: 'dyeBatch.statusLabels.scheduled',
  preparing: 'dyeBatch.statusLabels.preparing',
  dyeing: 'dyeBatch.statusLabels.dyeing',
  washing: 'dyeBatch.statusLabels.washing',
  fixing: 'dyeBatch.statusLabels.fixing',
  dehydrating: 'dyeBatch.statusLabels.dehydrating',
  drying: 'dyeBatch.statusLabels.drying',
  inspecting: 'dyeBatch.statusLabels.inspecting',
  stored: 'dyeBatch.statusLabels.stored',
  shipped: 'dyeBatch.statusLabels.shipped',
  cancelled: 'dyeBatch.statusLabels.cancelled',
  terminated: 'dyeBatch.statusLabels.terminated',
  rework: 'dyeBatch.statusLabels.rework',
  on_hold: 'dyeBatch.statusLabels.on_hold',
  failed: 'dyeBatch.statusLabels.failed',
} as Record<DyeBatchLifecycleStatus, string>;

/** 状态 → el-tag 配色 */
export const DYE_BATCH_STATUS_TAG_TYPES: Record<DyeBatchLifecycleStatus, DyeBatchTagType> = {
  pending_schedule: 'info',
  scheduled: '',
  preparing: 'warning',
  dyeing: 'warning',
  washing: 'warning',
  fixing: 'warning',
  dehydrating: 'warning',
  drying: 'warning',
  inspecting: 'warning',
  stored: 'success',
  shipped: 'success',
  cancelled: 'danger',
  terminated: 'danger',
  rework: 'danger',
  on_hold: 'info',
  failed: 'danger',
};

/**
 * 把后端返回的缸号状态归一到已知枚举。
 * 词表外的值（含大小写写错）不回显裸枚举，而是记错误日志并抛错——映射缺口必须让页面和
 * 日志同时报错，而不是伪装成"看起来像个状态"。
 * dye_batch.status 为可空列（Option<String>）：null/空串是合法的数据缺省，交由调用方按缺省处理，
 * 不算词表违规；只有携带了词表外取值才算脏数据。
 */
export function normalizeDyeBatchStatus(
  status: string | null | undefined
): DyeBatchLifecycleStatus {
  if ((DYE_BATCH_LIFECYCLE_STATUSES as readonly string[]).includes(status ?? '')) {
    return status as DyeBatchLifecycleStatus;
  }
  const message =
    `未识别的缸号状态「${String(status)}」，` +
    '合法取值见后端 models/status/quality_dyeing.rs 的 dye_batch_lifecycle_status 模块（16 态小写）';
  logger.error(message, { status });
  throw new Error(message);
}

/** 是否为合法生命周期状态（可空列判定用：null/空串视为缺省，不抛错） */
export function isDyeBatchStatus(
  status: string | null | undefined
): status is DyeBatchLifecycleStatus {
  return (DYE_BATCH_LIFECYCLE_STATUSES as readonly string[]).includes(status ?? '');
}

export function dyeBatchStatusLabelKey(status: DyeBatchLifecycleStatus): string {
  return DYE_BATCH_STATUS_LABEL_KEYS[status];
}

export function dyeBatchStatusTagType(status: DyeBatchLifecycleStatus): DyeBatchTagType {
  return DYE_BATCH_STATUS_TAG_TYPES[status];
}
