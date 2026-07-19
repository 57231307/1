<!--
  PrRtnFilter.vue - 采购退货过滤栏
  任务编号: P14 批 2 I-3 第 2 批（拆分原 purchase-return/index.vue）
  批次 286：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery" aria-label="采购退货筛选表单">
      <el-form-item label="退货单号">
        <el-input
          v-model="localQuery.keyword"
          placeholder="请输入退货单号"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item label="供应商">
        <el-select
          v-model="localQuery.supplierId"
          placeholder="选择供应商"
          clearable
          filterable
          @change="handleSearch"
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
          v-model="localQuery.status"
          placeholder="选择状态"
          clearable
          @change="handleSearch"
        >
          <el-option label="草稿" value="draft" />
          <el-option label="待审批" value="pending" />
          <el-option label="已审批" value="approved" />
          <el-option label="已拒绝" value="rejected" />
          <el-option label="已完成" value="completed" />
        </el-select>
      </el-form-item>
      <el-form-item label="退货日期">
        <el-date-picker
          v-model="localDateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @update:model-value="onDateChange"
        />
      </el-form-item>
      <el-form-item>
        <el-button type="primary" @click="handleSearch">查询</el-button>
        <el-button @click="handleReset">重置</el-button>
      </el-form-item>
    </el-form>
  </el-card>
</template>

<script setup lang="ts">
import { reactive, ref } from 'vue'

// 供应商数据结构
interface Supplier {
  id: number
  name: string
}

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>
  // 供应商列表
  suppliers: Supplier[]
  // 日期范围（由父组件管理，子组件通过 emit('date-change') 回写）
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  // 触发加载
  fetch: []
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>]
  // 日期变化事件
  'date-change': [value: [Date, Date] | null]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string
  supplierId: number | undefined
  status: string
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  supplierId: props.queryParams.supplierId as number | undefined,
  status: (props.queryParams.status as string) ?? '',
})

// 本地日期范围镜像（避免直接修改 prop）
const localDateRange = ref<[Date, Date] | null>(props.dateRange)

/** 日期范围变化：同步本地镜像 + emit 通知父组件 */
const onDateChange = (v: [Date, Date] | null) => {
  localDateRange.value = v
  emit('date-change', v)
}

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.keyword = ''
  localQuery.supplierId = undefined
  localQuery.status = ''
  localDateRange.value = null
  emit('date-change', null)
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}
</script>

<style scoped>
.filter-card {
  margin-bottom: 20px;
}
</style>
