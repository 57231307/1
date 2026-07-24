<!--
  ArReconciliationFilter.vue - AR 对账过滤与操作栏
  拆分自 arReconciliation/enhanced.vue（P14 批 1 B3 I-2）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          v-model="localSearchForm.customer_name"
          placeholder="客户名称"
          clearable
          @keyup.enter="emit('search')"
        />
      </el-col>
      <el-col :span="5">
        <el-select
          v-model="localSearchForm.match_status"
          placeholder="匹配状态"
          clearable
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
          v-model="localSearchForm.start_date"
          type="date"
          placeholder="开始日期"
          class="w-100"
        />
      </el-col>
      <el-col :span="5">
        <el-date-picker
          v-model="localSearchForm.end_date"
          type="date"
          placeholder="结束日期"
          class="w-100"
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
import { ref, watch, nextTick } from 'vue'
import { Refresh, Promotion, CircleClose } from '@element-plus/icons-vue'
import { MATCH_OPTIONS } from '../composables/arRecFmts'

// 搜索表单类型
interface ArSearchForm {
  customer_name: string
  match_status: string
  start_date: string
  end_date: string
}

const props = defineProps<{
  // 搜索表单（由父组件管理，子组件通过 emit 回写）
  searchForm: ArSearchForm
  // 自动对账加载中状态
  reconcileLoading: boolean
}>()

const emit = defineEmits<{
  search: []
  reset: []
  'auto-reconcile': []
  'view-confirmations': []
  'open-dispute': []
  // 整体回写搜索表单（父组件监听此事件并 Object.assign 到自己的 searchForm）
  'update:searchForm': [v: ArSearchForm]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localSearchForm = ref<ArSearchForm>({ ...props.searchForm })

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
