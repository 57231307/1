/**
 * useTnt - 多租户管理 tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 3 个 tab）
 */
import { ref } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listTenants,
  createTenant,
  updateTenant,
  deleteTenant as deleteTenantApi,
} from '@/api/advanced'
import { logger } from '@/utils/logger'

/**
 * 租户对话框表单数据结构
 */
export interface TenantFormData {
  id: number | null
  name: string
  code: string
  contact_person: string
  contact_phone: string
  email: string
  address: string
  status: string
}

/**
 * 多租户管理 tab 业务逻辑封装
 * 包含租户列表、创建/编辑/删除/更新状态
 */
export function useTnt() {
  const tenants = ref<any[]>([])
  const tenantLoading = ref(false)
  const tenantDialogVisible = ref(false)
  const tenantDialogTitle = ref('新建租户')

  /**
   * 租户表单初始默认值
   */
  const defaultForm = (): TenantFormData => ({
    id: null,
    name: '',
    code: '',
    contact_person: '',
    contact_phone: '',
    email: '',
    address: '',
    status: 'active',
  })

  const tenantForm = ref<TenantFormData>(defaultForm())

  /**
   * 加载租户列表
   */
  const fetchTenants = async () => {
    tenantLoading.value = true
    try {
      const res: any = await listTenants()
      tenants.value = res.data! || []
    } finally {
      tenantLoading.value = false
    }
  }

  /**
   * 打开租户对话框（新建/编辑）
   */
  const openTenantDialog = (row?: any) => {
    if (row) {
      tenantDialogTitle.value = '编辑租户'
      tenantForm.value = { ...row }
    } else {
      tenantDialogTitle.value = '新建租户'
      tenantForm.value = defaultForm()
    }
    tenantDialogVisible.value = true
  }

  /**
   * 提交租户表单（创建/更新）
   */
  const submitTenant = async () => {
    try {
      if (tenantForm.value.id) {
        await updateTenant(tenantForm.value.id, tenantForm.value)
        ElMessage.success('更新成功')
      } else {
        await createTenant(tenantForm.value)
        ElMessage.success('创建成功')
      }
      tenantDialogVisible.value = false
      fetchTenants()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    }
  }

  /**
   * 切换租户启用/停用状态
   */
  const updateTenantStatus = async (row: any) => {
    try {
      const newStatus = row.status === 'active' ? 'inactive' : 'active'
      await ElMessageBox.confirm(
        `确定${newStatus === 'active' ? '启用' : '禁用'}租户 "${row.name}" 吗？`,
        '确认',
        { type: 'warning' }
      )
      await updateTenant(row.id, { status: newStatus })
      ElMessage.success('状态更新成功')
      fetchTenants()
    } catch (e) {
      if (e !== 'cancel') logger.error(String(e))
    }
  }

  /**
   * 删除租户
   */
  const deleteTenant = async (row: any) => {
    try {
      await ElMessageBox.confirm(`确定删除租户 "${row.name}" 吗？`, '确认', { type: 'warning' })
      await deleteTenantApi(row.id)
      ElMessage.success('删除成功')
      fetchTenants()
    } catch (e) {
      if (e !== 'cancel') logger.error(String(e))
    }
  }

  return {
    tenants,
    tenantLoading,
    tenantDialogVisible,
    tenantDialogTitle,
    tenantForm,
    fetchTenants,
    openTenantDialog,
    submitTenant,
    updateTenantStatus,
    deleteTenant,
  }
}
