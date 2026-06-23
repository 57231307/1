<!--
  VchrLstFilter.vue - 凭证列表过滤与操作栏
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <ElRow :gutter="20">
      <ElCol :span="6">
        <ElInput
          v-model="localSearchForm.voucher_no"
          placeholder="凭证号"
          class="filter-item"
          @keyup.enter="emit('search')"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          v-model="localSearchForm.voucher_date_start"
          type="date"
          placeholder="开始日期"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElDatePicker
          v-model="localSearchForm.voucher_date_end"
          type="date"
          placeholder="结束日期"
          class="filter-item"
        />
      </ElCol>
      <ElCol :span="6">
        <ElSelect v-model="localSearchForm.status" placeholder="状态" class="filter-item">
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
import { ref, watch, nextTick } from 'vue'
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
 * 接收父组件传入的查询对象，通过 emit('update:searchForm') 回写
 */
const props = defineProps<{
  // 凭证查询条件（由父组件管理，子组件通过 emit 回写）
  searchForm: VoucherSearchForm
}>()

const emit = defineEmits<{
  // 查询按钮点击
  (e: 'search'): void
  // 重置按钮点击
  (e: 'reset'): void
  // 新增凭证
  (e: 'add'): void
  // 打印
  (e: 'print'): void
  // 导出
  (e: 'export'): void
  // 整体回写查询条件（父组件监听此事件并 Object.assign 到自己的 searchForm）
  (e: 'update:searchForm', searchForm: VoucherSearchForm): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localSearchForm = ref<VoucherSearchForm>({ ...props.searchForm })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.searchForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    localSearchForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localSearchForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:searchForm', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>
