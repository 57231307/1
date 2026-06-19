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
          :endpoints="ep.endpoints"
          :loading="ep.endpointLoading"
          :total="ep.endpointTotal"
          :query-params="ep.endpointQuery"
          :method-type-map="ep.methodTypeMap"
          :status-type-map="ep.endpointStatusTypeMap"
          :status-map="ep.endpointStatusMap"
          @fetch="ep.fetchEndpoints"
          @new-endpoint="ep.openEndpointDialog()"
          @edit-endpoint="ep.openEndpointDialog"
          @delete-endpoint="ep.handleDeleteEndpoint"
          @update:query-params="(v: any) => Object.assign(ep.endpointQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="API 密钥" name="keys">
        <ApiKeyTab
          :api-keys="key.keys"
          :loading="key.keyLoading"
          :total="key.keyTotal"
          :query-params="key.keyQuery"
          @fetch="key.fetchKeys"
          @new-key="key.openKeyDialog()"
          @view-key="key.viewKeyDetail"
          @toggle-key="key.handleToggleKey"
          @delete-key="key.handleDeleteKey"
          @update:query-params="(v: any) => Object.assign(key.keyQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="调用日志" name="logs">
        <ApiLogTab
          :logs="log.logs"
          :loading="log.logLoading"
          :total="log.logTotal"
          :query-params="log.logQuery"
          :method-type-map="log.methodTypeMap"
          @fetch="log.fetchLogs"
          @view-log="log.viewLogDetail"
          @update:query-params="(v: any) => Object.assign(log.logQuery, v)"
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

const methodTypeMap: Record<string, string> = {
  GET: 'success',
  POST: 'primary',
  PUT: 'warning',
  DELETE: 'danger',
  PATCH: 'info',
}

const endpointStatusMap: Record<string, string> = {
  active: '启用',
  inactive: '停用',
  deprecated: '废弃',
}

const endpointStatusTypeMap: Record<string, string> = {
  active: 'success',
  inactive: 'info',
  deprecated: 'warning',
}

const fetchEndpoints = async () => {
  endpointLoading.value = true
  try {
    const res = await listApiEndpoints(endpointQuery)
    endpoints.value = res.data || []
    endpointTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取接口失败')
  } finally {
    endpointLoading.value = false
  }
}

// API 网关 Tab 子组件
import ApiEndpointTab from './tabs/ApiEndpointTab.vue'
import ApiKeyTab from './tabs/ApiKeyTab.vue'
import ApiLogTab from './tabs/ApiLogTab.vue'

// 接口表单
const endpointDialogVisible = ref(false)
const endpointFormRef = ref<FormInstance>()
const endpointSubmitLoading = ref(false)
const authorizationText = ref('')
const requestSchemaText = ref('')
const responseSchemaText = ref('')
const endpointForm = reactive<Partial<ApiEndpoint>>({
  id: undefined,
  path: '',
  method: 'GET',
  description: '',
  module: '',
  status: 'active',
  rate_limit: 0,
  timeout: 30000,
  authentication: true,
  authorization: [],
  request_schema: {},
  response_schema: {},
})

const endpointRules: FormRules = {
  path: [{ required: true, message: '请输入接口路径', trigger: 'blur' }],
  method: [{ required: true, message: '请选择请求方法', trigger: 'change' }],
  description: [{ required: true, message: '请输入描述', trigger: 'blur' }],
}

const openEndpointDialog = (row?: ApiEndpoint) => {
  if (row) {
    Object.assign(endpointForm, row)
    authorizationText.value = (row.authorization || []).join(',')
    requestSchemaText.value = JSON.stringify(row.request_schema || {}, null, 2)
    responseSchemaText.value = JSON.stringify(row.response_schema || {}, null, 2)
  } else {
    Object.assign(endpointForm, {
      id: undefined,
      path: '',
      method: 'GET',
      description: '',
      module: '',
      status: 'active',
      rate_limit: 0,
      timeout: 30000,
      authentication: true,
      authorization: [],
      request_schema: {},
      response_schema: {},
    })
    authorizationText.value = ''
    requestSchemaText.value = ''
    responseSchemaText.value = ''
  }
  endpointDialogVisible.value = true
}

