/**
 * useApiKey.ts - API 网关密钥管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供 API 密钥列表查询、新建、编辑、删除、重新生成等业务方法
 * 批次 281：接入 useTableApi，移除手写 keys/keyTotal/keyLoading/keyQuery + fetchKeys
 * wave4 契约收口：编辑对话框回显 description / expires_at 真值，
 * 清空即送显式 null（后端落 NULL / 永不过期），局部更新保持"键缺席=不动"。
 */
import { ref, reactive } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { msg } from '@/utils/message';
import { i18n } from '@/i18n';
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
 * 密钥新建/编辑对话框的表单模型（UI 态，非线上契约）。
 *
 * 可空列与后端出参同形：`null` 就是"未填/已清空"，不用空串冒充
 * （空串会被原样落库并显示为空，正是本轮修掉的缺陷形态）。
 * `expires_at` 用 el-date-picker 的 `value-format="X"`（unix 秒），
 * 提交时由 `toWireExpiresAt` 转成 ISO 8601 送后端，`null` → 显式送 null（永不过期）。
 */
export interface ApiKeyFormValues {
  id?: number;
  key_name: string;
  description: string | null;
  permissions: string[];
  rate_limit: number;
  expires_at: number | null;
  status: ApiKey['status'];
}

/**
 * 后端 ISO 8601 → 选择器所需 unix 秒。
 * null 透传 null（永不过期）；解析不了的值必须显式抛错并外显，
 * 绝不静默当成"未填"——那等于悄悄把有效期清掉。
 */
function toPickerSeconds(iso: string | null): number | null {
  if (iso === null) return null;
  const ms = Date.parse(iso);
  if (Number.isNaN(ms)) {
    throw new Error(`有效期不是合法的 ISO 8601，无法回显：${iso}`);
  }
  return Math.floor(ms / 1000);
}

/** 表单态（unix 秒）→ 线上契约：有值转 ISO 8601，清空（null）送显式 null（=永不过期） */
export function toWireExpiresAt(seconds: number | null | undefined): string | null {
  if (seconds === null || seconds === undefined) return null;
  return new Date(seconds * 1000).toISOString();
}

/** 表单态（文本框清空后为 ''）→ 线上契约：空文本送显式 null（后端落 NULL），有文本原样覆盖 */
export function toWireDescription(value: string | null | undefined): string | null {
  if (value === null || value === undefined || value === '') return null;
  return value;
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

  const emptyKeyForm = (): ApiKeyFormValues => ({
    id: undefined,
    key_name: '',
    description: null,
    permissions: [],
    rate_limit: 100,
    expires_at: null,
    status: 'active',
  });

  const keyForm = reactive<ApiKeyFormValues>(emptyKeyForm());

  const keyRules: FormRules = {
    key_name: [{ required: true, message: '请输入密钥名称', trigger: 'blur' }],
  };

  const toggleShowKey = (id: number) => {
    showKeyMap.value[id] = !showKeyMap.value[id];
  };

  /**
   * 打开新建/编辑对话框。
   * 编辑时逐字段回填后端真值（description / expires_at 原值回显，null 保持 null），
   * 不把响应键（api_key/created_at/created_by…）灌进表单模型。
   */
  const openKeyDialog = (row?: ApiKey) => {
    if (row) {
      let expirySeconds: number | null;
      try {
        expirySeconds = toPickerSeconds(row.expires_at);
      } catch (error: unknown) {
        ElMessage.error(
          (error instanceof Error ? error.message : String(error)) ||
            msg.translate('operationFailed')
        );
        return;
      }
      Object.assign(keyForm, emptyKeyForm(), {
        id: row.id,
        key_name: row.key_name,
        description: row.description,
        permissions: row.permissions ?? [],
        rate_limit: row.rate_limit,
        expires_at: expirySeconds,
        status: row.status,
      });
      permissionsText.value = (row.permissions ?? []).join(',');
    } else {
      Object.assign(keyForm, emptyKeyForm());
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
        if (keyForm.id) {
          // 三态中的「有值/清空」：对话框始终采集这两个字段——未改动即原值回传（覆盖同值），
          // 输入框清空即显式送 null（后端 description→NULL、expires_at→永不过期）。
          // 「键缺席=保持原值」由局部更新调用方表达（见 handleToggleKey 只送 status）。
          const payload: UpdateApiKeyRequest = {
            key_name: keyForm.key_name,
            permissions: keyForm.permissions,
            rate_limit: keyForm.rate_limit,
            status: keyForm.status,
            description: toWireDescription(keyForm.description),
            expires_at: toWireExpiresAt(keyForm.expires_at),
          };
          await updateApiKey(keyForm.id, payload);
        } else {
          const payload: CreateApiKeyRequest = {
            key_name: keyForm.key_name,
            permissions: keyForm.permissions,
            rate_limit: keyForm.rate_limit,
            description: toWireDescription(keyForm.description),
            expires_at: toWireExpiresAt(keyForm.expires_at),
          };
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
    // expires_at 为 null 是列真值（永不过期），用 i18n 文案显式表达，不做 `|| '永久'` 之类的假默认
    const expiryText =
      row.expires_at === null ? i18n.global.t('apiGateway.keyTab.neverExpires') : row.expires_at;
    ElMessageBox.alert(
      `应用 ID: ${row.key_name}\n密钥值: ${row.api_key || '（已隐藏）'}\n过期时间: ${expiryText}`,
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
