/**
 * useApiKey.ts - API 网关密钥管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供 API 密钥列表查询、新建、编辑、删除、重新生成等业务方法
 * 行为完全保持一致（仅结构重构）
 * 批次 281：接入 useTableApi，移除手写 keys/keyTotal/keyLoading/keyQuery + fetchKeys
 */
import { ref, reactive } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import {
  createApiKey,
  updateApiKey,
  deleteApiKey,
  regenerateApiKey,
  type ApiKey,
  type CreateApiKeyRequest,
  type UpdateApiKeyRequest,
} from '@/api/api-gateway';
import { useTableApi } from '@/composables/useTableApi';

/**
 * 密钥表单模型（UI 状态；提交时按后端 create/update DTO 键集显式构造载荷，
 * 禁止整表单当载荷——api_key/id/created_at/created_by 等响应键非 DTO字段，
 * 发出去只会被 serde 静默丢弃；创建 DTO 无 description/status，属后端缺口已登记串行清单）。
 */
interface ApiKeyFormModel {
  id?: number;
  key_name: string;
  description: string;
  permissions: string[];
  rate_limit: number;
  expires_at: string;
  status: ApiKey['status'];
}

/**
 * 密钥管理 composable
 * 批次 281：返回 reactive 包装，父组件可直接 .字段 访问（无需 .value）
 */
export function useApiKey() {
  const {
    data: keys,
    total: keyTotal,
    loading: keyLoading,
    page,
    pageSize,
    queryParams: keyQuery,
    refresh: fetchKeys,
  } = useTableApi<ApiKey>({
    url: '/api-gateway/keys',
    onError: (err: unknown) =>
      ElMessage.error(
        (err instanceof Error ? err.message : String(err)) || msg.translate('loadApiKeyFailed')
      ),
  });

  const showKeyMap = ref<Record<number, boolean>>({});

  const keyDialogVisible = ref(false);
  const keyFormRef = ref<FormInstance>();
  const keySubmitLoading = ref(false);
  const permissionsText = ref('');
  const keyForm = reactive<ApiKeyFormModel>({
    id: undefined,
    key_name: '',
    description: '',
    permissions: [],
    rate_limit: 100,
    expires_at: '',
    status: 'active',
  });

  const keyRules: FormRules = {
    key_name: [{ required: true, message: '请输入密钥名称', trigger: 'blur' }],
  };

  const toggleShowKey = (id: number) => {
    showKeyMap.value[id] = !showKeyMap.value[id];
  };

  const openKeyDialog = (row?: ApiKey) => {
    if (row) {
      Object.assign(keyForm, row);
      permissionsText.value = (row.permissions || []).join(',');
    } else {
      Object.assign(keyForm, {
        id: undefined,
        key_name: '',
        description: '',
        permissions: [],
        rate_limit: 100,
        expires_at: '',
        status: 'active',
      });
      permissionsText.value = '';
    }
    keyDialogVisible.value = true;
  };

  const handleKeySubmit = async () => {
    if (!keyFormRef.value) return;
    await keyFormRef.value.validate(async valid => {
      if (!valid) return;

      keySubmitLoading.value = true;
      try {
        keyForm.permissions = permissionsText.value
          ? permissionsText.value.split(',').map((s: string) => s.trim())
          : [];
        // expires_at 空串必须省略该键：后端 update 把解析失败映射为"清空白名单语义"(Some(None))、
        // create 则按空串跳过——均非"保持原值"意图（范式见 views/system/UserTab.vue Option<String> 空值省略键）
        if (keyForm.id) {
          const payload: UpdateApiKeyRequest = {
            key_name: keyForm.key_name,
            permissions: keyForm.permissions,
            rate_limit: keyForm.rate_limit,
            status: keyForm.status,
          };
          if (keyForm.description) payload.description = keyForm.description;
          if (keyForm.expires_at) payload.expires_at = keyForm.expires_at;
          await updateApiKey(keyForm.id, payload);
        } else {
          const payload: CreateApiKeyRequest = {
            key_name: keyForm.key_name,
            permissions: keyForm.permissions,
            rate_limit: keyForm.rate_limit,
          };
          if (keyForm.expires_at) payload.expires_at = keyForm.expires_at;
          await createApiKey(payload);
        }
        msg.success('operationSuccess');
        keyDialogVisible.value = false;
        await fetchKeys();
      } catch (error: unknown) {
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('operationFailed')
        );
      } finally {
        keySubmitLoading.value = false;
      }
    });
  };

  const handleDeleteKey = async (row: ApiKey) => {
    try {
      await ElMessageBox.confirm('确定要删除此密钥吗？', '确认删除', { type: 'warning' });
      await deleteApiKey(row.id);
      msg.success('deleteSuccess');
      await fetchKeys();
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) || msg.translate('deleteFailed')
        );
    }
  };

  const handleRegenerateKey = async (row: ApiKey) => {
    try {
      await ElMessageBox.confirm('确定要重新生成此密钥吗？旧密钥将立即失效。', '确认重新生成', {
        type: 'warning',
      });
      await regenerateApiKey(row.id);
      msg.success('regenerateSuccess');
      await fetchKeys();
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('regenerateFailed')
        );
    }
  };

  /** 查看密钥详情 */
  const viewKeyDetail = (row: ApiKey) => {
    ElMessageBox.alert(
      `应用 ID: ${row.key_name}\n密钥值: ${row.api_key || '（已隐藏）'}\n过期时间: ${row.expires_at || '永久'}`,
      '密钥详情',
      { type: 'info' }
    );
  };

  /** 切换密钥启用/停用状态 */
  const handleToggleKey = async (row: ApiKey) => {
    const nextStatus: ApiKey['status'] = row.status === 'active' ? 'inactive' : 'active';
    try {
      await ElMessageBox.confirm(
        `确定要${nextStatus === 'active' ? '启用' : '停用'}此密钥吗？`,
        '提示',
        { type: 'warning' }
      );
      await updateApiKey(row.id, { status: nextStatus });
      msg.success('operationSuccess');
      await fetchKeys();
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('operationFailed')
        );
    }
  };

  return reactive({
    keys,
    keyTotal,
    keyLoading,
    keyQuery,
    page,
    pageSize,
    fetchKeys,
    showKeyMap,
    toggleShowKey,
    keyDialogVisible,
    keyFormRef,
    keySubmitLoading,
    permissionsText,
    keyForm,
    keyRules,
    openKeyDialog,
    handleKeySubmit,
    handleDeleteKey,
    handleRegenerateKey,
    viewKeyDetail,
    handleToggleKey,
  });
}
