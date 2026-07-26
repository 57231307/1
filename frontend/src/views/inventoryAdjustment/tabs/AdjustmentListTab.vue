<!--
  AdjustmentListTab.vue - 库存调整列表 Tab
  来源：原 inventoryAdjustment/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="adjustment-list">
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryAdjustment.listTab.statLabelTotal') }}</div>
              <div class="stat-value">{{ stats.total }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryAdjustment.listTab.statLabelPending') }}</div>
              <div class="stat-value">{{ stats.pending }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card success">
          <div class="stat-content">
            <div class="stat-icon approved-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ t('inventoryAdjustment.listTab.statLabelApproved') }}</div>
              <div class="stat-value">{{ stats.approved }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">
                {{ t('inventoryAdjustment.listTab.statLabelTotalAmount') }}
              </div>
              <div class="stat-value">{{ formatCurrency(stats.totalAmount) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="queryParams"
        class="filter-form"
        :aria-label="t('inventoryAdjustment.listTab.ariaLabelFilter')"
      >
        <el-form-item :label="t('inventoryAdjustment.listTab.labelAdjustNo')">
          <el-input
            v-model="queryParams.adjust_no"
            :placeholder="t('inventoryAdjustment.listTab.placeholderAdjustNo')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('inventoryAdjustment.listTab.labelStatus')">
          <el-select
            v-model="queryParams.status"
            :placeholder="t('inventoryAdjustment.listTab.placeholderStatus')"
            clearable
          >
            <el-option :label="t('inventoryAdjustment.listTab.statusPending')" value="pending" />
            <el-option :label="t('inventoryAdjustment.listTab.statusApproved')" value="approved" />
            <el-option :label="t('inventoryAdjustment.listTab.statusRejected')" value="rejected" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">{{
            t('inventoryAdjustment.listTab.buttonSearch')
          }}</el-button>
          <el-button @click="handleReset">{{
            t('inventoryAdjustment.listTab.buttonReset')
          }}</el-button>
          <el-button
            v-permission="'inventory:create'"
            type="primary"
            @click="emit('openForm', 'create', null)"
          >
            <el-icon><Plus /></el-icon>{{ t('inventoryAdjustment.listTab.buttonCreate') }}
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table
        v-loading="loading"
        :data="adjustments"
        stripe
        :aria-label="t('inventoryAdjustment.listTab.ariaLabelTable')"
      >
        <el-table-column
          prop="adjust_no"
          :label="t('inventoryAdjustment.listTab.colAdjustNo')"
          width="160"
          fixed
        />
        <el-table-column
          prop="adjust_date"
          :label="t('inventoryAdjustment.listTab.colAdjustDate')"
          width="120"
        />
        <el-table-column
          prop="warehouse_name"
          :label="t('inventoryAdjustment.listTab.colWarehouse')"
          width="120"
        />
        <el-table-column
          prop="reason"
          :label="t('inventoryAdjustment.listTab.colReason')"
          min-width="200"
          show-overflow-tooltip
        />
        <el-table-column
          prop="total_amount"
          :label="t('inventoryAdjustment.listTab.colAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">{{ formatCurrency(row.total_amount) }}</template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('inventoryAdjustment.listTab.colStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="created_by_name"
          :label="t('inventoryAdjustment.listTab.colCreatedBy')"
          width="100"
        />
        <el-table-column
          prop="created_at"
          :label="t('inventoryAdjustment.listTab.colCreatedAt')"
          width="160"
        />
        <el-table-column
          :label="t('inventoryAdjustment.listTab.colAction')"
          width="200"
          fixed="right"
        >
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openForm', 'view', row)">{{
              t('inventoryAdjustment.listTab.buttonDetail')
            }}</el-button>
            <el-button
              v-if="row.status === 'pending'"
              type="primary"
              link
              size="small"
              @click="emit('openForm', 'edit', row)"
              >{{ t('inventoryAdjustment.listTab.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="emit('openApprove', row)"
              >{{ t('inventoryAdjustment.listTab.buttonApprove') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          :aria-label="t('inventoryAdjustment.listTab.ariaLabelPagination')"
        />
      </div>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import { Document, Clock, CircleCheck, Money, Plus } from '@element-plus/icons-vue'
import { type InventoryAdjustmentEntity } from '@/api/inventoryAdjustment'
import { useTableApi } from '@/composables/useTableApi'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: InventoryAdjustmentEntity | null]
  openApprove: [row: InventoryAdjustmentEntity]
}>()

const {
  data: adjustments,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  refresh: fetchAdjustments,
} = useTableApi<InventoryAdjustmentEntity>({
  url: '/inventory/adjustments',
  defaultPageSize: 20,
  defaultParams: {
    adjust_no: '',
    status: '',
  },
  onError: (err: unknown) => {
    logger.error('获取库存调整单失败', err)
    ElMessage.error(t('inventoryAdjustment.listTab.messageFetchFailed'))
  },
})

const stats = reactive({
  total: 0,
  pending: 0,
  approved: 0,
  totalAmount: 0,
})

watch(
  adjustments,
  newData => {
    stats.total = total.value
    stats.pending = newData.filter(a => a.status === 'pending').length
    stats.approved = newData.filter(a => a.status === 'approved').length
    stats.totalAmount = newData.reduce((sum, a) => sum + (a.total_amount || 0), 0)
  },
  { immediate: true }
)

/** 状态标签 i18n 映射 */
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: t('inventoryAdjustment.listTab.statusPending'),
    approved: t('inventoryAdjustment.listTab.statusApproved'),
    rejected: t('inventoryAdjustment.listTab.statusRejected'),
  }
  return map[status] || status
}

/** 状态 el-tag 类型映射 */
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}

const formatCurrency = (amount: number) => `¥${(amount || 0).toFixed(2)}`

const handleQuery = () => {
  page.value = 1
  fetchAdjustments()
}
const handleReset = () => {
  queryParams.value = {
    adjust_no: '',
    status: '',
  }
  handleQuery()
}

defineExpose({ fetchAdjustments })
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.success {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-card.success :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value),
.stat-card.success :deep(.stat-label),
.stat-card.success :deep(.stat-value),
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value) {
  color: white;
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-icon.pending-icon),
:deep(.stat-icon.approved-icon),
:deep(.stat-icon.amount-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
