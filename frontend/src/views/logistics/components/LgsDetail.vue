<!--
  LgsDetail.vue - 物流管理运单详情
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="运单详情"
    width="600px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item label="运单号">{{ detail.waybill_no }}</el-descriptions-item>
      <el-descriptions-item label="关联订单">{{ detail.order_no }}</el-descriptions-item>
      <el-descriptions-item label="物流公司">{{ detail.logistics_company }}</el-descriptions-item>
      <el-descriptions-item label="快递单号">{{ detail.tracking_number }}</el-descriptions-item>
      <el-descriptions-item label="司机姓名">{{ detail.driver_name || '-' }}</el-descriptions-item>
      <el-descriptions-item label="司机电话">{{ detail.driver_phone || '-' }}</el-descriptions-item>
      <el-descriptions-item label="运费">¥{{ detail.freight_fee || 0 }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="(getStatusTypeFmt(detail.status) as any)">
          {{ getStatusTextFmt(detail.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="预计到达">{{
        detail.expected_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="实际到达">{{
        detail.actual_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item label="备注" :span="2">{{ detail.notes || '-' }}</el-descriptions-item>
    </el-descriptions>
  </el-dialog>
</template>

<script setup lang="ts">
import type { LogisticsWaybill } from '@/api/logistics'
import { getStatusType, getStatusText } from '../composables/lgsFmts'

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
const getStatusTextFmt = getStatusText
</script>
