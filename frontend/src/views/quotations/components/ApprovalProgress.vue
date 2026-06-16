<!--
  审批进度组件
  - 4 步进度条：草稿 → 提交审批 → 批准/拒绝 → 转订单（可选）
  - 状态: process / success / error / wait
-->
<template>
  <div class="approval-progress">
    <el-steps :active="activeStep" align-center finish-status="success">
      <el-step title="草稿" description="编辑中" />
      <el-step title="提交审批" :description="pendingDescription" />
      <el-step
        :title="approved ? '已批准' : '已拒绝'"
        :description="finalDescription"
        :status="finalStatus"
      />
      <el-step
        v-if="showConverted"
        title="已转订单"
        :description="convertedDescription"
        status="success"
      />
      <el-step v-if="cancelled" title="已取消" description="报价单已取消" status="error" />
    </el-steps>
  </div>
</template>

<script setup lang="ts">
// 审批进度组件脚本
// - 根据 status 计算当前步骤
// - 4 步：草稿 / 提交审批 / 批准或拒绝 / 已转订单（条件）
import { computed } from 'vue'
import type { QuotationStatus } from '@/api/quotation'

const props = defineProps<{
  status: QuotationStatus | string
  approvedAt?: string
  approvedByName?: string
  rejectionReason?: string
  convertedAt?: string
  convertedOrderId?: number
}>()

/** 当前活动步骤 */
const activeStep = computed(() => {
  switch (props.status) {
    case 'draft':
      return 0
    case 'pending_approval':
      return 1
    case 'approved':
    case 'rejected':
    case 'cancelled':
    case 'expired':
      return 2
    case 'converted':
      return 3
    default:
      return 0
  }
})

/** 是否已批准（用于"已批准"和"已转订单"步骤） */
const approved = computed(() => ['approved', 'converted'].includes(props.status as string))

/** 是否已转订单 */
const converted = computed(() => props.status === 'converted')

/** 是否显示"已转订单"步骤 */
const showConverted = computed(() => converted.value)

/** 是否显示"已取消"步骤 */
const cancelled = computed(() => props.status === 'cancelled')

/** 第三步状态：success / error / process */
const finalStatus = computed(() => {
  if (approved.value) return 'success'
  if (props.status === 'rejected') return 'error'
  if (props.status === 'cancelled') return 'error'
  if (props.status === 'expired') return 'error'
  if (props.status === 'pending_approval') return 'process'
  return 'success'
})

/** 提交审批描述 */
const pendingDescription = computed(() => {
  if (props.status === 'pending_approval') return '审批中...'
  if (['approved', 'converted'].includes(props.status as string)) return '已审批'
  if (props.status === 'rejected') return '已驳回'
  return '待提交'
})

/** 第三步描述 */
const finalDescription = computed(() => {
  if (approved.value) {
    return props.approvedAt
      ? `审批人：${props.approvedByName || ''} ${props.approvedAt}`
      : '审批通过'
  }
  if (props.status === 'rejected') {
    return props.rejectionReason ? `原因：${props.rejectionReason}` : '已拒绝'
  }
  if (props.status === 'expired') return '已过期'
  if (props.status === 'cancelled') return '已取消'
  return ''
})

/** 第四步描述 */
const convertedDescription = computed(() => {
  if (!converted.value) return ''
  const id = props.convertedOrderId ? `（订单 ID: ${props.convertedOrderId}）` : ''
  return props.convertedAt ? `${id} ${props.convertedAt}` : id
})
</script>

<style scoped>
.approval-progress {
  padding: 24px 0;
}
</style>
