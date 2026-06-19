<!--
  bpm/definitions.vue - BPM 流程定义管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：579 行 → ~130 行 + 5 子组件 + 2 composable + 1 工具
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="bpm-definitions">
    <div class="page-header">
      <h2>流程定义</h2>
      <el-button type="primary" @click="bpmDfProc.handleCreate">
        <el-icon><Plus /></el-icon>
        新建流程
      </el-button>
    </div>

    <BpmDfFilter
      :params="bpmDf.queryParams"
      @search="bpmDfProc.handleSearch"
      @reset="bpmDfProc.handleReset"
    />

    <BpmDfTbl
      :data="bpmDf.definitions"
      :loading="bpmDf.loading"
      :total="bpmDf.total"
      :pagination="bpmDf.queryParams"
      @edit="bpmDfProc.handleEdit"
      @versions="bpmDfProc.handleOpenVersions"
      @save-as-template="bpmDfProc.handleOpenSaveAsTemplate"
      @delete="bpmDfProc.handleDelete"
      @reload="bpmDf.fetchDefinitions"
      @update:page="(v: number) => (bpmDf.queryParams.page = v)"
      @update:size="(v: number) => (bpmDf.queryParams.page_size = v)"
    />

    <BpmDfForm
      v-model:visible="bpmDf.dialogVisible"
      :is-edit="bpmDf.isEdit"
      :form-data="bpmDf.formData"
      :rules="formRules"
      :submit-loading="bpmDf.submitLoading"
      @add-node="handleAddNode"
      @remove-node="handleRemoveNode"
      @submit="bpmDfProc.handleSubmit"
    />

    <BpmDfVerDlg
      v-model:visible="bpmDf.versionDialogVisible"
      :definition="bpmDf.currentDefinition"
      :data="bpmDf.versions"
      :loading="bpmDf.versionLoading"
      @create-version="bpmDfProc.handleCreateVersion"
      @activate="bpmDfProc.handleActivateVersion"
    />

    <BpmDfTplDlg
      v-model:visible="bpmDf.templateDialogVisible"
      :loading="bpmDf.templateLoading"
      :form-data="bpmDf.templateForm"
      :rules="templateRules"
      @submit="bpmDfProc.handleSaveAsTemplate"
    />
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, type FormRules } from 'vue'
import { Plus } from '@element-plus/icons-vue'
import { useBpmDf } from './definitions/composables/useBpmDf'
import { useBpmDfProc } from './definitions/composables/useBpmDfProc'
import BpmDfFilter from './definitions/components/BpmDfFilter.vue'
import BpmDfTbl from './definitions/components/BpmDfTbl.vue'
import BpmDfForm from './definitions/components/BpmDfForm.vue'
import BpmDfVerDlg from './definitions/components/BpmDfVerDlg.vue'
import BpmDfTplDlg from './definitions/components/BpmDfTplDlg.vue'

// 业务状态
const bpmDf = useBpmDf()
const bpmDfProc = useBpmDfProc({
  definitions: bpmDf.definitions,
  loading: bpmDf.loading,
  total: bpmDf.total,
  queryParams: bpmDf.queryParams,
  dialogVisible: bpmDf.dialogVisible,
  isEdit: bpmDf.isEdit,
  submitLoading: bpmDf.submitLoading,
  formData: bpmDf.formData,
  versionDialogVisible: bpmDf.versionDialogVisible,
  versionLoading: bpmDf.versionLoading,
  currentDefinition: bpmDf.currentDefinition,
  versions: bpmDf.versions,
  templateDialogVisible: bpmDf.templateDialogVisible,
  templateLoading: bpmDf.templateLoading,
  templateForm: bpmDf.templateForm,
  fetchDefinitions: bpmDf.fetchDefinitions,
  fetchVersions: bpmDf.fetchVersions,
})

// 表单验证规则
const formRules = reactive<FormRules>({
  process_key: [{ required: true, message: '请输入流程标识', trigger: 'blur' }],
  process_name: [{ required: true, message: '请输入流程名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择分类', trigger: 'change' }],
})

// 模板表单验证规则
const templateRules = reactive<FormRules>({
  template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择分类', trigger: 'change' }],
})

/** 添加节点 */
const handleAddNode = () => {
  bpmDf.formData.nodes.push({
    id: `node_${Date.now()}`,
    type: 'approval',
    name: `节点${bpmDf.formData.nodes.length + 1}`,
    assignee_type: 'user',
    assignee_value: '',
  })
}

/** 删除节点 */
const handleRemoveNode = (index: number) => {
  bpmDf.formData.nodes.splice(index, 1)
}

onMounted(() => {
  bpmDf.fetchDefinitions()
})
</script>

<style scoped>
.bpm-definitions {
  padding: 20px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}
</style>
