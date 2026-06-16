<!--
  CountDetailDialogTab.vue - 盘点单详情对话框
  来源：原 inventoryCount/index.vue 中 盘点单详情弹窗
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="盘点单详情"
    width="800px"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-descriptions v-if="currentRow" :column="2" border>
      <el-descriptions-item label="盘点单号">{{ currentRow.count_no }}</el-descriptions-item>
      <el-descriptions-item label="盘点日期">{{ currentRow.count_date }}</el-descriptions-item>
      <el-descriptions-item label="仓库">{{ currentRow.warehouse_name }}</el-descriptions-item>
      <el-descriptions-item label="状态">
        <el-tag :type="currentRow.status === 'completed' ? 'success' : 'warning'" size="small">
          {{ currentRow.status === 'completed' ? '已完成' : '进行中' }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item label="创建人">{{ currentRow.created_by_name }}</el-descriptions-item>
      <el-descriptions-item label="创建时间">{{ currentRow.created_at }}</el-descriptions-item>
      <el-descriptions-item label="完成时间" :span="2">
        {{ currentRow.completed_at || '-' }}
      </el-descriptions-item>
    </el-descriptions>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">关闭</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import type { InventoryCountEntity } from '@/api/inventoryCount'

interface Props {
  modelValue: boolean
  currentRow: InventoryCountEntity | null
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
}

defineProps<Props>()
const emit = defineEmits<Emits>()
</script>
