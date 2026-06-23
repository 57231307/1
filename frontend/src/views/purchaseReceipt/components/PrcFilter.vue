<!--
  PrcFilter.vue - 采购入库过滤栏
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          :model-value="localForm.receipt_no"
          placeholder="入库单号"
          class="filter-item"
          @update:model-value="(v: string) => (localForm.receipt_no = v)"
          @keyup.enter="emit('search')"
        />
      </el-col>
      <el-col :span="6">
        <el-select
          :model-value="localForm.supplier_id"
          placeholder="选择供应商"
          class="filter-item"
          @update:model-value="(v: string) => (localForm.supplier_id = v)"
        >
          <el-option label="全部" value="" />
          <el-option
            v-for="s in suppliers"
            :key="s.value"
            :label="s.label"
            :value="String(s.value)"
          />
        </el-select>
      </el-col>
      <el-col :span="6">
        <el-select
          :model-value="localForm.warehouse_id"
          placeholder="选择仓库"
          class="filter-item"
          @update:model-value="(v: string) => (localForm.warehouse_id = v)"
        >
          <el-option label="全部" value="" />
          <el-option
            v-for="w in warehouses"
            :key="w.value"
            :label="w.label"
            :value="String(w.value)"
          />
        </el-select>
      </el-col>
      <el-col :span="6">
        <el-select
          :model-value="localForm.status"
          placeholder="状态"
          class="filter-item"
          @update:model-value="(v: string) => (localForm.status = v)"
        >
          <el-option
            v-for="s in statusOptions"
            :key="s.value"
            :label="s.label"
            :value="s.value"
          />
        </el-select>
      </el-col>
    </el-row>
    <div class="filter-actions">
      <el-button type="primary" @click="emit('search')">查询</el-button>
      <el-button @click="emit('reset')">重置</el-button>
      <el-button type="success" @click="emit('add')">
        <el-icon><Plus /></el-icon>
        新增入库单
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { Plus } from '@element-plus/icons-vue'

// 搜索表单类型
interface SearchForm {
  receipt_no: string
  supplier_id: string
  warehouse_id: string
  status: string
}

// 选项类型
interface OptItem {
  label: string
  value: number
}

// 状态选项类型
interface StatusOptItem {
  label: string
  value: string
}

const props = defineProps<{
  // 搜索表单（由父组件管理，子组件通过 emit('update:form') 回写）
  form: SearchForm
  // 供应商选项
  suppliers: OptItem[]
  // 仓库选项
  warehouses: OptItem[]
  // 状态选项
  statusOptions: StatusOptItem[]
}>()

const emit = defineEmits<{
  (e: 'search'): void
  (e: 'reset'): void
  (e: 'add'): void
  // 整体回写表单（父组件监听此事件并回写到自己的 form）
  (e: 'update:form', form: SearchForm): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<SearchForm>({ ...props.form })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件重置）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', { ...newForm })
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
}

.filter-item {
  width: 100%;
}

.filter-actions {
  margin-top: 10px;
}
</style>
