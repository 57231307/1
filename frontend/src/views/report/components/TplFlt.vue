<script setup lang="ts">
/**
 * TplFlt - 报表模板筛选条件配置（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 筛选条件）
 */
import { Plus } from '@element-plus/icons-vue'
import type {
  ReportFilterCondition,
  ReportField,
} from '@/api/report-enhanced'

interface Props {
  filterConditions: ReportFilterCondition[]
  availableFields: ReportField[]
  operatorOptions: { label: string; value: string }[]
  onAdd: () => void
  onRemove: (index: number) => void
}

defineProps<Props>()
</script>

<template>
  <div class="filter-config-area">
    <el-button size="small" @click="onAdd">
      <el-icon><Plus /></el-icon> 添加筛选条件
    </el-button>
    <div v-for="(condition, index) in filterConditions" :key="index" class="filter-row">
      <el-select
        v-model="condition.field"
        placeholder="字段"
        size="small"
        style="width: 160px"
      >
        <el-option
          v-for="f in availableFields"
          :key="f.key"
          :label="f.label"
          :value="f.key"
        />
      </el-select>
      <el-select
        v-model="condition.operator"
        placeholder="操作符"
        size="small"
        style="width: 120px; margin-left: 8px"
      >
        <el-option
          v-for="op in operatorOptions"
          :key="op.value"
          :label="op.label"
          :value="op.value"
        />
      </el-select>
      <el-input
        v-model="condition.value"
        placeholder="值"
        size="small"
        style="width: 160px; margin-left: 8px"
      />
      <el-button
        size="small"
        type="danger"
        style="margin-left: 8px"
        @click="onRemove(index)"
        >删除</el-button
      >
    </div>
  </div>
</template>
