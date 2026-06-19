<!--
  LgsFilter.vue - 物流管理过滤栏
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="params">
      <el-form-item label="运单号">
        <el-input
          :model-value="params.keyword"
          placeholder="请输入运单号"
          clearable
          @update:model-value="(v: string) => (params.keyword = v)"
        />
      </el-form-item>
      <el-form-item label="物流公司">
        <el-select
          :model-value="params.logistics_company"
          placeholder="选择物流公司"
          clearable
          @update:model-value="(v: string) => (params.logistics_company = v)"
        >
          <el-option label="顺丰速运" value="顺丰速运" />
          <el-option label="中通快递" value="中通快递" />
          <el-option label="圆通速递" value="圆通速递" />
          <el-option label="韵达快递" value="韵达快递" />
          <el-option label="京东物流" value="京东物流" />
        </el-select>
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          :model-value="params.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (params.status = v)"
        >
          <el-option label="待发货" value="pending" />
          <el-option label="已发货" value="shipped" />
          <el-option label="运输中" value="in_transit" />
          <el-option label="已签收" value="delivered" />
          <el-option label="已取消" value="cancelled" />
        </el-select>
      </el-form-item>
      <el-form-item label="日期范围">
        <el-date-picker
          :model-value="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="(v: [Date, Date] | null) => (dateRange = v)"
        />
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
  keyword: string
  logistics_company: string
  status: string
  page: number
  page_size: number
}

/**
 * 物流过滤栏组件
 */
defineProps<{
  // 查询参数
  params: QryParams
  // 日期范围
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  search: []
  reset: []
  // 日期范围变化（已直接通过 v-model 处理，此处保留占位）
  'date-change': []
}>()
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
