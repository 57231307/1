<!--
  AuditTab.vue - 审计日志 Tab
  来源：原 system/index.vue 中 审计日志 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="audit-tab">
    <div class="page-header">
      <h2 class="page-title">审计日志</h2>
    </div>
    <el-card shadow="hover">
      <el-form :inline="true" :model="auditQuery" class="mb-4">
        <el-form-item label="操作人">
          <el-input v-model="auditQuery.operator" placeholder="用户名" clearable />
        </el-form-item>
        <el-form-item label="模块">
          <el-input v-model="auditQuery.module" placeholder="模块名" clearable />
        </el-form-item>
        <el-form-item label="时间范围">
          <el-date-picker
            v-model="auditQuery.dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchAuditLogs">查询</el-button>
        </el-form-item>
      </el-form>
      <el-table v-loading="auditLoading" :data="auditLogs" stripe>
        <el-table-column prop="created_at" label="时间" width="180" />
        <el-table-column prop="operator_name" label="操作人" width="120" />
        <el-table-column prop="module" label="模块" width="120" />
        <el-table-column prop="action" label="操作" width="100" />
        <el-table-column prop="resource_type" label="资源" width="120" />
        <el-table-column prop="ip_address" label="IP" width="130" />
        <el-table-column prop="detail" label="详情" min-width="200" show-overflow-tooltip />
      </el-table>
      <el-pagination
        v-model:current-page="auditQuery.page"
        v-model:page-size="auditQuery.page_size"
        :total="auditTotal"
        :page-sizes="[20, 50, 100]"
        layout="total, sizes, prev, pager, next"
        class="mt-4"
        @current-change="fetchAuditLogs"
        @size-change="fetchAuditLogs"
      />
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { request } from '@/api/request'

interface AuditLog {
  created_at: string
  operator_name: string
  module: string
  action: string
  resource_type: string
  ip_address: string
  detail: string
}

const auditLogs = ref<AuditLog[]>([])
const auditTotal = ref(0)
const auditLoading = ref(false)
const auditQuery = reactive<{
  operator: string
  module: string
  dateRange: [string, string] | null
  page: number
  page_size: number
}>({
  operator: '',
  module: '',
  dateRange: null,
  page: 1,
  page_size: 20,
})

const fetchAuditLogs = async () => {
  auditLoading.value = true
  try {
    const params: Record<string, string | number> = {
      page: auditQuery.page,
      page_size: auditQuery.page_size,
    }
    if (auditQuery.operator) params.operator = auditQuery.operator
    if (auditQuery.module) params.module = auditQuery.module
    if (auditQuery.dateRange && auditQuery.dateRange.length === 2) {
      params.start_date = auditQuery.dateRange[0]
      params.end_date = auditQuery.dateRange[1]
    }
    const res = await request.get<{ items?: AuditLog[]; total?: number } | AuditLog[]>(
      '/audit/logs',
      { params }
    )
    const d = res
    if (d && typeof d === 'object' && 'items' in d) {
      auditLogs.value = d.items || []
      auditTotal.value = d.total || 0
    } else {
      auditLogs.value = (d as AuditLog[]) || []
      auditTotal.value = auditLogs.value.length
    }
  } catch (_e) {
    // 静默：审计日志查询失败不向用户弹出错误
  } finally {
    auditLoading.value = false
  }
}

defineExpose({ refresh: fetchAuditLogs })

onMounted(() => {
  fetchAuditLogs()
})
</script>
