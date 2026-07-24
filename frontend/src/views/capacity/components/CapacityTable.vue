<!--
  CapacityTable.vue - 工作中心列表表格（带分页 + 负荷率/状态/瓶颈）
  拆分自 capacity/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <template #header>
      <div class="card-header">
        <span>{{ $t('capacityModule.table.title') }}</span>
        <el-button type="primary" link @click="emit('refresh')">
          <el-icon><Refresh /></el-icon>
          {{ $t('capacityModule.table.refresh') }}
        </el-button>
      </div>
    </template>
    <el-table v-loading="tableLoading" :data="data" stripe style="width: 100%" :aria-label="$t('capacityModule.table.ariaLabel')">
      <el-table-column prop="code" :label="$t('capacityModule.table.code')" width="120" />
      <el-table-column prop="name" :label="$t('capacityModule.table.name')" width="150" />
      <el-table-column prop="capacity_hours" :label="$t('capacityModule.table.capacityHours')" width="120" />
      <el-table-column prop="used_hours" :label="$t('capacityModule.table.usedHours')" width="120" />
      <el-table-column prop="load_rate" :label="$t('capacityModule.table.loadRate')" width="120">
        <template #default="{ row }">
          <el-tag :type="getLoadRateType(row.load_rate)">{{
            (row.load_rate * 100).toFixed(1)
          }}%</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status" :label="$t('capacityModule.table.status')" width="100">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="bottleneck" :label="$t('capacityModule.table.bottleneck')" width="80">
        <template #default="{ row }">
          <el-tag v-if="row.bottleneck" type="danger" size="small">{{ $t('capacityModule.common.yes') }}</el-tag>
          <span v-else>-</span>
        </template>
      </el-table-column>
    </el-table>
    <el-pagination
      :current-page="page"
      :page-size="pageSize"
      :total="total"
      :page-sizes="[10, 20, 50]"
      layout="total, sizes, prev, pager, next"
      class="pagination"
      :aria-label="$t('capacityModule.table.paginationAriaLabel')"
      @update:current-page="(v: number) => emit('update:page', v)"
      @update:page-size="(v: number) => emit('update:size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Refresh } from '@element-plus/icons-vue'
import type { WorkCenter } from '@/api/capacity'
import { getStatusType, getLoadRateType } from '../composables/cpFmts'

const { t } = useI18n({ useScope: 'global' })

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getStatusLabel = (status: string) => {
  return t(`capacityModule.workCenterStatus.${status}`) || status
}

defineProps<{
  data: WorkCenter[]
  tableLoading: boolean
  total: number
  page: number
  pageSize: number
}>()

const emit = defineEmits<{
  refresh: []
  'update:page': [v: number]
  'update:size': [v: number]
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}
</style>
