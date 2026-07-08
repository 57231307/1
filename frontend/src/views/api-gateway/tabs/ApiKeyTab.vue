<!--
  ApiKeyTab.vue - API 网关密钥管理 Tab
  来源：原 api-gateway/index.vue 中 keys tab
  拆分日期：2026-06-17 P1-3-Batch-5
-->
<template>
  <el-card shadow="hover">
    <div class="filter-container">
      <el-input
        v-model="localQuery.keyword"
        placeholder="搜索密钥名称"
        style="width: 200px"
        clearable
        @clear="emit('fetch')"
        @keyup.enter="emit('fetch')"
      />
      <el-select v-model="localQuery.status" placeholder="状态" clearable style="width: 120px">
        <el-option label="启用" value="active" />
        <el-option label="停用" value="inactive" />
      </el-select>
      <el-button type="primary" @click="emit('fetch')">
        <el-icon><Search /></el-icon>
        搜索
      </el-button>
      <el-button type="primary" @click="emit('new-key')">
        <el-icon><Plus /></el-icon>
        创建密钥
      </el-button>
    </div>

    <el-table v-loading="loading" :data="apiKeys" stripe>
      <el-table-column prop="key_name" label="密钥名称" width="200" />
      <el-table-column prop="app_id" label="应用 ID" width="200" />
      <el-table-column label="密钥" min-width="200">
        <template #default="{ row }">
          <span class="key-text">{{ maskKey(row.api_key) }}</span>
          <el-button
            type="primary"
            link
            size="small"
            @click="emit('view-key', row)"
            style="margin-left: 8px"
            >查看</el-button
          >
        </template>
      </el-table-column>
      <el-table-column prop="expires_at" label="过期时间" width="160" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
            {{ row.status === 'active' ? '启用' : '停用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="last_used_at" label="最后使用" width="160" />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button v-permission="'api_key:update'" type="warning" link size="small" @click="emit('toggle-key', row)">
            {{ row.status === 'active' ? '停用' : '启用' }}
          </el-button>
          <el-button v-permission="'api_key:delete'" type="danger" link size="small" @click="emit('delete-key', row)"
            >删除</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-container">
      <el-pagination
        v-model:current-page="localQuery.page"
        v-model:page-size="localQuery.page_size"
        :page-sizes="[10, 20, 50]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('fetch')"
        @current-change="emit('fetch')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { Search, Plus } from '@element-plus/icons-vue'
import type { ApiKey } from '@/api/api-gateway'

export interface ApiKeyQuery {
  page: number
  page_size: number
  keyword: string
  status: string
}

const props = defineProps<{
  apiKeys: ApiKey[]
  loading: boolean
  total: number
  queryParams: ApiKeyQuery
}>()

const emit = defineEmits<{
  fetch: []
  'new-key': []
  'view-key': [row: ApiKey]
  'toggle-key': [row: ApiKey]
  'delete-key': [row: ApiKey]
  'update:queryParams': [value: ApiKeyQuery]
}>()

const localQuery = reactive<ApiKeyQuery>({ ...props.queryParams })

watch(
  () => props.queryParams,
  newQuery => Object.assign(localQuery, newQuery),
  { deep: true }
)

const maskKey = (key: string) => {
  if (!key) return ''
  if (key.length <= 8) return '*'.repeat(key.length)
  return key.substring(0, 4) + '*'.repeat(key.length - 8) + key.substring(key.length - 4)
}
</script>

<style scoped>
.filter-container {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 16px;
}

.key-text {
  font-family: monospace;
  color: #909399;
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
