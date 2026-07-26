<!--
  SecurityLogTable.vue - 登录日志表（含过滤栏 + 分页）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
  批次 282：接入 useTableApi 模式（page/pageSize props + handleSearch 同步筛选条件）
  批次 D05 B4：接入 useI18n
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>{{ t('security.logTable.title') }}</span>
        <el-form :inline="true" :model="localQuery" class="filter-form" :aria-label="t('security.logTable.ariaLabel.filterForm')">
          <el-form-item :label="t('security.logTable.filter.username')">
            <el-input
              v-model="localQuery.username"
              :placeholder="t('security.logTable.placeholder.username')"
              clearable
              @clear="handleSearch"
            />
          </el-form-item>
          <el-form-item :label="t('security.logTable.filter.status')">
            <el-select
              v-model="localQuery.status"
              :placeholder="t('security.logTable.placeholder.status')"
              clearable
              @change="handleSearch"
            >
              <el-option :label="t('security.logTable.status.SUCCESS')" value="SUCCESS" />
              <el-option :label="t('security.logTable.status.FAILED')" value="FAILED" />
            </el-select>
          </el-form-item>
          <el-form-item :label="t('security.logTable.filter.dateRange')">
            <el-date-picker
              v-model="localQuery.date_range"
              type="daterange"
              :range-separator="t('security.logTable.placeholder.rangeSeparator')"
              :start-placeholder="t('security.logTable.placeholder.startDate')"
              :end-placeholder="t('security.logTable.placeholder.endDate')"
              @change="handleSearch"
            />
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="handleSearch">
              <el-icon><Search /></el-icon>
              {{ t('security.logTable.button.search') }}
            </el-button>
          </el-form-item>
        </el-form>
      </div>
    </template>

    <el-table v-loading="loading" :data="data" border stripe :aria-label="t('security.logTable.ariaLabel.table')">
      <el-table-column type="index" :label="t('security.logTable.column.index')" width="60" align="center" />
      <el-table-column prop="username" :label="t('security.logTable.column.username')" width="120" show-overflow-tooltip />
      <el-table-column prop="login_type" :label="t('security.logTable.column.loginType')" width="100" align="center">
        <template #default="{ row }">
          <el-tag>{{ getTypeLabel(row.login_type) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="ip_address" :label="t('security.logTable.column.ipAddress')" width="150" show-overflow-tooltip />
      <el-table-column prop="user_agent" :label="t('security.logTable.column.userAgent')" min-width="200" show-overflow-tooltip />
      <el-table-column prop="status" :label="t('security.logTable.column.status')" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="fail_reason" :label="t('security.logTable.column.failReason')" width="150" show-overflow-tooltip />
      <el-table-column prop="login_time" :label="t('security.logTable.column.loginTime')" width="180" align="center" />
    </el-table>

    <div class="pagination-container">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('security.logTable.ariaLabel.pagination')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { Search } from '@element-plus/icons-vue'
import type { LoginLog } from '@/api/security'
import { getStatusType } from '../composables/secFmts'

const { t } = useI18n({ useScope: 'global' })

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

// 登录类型/状态码 → 本地化标签（动态 t() 调用确保语言切换响应）
const getTypeLabel = (type: string) => t(`security.logTable.loginType.${type}`)
const getStatusLabel = (status: string) => t(`security.logTable.status.${status}`)

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
