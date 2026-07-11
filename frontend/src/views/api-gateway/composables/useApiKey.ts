/**
 * useApiKey.ts - API 网关密钥管理 composable
 * 任务编号: P14 批 1 B3 I-2
 * 提供 API 密钥列表查询、新建、编辑、删除、重新生成等业务方法
 * 行为完全保持一致（仅结构重构）
 * 批次 281：接入 useTableApi，移除手写 keys/keyTotal/keyLoading/keyQuery + fetchKeys
 */
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  createApiKey,
  updateApiKey,
  deleteApiKey,
  regenerateApiKey,
  type ApiKey,
} from '@/api/api-gateway'
import { useTableApi } from '@/composables/useTableApi'

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
      ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取密钥失败'),
  })

  const showKeyMap = ref<Record<number, boolean>>({})

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

  const toggleShowKey = (id: number) => {
    showKeyMap.value[id] = !showKeyMap.value[id]
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
          ? permissionsText.value.split(',').map((s: string) => s.trim())
          : []
        if (keyForm.id) {
          await updateApiKey(keyForm.id, keyForm)
        } else {
          await createApiKey(keyForm)
        }
        ElMessage.success('操作成功')
        keyDialogVisible.value = false
        await fetchKeys()
      } catch (error: unknown) {
        ElMessage.error((error instanceof Error ? error.message : String(error)) || '操作失败')
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
      await fetchKeys()
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
    }
  }

  const handleRegenerateKey = async (row: ApiKey) => {
    try {
      await ElMessageBox.confirm('确定要重新生成此密钥吗？旧密钥将立即失效。', '确认重新生成', {
        type: 'warning',
      })
      await regenerateApiKey(row.id)
      ElMessage.success('重新生成成功')
      await fetchKeys()
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error((error instanceof Error ? error.message : String(error)) || '重新生成失败')
    }
  }

  /** 查看密钥详情 */
  const viewKeyDetail = (row: ApiKey) => {
    ElMessageBox.alert(
      `应用 ID: ${row.key_name}\n密钥值: ${row.api_key || '（已隐藏）'}\n过期时间: ${row.expires_at || '永久'}`,
      '密钥详情',
      { type: 'info' }
    )
  }

  /** 切换密钥启用/停用状态 */
  const handleToggleKey = async (row: ApiKey) => {
    const nextStatus: ApiKey['status'] = row.status === 'active' ? 'inactive' : 'active'
    try {
      await ElMessageBox.confirm(
        `确定要${nextStatus === 'active' ? '启用' : '停用'}此密钥吗？`,
        '提示',
        { type: 'warning' }
      )
      await updateApiKey(row.id, { status: nextStatus })
      ElMessage.success('操作成功')
      await fetchKeys()
    } catch (error: unknown) {
      if (error !== 'cancel')
        ElMessage.error((error instanceof Error ? error.message : String(error)) || '操作失败')
    }
  }

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
  })
}
