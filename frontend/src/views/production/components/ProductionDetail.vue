<!--
  ProductionDetail.vue - 生产管理订单详情
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('production.detail.title')"
    width="800px"
    destroy-on-close
    :aria-label="t('production.detail.ariaLabel')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="order" class="detail-content">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('production.detail.labelOrderNo')">{{
          order.order_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelProductId')">{{
          order.product_id
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelProductName')">{{
          order.product_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelWorkCenter')">{{
          order.work_center_id || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelPlannedQuantity')">{{
          order.planned_quantity
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualQuantity')">{{
          order.actual_quantity || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelScheduledStart')">{{
          order.scheduled_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelScheduledEnd')">{{
          order.scheduled_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualStart')">{{
          order.actual_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelActualEnd')">{{
          order.actual_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelStatus')">
          <el-tag :type="statusTagType">{{ statusLabel(order.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelPriority')">{{
          order.priority
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelCreateTime')" :span="2">
          {{ order.created_at || '-' }}
        </el-descriptions-item>
        <el-descriptions-item :label="t('production.detail.labelRemark')" :span="2">{{
          order.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('production.detail.buttonClose')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { PRODUCTION_ORDER_STATUS, type ProductionOrder } from '@/api/production'

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{
  visible: boolean
  order: ProductionOrder | null
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

/** 状态标签：优先 i18n，回退到 PRODUCTION_ORDER_STATUS 字典 */
const statusLabel = (status: string): string => {
  const key = `production.detail.status${status.charAt(0).toUpperCase() + status.slice(1)}`
  const translated = t(key)
  return translated === key
    ? PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || status
    : translated
}

// el-tag 组件支持的 type 联合类型
type TagType = '' | 'success' | 'warning' | 'info' | 'danger'

// 合法 TagType 集合
const VALID_TAG_TYPES: ReadonlySet<TagType> = new Set(['', 'success', 'warning', 'info', 'danger'])

/** 将任意字符串安全转换为 el-tag 合法 TagType */
const toTagType = (s: string): TagType => (VALID_TAG_TYPES.has(s as TagType) ? (s as TagType) : '')

// 状态字符串到 el-tag type 的原始映射
const statusTagTypeMap: Record<string, string> = {
  draft: 'info',
  planned: 'primary',
  in_progress: 'warning',
  completed: 'success',
  cancelled: 'danger',
}

// 状态对应的 el-tag type
const statusTagType = computed<TagType>(() => {
  const status = props.order?.status || ''
  return toTagType(statusTagTypeMap[status] || 'info')
})
</script>
