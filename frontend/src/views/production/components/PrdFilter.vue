<!--
  PrdFilter.vue - 生产管理过滤栏
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-card shadow="never" class="filter-card">
    <el-form :inline="true" :model="form" @submit.prevent>
      <el-form-item label="订单编号">
        <el-input
          :model-value="form.order_no"
          placeholder="请输入订单编号"
          clearable
          style="width: 200px"
          @update:model-value="(v: string) => (form.order_no = v)"
        />
      </el-form-item>
      <el-form-item label="状态">
        <el-select
          :model-value="form.status"
          placeholder="请选择状态"
          clearable
          style="width: 150px"
          @update:model-value="(v: string) => (form.status = v)"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="已计划" value="planned" />
          <el-option label="进行中" value="in_progress" />
          <el-option label="已完成" value="completed" />
          <el-option label="已取消" value="cancelled" />
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

// 过滤表单字段类型
interface FilterForm {
  order_no: string
  status: string
}

/**
 * 生产管理过滤栏组件
 */
defineProps<{
  // 过滤表单数据
  form: FilterForm
}>()

const emit = defineEmits<{
  search: []
  reset: []
}>()
</script>

<style scoped>
.filter-card {
  margin-bottom: 16px;
}
</style>
