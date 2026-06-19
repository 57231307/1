<!--
  BpmDfFilter.vue - BPM 流程定义过滤栏
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="params">
      <el-form-item label="流程名称">
        <el-input
          :model-value="params.keyword"
          placeholder="请输入流程名称"
          clearable
          @update:model-value="(v: string) => (params.keyword = v)"
        />
      </el-form-item>
      <el-form-item label="流程分类">
        <el-select
          :model-value="params.category"
          placeholder="选择分类"
          clearable
          @update:model-value="(v: string) => (params.category = v)"
        >
          <el-option label="财务" value="finance" />
          <el-option label="人事" value="hr" />
          <el-option label="采购" value="purchase" />
          <el-option label="销售" value="sales" />
          <el-option label="生产" value="production" />
          <el-option label="库存" value="inventory" />
          <el-option label="其他" value="other" />
        </el-select>
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('search')">查询</el-button>
        <el-button @click="emit('reset')">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */

// 查询参数类型
interface QryParams {
  page: number
  page_size: number
  keyword: string
  category: string
}

/**
 * 过滤栏组件
 */
defineProps<{
  // 查询参数
  params: QryParams
}>()

const emit = defineEmits<{
  search: []
  reset: []
}>()
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
