<!--
  VchrLstFilter.vue - 凭证列表过滤与操作栏
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <div class="filter-container">
    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElInput
          :model-value="searchForm.voucher_no"
          placeholder="凭证号"
          class="filter-item"
          @update:model-value="(v: string) => (searchForm.voucher_no = v)"
          @keyup.enter="emit('search')"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          :model-value="searchForm.voucher_date_start"
          type="date"
          placeholder="开始日期"
          class="filter-item"
          @update:model-value="(v: string) => (searchForm.voucher_date_start = v)"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          :model-value="searchForm.voucher_date_end"
          type="date"
          placeholder="结束日期"
          class="filter-item"
          @update:model-value="(v: string) => (searchForm.voucher_date_end = v)"
        />
      </ElCol>
      <ElCol :span="6">
        <ElSelect
          :model-value="searchForm.status"
          placeholder="状态"
          class="filter-item"
          @update:model-value="(v: string) => (searchForm.status = v)"
        >
          <ElOption v-for="s in STATUS_OPTIONS" :key="s.value" :label="s.label" :value="s.value" />
        </ElSelect>
      </ElCol>
    </ElRow>
    <div class="filter-actions">
      <ElButton type="primary" @click="emit('search')">查询</ElButton>
      <ElButton @click="emit('reset')">重置</ElButton>
      <ElButton type="success" @click="emit('add')"> <Plus /> 新增凭证</ElButton>
      <ElButton @click="emit('print')"> <Printer /> 打印</ElButton>
      <ElButton @click="emit('export')"> <Download /> 导出</ElButton>
    </div>
  </div>
</template>

<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import { STATUS_OPTIONS } from '../composables/vchrLstFmts'

interface VoucherSearchForm {
  voucher_no: string
  voucher_date_start: string
  voucher_date_end: string
  type: string
  status: string
}

/**
 * 凭证列表过滤与操作栏组件
 * 接收父组件传入的查询对象，双向同步字段值
 */
const props = defineProps<{
  // 凭证查询条件（双向同步）
  searchForm: VoucherSearchForm
}>()

const emit = defineEmits<{
  // 查询按钮点击
  search: []
  // 重置按钮点击
  reset: []
  // 新增凭证
  add: []
  // 打印
  print: []
  // 导出
  export: []
}>()

void props
</script>
