<!--
  ArReconciliationConfirm.vue - AR 对账客户确认记录对话框
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="$t('arReconciliationModule.confirmationRecords')"
    width="900px"
    :aria-label="$t('arReconciliationModule.confirmationDialogAria')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-table :data="data" border style="width: 100%" :aria-label="$t('arReconciliationModule.confirmationListAria')">
      <el-table-column prop="customer_name" :label="$t('arReconciliationModule.customerName')" width="160" />
      <el-table-column :label="$t('arReconciliationModule.confirmStatus')" width="100">
        <template #default="scope">
          <el-tag size="small" :type="getConfirmType(scope.row.confirm_status)">
            {{ $t(getConfirmLabel(scope.row.confirm_status)) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="confirm_amount" :label="$t('arReconciliationModule.confirmAmount')" width="120" align="right">
        <template #default="scope">{{ scope.row.confirm_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="disputed_amount" :label="$t('arReconciliationModule.disputeAmount')" width="120" align="right">
        <template #default="scope">{{ scope.row.disputed_amount.toFixed(2) }}</template>
      </el-table-column>
      <el-table-column prop="confirmed_at" :label="$t('arReconciliationModule.confirmTime')" width="160" />
      <el-table-column prop="remark" :label="$t('arReconciliationModule.remark')" />
      <el-table-column :label="$t('common.operation')" width="180" align="center">
        <template #default="scope">
          <el-button
            v-if="scope.row.confirm_status === 'pending'"
            size="small"
            type="success"
            @click="emit('confirm-status', scope.row, 'confirmed')"
          >
            <el-icon><CircleCheck /></el-icon> {{ $t('common.confirm') }}
          </el-button>
          <el-button
            v-if="scope.row.confirm_status === 'pending'"
            size="small"
            type="danger"
            @click="emit('confirm-status', scope.row, 'disputed')"
          >
            <el-icon><CircleClose /></el-icon> {{ $t('arReconciliationModule.dispute') }}
          </el-button>
        </template>
      </el-table-column>
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { CircleCheck, CircleClose } from '@element-plus/icons-vue'
import type { CustomerConfirmation } from '@/api/ar-reconciliation-enhanced'
import { getConfirmLabel, getConfirmType } from '../composables/arRecFmts'

const { t } = useI18n({ useScope: 'global' })
void t

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
