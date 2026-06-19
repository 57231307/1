<!--
  EpFilter.vue - 接口管理过滤栏
  拆分自 api-gateway/index.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <div class="filter-container">
    <el-input
      :model-value="query.keyword"
      placeholder="搜索接口路径/描述"
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
      <el-option label="PATCH" value="PATCH" />
    </el-select>
    <el-select
      :model-value="query.status"
      placeholder="状态"
      clearable
      style="width: 120px"
      @update:model-value="(v: string) => (query.status = v ?? '')"
    >
      <el-option label="启用" value="active" />
      <el-option label="停用" value="inactive" />
      <el-option label="废弃" value="deprecated" />
    </el-select>
    <el-button type="primary" @click="emit('search')">
      <el-icon><Search /></el-icon>
      搜索
    </el-button>
    <el-button type="primary" @click="emit('create')">
      <el-icon><Plus /></el-icon>
      新建接口
    </el-button>
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Plus, Search } from '@element-plus/icons-vue'

interface EpQuery {
  page: number
  page_size: number
  keyword: string
  method: string
  status: string
}

const props = defineProps<{
  // 接口查询条件（双向同步）
  query: EpQuery
}>()

const emit = defineEmits<{
  // 搜索
  search: []
  // 新建
  create: []
}>()

void props
</script>
