/**
 * useApiEp.ts - API 网关接口管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供接口列表查询、新建、编辑、删除等业务方法
 * 行为完全保持一致（仅结构重构）
 * 批次 281：接入 useTableApi，移除手写 endpoints/endpointTotal/endpointLoading/endpointQuery + fetchEndpoints
 */
import { ref, reactive } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import {
  createApiEndpoint,
  updateApiEndpoint,
  deleteApiEndpoint,
  type ApiEndpoint,
  type CreateApiEndpointRequest,
} from '@/api/api-gateway';
import { useTableApi } from '@/composables/useTableApi';

/**
 * 接口管理 composable
 * 批次 281：返回 reactive 包装，父组件可直接 .字段 访问（无需 .value）
 */
export function useApiEp() {
  const {
    data: endpoints,
    total: endpointTotal,
    loading: endpointLoading,
    page,
    pageSize,
    queryParams: endpointQuery,
    refresh: fetchEndpoints,
  } = useTableApi<ApiEndpoint>({
    url: '/api-gateway/endpoints',
    onError: (err: unknown) =>
      ElMessage.error(
        (err instanceof Error ? err.message : String(err)) || msg.translate('loadApiEndpointFailed')
      ),
  });

  const endpointDialogVisible = ref(false);
  const endpointFormRef = ref<FormInstance>();
  const endpointSubmitLoading = ref(false);
  const authorizationText = ref('');
  const requestSchemaText = ref('');
  const responseSchemaText = ref('');
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
  });

  const endpointRules: FormRules = {
    path: [{ required: true, message: '请输入接口路径', trigger: 'blur' }],
    method: [{ required: true, message: '请选择请求方法', trigger: 'change' }],
    description: [{ required: true, message: '请输入描述', trigger: 'blur' }],
  };

  const openEndpointDialog = (row?: ApiEndpoint) => {
    if (row) {
      Object.assign(endpointForm, row);
      authorizationText.value = (row.authorization || []).join(',');
      requestSchemaText.value = JSON.stringify(row.request_schema || {}, null, 2);
      responseSchemaText.value = JSON.stringify(row.response_schema || {}, null, 2);
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
      });
      authorizationText.value = '';
      requestSchemaText.value = '';
      responseSchemaText.value = '';
    }
    endpointDialogVisible.value = true;
  };

  const handleEndpointSubmit = async () => {
    if (!endpointFormRef.value) return;
    await endpointFormRef.value.validate(async valid => {
      if (!valid) return;

      endpointSubmitLoading.value = true;
      try {
        endpointForm.authorization = authorizationText.value
          ? authorizationText.value.split(',').map((s: string) => s.trim())
          : [];
        if (requestSchemaText.value) {
          try {
            endpointForm.request_schema = JSON.parse(requestSchemaText.value);
          } catch (_e) {
            msg.error('invalidRequestSchema');
            return;
          }
        }
        if (responseSchemaText.value) {
          try {
            endpointForm.response_schema = JSON.parse(responseSchemaText.value);
          } catch (_e) {
            msg.error('invalidResponseSchema');
            return;
          }
        }
        // 载荷按后端 UpsertApiEndpointRequest 键集显式构造：
        // 禁止把表单整体当载荷（id/created_at/updated_at/version/deprecated_at 等非 DTO 管理键
        // 会被 serde 静默丢弃，形成假提交）。version/deprecation 字段表单未维护 → 省略（后端保持原值）。
        const payload: CreateApiEndpointRequest = {
          path: endpointForm.path ?? '',
          method: endpointForm.method ?? 'GET',
          description: endpointForm.description ?? '',
          module: endpointForm.module ?? '',
          status: endpointForm.status ?? 'active',
          rate_limit: endpointForm.rate_limit ?? 0,
          timeout: endpointForm.timeout ?? 30000,
          authentication: endpointForm.authentication ?? true,
          authorization: endpointForm.authorization ?? [],
          request_schema: endpointForm.request_schema ?? {},
          response_schema: endpointForm.response_schema ?? {},
        };
        if (endpointForm.id) {
          await updateApiEndpoint(endpointForm.id, payload);
        } else {
          await createApiEndpoint(payload);
        }
        msg.success('operationSuccess');
        endpointDialogVisible.value = false;
        await fetchEndpoints();
      } catch (error: unknown) {
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('operationFailed')
        );
      } finally {
        endpointSubmitLoading.value = false;
      }
    });
  };

  const handleDeleteEndpoint = async (row: ApiEndpoint) => {
    try {
      await ElMessageBox.confirm('确定要删除此接口吗？', '确认删除', { type: 'warning' });
      await deleteApiEndpoint(row.id);
      msg.success('deleteSuccess');
      await fetchEndpoints();
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) || msg.translate('deleteFailed')
        );
    }
  };

  return reactive({
    endpoints,
    endpointTotal,
    endpointLoading,
    endpointQuery,
    page,
    pageSize,
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
  });
}
