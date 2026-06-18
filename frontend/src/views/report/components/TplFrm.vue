<script setup lang="ts">
/**
 * TplFrm - 报表模板创建/编辑对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue 编辑表单对话框）
 */
import type {
  ReportTemplate,
  ReportField,
  ReportTemplateField,
  ReportFilterCondition,
} from '@/api/report-enhanced'

interface Props {
  modelValue: boolean
  title: string
  form: Partial<ReportTemplate>
  availableFields: ReportField[]
  selectedFieldKeys: string[]
  selectedFields: ReportField[]
  fieldConfigs: Record<string, Partial<ReportTemplateField>>
  filterConditions: ReportFilterCondition[]
  templateTypes: { label: string; value: string }[]
  categories: { label: string; value: string }[]
  chartTypeOptions: { label: string; value: string }[]
  operatorOptions: { label: string; value: string }[]
  onTypeChange: () => void
  onAddFilter: () => void
  onRemoveFilter: (index: number) => void
  onSubmit: () => void
  onCancel: () => void
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    :title="title"
    width="900px"
    :close-on-click-modal="false"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="form" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="模板名称" required>
            <el-input v-model="form.name" placeholder="请输入模板名称" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="报表类型" required>
            <el-select
              v-model="form.type"
              placeholder="请选择报表类型"
              @change="onTypeChange"
            >
              <el-option
                v-for="t in templateTypes"
                :key="t.value"
                :label="t.label"
                :value="t.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="分类">
            <el-select v-model="form.category" placeholder="请选择分类">
              <el-option
                v-for="c in categories"
                :key="c.value"
                :label="c.label"
                :value="c.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="图表类型">
            <el-select v-model="form.chart_type" placeholder="请选择图表类型">
              <el-option
                v-for="c in chartTypeOptions"
                :key="c.value"
                :label="c.label"
                :value="c.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="描述">
        <el-input
          v-model="form.description"
          type="textarea"
          :rows="2"
          placeholder="请输入模板描述"
        />
      </el-form-item>

      <el-divider content-position="left">字段配置</el-divider>
      <TplFld
        :available-fields="availableFields"
        :selected-field-keys="selectedFieldKeys"
        :selected-fields="selectedFields"
        :field-configs="fieldConfigs"
        :on-field-config-change="() => {}"
      />

      <el-divider content-position="left">筛选条件</el-divider>
      <TplFlt
        :filter-conditions="filterConditions"
        :available-fields="availableFields"
        :operator-options="operatorOptions"
        :on-add="onAddFilter"
        :on-remove="onRemoveFilter"
      />
    </el-form>
    <template #footer>
      <el-button @click="onCancel">取消</el-button>
      <el-button type="primary" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>
