<!--
  ArReconcileFilter.vue - 应收对账增强版筛选组件
  来源：原 arReconciliation/enhanced.vue 中 筛选区
  拆分日期：2026-06-17 P1-3-Batch-4
-->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          v-model="localForm.customer_name"
          placeholder="客户名称"
          clearable
          @keyup.enter="emit('search')"
        />
      </el-col>
      <el-col :span="5">
        <el-select v-model="localForm.match_status" placeholder="匹配状态" clearable>
          <el-option
            v-for="s in matchStatusOptions"
            :key="s.value"
            :label="s.label"
            :value="s.value"
          />
        </el-select>
      </el-col>
      <el-col :span="5">
        <el-date-picker
          v-model="localForm.start_date"
          type="date"
          placeholder="开始日期"
          class="w-100"
        />
      </el-col>
      <el-col :span="5">
        <el-date-picker
          v-model="localForm.end_date" type="date" placeholder="结束日期" class="w-100" />
      </el-col>
      <el-col :span="3">
        <el-button type="primary" @click="emit('search')">查询</el-button>
        <el-button @click="emit('reset')">重置</el-button>
      </el-col>
    </el-row>
    <div class="filter-actions">
      <el-button type="success" :loading="reconcileLoading" @click="emit('auto-reconcile')">
        <el-icon><Refresh /></el-icon> 自动对账
      </el-button>
      <el-button type="warning" @click="emit('view-confirmations')">
        <el-icon><View /></el-icon> 客户确认
      </el-button>
      <el-button type="danger" @click="emit('open-dispute')">
        <el-icon><CircleClose /></el-icon> 争议处理
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { Refresh, View, CircleClose } from '@element-plus/icons-vue'

export interface ArSearchForm {
  customer_name: string
  match_status: string
  start_date: string
  end_date: string
}

const props = defineProps<{
  searchForm: ArSearchForm
  reconcileLoading: boolean
}>()

const emit = defineEmits<{
  search: []
  reset: []
  'auto-reconcile': []
  'view-confirmations': []
  'open-dispute': []
  'update:searchForm': [value: ArSearchForm]
}>()

const localForm = reactive<ArSearchForm>({ ...props.searchForm })

watch(
  () => props.searchForm,
  newForm => {
    Object.assign(localForm, newForm)
  },
  { deep: true }
)

watch(
  localForm,
  newForm => {
    emit('update:searchForm', { ...newForm })
  },
  { deep: true }
)

const matchStatusOptions = [
  { label: '全部', value: '' },
  { label: '已匹配', value: 'matched' },
  { label: '部分匹配', value: 'partial' },
  { label: '未匹配', value: 'unmatched' },
]
</script>

<style scoped>
.filter-container {
  background: #fff;
  padding: 16px;
  border-radius: 8px;
  margin-bottom: 16px;
}

.filter-actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}

.w-100 {
  width: 100%;
}
</style>
