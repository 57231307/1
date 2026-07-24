<!--
  bpm/definitions.vue - BPM 流程定义管理（拆分重构版）
  任务编号: P14 批 2 I-3 第 5 批
  拆分：579 行 → ~130 行 + 5 子组件 + 2 composable + 1 工具
  批次 282：BpmDefinitionFilter/BpmDefinitionTable 接入 useTableApi（v-model:page/page-size + @fetch + @update:queryParams）
-->
<template>
  <div class="bpm-definitions">
    <div class="page-header">
      <h2>{{ $t('bpm.definitions.title') }}</h2>
      <el-button type="primary" @click="bpmDfProc.handleCreate">
        <el-icon><Plus /></el-icon>
        {{ $t('bpm.definitions.create') }}
      </el-button>
    </div>

    <BpmDefinitionFilter
      :query-params="bpmDf.queryParams"
      @fetch="bpmDfProc.handleSearch"
      @update:query-params="(v) => Object.assign(bpmDf.queryParams, v)"
    />

    <BpmDefinitionTable
      v-model:page="bpmDf.page"
      v-model:page-size="bpmDf.pageSize"
      :data="bpmDf.definitions"
      :loading="bpmDf.loading"
      :total="bpmDf.total"
      @edit="bpmDfProc.handleEdit"
      @versions="bpmDfProc.handleOpenVersions"
      @save-as-template="bpmDfProc.handleOpenSaveAsTemplate"
      @delete="bpmDfProc.handleDelete"
    />

    <BpmDefinitionForm
      v-model:visible="bpmDf.dialogVisible"
      :is-edit="bpmDf.isEdit"
      :form-data="bpmDf.formData"
      :rules="formRules"
      :submit-loading="bpmDf.submitLoading"
      @add-node="handleAddNode"
      @remove-node="handleRemoveNode"
      @submit="bpmDfProc.handleSubmit"
      @update:form-data="(v) => Object.assign(bpmDf.formData, v)"
    />

    <BpmDefinitionVersionDialog
      v-model:visible="bpmDf.versionDialogVisible"
      :definition="bpmDf.currentDefinition"
      :data="bpmDf.versions"
      :loading="bpmDf.versionLoading"
      @create-version="bpmDfProc.handleCreateVersion"
      @activate="bpmDfProc.handleActivateVersion"
    />

    <BpmDefinitionTemplateDialog
      v-model:visible="bpmDf.templateDialogVisible"
      :loading="bpmDf.templateLoading"
      :form-data="bpmDf.templateForm"
      :rules="templateRules"
      @submit="bpmDfProc.handleSaveAsTemplate"
      @update:form-data="(v) => Object.assign(bpmDf.templateForm, v)"
    />
  </div>
</template>

<script setup lang="ts">
import { reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useBpmDf } from './definitions/composables/useBpmDf'
import { useBpmDfProc } from './definitions/composables/useBpmDfProc'
import BpmDefinitionFilter from './definitions/components/BpmDefinitionFilter.vue'
import BpmDefinitionTable from './definitions/components/BpmDefinitionTable.vue'
import BpmDefinitionForm from './definitions/components/BpmDefinitionForm.vue'
import BpmDefinitionVersionDialog from './definitions/components/BpmDefinitionVersionDialog.vue'
import BpmDefinitionTemplateDialog from './definitions/components/BpmDefinitionTemplateDialog.vue'

const { t } = useI18n({ useScope: 'global' })

// 业务状态
const bpmDf = useBpmDf()
const bpmDfProc = useBpmDfProc({
  definitions: bpmDf.definitions,
  loading: bpmDf.loading,
  total: bpmDf.total,
  page: bpmDf.page,
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
  process_key: [{ required: true, message: t('bpm.definitions.formRules.processKeyRequired'), trigger: 'blur' }],
  process_name: [{ required: true, message: t('bpm.definitions.formRules.processNameRequired'), trigger: 'blur' }],
  category: [{ required: true, message: t('bpm.definitions.formRules.categoryRequired'), trigger: 'change' }],
})

// 模板表单验证规则
const templateRules = reactive<FormRules>({
  template_name: [{ required: true, message: t('bpm.definitions.templateRules.templateNameRequired'), trigger: 'blur' }],
  category: [{ required: true, message: t('bpm.definitions.templateRules.categoryRequired'), trigger: 'change' }],
})

/** 添加节点 */
const handleAddNode = () => {
  bpmDf.formData.nodes.push({
    id: `node_${Date.now()}`,
    type: 'approval',
    name: t('bpm.definitions.nodePrefix', { index: bpmDf.formData.nodes.length + 1 }),
    assignee_type: 'user',
    assignee_value: '',
  })
}

/** 删除节点 */
const handleRemoveNode = (index: number) => {
  bpmDf.formData.nodes.splice(index, 1)
}
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
