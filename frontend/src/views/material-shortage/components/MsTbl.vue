<!--
  MsTbl.vue - 物料短缺列表（含过滤栏、操作按钮）
  拆分自 material-shortage/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <div class="filter-bar">
      <el-select
        :model-value="filterSeverity"
        placeholder="缺料严重程度"
        clearable
        style="width: 160px"
        @update:model-value="(v: string) => emit('update:filter-severity', v)"
      >
        <el-option label="严重" value="critical" />
        <el-option label="高" value="high" />
        <el-option label="中" value="medium" />
        <el-option label="低" value="low" />
      </el-select>
      <el-select
        :model-value="filterStatus"
        placeholder="状态"
        clearable
        style="width: 140px"
        @update:model-value="(v: string) => emit('update:filter-status', v)"
      >
        <el-option label="待处理" value="pending" />
        <el-option label="已通知" value="notified" />
        <el-option label="已解决" value="resolved" />
      </el-select>
      <el-button type="primary" @click="emit('filter-change')">
        <el-icon><Search /></el-icon>
        查询
      </el-button>
      <el-button type="success" :loading="checking" @click="emit('check')">
        <el-icon><Refresh /></el-icon>
        触发检查
      </el-button>
    </div>

    <el-table v-loading="loading" :data="data" stripe>
      <el-table-column prop="material_code" label="物料编号" min-width="140" />
      <el-table-column prop="material_name" label="物料名称" min-width="160" />
      <el-table-column prop="shortage_quantity" label="缺料数量" width="100" align="right" />
      <el-table-column prop="required_quantity" label="需求数量" width="100" align="right" />
      <el-table-column prop="available_quantity" label="可用库存" width="100" align="right" />
      <el-table-column prop="severity" label="严重程度" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getSeverityColor(row.severity)">
            {{ getSeverityLabel(row.severity) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusColor(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="source_type" label="来源类型" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getSourceTypeColor(row.source_type)">
            {{ getSourceTypeLabel(row.source_type) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="source_no" label="来源单号" min-width="140" />
      <el-table-column prop="expected_arrival_date" label="预计到货" min-width="120" />
      <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
      <el-table-column label="操作" width="180" fixed="right">
        <template #default="{ row }">
          <el-button
            v-if="row.status === 'pending'"
            type="primary"
            link
            size="small"
            @click="emit('notify', row)"
          >
            发送通知
          </el-button>
          <el-button
            v-if="row.status !== 'resolved'"
            type="success"
            link
            size="small"
            @click="emit('resolve', row)"
          >
            标记解决
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="currentPage"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { Search, Refresh } from '@element-plus/icons-vue'
import {
  getSeverityColor,
  getSeverityLabel,
  getStatusColor,
  getStatusLabel,
  getSourceTypeColor,
  getSourceTypeLabel,
} from '../composables/msFmts'
import type { MaterialShortage } from '@/api/material-shortage'

/**
 * 列表组件（含过滤栏 + 操作）
 */
defineProps<{
  // 列表数据
  data: MaterialShortage[]
  // 总数
  total: number
  // 加载状态
  loading: boolean
  // 检查中
  checking: boolean
  // 分页
  currentPage: number
  pageSize: number
  // 过滤
  filterSeverity: string
  filterStatus: string
}>()

const emit = defineEmits<{
  // 过滤变化
  'filter-change': []
  // 触发检查
  check: []
  // 通知
  notify: [row: MaterialShortage]
  // 解决
  resolve: [row: MaterialShortage]
  // 分页
  'update:page': [v: number]
  'update:size': [v: number]
  // 过滤值变化
  'update:filter-severity': [v: string]
  'update:filter-status': [v: string]
}>()
</script>

<style scoped>
.filter-bar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
  align-items: center;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
