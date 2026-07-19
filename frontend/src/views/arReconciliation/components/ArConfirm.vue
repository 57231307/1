<!--
  ArConfirm.vue - AR 对账客户确认记录对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="客户确认记录"
    width="900px"
    aria-label="客户确认记录对话框"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-table :data="data" border style="width: 100%" aria-label="客户确认记录列表">
      <el-table-column prop="customer_name" label="客户名称" width="160" />
      <el-table-column label="确认状态" width="100">
        <template #default="scope">
          <el-tag size="small" :type="getConfirmType(scope.row.confirm_status)">
            {{ getConfirmLabel(scope.row.confirm_status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="confirm_amount" label="确认金额" width="120" align="right">
        <template #default="scope">{{ scope.row.confirm_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="disputed_amount" label="争议金额" width="120" align="right">
        <template #default="scope">{{ scope.row.disputed_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="confirmed_at" label="确认时间" width="160" />
      <el-table-column prop="remark" label="备注" />
      <el-table-column label="操作" width="180" align="center">
        <template #default="scope">
          <el-button
            v-if="scope.row.confirm_status === 'pending'"
            size="small"
            type="success"
            @click="emit('confirm-status', scope.row, 'confirmed')"
          >
            <el-icon><CircleCheck /></el-icon> 确认
          </el-button>
          <el-button
            v-if="scope.row.confirm_status === 'pending'"
            size="small"
            type="danger"
            @click="emit('confirm-status', scope.row, 'disputed')"
          >
            <el-icon><CircleClose /></el-icon> 争议
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { CircleCheck, CircleClose } from '@element-plus/icons-vue'
import type { CustomerConfirmation } from '@/api/ar-reconciliation-enhanced'
import { getConfirmLabel, getConfirmType } from '../composables/arRecFmts'

/**
 * 客户确认记录对话框组件
 */
const props = defineProps<{
  visible: boolean
  data: CustomerConfirmation[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  'confirm-status': [row: CustomerConfirmation, status: 'confirmed' | 'disputed']
}>()

void props
</script>
