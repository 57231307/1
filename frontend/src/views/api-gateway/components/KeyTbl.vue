<!--
  KeyTbl.vue - API 密钥列表
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-table v-loading="loading" :data="keys" stripe>
    <el-table-column prop="key_name" label="密钥名称" width="150" />
    <el-table-column prop="api_key" label="API Key" min-width="200">
      <template #default="{ row }">
        <div class="api-key-cell">
          <span>{{ showKeyMap[row.id] ? row.api_key : maskApiKey(row.api_key) }}</span>
          <el-button type="primary" link size="small" @click="emit('toggle-show', row.id)">
            {{ showKeyMap[row.id] ? '隐藏' : '显示' }}
          </el-button>
        </div>
      </template>
    </el-table-column>
    <el-table-column prop="description" label="描述" min-width="150" />
    <el-table-column prop="rate_limit" label="限流" width="100">
      <template #default="{ row }">
        {{ row.rate_limit ? `${row.rate_limit}/s` : '-' }}
      </template>
    </el-table-column>
    <el-table-column prop="expires_at" label="过期时间" width="160" />
    <el-table-column prop="status" label="状态" width="80" align="center">
      <template #default="{ row }">
        <el-tag :type="KEY_STATUS_TYPE_MAP[row.status]" size="small">
          {{ KEY_STATUS_LABEL_MAP[row.status] }}
        </el-tag>
      </template>
    </el-table-column>
    <el-table-column prop="last_used_at" label="最后使用" width="160" />
    <el-table-column label="操作" width="250" fixed="right">
      <template #default="{ row }">
        <el-button type="primary" link size="small" @click="emit('edit', row)">编辑</el-button>
        <el-button type="warning" link size="small" @click="emit('regenerate', row)"
          >重新生成</el-button
        >
        <el-button type="danger" link size="small" @click="emit('delete', row)">删除</el-button>
      </template>
    </el-table-column>
  </el-table>
  <div class="pagination-container">
    <el-pagination
      :current-page="query.page"
      :page-size="query.page_size"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      layout="total, sizes, prev, pager, next, jumper"
      @size-change="emit('page-change')"
      @current-change="emit('page-change')"
    />
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import type { ApiKey } from '@/api/api-gateway'
import { KEY_STATUS_LABEL_MAP, KEY_STATUS_TYPE_MAP, maskApiKey } from '../composables/apiGwFmts'

interface KeyQuery {
  page: number
  page_size: number
  keyword: string
  status: string
}

const props = defineProps<{
  keys: ApiKey[]
  loading: boolean
  total: number
  query: KeyQuery
  showKeyMap: Record<number, boolean>
}>()

const emit = defineEmits<{
  edit: [row: ApiKey]
  regenerate: [row: ApiKey]
  delete: [row: ApiKey]
  'toggle-show': [id: number]
  'page-change': []
}>()

void props
</script>

<style scoped>
.api-key-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
</style>
