<!--
  VoucherListFilter.vue - 凭证列表过滤与操作栏
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  批次 287：改造为 localQuery + handleSearch 模式，接入 useTableApi queryParams
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElInput
          v-model="localQuery.voucher_no"
          placeholder="凭证号"
          class="filter-item"
          @keyup.enter="handleSearch"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          v-model="localQuery.voucher_date_start"
          type="date"
          placeholder="开始日期"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          v-model="localQuery.voucher_date_end"
          type="date"
          placeholder="结束日期"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElSelect v-model="localQuery.status" placeholder="状态" class="filter-item">
          <ElOption v-for="s in STATUS_OPTIONS" :key="s.value" :label="s.label" :value="s.value" />
        </ElSelect>
      </ElCol>
    </ElRow>
    <div class="filter-actions">
      <ElButton type="primary" @click="handleSearch">查询</ElButton>
      <ElButton @click="handleReset">重置</ElButton>
      <ElButton type="success" @click="emit('add')"> <Plus /> 新增凭证</ElButton>
      <ElButton @click="emit('print')"> <Printer /> 打印</ElButton>
      <ElButton @click="emit('export')"> <Download /> 导出</ElButton>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import { STATUS_OPTIONS } from '../composables/vchrLstFmts'

/**
 * 凭证列表过滤与操作栏组件
 * 接收父组件传入的 queryParams，通过 emit('update:queryParams') 同步筛选条件
 * 查询/重置时先同步 queryParams 再触发 fetch
 */
const props = defineProps<{
  // 查询条件（由父组件 useTableApi 管理，类型放宽为 Record 兼容 useTableApi）
  queryParams: Record<string, unknown>
}>()

const emit = defineEmits<{
  // 触发查询（父组件监听后调用 handleSearch 重置页码并加载）
  (e: 'fetch'): void
  // 同步查询条件到父组件
  (e: 'update:queryParams', params: Record<string, unknown>): void
  // 新增凭证
  (e: 'add'): void
  // 打印
  (e: 'print'): void
  // 导出
  (e: 'export'): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localQuery = reactive({
  voucher_no: props.queryParams.voucher_no as string,
  voucher_date_start: props.queryParams.voucher_date_start as string,
  voucher_date_end: props.queryParams.voucher_date_end as string,
  type: props.queryParams.type as string,
  status: props.queryParams.status as string,
})

/** 查询：先同步筛选条件到父组件，再触发 fetch */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空本地筛选条件，同步后触发 fetch */
const handleReset = () => {
  localQuery.voucher_no = ''
  localQuery.voucher_date_start = ''
  localQuery.voucher_date_end = ''
  localQuery.type = ''
  localQuery.status = ''
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}
</script>
