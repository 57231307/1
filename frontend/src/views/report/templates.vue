<script setup lang="ts">
/**
 * templates.vue - 报表模板管理页
 * 任务编号: P13 批 1 B3 I-1（拆分原 963 行大 .vue）
 * 拆分后：3 个 composable + 9 个展示子组件
 * 行为完全保持一致（仅结构重构）
 */
import { onMounted } from 'vue'
import { useRptList } from './composables/useRptList'
import { useRptEdit } from './composables/useRptEdit'
import { useRptFltr } from './composables/useRptFltr'
import { useRptExp } from './composables/useRptExp'
import { useRptSub } from './composables/useRptSub'
import TplSearch from './components/TplSearch.vue'
import TplTbl from './components/TplTbl.vue'
import TplPgn from './components/TplPgn.vue'
import TplFrm from './components/TplFrm.vue'
import TplPrev from './components/TplPrev.vue'
import TplExp from './components/TplExp.vue'
import TplSub from './components/TplSub.vue'
import TplSubF from './components/TplSubF.vue'
import type { ReportTemplate } from '@/api/report-enhanced'

// 5 个 composable 实例集中创建
const list = useRptList()
const edit = useRptEdit()
const fltr = useRptFltr()
const exp = useRptExp()
const sub = useRptSub()

/**
 * 包装提交处理：传入筛选条件 + 列表刷新回调
 */
const handleSubmitTemplate = () =>
  edit.handleSubmit(fltr.filterConditions.value, list.loadTemplates)

/**
 * 重置筛选条件（编辑表单打开时同步）
 */
const handleOpenCreate = () => {
  fltr.reset()
  edit.openCreateDialog()
}

const handleOpenEdit = (row: ReportTemplate) => {
  fltr.filterConditions.value = row.filters || []
  edit.openEditDialog(row)
}

onMounted(() => {
  list.loadTemplates()
})
</script>

<template>
  <div class="app-container">
    <TplSearch
      :search-form="list.searchForm.value"
      :template-types="list.templateTypes"
      :categories="list.categories"
      :on-search="list.handleSearch"
      :on-reset="list.handleReset"
      :on-create="handleOpenCreate"
    />

    <TplTbl
      :templates="list.templates.value"
      :loading="list.loading.value"
      :template-types="list.templateTypes"
      :on-preview="exp.handlePreview"
      :on-export="exp.handleExport"
      :on-subscriptions="sub.handleSubscriptions"
      :on-edit="handleOpenEdit"
      :on-delete="list.handleDelete"
    />

    <TplPgn
      :page="list.pagination.value.page"
      :page-size="list.pagination.value.pageSize"
      :total="list.total.value"
      :on-page-change="list.handlePageChange"
      :on-page-size-change="list.handlePageSizeChange"
    />

    <!-- 创建/编辑模板对话框 -->
    <TplFrm
      :model-value="edit.dialogVisible.value"
      :title="edit.dialogTitle.value"
      :form="edit.form.value"
      :available-fields="edit.availableFields.value"
      :selected-field-keys="edit.selectedFieldKeys.value"
      :selected-fields="edit.selectedFields.value"
      :field-configs="edit.fieldConfigs.value"
      :filter-conditions="fltr.filterConditions.value"
      :template-types="list.templateTypes"
      :categories="list.categories"
      :chart-type-options="edit.chartTypeOptions"
      :operator-options="fltr.operatorOptions"
      :on-type-change="edit.handleTypeChange"
      :on-add-filter="fltr.addFilter"
      :on-remove-filter="fltr.removeFilter"
      :on-submit="handleSubmitTemplate"
      :on-cancel="() => (edit.dialogVisible.value = false)"
      @update:model-value="(v: boolean) => (edit.dialogVisible.value = v)"
    />

    <!-- 预览对话框 -->
    <TplPrev
      :model-value="exp.previewDialogVisible.value"
      :preview-data="exp.previewData.value"
      @update:model-value="(v: boolean) => (exp.previewDialogVisible.value = v)"
    />

    <!-- 导出对话框 -->
    <TplExp
      :model-value="exp.exportDialogVisible.value"
      :export-form="exp.exportForm.value"
      :on-submit="exp.doExport"
      :on-cancel="() => (exp.exportDialogVisible.value = false)"
      @update:model-value="(v: boolean) => (exp.exportDialogVisible.value = v)"
    />

    <!-- 订阅管理对话框 -->
    <TplSub
      :model-value="sub.subscriptionDialogVisible.value"
      :template-name="sub.subForm.value.template_name"
      :subscriptions="sub.subscriptions.value"
      :on-create="() => sub.openSubForm()"
      :on-edit="sub.openSubForm"
      :on-toggle="sub.handleToggleSubscription"
      :on-send-now="sub.handleSendNow"
      :on-delete="sub.handleDeleteSubscription"
      :get-schedule-label="sub.getScheduleLabel"
      :get-format-label="sub.getFormatLabel"
      @update:model-value="(v: boolean) => (sub.subscriptionDialogVisible.value = v)"
    />

    <!-- 订阅表单对话框 -->
    <TplSubF
      :model-value="sub.subFormVisible.value"
      :sub-form="sub.subForm.value"
      :on-submit="sub.handleSubmitSubscription"
      :on-cancel="() => (sub.subFormVisible.value = false)"
      @update:model-value="(v: boolean) => (sub.subFormVisible.value = v)"
    />
  </div>
</template>

<style scoped>
.app-container {
  padding: 20px;
}

.filter-container {
  margin-bottom: 20px;
  background: #fff;
  padding: 20px;
  border-radius: 8px;
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.05);
}

.pagination-container {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.field-config-area {
  margin: 16px 0;
}

.field-checkbox-group {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}

.field-checkbox {
  margin-right: 0;
}

.field-config-detail {
  margin-top: 16px;
}

.field-config-detail h4 {
  margin-bottom: 12px;
  color: #303133;
}

.filter-config-area {
  margin: 16px 0;
}

.filter-row {
  display: flex;
  align-items: center;
  margin-top: 8px;
}

.sub-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 16px;
  font-weight: 500;
}

.preview-total {
  margin-top: 16px;
  text-align: right;
  color: #909399;
}
</style>
