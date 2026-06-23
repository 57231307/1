<!--
  BpmDfFilter.vue - BPM 流程定义过滤栏
  拆分自 bpm/definitions.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localParams">
      <el-form-item label="流程名称">
        <el-input
          v-model="localParams.keyword"
          placeholder="请输入流程名称"
          clearable
        />
      </el-form-item>
      <el-form-item label="流程分类">
        <el-select
          v-model="localParams.category"
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
        <el-button type="primary" @click="emit('search')">查询</el-button>
        <el-button @click="emit('reset')">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'

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
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit 回写）
  params: QryParams
}>()

const emit = defineEmits<{
  search: []
  reset: []
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 params）
  'update:params': [v: QryParams]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localParams = ref<QryParams>({ ...props.params })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.params,
  (newParams) => {
    if (syncing) return
    syncing = true
    localParams.value = { ...newParams }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    emit('update:params', { ...newParams })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
