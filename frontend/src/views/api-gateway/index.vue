<template>
  <div class="api-gateway-page">
    <div class="page-header">
      <h2 class="page-title">API 网关</h2>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane label="接口管理" name="endpoints">
        <ApiEndpointTab
          :endpoints="endpoints"
          :loading="endpointLoading"
          :total="endpointTotal"
          :query-params="endpointQuery"
          :method-type-map="methodTypeMap"
          :status-type-map="endpointStatusTypeMap"
          :status-map="endpointStatusMap"
          @fetch="fetchEndpoints"
          @new-endpoint="openEndpointDialog()"
          @edit-endpoint="openEndpointDialog"
          @delete-endpoint="handleDeleteEndpoint"
          @update:query-params="(v: any) => Object.assign(endpointQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="API 密钥" name="keys">
        <ApiKeyTab
          :api-keys="keys"
          :loading="keyLoading"
          :total="keyTotal"
          :query-params="keyQuery"
          @fetch="fetchKeys"
          @new-key="openKeyDialog()"
          @view-key="viewKeyDetail"
          @toggle-key="handleToggleKey"
          @delete-key="handleDeleteKey"
          @update:query-params="(v: any) => Object.assign(keyQuery, v)"
        />
      </el-tab-pane>

      <el-tab-pane label="调用日志" name="logs">
        <ApiLogTab
          :logs="logs"
          :loading="logLoading"
          :total="logTotal"
          :query-params="logQuery"
          :method-type-map="methodTypeMap"
          @fetch="fetchLogs"
          @view-log="viewLogDetail"
          @update:query-params="(v: any) => Object.assign(logQuery, v)"
        />
      </el-tab-pane>
    </el-tabs>

    <el-dialog
      v-model="endpointDialogVisible"
      :title="endpointForm.id ? '编辑接口' : '新建接口'"
      width="700px"
    >
      <el-form
        ref="endpointFormRef"
        :model="endpointForm"
        :rules="endpointRules"
        label-width="100px"
      >
        <el-row :gutter="20">
          <el-col :span="16">
            <el-form-item label="接口路径" prop="path">
              <el-input v-model="endpointForm.path" placeholder="例如：/api/v1/users" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="请求方法" prop="method">
              <el-select v-model="endpointForm.method" style="width: 100%">
                <el-option label="GET" value="GET" />
                <el-option label="POST" value="POST" />
                <el-option label="PUT" value="PUT" />
                <el-option label="DELETE" value="DELETE" />
                <el-option label="PATCH" value="PATCH" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input v-model="endpointForm.description" placeholder="请输入接口描述" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="模块" prop="module">
              <el-input v-model="endpointForm.module" placeholder="模块名称" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="限流(/秒)" prop="rate_limit">
              <el-input-number v-model="endpointForm.rate_limit" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="超时(ms)" prop="timeout">
              <el-input-number v-model="endpointForm.timeout" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="需要认证" prop="authentication">
              <el-switch v-model="endpointForm.authentication" />
            </el-form-item>
          </el-col>
          <el-col :span="16">
            <el-form-item label="权限" prop="authorization">
              <el-input v-model="authorizationText" placeholder="多个权限用逗号分隔" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="请求Schema" prop="request_schema">
          <el-input
            v-model="requestSchemaText"
            type="textarea"
            :rows="4"
            placeholder="JSON格式请求Schema"
          />
        </el-form-item>
        <el-form-item label="响应Schema" prop="response_schema">
          <el-input
            v-model="responseSchemaText"
            type="textarea"
            :rows="4"
            placeholder="JSON格式响应Schema"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="endpointDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="endpointSubmitLoading" @click="handleEndpointSubmit"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog
      v-model="keyDialogVisible"
      :title="keyForm.id ? '编辑密钥' : '新建密钥'"
      width="600px"
    >
      <el-form ref="keyFormRef" :model="keyForm" :rules="keyRules" label-width="100px">
        <el-form-item label="密钥名称" prop="key_name">
          <el-input v-model="keyForm.key_name" placeholder="请输入密钥名称" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input
            v-model="keyForm.description"
            type="textarea"
            :rows="3"
            placeholder="请输入描述"
          />
        </el-form-item>
        <el-form-item label="权限" prop="permissions">
          <el-input v-model="permissionsText" placeholder="多个权限用逗号分隔" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="限流(/秒)" prop="rate_limit">
              <el-input-number v-model="keyForm.rate_limit" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="过期时间" prop="expires_at">
              <el-date-picker
                v-model="keyForm.expires_at"
                type="datetime"
                placeholder="选择过期时间"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="keyDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="keySubmitLoading" @click="handleKeySubmit"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="logDetailVisible" title="日志详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="接口路径">{{
          currentLog?.endpoint_path
        }}</el-descriptions-item>
        <el-descriptions-item label="请求方法">{{ currentLog?.method }}</el-descriptions-item>
        <el-descriptions-item label="状态码">{{ currentLog?.status_code }}</el-descriptions-item>
        <el-descriptions-item label="响应时间"
          >{{ currentLog?.response_time }}ms</el-descriptions-item
        >
        <el-descriptions-item label="IP地址">{{ currentLog?.ip_address }}</el-descriptions-item>
        <el-descriptions-item label="用户">{{ currentLog?.user_name }}</el-descriptions-item>
        <el-descriptions-item label="请求时间" :span="2">{{
          currentLog?.created_at
        }}</el-descriptions-item>
      </el-descriptions>
      <div class="log-section">
        <h4>请求体</h4>
        <pre>{{ currentLog?.request_body || '无' }}</pre>
      </div>
      <div class="log-section">
        <h4>响应体</h4>
        <pre>{{ currentLog?.response_body || '无' }}</pre>
      </div>
      <template #footer>
        <el-button @click="logDetailVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
import {
  listApiEndpoints,
  createApiEndpoint,
  updateApiEndpoint,
  deleteApiEndpoint,
  listApiKeys,
  createApiKey,
  updateApiKey,
  deleteApiKey,
  regenerateApiKey,
  listApiLogs,
  type ApiEndpoint,
  type ApiKey,
  type ApiLog,
} from '@/api/api-gateway'

const activeTab = ref('endpoints')

// 接口相关
const endpoints = ref<ApiEndpoint[]>([])
const endpointTotal = ref(0)
const endpointLoading = ref(false)
const endpointQuery = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  method: '',
  status: '',
})

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
      fetchEndpoints()
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
    fetchEndpoints()
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
  fetchEndpoints()
  fetchKeys()
  fetchLogs()
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
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
.api-key-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}
.log-section {
  margin-top: 16px;
}
.log-section h4 {
  margin-bottom: 8px;
  color: #303133;
}
.log-section pre {
  background-color: #f5f7fa;
  padding: 12px;
  border-radius: 4px;
  max-height: 200px;
  overflow-y: auto;
  font-size: 12px;
  line-height: 1.5;
}
</style>
