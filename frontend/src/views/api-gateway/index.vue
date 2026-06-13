<template>
  <div class="api-gateway-page">
    <div class="page-header">
      <h2 class="page-title">API 网关</h2>
    </div>

    <el-tabs v-model="activeTab">
      <el-tab-pane label="接口管理" name="endpoints">
        <el-card shadow="hover">
          <div class="filter-container">
            <el-input
              v-model="endpointQuery.keyword"
              placeholder="搜索接口路径/描述"
              style="width: 200px"
              clearable
              @clear="fetchEndpoints"
              @keyup.enter="fetchEndpoints"
            />
            <el-select
              v-model="endpointQuery.method"
              placeholder="请求方法"
              clearable
              style="width: 120px"
            >
              <el-option label="GET" value="GET" />
              <el-option label="POST" value="POST" />
              <el-option label="PUT" value="PUT" />
              <el-option label="DELETE" value="DELETE" />
              <el-option label="PATCH" value="PATCH" />
            </el-select>
            <el-select
              v-model="endpointQuery.status"
              placeholder="状态"
              clearable
              style="width: 120px"
            >
              <el-option label="启用" value="active" />
              <el-option label="停用" value="inactive" />
              <el-option label="废弃" value="deprecated" />
            </el-select>
            <el-button type="primary" @click="fetchEndpoints">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
            <el-button type="primary" @click="openEndpointDialog()">
              <el-icon><Plus /></el-icon>
              新建接口
            </el-button>
          </div>

          <el-table v-loading="endpointLoading" :data="endpoints" stripe>
            <el-table-column prop="path" label="接口路径" min-width="200" />
            <el-table-column prop="method" label="方法" width="80">
              <template #default="{ row }">
                <el-tag :type="methodTypeMap[row.method]" size="small">
                  {{ row.method }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="description" label="描述" min-width="150" />
            <el-table-column prop="module" label="模块" width="100" />
            <el-table-column prop="rate_limit" label="限流" width="80">
              <template #default="{ row }">
                {{ row.rate_limit ? `${row.rate_limit}/s` : '-' }}
              </template>
            </el-table-column>
            <el-table-column prop="timeout" label="超时(ms)" width="100" />
            <el-table-column prop="authentication" label="认证" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.authentication ? 'success' : 'info'" size="small">
                  {{ row.authentication ? '是' : '否' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="endpointStatusTypeMap[row.status]" size="small">
                  {{ endpointStatusMap[row.status] }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openEndpointDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="handleDeleteEndpoint(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="endpointQuery.page"
              v-model:page-size="endpointQuery.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="endpointTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchEndpoints"
              @current-change="fetchEndpoints"
            />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="API 密钥" name="keys">
        <el-card shadow="hover">
          <div class="filter-container">
            <el-input
              v-model="keyQuery.keyword"
              placeholder="搜索密钥名称"
              style="width: 200px"
              clearable
              @clear="fetchKeys"
              @keyup.enter="fetchKeys"
            />
            <el-select v-model="keyQuery.status" placeholder="状态" clearable style="width: 120px">
              <el-option label="启用" value="active" />
              <el-option label="停用" value="inactive" />
              <el-option label="已过期" value="expired" />
            </el-select>
            <el-button type="primary" @click="fetchKeys">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
            <el-button type="primary" @click="openKeyDialog()">
              <el-icon><Plus /></el-icon>
              新建密钥
            </el-button>
          </div>

          <el-table v-loading="keyLoading" :data="keys" stripe>
            <el-table-column prop="key_name" label="密钥名称" width="150" />
            <el-table-column prop="api_key" label="API Key" min-width="200">
              <template #default="{ row }">
                <div class="api-key-cell">
                  <span>{{ showKeyMap[row.id] ? row.api_key : maskApiKey(row.api_key) }}</span>
                  <el-button type="primary" link size="small" @click="toggleShowKey(row.id)">
                    {{ showKeyMap[row.id] ? '隐藏' : '显示' }}
                  </el-button>
                </div>
              </template>
            </el-table-column>
            <el-table-column prop="description" label="描述" min-width="150" />
            <el-table-column prop="rate_limit" label="限流" width="100">
              <template #default="{ row }">
                {{ row.rate_limit ? `${row.rate_limit}/s` : '-' }}
              </template>
            </el-table-column>
            <el-table-column prop="expires_at" label="过期时间" width="160" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="keyStatusTypeMap[row.status]" size="small">
                  {{ keyStatusMap[row.status] }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="last_used_at" label="最后使用" width="160" />
            <el-table-column label="操作" width="250" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openKeyDialog(row)"
                  >编辑</el-button
                >
                <el-button type="warning" link size="small" @click="handleRegenerateKey(row)"
                  >重新生成</el-button
                >
                <el-button type="danger" link size="small" @click="handleDeleteKey(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="keyQuery.page"
              v-model:page-size="keyQuery.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="keyTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchKeys"
              @current-change="fetchKeys"
            />
          </div>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="调用日志" name="logs">
        <el-card shadow="hover">
          <div class="filter-container">
            <el-input
              v-model="logQuery.keyword"
              placeholder="搜索接口路径"
              style="width: 200px"
              clearable
              @clear="fetchLogs"
              @keyup.enter="fetchLogs"
            />
            <el-select
              v-model="logQuery.method"
              placeholder="请求方法"
              clearable
              style="width: 120px"
            >
              <el-option label="GET" value="GET" />
              <el-option label="POST" value="POST" />
              <el-option label="PUT" value="PUT" />
              <el-option label="DELETE" value="DELETE" />
            </el-select>
            <el-input
              v-model="logQuery.status_code"
              placeholder="状态码"
              style="width: 100px"
              clearable
              @clear="fetchLogs"
              @keyup.enter="fetchLogs"
            />
            <el-button type="primary" @click="fetchLogs">
              <el-icon><Search /></el-icon>
              搜索
            </el-button>
          </div>

          <el-table v-loading="logLoading" :data="logs" stripe>
            <el-table-column prop="endpoint_path" label="接口路径" min-width="200" />
            <el-table-column prop="method" label="方法" width="80">
              <template #default="{ row }">
                <el-tag :type="methodTypeMap[row.method]" size="small">
                  {{ row.method }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="status_code" label="状态码" width="80">
              <template #default="{ row }">
                <el-tag :type="row.status_code < 400 ? 'success' : 'danger'" size="small">
                  {{ row.status_code }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="response_time" label="响应时间" width="100">
              <template #default="{ row }"> {{ row.response_time }}ms </template>
            </el-table-column>
            <el-table-column prop="ip_address" label="IP地址" width="140" />
            <el-table-column prop="user_name" label="用户" width="100" />
            <el-table-column prop="created_at" label="请求时间" width="160" />
            <el-table-column label="操作" width="100" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="viewLogDetail(row)"
                  >详情</el-button
                >
              </template>
            </el-table-column>
          </el-table>

          <div class="pagination-container">
            <el-pagination
              v-model:current-page="logQuery.page"
              v-model:page-size="logQuery.page_size"
              :page-sizes="[10, 20, 50, 100]"
              :total="logTotal"
              layout="total, sizes, prev, pager, next, jumper"
              @size-change="fetchLogs"
              @current-change="fetchLogs"
            />
          </div>
        </el-card>
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
