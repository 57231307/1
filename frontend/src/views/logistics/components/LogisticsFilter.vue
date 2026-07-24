<!--
  LogisticsFilter.vue - 物流管理过滤栏
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  批次 287：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="filter-card">
    <el-form :inline="true" :model="localQuery" aria-label="物流运单筛选表单">
      <el-form-item label="运单号">
        <el-input
          v-model="localQuery.keyword"
          placeholder="请输入运单号"
          clearable
          @keyup.enter="handleSearch"
        />
      </el-form-item>
      <el-form-item label="物流公司">
        <el-select
          v-model="localQuery.logistics_company"
          placeholder="选择物流公司"
          clearable
          @change="handleSearch"
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
          v-model="localQuery.status"
          placeholder="选择状态"
          clearable
          @change="handleSearch"
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

const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>
  // 日期范围（由父组件管理，子组件通过 emit('date-change') 回写）
  dateRange: [Date, Date] | null
}>()

const emit = defineEmits<{
  // 触发加载
  fetch: []
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>]
  // 日期范围变化
  'date-change': [value: [Date, Date] | null]
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  keyword: string
  logistics_company: string
  status: string
}>({
  keyword: (props.queryParams.keyword as string) ?? '',
  logistics_company: (props.queryParams.logistics_company as string) ?? '',
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
  localQuery.logistics_company = ''
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
