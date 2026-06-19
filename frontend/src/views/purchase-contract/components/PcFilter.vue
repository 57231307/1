<!--
  PcFilter.vue - 采购合同过滤栏
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="queryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          :model-value="queryParams.keyword"
          placeholder="合同编号/合同名称"
          clearable
          @update:model-value="(v: string) => (queryParams.keyword = v)"
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          :model-value="queryParams.supplier_id"
          placeholder="选择供应商"
          clearable
          @update:model-value="(v: number) => (queryParams.supplier_id = v)"
          @change="emit('query')"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="合同状态">
        <el-select
          :model-value="queryParams.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (queryParams.status = v)"
          @change="emit('query')"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="待审批" value="pending" />
          <el-option label="已生效" value="active" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="签订日期">
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
        <el-button @click="emit('reset')">
          <el-icon><Refresh /></el-icon>
          重置
        </el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Search, Refresh } from '@element-plus/icons-vue'
import type { Supplier } from '@/api/supplier'

// 采购合同查询参数类型
interface PcQueryParams {
  keyword: string
  supplier_id: number | undefined
  status: string
  date_range: string[]
  page: number
  page_size: number
}

/**
 * 采购合同过滤栏组件
 */
defineProps<{
  // 查询参数
  queryParams: PcQueryParams
  // 供应商列表
  suppliers: Supplier[]
}>()

const emit = defineEmits<{
  // 查询
  query: []
  // 重置
  reset: []
}>()
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
