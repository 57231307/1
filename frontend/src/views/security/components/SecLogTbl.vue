<!--
  SecLogTbl.vue - 登录日志表（含过滤栏 + 分页）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>登录日志</span>
        <el-form :inline="true" :model="queryParams" class="filter-form">
          <el-form-item label="用户名">
            <el-input
              :model-value="queryParams.username"
              placeholder="请输入用户名"
              clearable
              @update:model-value="(v: string) => (queryParams.username = v)"
              @clear="emit('query')"
            />
          </el-form-item>
          <el-form-item label="登录状态">
            <el-select
              :model-value="queryParams.status"
              placeholder="选择状态"
              clearable
              @update:model-value="(v: string) => (queryParams.status = v)"
              @change="emit('query')"
            >
              <el-option label="成功" value="SUCCESS" />
              <el-option label="失败" value="FAILED" />
            </el-select>
          </el-form-item>
          <el-form-item label="登录时间">
            <el-date-picker
              :model-value="queryParams.date_range"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              @update:model-value="(v: string[]) => (queryParams.date_range = v)"
              @change="emit('query')"
            />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="emit('query')">
              <el-icon><Search /></el-icon>
              查询
            </el-button>
          </el-form-item>
        </el-form>
      </div>
    </template>

    <el-table v-loading="loading" :data="data" border stripe>
      <el-table-column type="index" label="序号" width="60" align="center" />
      <el-table-column prop="username" label="用户名" width="120" show-overflow-tooltip />
      <el-table-column prop="login_type" label="登录类型" width="100" align="center">
        <template #default="{ row }">
          <el-tag>{{ getTypeLabel(row.login_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="ip_address" label="IP地址" width="150" show-overflow-tooltip />
      <el-table-column prop="user_agent" label="浏览器" min-width="200" show-overflow-tooltip />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="fail_reason" label="失败原因" width="150" show-overflow-tooltip />
      <el-table-column prop="login_time" label="登录时间" width="180" align="center" />
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="queryParams.page"
        :page-size="queryParams.page_size"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        @update:current-page="(v: number) => emit('size-or-current', v, 'current')"
        @update:page-size="(v: number) => emit('size-or-current', v, 'size')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { Search } from '@element-plus/icons-vue'
import type { LoginLog, SecurityQueryParams } from '@/api/security'
import { getTypeLabel, getStatusType, getStatusLabel } from '../composables/secFmts'

defineProps<{
  data: LoginLog[]
  loading: boolean
  total: number
  queryParams: SecurityQueryParams
}>()

const emit = defineEmits<{
  query: []
  'size-or-current': [val: number, type: 'size' | 'current']
}>()
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
