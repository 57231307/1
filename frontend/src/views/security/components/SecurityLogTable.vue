<!--
  SecurityLogTable.vue - 登录日志表（含过滤栏 + 分页）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
  批次 282：接入 useTableApi 模式（page/pageSize props + handleSearch 同步筛选条件）
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>登录日志</span>
        <el-form :inline="true" :model="localQuery" class="filter-form" aria-label="登录日志筛选表单">
          <el-form-item label="用户名">
            <el-input
              v-model="localQuery.username"
              placeholder="请输入用户名"
              clearable
              @clear="handleSearch"
            />
          </el-form-item>
          <el-form-item label="登录状态">
            <el-select
              v-model="localQuery.status"
              placeholder="选择状态"
              clearable
              @change="handleSearch"
            >
              <el-option label="成功" value="SUCCESS" />
              <el-option label="失败" value="FAILED" />
            </el-select>
          </el-form-item>
          <el-form-item label="登录时间">
            <el-date-picker
              v-model="localQuery.date_range"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
              @change="handleSearch"
            />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleSearch">
              <el-icon><Search /></el-icon>
              查询
            </el-button>
          </el-form-item>
        </el-form>
      </div>
    </template>

    <el-table v-loading="loading" :data="data" border stripe aria-label="登录日志表">
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
        :current-page="page"
        :page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        aria-label="登录日志分页"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { LoginLog } from '@/api/security'
import { getTypeLabel, getStatusType, getStatusLabel } from '../composables/secFmts'

// 批次 282：queryParams 类型放宽为 Record<string, unknown>（兼容 useTableApi）
const props = defineProps<{
  data: LoginLog[]
  loading: boolean
  total: number
  page: number
  pageSize: number
  queryParams: Record<string, unknown>
}>()

const emit = defineEmits<{
  fetch: []
  'update:page': [value: number]
  'update:page-size': [value: number]
  'update:queryParams': [value: Record<string, unknown>]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  username: string
  status: string
  date_range: string[]
}>({
  username: (props.queryParams.username as string) ?? '',
  status: (props.queryParams.status as string) ?? '',
  date_range: (props.queryParams.date_range as string[]) ?? [],
})

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}
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
