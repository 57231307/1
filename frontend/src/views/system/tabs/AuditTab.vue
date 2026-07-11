<!--
  AuditTab.vue - 审计日志 Tab
  来源：原 system/index.vue 中 审计日志 tab 内容
  拆分日期：2026-06-15 B3-1
  批次 281：接入 useTableApi，移除手写分页 + 直接 request 调用
-->
<template>
  <div class="audit-tab">
    <div class="page-header">
      <h2 class="page-title">审计日志</h2>
    </div>
    <el-card shadow="hover">
      <el-form :inline="true" :model="filterForm" class="mb-4">
        <el-form-item label="操作人">
          <el-input v-model="filterForm.operator" placeholder="用户名" clearable />
        </el-form-item>
        <el-form-item label="模块">
          <el-input v-model="filterForm.module" placeholder="模块名" clearable />
        </el-form-item>
        <el-form-item label="时间范围">
          <el-date-picker
            v-model="filterForm.dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
        </el-form-item>
      </el-form>
      <el-table v-loading="loading" :data="auditLogs" stripe>
        <el-table-column prop="created_at" label="时间" width="180" />
        <el-table-column prop="operator_name" label="操作人" width="120" />
        <el-table-column prop="module" label="模块" width="120" />
        <el-table-column prop="action" label="操作" width="100" />
        <el-table-column prop="resource_type" label="资源" width="120" />
        <el-table-column prop="ip_address" label="IP" width="130" />
        <el-table-column prop="detail" label="详情" min-width="200" show-overflow-tooltip />
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        class="mt-4"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useTableApi } from '@/composables/useTableApi'

interface AuditLog {
  created_at: string
  operator_name: string
  module: string
  action: string
  resource_type: string
  ip_address: string
  detail: string
}

// 批次 281：filterForm 仅保留筛选字段，分页字段由 useTableApi 管理
const filterForm = reactive({
  operator: '',
  module: '',
  dateRange: null as [string, string] | null,
})

const {
  data: auditLogs,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchAuditLogs,
  setQueryParam,
} = useTableApi<AuditLog>({
  url: '/audit/logs',
  defaultPageSize: 20,
  // 静默：审计日志查询失败不向用户弹出错误（保持原行为）
  onError: () => {},
})

const syncQueryParams = () => {
  setQueryParam('operator', filterForm.operator || undefined)
  setQueryParam('module', filterForm.module || undefined)
  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    setQueryParam('start_date', filterForm.dateRange[0])
    setQueryParam('end_date', filterForm.dateRange[1])
  } else {
    setQueryParam('start_date', undefined)
    setQueryParam('end_date', undefined)
  }
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  // 静默处理：审计日志查询失败不向用户弹出错误
  fetchAuditLogs().catch(() => {})
}

defineExpose({ refresh: fetchAuditLogs })
</script>
