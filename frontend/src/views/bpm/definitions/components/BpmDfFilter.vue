<!--
  BpmDfFilter.vue - BPM 流程定义过滤栏
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  批次 282：接入 useTableApi 模式（handleSearch 同步筛选条件 + emit fetch）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery">
      <el-form-item label="流程名称">
        <el-input
          v-model="localQuery.keyword"
          placeholder="请输入流程名称"
          clearable
        />
      </el-form-item>
      <el-form-item label="流程分类">
        <el-select
          v-model="localQuery.category"
          placeholder="选择分类"
          clearable
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
        <el-button type="primary" @click="handleSearch">查询</el-button>
        <el-button @click="handleReset">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive } from 'vue'

// 批次 282：queryParams 类型放宽为 Record<string, unknown>（兼容 useTableApi）
const props = defineProps<{
  queryParams: Record<string, unknown>
}>()

const emit = defineEmits<{
  fetch: []
  'update:queryParams': [value: Record<string, unknown>]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{ keyword: string; category: string }>({
  keyword: (props.queryParams.keyword as string) ?? '',
  category: (props.queryParams.category as string) ?? '',
})

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = ''
  localQuery.category = ''
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
