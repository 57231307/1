<!--
  PrcFilter.vue - 采购入库过滤栏
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          :model-value="form.receipt_no"
          placeholder="入库单号"
          class="filter-item"
          @update:model-value="(v: string) => (form.receipt_no = v)"
          @keyup.enter="emit('search')"
        />
      </el-col>
      <el-col :span="6">
        <el-select
          :model-value="form.supplier_id"
          placeholder="选择供应商"
          class="filter-item"
          @update:model-value="(v: string) => (form.supplier_id = v)"
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
          :model-value="form.warehouse_id"
          placeholder="选择仓库"
          class="filter-item"
          @update:model-value="(v: string) => (form.warehouse_id = v)"
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
          :model-value="form.status"
          placeholder="状态"
          class="filter-item"
          @update:model-value="(v: string) => (form.status = v)"
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
/* eslint-disable vue/no-mutating-props */
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

/**
 * 采购入库过滤栏组件
 */
defineProps<{
  // 搜索表单
  form: SearchForm
  // 供应商选项
  suppliers: OptItem[]
  // 仓库选项
  warehouses: OptItem[]
  // 状态选项
  statusOptions: StatusOptItem[]
}>()

const emit = defineEmits<{
  search: []
  reset: []
  add: []
}>()
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
