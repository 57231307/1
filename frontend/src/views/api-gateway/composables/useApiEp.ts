/* eslint-disable @typescript-eslint/no-explicit-any */
/**
 * useApiEp.ts - API 网关接口管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供接口列表查询、新建、编辑、删除等业务方法
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  listApiEndpoints,
  createApiEndpoint,
  updateApiEndpoint,
  deleteApiEndpoint,
  type ApiEndpoint,
} from '@/api/api-gateway'

/**
 * 接口管理 composable
 */
export function useApiEp() {
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
          ? authorizationText.value.split(',').map((s: string) => s.trim())
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
        await fetchEndpoints()
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
      await fetchEndpoints()
    } catch (error: any) {
      if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
    }
  }

  return {
    endpoints,
    endpointTotal,
    endpointLoading,
    endpointQuery,
    methodTypeMap: {
      GET: 'primary',
      POST: 'success',
      PUT: 'warning',
      DELETE: 'danger',
      PATCH: 'info',
    } as Record<string, string>,
    endpointStatusTypeMap: {
      active: 'success',
      inactive: 'info',
    } as Record<string, string>,
    endpointStatusMap: {
      active: '已激活',
      inactive: '未激活',
    } as Record<string, string>,
    fetchEndpoints,
    endpointDialogVisible,
    endpointFormRef,
    endpointSubmitLoading,
    authorizationText,
    requestSchemaText,
    responseSchemaText,
    endpointForm,
    endpointRules,
    openEndpointDialog,
    handleEndpointSubmit,
    handleDeleteEndpoint,
  }
}
