<!--
  PrdDetail.vue - 生产管理订单详情
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    title="订单详情"
    width="800px"
    destroy-on-close
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <div v-if="order" class="detail-content">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="订单编号">{{ order.order_no }}</el-descriptions-item>
        <el-descriptions-item label="产品ID">{{ order.product_id }}</el-descriptions-item>
        <el-descriptions-item label="产品名称">{{
          order.product_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="工作中心">{{
          order.work_center_id || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="计划数量">{{ order.planned_quantity }}</el-descriptions-item>
        <el-descriptions-item label="实际数量">{{
          order.actual_quantity || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="计划开始">{{
          order.scheduled_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="计划结束">{{
          order.scheduled_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际开始">{{
          order.actual_start_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际结束">{{
          order.actual_end_date?.substring(0, 10) || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="(statusTagType as any)">{{ getStatusLabelFmt(order.status) }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="优先级">{{ order.priority }}</el-descriptions-item>
        <el-descriptions-item label="创建时间" :span="2">
          {{ order.created_at || '-' }}
        </el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{ order.remark || '-' }}</el-descriptions-item>
      </el-descriptions>
    </div>
    <template #footer>
      <el-button @click="emit('update:visible', false)">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { computed } from 'vue'
import type { ProductionOrder } from '@/api/production'
import { getStatusLabel } from '../composables/prdFmts'

/**
 * 生产管理订单详情组件
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 订单数据
  order: ProductionOrder | null
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
}>()

// 透传格式化函数
const getStatusLabelFmt = getStatusLabel

// 状态对应的 el-tag type
const statusTagType = computed(() => {
  const map: Record<string, string> = {
    draft: 'info',
    planned: 'primary',
    in_progress: 'warning',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[props.order?.status || ''] || 'info'
})
</script>
