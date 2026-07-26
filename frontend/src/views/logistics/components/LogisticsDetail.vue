<!--
  LogisticsDetail.vue - 物流管理运单详情
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('logistics.detail.title')"
    width="600px"
    :aria-label="t('logistics.detail.aria.dialog')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('logistics.detail.label.waybillNo')">{{ detail.waybill_no }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.relatedOrder')">{{ detail.order_no }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.logisticsCompany')">{{ detail.logistics_company }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.trackingNumber')">{{ detail.tracking_number }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.driverName')">{{ detail.driver_name || '-' }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.driverPhone')">{{ detail.driver_phone || '-' }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.freight')">¥{{ detail.freight_fee || 0 }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.status')">
        <el-tag :type="getStatusTypeFmt(detail.status)">
          {{ statusTextFmt(detail.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.expectedArrival')">{{
        detail.expected_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.actualArrival')">{{
        detail.actual_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.notes')" :span="2">{{ detail.notes || '-' }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import type { LogisticsWaybill } from '@/api/logistics'
import { getStatusType } from '../composables/lgsFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 物流运单详情组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 详情数据
  detail: LogisticsWaybill
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

// 透传格式化函数
const getStatusTypeFmt = getStatusType

/** 状态文本：优先 i18n，未知状态回退到原始 status 字符串 */
const statusTextFmt = (status: string): string => {
  const key = `logistics.common.status.${status}`
  const translated = t(key)
  return translated === key ? status : translated
}
</script>
