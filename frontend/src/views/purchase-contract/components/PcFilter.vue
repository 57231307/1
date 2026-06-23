<!--
  PcFilter.vue - 采购合同过滤栏
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover" class="filter-card">
    <el-form :inline="true" :model="localQueryParams" class="filter-form">
      <el-form-item label="关键词">
        <el-input
          :model-value="localQueryParams.keyword"
          placeholder="合同编号/合同名称"
          clearable
          @update:model-value="(v: string) => (localQueryParams.keyword = v)"
          @clear="emit('query')"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          :model-value="localQueryParams.supplier_id"
          placeholder="选择供应商"
          clearable
          @update:model-value="(v: number) => (localQueryParams.supplier_id = v)"
          @change="emit('query')"
        >
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-form-item label="合同状态">
        <el-select
          :model-value="localQueryParams.status"
          placeholder="选择状态"
          clearable
          @update:model-value="(v: string) => (localQueryParams.status = v)"
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
          :model-value="localQueryParams.date_range"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="(v: string[]) => (localQueryParams.date_range = v)"
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
import { ref, watch, nextTick } from 'vue'
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

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: PcQueryParams
  // 供应商列表
  suppliers: Supplier[]
}>()

const emit = defineEmits<{
  (e: 'query'): void
  (e: 'reset'): void
  // 整体回写查询参数（父组件监听此事件并 Object.assign 到自己的 queryParams）
  (e: 'update:queryParams', queryParams: PcQueryParams): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQueryParams = ref<PcQueryParams>({
  ...props.queryParams,
  date_range: [...(props.queryParams.date_range || [])],
})

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.queryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    localQueryParams.value = {
      ...newParams,
      date_range: [...(newParams.date_range || [])],
    }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localQueryParams,
  (newParams) => {
    if (syncing) return
    syncing = true
    emit('update:queryParams', {
      ...newParams,
      date_range: [...(newParams.date_range || [])],
    })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<style scoped>
.filter-card {
  margin-bottom: 16px;
}
.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}
</style>