const handleEndpointSubmit = async () => {
  if (!endpointFormRef.value) return
  await endpointFormRef.value.validate(async valid => {
    if (!valid) return

    endpointSubmitLoading.value = true
    try {
      endpointForm.authorization = authorizationText.value
        ? authorizationText.value.split(',').map(s => s.trim())
        : []
      if (requestSchemaText.value) {
        try {
          endpointForm.request_schema = JSON.parse(requestSchemaText.value)
        } catch (e) {
          ElMessage.error('请求Schema格式错误')
          return
        }
      }
      if (responseSchemaText.value) {
        try {
          endpointForm.response_schema = JSON.parse(responseSchemaText.value)
        } catch (e) {
          ElMessage.error('响应Schema格式错误')
          return
        }
      }
      if (endpointForm.id) {
        await updateApiEndpoint(endpointForm.id, endpointForm)
      } else {
        await createApiEndpoint(endpointForm)
      }
      ElMessage.success('操作成功')
      endpointDialogVisible.value = false
      ep.fetchEndpoints()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      endpointSubmitLoading.value = false
    }
  })
}

const handleDeleteEndpoint = async (row: ApiEndpoint) => {
  try {
    await ElMessageBox.confirm('确定要删除此接口吗？', '确认删除', { type: 'warning' })
    await deleteApiEndpoint(row.id)
    ElMessage.success('删除成功')
    ep.fetchEndpoints()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

// 密钥相关
const keys = ref<ApiKey[]>([])
const keyTotal = ref(0)
const keyLoading = ref(false)
const keyQuery = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  status: '',
})

const keyStatusMap: Record<string, string> = {
  active: '启用',
  inactive: '停用',
  expired: '已过期',
}

const keyStatusTypeMap: Record<string, string> = {
  active: 'success',
  inactive: 'info',
  expired: 'warning',
}

const showKeyMap = ref<Record<number, boolean>>({})

const maskApiKey = (key: string) => {
  if (!key || key.length < 8) return '***'
  return key.substring(0, 4) + '****' + key.substring(key.length - 4)
}

const toggleShowKey = (id: number) => {
  showKeyMap.value[id] = !showKeyMap.value[id]
}

const fetchKeys = async () => {
  keyLoading.value = true
  try {
    const res = await listApiKeys(keyQuery)
    keys.value = res.data || []
    keyTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取密钥失败')
  } finally {
    keyLoading.value = false
  }
}

// 密钥表单
const keyDialogVisible = ref(false)
const keyFormRef = ref<FormInstance>()
const keySubmitLoading = ref(false)
const permissionsText = ref('')
const keyForm = reactive<Partial<ApiKey>>({
  id: undefined,
  key_name: '',
  description: '',
  permissions: [],
  rate_limit: 100,
  expires_at: '',
  status: 'active',
})

const keyRules: FormRules = {
  key_name: [{ required: true, message: '请输入密钥名称', trigger: 'blur' }],
}

const openKeyDialog = (row?: ApiKey) => {
  if (row) {
    Object.assign(keyForm, row)
    permissionsText.value = (row.permissions || []).join(',')
  } else {
    Object.assign(keyForm, {
      id: undefined,
      key_name: '',
      description: '',
      permissions: [],
      rate_limit: 100,
      expires_at: '',
      status: 'active',
    })
    permissionsText.value = ''
  }
  keyDialogVisible.value = true
}

const handleKeySubmit = async () => {
  if (!keyFormRef.value) return
  await keyFormRef.value.validate(async valid => {
    if (!valid) return

    keySubmitLoading.value = true
    try {
      keyForm.permissions = permissionsText.value
        ? permissionsText.value.split(',').map(s => s.trim())
        : []
      if (keyForm.id) {
        await updateApiKey(keyForm.id, keyForm)
      } else {
        await createApiKey(keyForm)
      }
      ElMessage.success('操作成功')
      keyDialogVisible.value = false
      fetchKeys()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      keySubmitLoading.value = false
    }
  })
}

const handleDeleteKey = async (row: ApiKey) => {
  try {
    await ElMessageBox.confirm('确定要删除此密钥吗？', '确认删除', { type: 'warning' })
    await deleteApiKey(row.id)
    ElMessage.success('删除成功')
    fetchKeys()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

const handleRegenerateKey = async (row: ApiKey) => {
  try {
    await ElMessageBox.confirm('确定要重新生成此密钥吗？旧密钥将立即失效。', '确认重新生成', {
      type: 'warning',
    })
    await regenerateApiKey(row.id)
    ElMessage.success('重新生成成功')
    fetchKeys()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '重新生成失败')
  }
}

// 日志相关
const logs = ref<ApiLog[]>([])
const logTotal = ref(0)
const logLoading = ref(false)
const logQuery = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  method: '',
  status_code: '',
})

const fetchLogs = async () => {
  logLoading.value = true
  try {
    const res = await listApiLogs(logQuery)
    logs.value = res.data || []
    logTotal.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取日志失败')
  } finally {
    logLoading.value = false
  }
}

const logDetailVisible = ref(false)
const currentLog = ref<ApiLog | null>(null)

const viewLogDetail = (row: ApiLog) => {
  currentLog.value = row
  logDetailVisible.value = true
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
