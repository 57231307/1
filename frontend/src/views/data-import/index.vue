<!--
  data-import/index.vue - 数据导入管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：596 行 → ~120 行 + 4 子组件 + 2 composable + 1 工具
  批次 289：适配 useTableApi（v-model:page/page-size, queryParams, 移除 onMounted fetch）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="data-import-page">
    <div class="page-header">
      <h2 class="page-title">数据导入</h2>
      <div class="header-actions">
        <el-button type="primary" @click="diProc.openTemplateDialog()">
          <el-icon><Plus /></el-icon>
          新建模板
        </el-button>
      </div>
    </div>

    <el-tabs v-model="di.activeTab">
      <el-tab-pane label="导入模板" name="templates">
        <DataImportTemplateTable
          :data="di.templates"
          :loading="di.templateLoading"
          :total="di.templateTotal"
          :query-params="di.templateQueryParams"
          v-model:page="di.templatePage"
          v-model:page-size="di.templatePageSize"
          @fetch="di.handleTemplateSearch"
          @update:query-params="(v) => Object.assign(di.templateQueryParams, v)"
          @edit="diProc.openTemplateDialog"
          @delete="diProc.handleDeleteTemplate"
          @download="diProc.handleDownloadTemplate"
          @upload="diProc.openUploadDialog"
        />
      </el-tab-pane>

      <el-tab-pane label="导入任务" name="tasks">
        <DataImportTaskTable
          :data="di.tasks"
          :loading="di.taskLoading"
          :total="di.taskTotal"
          :query-params="di.taskQueryParams"
          v-model:page="di.taskPage"
          v-model:page-size="di.taskPageSize"
          @fetch="di.handleTaskSearch"
          @update:query-params="(v) => Object.assign(di.taskQueryParams, v)"
          @retry="diProc.handleRetryTask"
          @cancel="diProc.handleCancelTask"
          @download-log="diProc.handleDownloadErrorLog"
        />
      </el-tab-pane>
    </el-tabs>

    <DataImportTemplateForm
      v-model:visible="diProc.templateDialogVisible"
      :params="diProc.templateForm"
      :rules="diProc.templateRules"
      :submit-loading="diProc.templateSubmitLoading"
      :columns-text="diProc.columnsText"
      @update:form="(v: DataImportTemplateFormData) => Object.assign(diProc.templateForm, v)"
      @update:columns-text="(v: string) => (diProc.columnsText = v)"
      @submit="diProc.handleTemplateSubmit"
    />

    <DataImportTemplateUpload
      v-model:visible="diProc.uploadDialogVisible"
      :loading="diProc.uploadLoading"
      @submit="diProc.handleUpload"
      @exceed="diProc.handleExceed"
      @file-change="diProc.handleFileChange"
    />
  </div>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'
import { useDi } from './composables/useDi'
import { useDiProc, type DataImportTemplateFormData } from './composables/useDiProc'
import DataImportTemplateTable from './components/DataImportTemplateTable.vue'
import DataImportTaskTable from './components/DataImportTaskTable.vue'
import DataImportTemplateForm from './components/DataImportTemplateForm.vue'
import DataImportTemplateUpload from './components/DataImportTemplateUpload.vue'

// 业务状态
const di = useDi()
const diProc = useDiProc({
  fetchTemplates: di.fetchTemplates,
  fetchTasks: di.fetchTasks,
  activeTab: di.activeTab,
})

// 列表由 useTableApi setup 自动加载，无需 onMounted 调用 fetch
</script>

<style scoped>
.data-import-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.header-actions {
  display: flex;
  align-items: center;
}
</style>
