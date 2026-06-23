<!--
  PiFilter.vue - 采购验货过滤栏
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localParams">
      <el-form-item label="检验单号">
        <el-input
          :model-value="localParams.keyword"
          placeholder="请输入检验单号"
          clearable
          @update:model-value="(v: string) => (localParams.keyword = v)"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          :model-value="localParams.supplier_id"
          placeholder="选择供应商"
          clearable
          filterable
          @update:model-value="(v: number) => (localParams.supplier_id = v)"
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
          :model-value="localParams.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (localParams.status = v)"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="待检验" value="pending" />
          <el-option label="已完成" value="completed" />
          <el-option label="已拒绝" value="rejected" />
        </el-select>
      </el-form-item>
      <el-form-item label="检验结果">
        <el-select
          :model-value="localParams.result"
          placeholder="选择结果"
          clearable
          @update:model-value="(v: string) => (localParams.result = v)"
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
import { ref, watch, nextTick } from 'vue'

// 查询参数类型
interface QryParams {
  page: number
  page_size: number
  keyword: string
  supplier_id?: number
  status: string
  result: string
}

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:params') 回写）
  params: QryParams
  // 日期范围
  dateRange: [Date, Date] | null
  // 供应商列表
  suppliers: { id: number; name: string }[]
}>()

const emit = defineEmits<{
  (e: 'query'): void
  (e: 'reset'): void
  // 日期范围变化
  (e: 'date-change', v: [Date, Date] | null): void
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 params）
  (e: 'update:params', params: QryParams): void
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
