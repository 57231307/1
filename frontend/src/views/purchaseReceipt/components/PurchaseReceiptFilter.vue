<!--
  PurchaseReceiptFilter.vue - 采购入库过滤栏
  拆分自 purchaseReceipt/index.vue（P14 批 2 I-3 第 4 批）
  批次 285：接入 useTableApi 模式（localQuery + handleSearch/handleReset）
-->
<template>
  <div class="filter-container">
    <el-row :gutter="20">
      <el-col :span="6">
        <el-input
          v-model="localQuery.receipt_no"
          placeholder="入库单号"
          class="filter-item"
          @keyup.enter="handleSearch"
        />
      </el-col>
      <el-col :span="6">
        <el-select
          v-model="localQuery.supplier_id"
          placeholder="选择供应商"
          class="filter-item"
          @change="handleSearch"
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
          v-model="localQuery.warehouse_id"
          placeholder="选择仓库"
          class="filter-item"
          @change="handleSearch"
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
          v-model="localQuery.status"
          placeholder="状态"
          class="filter-item"
          @change="handleSearch"
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
      <el-button type="primary" @click="handleSearch">查询</el-button>
      <el-button @click="handleReset">重置</el-button>
      <el-button type="success" @click="emit('add')">
        <el-icon><Plus /></el-icon>
        新增入库单
      </el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { Plus } from '@element-plus/icons-vue'

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
 * 采购入库过滤栏组件（批次 285：localQuery + handleSearch/handleReset 模式）
 */
const props = defineProps<{
  // 查询参数（由父组件管理，子组件通过 emit('update:queryParams') 回写）
  queryParams: Record<string, unknown>
  // 供应商选项
  suppliers: OptItem[]
  // 仓库选项
  warehouses: OptItem[]
  // 状态选项
  statusOptions: StatusOptItem[]
}>()

const emit = defineEmits<{
  // 触发加载
  fetch: []
  // 整体回写查询参数
  'update:queryParams': [value: Record<string, unknown>]
  // 新增
  add: []
}>()

// 本地查询条件（筛选字段，不含分页参数）
const localQuery = reactive<{
  receipt_no: string
  supplier_id: string
  warehouse_id: string
  status: string
}>({
  receipt_no: (props.queryParams.receipt_no as string) ?? '',
  supplier_id: (props.queryParams.supplier_id as string) ?? '',
  warehouse_id: (props.queryParams.warehouse_id as string) ?? '',
  status: (props.queryParams.status as string) ?? '',
})

/** 搜索：先同步筛选条件到父组件，再触发加载 */
const handleSearch = () => {
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}

/** 重置：清空筛选条件 + 同步 + 触发加载 */
const handleReset = () => {
  localQuery.receipt_no = ''
  localQuery.supplier_id = ''
  localQuery.warehouse_id = ''
  localQuery.status = ''
  emit('update:queryParams', { ...localQuery })
  emit('fetch')
}
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
