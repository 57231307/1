<!--
  LogFilter.vue - API 调用日志过滤栏
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <div class="filter-container">
    <el-input
      :model-value="query.keyword"
      placeholder="搜索接口路径"
      style="width: 200px"
      clearable
      @update:model-value="(v: string) => (query.keyword = v ?? '')"
      @clear="emit('search')"
      @keyup.enter="emit('search')"
    />
    <el-select
      :model-value="query.method"
      placeholder="请求方法"
      clearable
      style="width: 120px"
      @update:model-value="(v: string) => (query.method = v ?? '')"
    >
      <el-option label="GET" value="GET" />
      <el-option label="POST" value="POST" />
      <el-option label="PUT" value="PUT" />
      <el-option label="DELETE" value="DELETE" />
    </el-select>
    <el-input
      :model-value="query.status_code"
      placeholder="状态码"
      style="width: 100px"
      clearable
      @update:model-value="(v: string) => (query.status_code = v ?? '')"
      @clear="emit('search')"
      @keyup.enter="emit('search')"
    />
    <el-button type="primary" @click="emit('search')">
      <el-icon><Search /></el-icon>
      搜索
    </el-button>
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Search } from '@element-plus/icons-vue'

interface LogQuery {
  page: number
  page_size: number
  keyword: string
  method: string
  status_code: string
}

const props = defineProps<{
  query: LogQuery
}>()

const emit = defineEmits<{
  search: []
}>()

void props
</script>
