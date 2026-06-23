<!--
  LgsFilter.vue - 物流管理过滤栏
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localParams">
      <el-form-item label="运单号">
        <el-input v-model="localParams.keyword" placeholder="请输入运单号" clearable />
      </el-form-item>
      <el-form-item label="物流公司">
        <el-select v-model="localParams.logistics_company" placeholder="选择物流公司" clearable>
          <el-option label="顺丰速运" value="顺丰速运" />
          <el-option label="中通快递" value="中通快递" />
          <el-option label="圆通速递" value="圆通速递" />
          <el-option label="韵达快递" value="韵达快递" />
          <el-option label="京东物流" value="京东物流" />
        </el-select>
      </el-form-item>
      <el-form-item label="状态">
        <el-select v-model="localParams.status" placeholder="选择状态" clearable>
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
          @update:model-value="emit('date-change', $event as [Date, Date] | null)"
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
import { ref, watch, nextTick } from 'vue'

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
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit 回写）
  params: QryParams
  // 日期范围
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  (e: 'search'): void
  (e: 'reset'): void
  // 日期范围变化（父组件监听后更新 lgs.dateRange）
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
