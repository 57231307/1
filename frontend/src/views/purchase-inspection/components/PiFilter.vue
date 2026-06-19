<!--
  PiFilter.vue - 采购验货过滤栏
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="params">
      <el-form-item label="检验单号">
        <el-input
          :model-value="params.keyword"
          placeholder="请输入检验单号"
          clearable
          @update:model-value="(v: string) => (params.keyword = v)"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          :model-value="params.supplier_id"
          placeholder="选择供应商"
          clearable
          filterable
          @update:model-value="(v: number) => (params.supplier_id = v)"
        >
          <el-option
            v-for="supplier in suppliers"
            :key="supplier.id"
            :label="supplier.name"
            :value="supplier.id"
          />
        </el-select>
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          :model-value="params.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (params.status = v)"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="待检验" value="pending" />
          <el-option label="已完成" value="completed" />
          <el-option label="已拒绝" value="rejected" />
        </el-select>
      </el-form-item>
      <el-form-item label="检验结果">
        <el-select
          :model-value="params.result"
          placeholder="选择结果"
          clearable
          @update:model-value="(v: string) => (params.result = v)"
        >
          <el-option label="合格" value="pass" />
          <el-option label="不合格" value="fail" />
          <el-option label="部分合格" value="partial" />
        </el-select>
      </el-form-item>
      <el-form-item label="检验日期">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="emit('date-change', $event as [Date, Date] | null)"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="emit('query')">查询</el-button>
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
  supplier_id?: number
  status: string
  result: string
}

/**
 * 过滤栏组件
 */
defineProps<{
  // 查询参数
  params: QryParams
  // 日期范围
  dateRange: [Date, Date] | null
  // 供应商列表
  suppliers: { id: number; name: string }[]
}>()

const emit = defineEmits<{
  query: []
  reset: []
  // 日期范围变化
  'date-change': [v: [Date, Date] | null]
}>()
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
