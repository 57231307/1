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
        <el-card shadow="hover">
          <EpFilter
            :query="ep.endpointQuery"
            @search="ep.fetchEndpoints"
            @create="() => ep.openEndpointDialog()"
          />
          <EpTbl
            :endpoints="ep.endpoints.value"
            :loading="ep.endpointLoading.value"
            :total="ep.endpointTotal.value"
            :query="ep.endpointQuery"
            @edit="ep.openEndpointDialog"
            @delete="ep.handleDeleteEndpoint"
            @page-change="ep.fetchEndpoints"
          />
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="API 密钥" name="keys">
        <el-card shadow="hover">
          <KeyFilter
            :query="key.keyQuery"
            @search="key.fetchKeys"
            @create="() => key.openKeyDialog()"
          />
          <KeyTbl
            :keys="key.keys.value"
            :loading="key.keyLoading.value"
            :total="key.keyTotal.value"
            :query="key.keyQuery"
            :show-key-map="key.showKeyMap.value"
            @edit="key.openKeyDialog"
            @regenerate="key.handleRegenerateKey"
            @delete="key.handleDeleteKey"
            @toggle-show="key.toggleShowKey"
            @page-change="key.fetchKeys"
          />
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="调用日志" name="logs">
        <el-card shadow="hover">
          <LogFilter :query="log.logQuery" @search="log.fetchLogs" />
          <LogTbl
            :logs="log.logs.value"
            :loading="log.logLoading.value"
            :total="log.logTotal.value"
            :query="log.logQuery"
            @view="onViewLog"
            @page-change="log.fetchLogs"
          />
        </el-card>
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
    />

    <KeyForm
      v-model:visible="key.keyDialogVisible.value"
      :form-ref="key.keyFormRef"
      :form="key.keyForm"
      :submit-loading="key.keySubmitLoading.value"
      :rules="key.keyRules"
      v-model:permissions-text="key.permissionsText.value"
      @submit="key.handleKeySubmit"
    />

    <LogDetail
      v-model:visible="log.logDetailVisible.value"
      :current-log="log.currentLog.value"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useApiEp } from './composables/useApiEp'
import { useApiKey } from './composables/useApiKey'
import { useApiLog } from './composables/useApiLog'
import EpFilter from './components/EpFilter.vue'
import EpTbl from './components/EpTbl.vue'
import EpForm from './components/EpForm.vue'
import KeyFilter from './components/KeyFilter.vue'
import KeyTbl from './components/KeyTbl.vue'
import KeyForm from './components/KeyForm.vue'
import LogFilter from './components/LogFilter.vue'
import LogTbl from './components/LogTbl.vue'
import LogDetail from './components/LogDetail.vue'
import type { ApiLog } from '@/api/api-gateway'

const activeTab = ref('endpoints')

const ep = useApiEp()
const key = useApiKey()
const log = useApiLog()

const onViewLog = (row: ApiLog) => {
  log.viewLogDetail(row)
}

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
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
</style>
