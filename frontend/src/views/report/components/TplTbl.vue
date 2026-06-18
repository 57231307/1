<script setup lang="ts">
/**
 * TplTbl - 报表模板列表（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 主表格）
 */
import { View, Download, Bell, Edit, Delete } from '@element-plus/icons-vue'
import type { ReportTemplate } from '@/api/report-enhanced'

interface Props {
  templates: ReportTemplate[]
  loading: boolean
  templateTypes: { label: string; value: string }[]
  onPreview: (row: ReportTemplate) => void
  onExport: (row: ReportTemplate) => void
  onSubscriptions: (row: ReportTemplate) => void
  onEdit: (row: ReportTemplate) => void
  onDelete: (row: ReportTemplate) => void
}

defineProps<Props>()
</script>

<template>
  <el-table
    :data="templates"
    :loading="loading"
    border
    fit
    highlight-current-row
    style="width: 100%"
  >
    <el-table-column prop="name" label="模板名称" min-width="160" show-overflow-tooltip />
    <el-table-column prop="description" label="描述" min-width="180" show-overflow-tooltip />
    <el-table-column label="类型" width="100">
      <template #default="scope">
        {{ templateTypes.find(t => t.value === scope.row.type)?.label || scope.row.type }}
      </template>
    </el-table-column>
    <el-table-column label="分类" width="100">
      <template #default="scope">
        <el-tag size="small">{{ scope.row.category }}</el-tag>
      </template>
    </el-table-column>
    <el-table-column label="字段数" width="80" align="center">
      <template #default="scope">{{ scope.row.fields?.length || 0 }}</template>
    </el-table-column>
    <el-table-column label="图表" width="80" align="center">
      <template #default="scope">
        <el-tag v-if="scope.row.chart_type !== 'none'" size="small" type="success">{{
          scope.row.chart_type
        }}</el-tag>
        <span v-else>-</span>
      </template>
    </el-table-column>
    <el-table-column prop="updated_at" label="更新时间" width="160" />
    <el-table-column label="操作" width="280" align="center">
      <template #default="scope">
        <el-button size="small" @click="onPreview(scope.row as any)">
          <el-icon><View /></el-icon> 预览
        </el-button>
        <el-button size="small" type="warning" @click="onExport(scope.row as any)">
          <el-icon><Download /></el-icon> 导出
        </el-button>
        <el-button size="small" type="info" @click="onSubscriptions(scope.row as any)">
          <el-icon><Bell /></el-icon> 订阅
        </el-button>
        <el-button
          v-if="!scope.row.is_system"
          size="small"
          type="primary"
          @click="onEdit(scope.row as any)"
        >
          <el-icon><Edit /></el-icon>
        </el-button>
        <el-button
          v-if="!scope.row.is_system"
          size="small"
          type="danger"
          @click="onDelete(scope.row as any)"
        >
          <el-icon><Delete /></el-icon>
        </el-button>
      </template>
    </el-table-column>
  </el-table>
</template>
