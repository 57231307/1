<!--
  SecLogTbl.vue - 登录日志表（含过滤栏 + 分页）
  拆分自 security/index.vue（P14 批 2 I-3 第 6 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-card shadow="hover" class="table-card">
    <template #header>
      <div class="card-header">
        <span>登录日志</span>
        <el-form :inline="true" :model="localQueryParams" class="filter-form">
          <el-form-item label="用户名">
            <el-input
              v-model="localQueryParams.username"
              placeholder="请输入用户名"
              clearable
              @clear="emit('query')"
            />
          </el-form-item>
          <el-form-item label="登录状态">
            <el-select
              v-model="localQueryParams.status"
              placeholder="选择状态"
              clearable
              @change="emit('query')"
            >
              <el-option label="成功" value="SUCCESS" />
              <el-option label="失败" value="FAILED" />
            </el-select>
          </el-form-item>
          <el-form-item label="登录时间">
            <el-date-picker
              v-model="localQueryParams.date_range"
              type="daterange"
              range-separator="至"
              start-placeholder="开始日期"
              end-placeholder="结束日期"
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
        :current-page="localQueryParams.page"
        :page-size="localQueryParams.page_size"
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
import { ref, watch, nextTick } from 'vue'
import { Search } from '@element-plus/icons-vue'
import type { LoginLog, SecurityQueryParams } from '@/api/security'
import { getTypeLabel, getStatusType, getStatusLabel } from '../composables/secFmts'

const props = defineProps<{
  data: LoginLog[]
  loading: boolean
  total: number
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: SecurityQueryParams
}>()

const emit = defineEmits<{
  query: []
  'size-or-current': [val: number, type: 'size' | 'current']
  // 整体回写查询参数
  'update:queryParams': [queryParams: SecurityQueryParams]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<SecurityQueryParams>({ ...props.queryParams })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.queryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    localQueryParams.value = { ...newParams }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件
watch(
  localQueryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    emit('update:queryParams', { ...newParams })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
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
