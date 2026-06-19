<!--
  ArFilter.vue - AR 对账过滤与操作栏
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          :model-value="searchForm.customer_name"
          placeholder="客户名称"
          clearable
          @update:model-value="(v: string) => (searchForm.customer_name = v ?? '')"
          @keyup.enter="emit('search')"
        />
      </el-col>
      <el-col :span="5">
        <el-select
          :model-value="searchForm.match_status"
          placeholder="匹配状态"
          clearable
          @update:model-value="(v: string) => (searchForm.match_status = v ?? '')"
        >
          <el-option
            v-for="s in MATCH_OPTIONS"
            :key="s.value"
            :label="s.label"
            :value="s.value"
          />
        </el-select>
      </el-col>
      <el-col :span="5">
        <el-date-picker
          :model-value="searchForm.start_date"
          type="date"
          placeholder="开始日期"
          class="w-100"
          @update:model-value="(v: string) => (searchForm.start_date = v ?? '')"
        />
      </el-col>
      <el-col :span="5">
        <el-date-picker
          :model-value="searchForm.end_date"
          type="date"
          placeholder="结束日期"
          class="w-100"
          @update:model-value="(v: string) => (searchForm.end_date = v ?? '')"
        />
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
        <el-icon><Promotion /></el-icon> 客户确认
      </el-button>
      <el-button type="danger" @click="emit('open-dispute')">
        <el-icon><CircleClose /></el-icon> 争议处理
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Refresh, Promotion, CircleClose } from '@element-plus/icons-vue'
import { MATCH_OPTIONS } from '../composables/arRecFmts'

interface ArSearchForm {
  customer_name: string
  match_status: string
  start_date: string
  end_date: string
}

const props = defineProps<{
  /** 搜索表单（双向同步） */
  searchForm: ArSearchForm
  /** 自动对账加载中状态 */
  reconcileLoading: boolean
}>()

const emit = defineEmits<{
  search: []
  reset: []
  'auto-reconcile': []
  'view-confirmations': []
  'open-dispute': []
}>()

void props
</script>

<style scoped>
.filter-container {
  margin-bottom: 20px;
  background: #fff;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}
.filter-actions {
  margin-top: 16px;
  display: flex;
  gap: 8px;
}
.w-100 {
  width: 100%;
}
</style>
