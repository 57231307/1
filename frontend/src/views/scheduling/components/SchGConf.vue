<!-- eslint-disable vue/no-mutating-props -->
<!--
  SchGConf.vue - 排程冲突列表对话框
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
-->
<template>
  <el-dialog
    :model-value="visible"
    title="排程冲突"
    width="700px"
    @update:model-value="onVisibleChange"
  >
    <el-table :data="conflictList" stripe>
      <el-table-column prop="work_center_name" label="工作中心" width="140" />
      <el-table-column label="冲突工单" width="260">
        <template #default="{ row }">
          <span>{{ row.order_no_1 }}</span>
          <el-icon style="margin: 0 8px"><Switch /></el-icon>
          <span>{{ row.order_no_2 }}</span>
        </template>
      </el-table-column>
      <el-table-column label="重叠时间" width="220">
        <template #default="{ row }">
          <div>{{ formatTime(row.overlap_start) }}</div>
          <div>至</div>
          <div>{{ formatTime(row.overlap_end) }}</div>
        </template>
      </el-table-column>
      <el-table-column label="严重程度" width="100">
        <template #default="{ row }">
          <el-tag :type="row.severity === 'error' ? 'danger' : 'warning'" size="small">
            {{ row.severity === 'error' ? '严重' : '警告' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="suggestion" label="建议" />
    </el-table>
  </el-dialog>
</template>

<script setup lang="ts">
import { Switch } from '@element-plus/icons-vue'
import { formatTime } from '../composables/schGFmts'

// 冲突项类型
interface ConflictItem {
  work_center_name: string
  order_no_1: string
  order_no_2: string
  overlap_start: string
  overlap_end: string
  severity: string
  suggestion: string
}

// 排程冲突对话框属性
defineProps<{
  // 对话框可见性
  visible: boolean
  // 冲突列表
  conflictList: ConflictItem[]
}>()

// 定义事件
const emit = defineEmits<{
  // 关闭
  (e: 'update:visible', value: boolean): void
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>
