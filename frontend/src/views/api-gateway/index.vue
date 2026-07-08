<!--
  api-gateway/index.vue - API 网关（拆分重构版）
  任务编号: P14 批 1 B3 I-2
  拆分：835 行 → ~115 行 + 3 composable + 1 工具 + 8 子组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="api-gateway-page">
    <div class="page-header">
      <h2 class="page-title">API 网关</h2>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane label="接口管理" name="endpoints">
        <ApiEndpointTab
          :endpoints="ep.endpoints.value"
          :loading="ep.endpointLoading.value"
          :total="ep.endpointTotal.value"
          :query-params="ep.endpointQuery"
          :method-type-map="ep.methodTypeMap"
          :status-type-map="ep.endpointStatusTypeMap"
          :status-map="ep.endpointStatusMap"
          @fetch="ep.fetchEndpoints"
          @new-endpoint="ep.openEndpointDialog()"
          @edit-endpoint="ep.openEndpointDialog"
          @delete-endpoint="ep.handleDeleteEndpoint"
          @update:query-params="(v: EndpointQuery) => Object.assign(ep.endpointQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="API 密钥" name="keys">
        <ApiKeyTab
          :api-keys="key.keys.value"
          :loading="key.keyLoading.value"
          :total="key.keyTotal.value"
          :query-params="key.keyQuery"
          @fetch="key.fetchKeys"
          @new-key="key.openKeyDialog()"
          @view-key="key.viewKeyDetail"
          @toggle-key="key.handleToggleKey"
          @delete-key="key.handleDeleteKey"
          @update:query-params="(v: ApiKeyQuery) => Object.assign(key.keyQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="调用日志" name="logs">
        <ApiLogTab
          :logs="log.logs.value"
          :loading="log.logLoading.value"
          :total="log.logTotal.value"
          :query-params="log.logQuery"
          :method-type-map="log.methodTypeMap"
          @fetch="log.fetchLogs"
          @view-log="log.viewLogDetail"
          @update:query-params="(v: LogQuery) => Object.assign(log.logQuery, v)"
        />
      </el-tab-pane>
    </el-tabs>

    <EpForm
      v-model:visible="ep.endpointDialogVisible.value"
      :form-ref="ep.endpointFormRef"
      :form="ep.endpointForm"
      :submit-loading="ep.endpointSubmitLoading.value"
      :rules="ep.endpointRules"
      v-model:authorization-text="ep.authorizationText.value"
      v-model:request-schema-text="ep.requestSchemaText.value"
      v-model:response-schema-text="ep.responseSchemaText.value"
      @submit="ep.handleEndpointSubmit"
      @update:form="(v) => Object.assign(ep.endpointForm, v)"
    />

    <KeyForm
      v-model:visible="key.keyDialogVisible.value"
      :form-ref="key.keyFormRef"
      :form="key.keyForm"
      :submit-loading="key.keySubmitLoading.value"
      :rules="key.keyRules"
      v-model:permissions-text="key.permissionsText.value"
      @submit="key.handleKeySubmit"
      @update:form="(v) => Object.assign(key.keyForm, v)"
    />

    <LogDetail
      v-model:visible="log.logDetailVisible.value"
      :current-log="log.currentLog.value"
    />
  </div>
</template>

<script setup lang="ts">
// 此文件为 API 网关页面入口，组合 useApiEp/useApiKey/useApiLog 三个 composable。
// 历史版本在此处保留了重复实现，已在前次拆分中迁移到 composables 与子组件，
// 旧实现作为死代码（v-html/不引用）已删除。后续若需扩展请直接修改 composables。

import { ref, onMounted } from 'vue'
import { useApiEp } from './composables/useApiEp'
import { useApiKey } from './composables/useApiKey'
import { useApiLog } from './composables/useApiLog'
import EpForm from './components/EpForm.vue'
import KeyForm from './components/KeyForm.vue'
import LogDetail from './components/LogDetail.vue'
import ApiEndpointTab, { type EndpointQuery } from './tabs/ApiEndpointTab.vue'
import ApiKeyTab, { type ApiKeyQuery } from './tabs/ApiKeyTab.vue'
import ApiLogTab, { type LogQuery } from './tabs/ApiLogTab.vue'

const activeTab = ref('endpoints')

const ep = useApiEp()
const key = useApiKey()
const log = useApiLog()

onMounted(() => {
  ep.fetchEndpoints()
  key.fetchKeys()
  log.fetchLogs()
})
</script>

<style scoped>
.api-gateway-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
</style>
