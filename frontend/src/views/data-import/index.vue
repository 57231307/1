<!--
  data-import/index.vue - 数据导入管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：596 行 → ~120 行 + 4 子组件 + 2 composable + 1 工具
  P9-3 批次 F Pattern A 父组件配合：子组件 prop 改为 params，
  父组件通过 @update:params + Object.assign 整体回写查询参数/表单数据
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
        <DiTplTbl
          :data="di.templates"
          :loading="di.templateLoading"
          :total="di.templateTotal"
          :params="di.templateQuery"
          @update:params="(v: TplQuery) => Object.assign(di.templateQuery, v)"
          @search="di.fetchTemplates"
          @edit="diProc.openTemplateDialog"
          @delete="diProc.handleDeleteTemplate"
          @download="diProc.handleDownloadTemplate"
          @upload="diProc.openUploadDialog"
        />
      </el-tab-pane>

      <el-tab-pane label="导入任务" name="tasks">
        <DiTaskTbl
          :data="di.tasks"
          :loading="di.taskLoading"
          :total="di.taskTotal"
          :params="di.taskQuery"
          @update:params="(v: TaskQuery) => Object.assign(di.taskQuery, v)"
          @search="di.fetchTasks"
          @retry="diProc.handleRetryTask"
          @cancel="diProc.handleCancelTask"
          @download-log="diProc.handleDownloadErrorLog"
        />
      </el-tab-pane>
    </el-tabs>

    <DiTplForm
      v-model:visible="diProc.templateDialogVisible"
      :params="diProc.templateForm"
      :rules="diProc.templateRules"
      :submit-loading="diProc.templateSubmitLoading"
      :columns-text="diProc.columnsText"
      @update:form="(v: DiTplForm) => Object.assign(diProc.templateForm, v)"
      @update:columns-text="(v: string) => (diProc.columnsText = v)"
      @submit="diProc.handleTemplateSubmit"
    />

    <DiTplUpload
      v-model:visible="diProc.uploadDialogVisible"
      :loading="diProc.uploadLoading"
      @submit="diProc.handleUpload"
      @exceed="diProc.handleExceed"
      @file-change="diProc.handleFileChange"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { useDi, type TplQuery, type TaskQuery } from './composables/useDi'
import { useDiProc, type DiTplForm } from './composables/useDiProc'
import DiTplTbl from './components/DiTplTbl.vue'
import DiTaskTbl from './components/DiTaskTbl.vue'
import DiTplForm from './components/DiTplForm.vue'
import DiTplUpload from './components/DiTplUpload.vue'

// 业务状态
const di = useDi()
const diProc = useDiProc({
  templates: di.templates,
  templateTotal: di.templateTotal,
  templateLoading: di.templateLoading,
  templateQuery: di.templateQuery,
  fetchTemplates: di.fetchTemplates,
  tasks: di.tasks,
  taskTotal: di.taskTotal,
  taskLoading: di.taskLoading,
  taskQuery: di.taskQuery,
  fetchTasks: di.fetchTasks,
  activeTab: di.activeTab,
})

onMounted(() => {
  di.fetchTemplates()
  di.fetchTasks()
})
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
