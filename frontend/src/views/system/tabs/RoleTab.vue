<!--
  RoleTab.vue - 角色管理 Tab
  来源：原 system/index.vue 第 115-148 行（template）+ 900-1026 行（script）
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分而来，逻辑完整可独立运行
-->
<template>
  <div class="role-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.role.title') }}</h2>
      <el-button type="primary" @click="openRoleDialog()">
        <el-icon><Plus /></el-icon> {{ t('system.role.button.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="roleLoading" :data="roles" stripe :aria-label="t('system.role.aria.list')">
        <el-table-column prop="name" :label="t('system.role.column.name')" width="150" />
        <el-table-column prop="code" :label="t('system.role.column.code')" width="150" />
        <el-table-column prop="description" :label="t('system.role.column.description')" min-width="200" />
        <el-table-column prop="status" :label="t('system.role.column.status')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="t('system.role.column.createdAt')" width="160" />
        <el-table-column :label="t('system.role.column.action')" width="200" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'role:update'" size="small" link @click="openRoleDialog(row as Role)">{{ t('system.role.button.edit') }}</el-button>
            <el-button size="small" link @click="openPermissionDialog(row as Role)">{{ t('system.role.button.permission') }}</el-button>
            <el-button v-permission="'role:delete'" size="small" link type="danger" @click="deleteRole(row as Role)">{{ t('system.role.button.delete') }}</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 角色编辑对话框 -->
    <el-dialog
      v-model="roleDialogVisible"
      :title="roleForm.id ? t('system.role.dialog.editTitle') : t('system.role.dialog.createTitle')"
      width="600px"
      :aria-label="t('system.role.dialog.aria')"
    >
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleRules" label-width="100px" :aria-label="t('system.role.form.aria')">
        <el-form-item :label="t('system.role.form.label.name')" prop="name">
          <el-input v-model="roleForm.name" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item :label="t('system.role.form.label.code')" prop="code">
          <el-input v-model="roleForm.code" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item :label="t('system.role.form.label.description')" prop="description">
          <el-input v-model="roleForm.description" type="textarea" :rows="3" />
        </el-form-item>
        <el-form-item v-if="roleForm.id" :label="t('system.role.form.label.status')">
          <el-switch v-model="roleForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">{{ t('system.role.form.button.cancel') }}</el-button>
        <el-button type="primary" :loading="roleSubmitLoading" @click="submitRole">{{ t('system.role.form.button.confirm') }}</el-button>
      </template>
    </el-dialog>

    <!-- 权限配置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="t('system.role.permissionDialog.title', { name: currentRoleName })"
      width="600px"
      :aria-label="t('system.role.permissionDialog.aria')"
    >
      <el-card v-loading="permissionLoading">
        <el-tree
          ref="permissionTreeRef"
          :data="permissionTree"
          :props="{ label: 'name', children: 'children' }"
          show-checkbox
          node-key="id"
          :default-checked-keys="checkedPermissions"
          @check="handlePermissionCheck"
        />
      </el-card>
      <template #footer>
        <el-button @click="permissionDialogVisible = false">{{ t('system.role.permissionDialog.button.cancel') }}</el-button>
        <el-button type="primary" :loading="permissionSubmitLoading" @click="submitPermissions">{{ t('system.role.permissionDialog.button.save') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getRoleList,
  createRole,
  updateRole,
  deleteRole as deleteRoleApi,
  getRolePermissions,
  assignPermission,
  getPermissionList,
  type Role,
  type Permission,
} from '@/api/role'

const { t } = useI18n({ useScope: 'global' })

// 状态标签映射（响应式）
const getStatusLabel = (status: number): string =>
  status === 1 ? t('system.role.status.enabled') : t('system.role.status.disabled')

const roles = ref<Role[]>([])
const roleLoading = ref(false)

const fetchRoles = async () => {
  roleLoading.value = true
  try {
    const res = await getRoleList()
    // v11 批次 165 P2-1 修复：res.data as any 改为运行时安全访问
    const d = res.data as { items?: Role[]; data?: Role[] } | Role[] | undefined
    roles.value = (Array.isArray(d) ? d : d?.items || d?.data || []) as Role[]
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('system.role.message.fetchFailed'))
  } finally {
    roleLoading.value = false
  }
}

defineExpose({ refresh: fetchRoles })

const roleDialogVisible = ref(false)
const roleFormRef = ref<FormInstance>()
const roleSubmitLoading = ref(false)
const roleForm = reactive({
  id: 0,
  name: '',
  code: '',
  description: '',
  status: 1,
})

const roleRules: FormRules = {
  name: [{ required: true, message: t('system.role.message.requiredName'), trigger: 'blur' }],
  code: [{ required: true, message: t('system.role.message.requiredCode'), trigger: 'blur' }],
}

const openRoleDialog = (row?: Role) => {
  roleFormRef.value?.resetFields()
  if (row) {
    Object.assign(roleForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      description: row.description || '',
      status: row.status,
    })
  } else {
    Object.assign(roleForm, {
      id: 0,
      name: '',
      code: '',
      description: '',
      status: 1,
    })
  }
  roleDialogVisible.value = true
}

const submitRole = async () => {
  const valid = await roleFormRef.value?.validate()
  if (!valid) return
  roleSubmitLoading.value = true
  try {
    if (roleForm.id) {
      await updateRole(roleForm.id, {
        name: roleForm.name,
        description: roleForm.description,
        status: roleForm.status,
      })
      ElMessage.success(t('system.role.message.updateSuccess'))
    } else {
      await createRole({
        name: roleForm.name,
        code: roleForm.code,
        description: roleForm.description,
      })
      ElMessage.success(t('system.role.message.createSuccess'))
    }
    roleDialogVisible.value = false
    fetchRoles()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('system.role.message.operationFailed'))
  } finally {
    roleSubmitLoading.value = false
  }
}

const deleteRole = async (row: Role) => {
  try {
    await ElMessageBox.confirm(t('system.role.message.deleteConfirm', { name: row.name }), t('system.role.message.deleteTitle'), { type: 'warning' })
    await deleteRoleApi(row.id)
    ElMessage.success(t('system.role.message.deleteSuccess'))
    fetchRoles()
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') ElMessage.error((e instanceof Error ? e.message : String(e)) || t('system.role.message.deleteFailed'))
  }
}

// 权限配置
const permissionDialogVisible = ref(false)
const permissionTreeRef = ref()
const currentRoleId = ref(0)
const currentRoleName = ref('')
const permissionTree = ref<PermissionTreeNode[]>([])
const checkedPermissions = ref<number[]>([])
const permissionLoading = ref(false)
const permissionSubmitLoading = ref(false)

const openPermissionDialog = (row: Role) => {
  currentRoleId.value = row.id
  currentRoleName.value = row.name
  fetchRolePermissions(row.id)
  permissionDialogVisible.value = true
}

const fetchRolePermissions = async (roleId: number) => {
  permissionLoading.value = true
  try {
    const treeRes = await getPermissionList()
    permissionTree.value = buildPermissionTree(treeRes.data || [])
    const roleRes = await getRolePermissions(roleId)
    checkedPermissions.value = (roleRes.data || []).map((p: Permission) => p.id)
  } catch (e) {
    const { logger } = await import('@/utils/logger')
    logger.error(`${t('system.role.message.fetchPermissionFailed')}:`, e)
  } finally {
    permissionLoading.value = false
  }
}

// v11 批次 165 P2-1 修复：any[] 改为 PermissionTreeNode[]
interface PermissionTreeNode extends Permission {
  children: PermissionTreeNode[]
}

const buildPermissionTree = (perms: Permission[]): PermissionTreeNode[] => {
  const map = new Map<number, PermissionTreeNode>()
  const tree: PermissionTreeNode[] = []
  perms.forEach(p => map.set(p.id, { ...p, children: [] }))
  perms.forEach(p => {
    const node = map.get(p.id)!
    p.parent_id && map.has(p.parent_id)
      ? map.get(p.parent_id)!.children.push(node)
      : tree.push(node)
  })
  return tree
}

// v11 批次 165 P2-1 修复：_: any, { checkedKeys }: any 改为具体类型
const handlePermissionCheck = (_: unknown, { checkedKeys }: { checkedKeys: number[] }) => {
  checkedPermissions.value = checkedKeys
}

const submitPermissions = async () => {
  permissionSubmitLoading.value = true
  try {
    await assignPermission(currentRoleId.value, { permission_ids: checkedPermissions.value })
    ElMessage.success(t('system.role.message.permissionSuccess'))
    permissionDialogVisible.value = false
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error((e instanceof Error ? e.message : String(e)) || t('system.role.message.permissionFailed'))
  } finally {
    permissionSubmitLoading.value = false
  }
}

onMounted(() => {
  fetchRoles()
})
</script>
