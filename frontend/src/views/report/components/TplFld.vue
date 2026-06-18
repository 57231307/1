<script setup lang="ts">
/**
 * TplFld - 报表模板字段配置（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 字段配置）
 */
import type { ReportField, ReportTemplateField } from '@/api/report-enhanced'

interface Props {
  availableFields: ReportField[]
  selectedFieldKeys: string[]
  selectedFields: ReportField[]
  fieldConfigs: Record<string, Partial<ReportTemplateField>>
  onFieldConfigChange: () => void
}

defineProps<Props>()
</script>

<template>
  <div v-if="availableFields.length > 0" class="field-config-area">
    <el-checkbox-group
      v-model="selectedFieldKeys"
      class="field-checkbox-group"
      @change="onFieldConfigChange"
    >
      <el-checkbox
        v-for="field in availableFields"
        :key="field.key"
        :value="field.key"
        border
        class="field-checkbox"
      >
        {{ field.label }}
        <el-tag size="small" type="info">{{ field.type }}</el-tag>
      </el-checkbox>
    </el-checkbox-group>

    <div v-if="selectedFields.length > 0" class="field-config-detail">
      <h4>字段属性配置</h4>
      <el-table :data="selectedFields" border size="small">
        <el-table-column prop="label" label="字段名" width="150" />
        <el-table-column label="显示名称" width="180">
          <template #default="scope">
            <el-input
              v-model="fieldConfigs[scope.row.key].display_label"
              size="small"
              :placeholder="scope.row.label"
            />
          </template>
        </el-table-column>
        <el-table-column label="宽度" width="100">
          <template #default="scope">
            <el-input-number
              v-model="fieldConfigs[scope.row.key].width"
              size="small"
              :min="50"
              :max="500"
              :step="10"
            />
          </template>
        </el-table-column>
        <el-table-column label="格式化">
          <template #default="scope">
            <el-input
              v-model="fieldConfigs[scope.row.key].format"
              size="small"
              placeholder="如: YYYY-MM-DD, ¥#,##0.00"
            />
          </template>
        </el-table-column>
      </el-table>
    </div>
  </div>
  <el-empty v-else description="请先选择报表类型" :image-size="80" />
</template>
