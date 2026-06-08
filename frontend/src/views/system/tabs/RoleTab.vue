<!--
  RoleTab.vue - 角色管理 Tab
  来源：原 system/index.vue 第 115-148 行（template）+ 900-1026 行（script）
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分而来，逻辑完整可独立运行
-->
<template>
  <div class="role-tab">
    <div class="page-header">
      <h2 class="page-title">角色管理</h2>
      <el-button type="primary" @click="openRoleDialog()">
        <el-icon><Plus /></el-icon> 新建角色
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="roleLoading" :data="roles" stripe>
        <el-table-column prop="name" label="角色名称" width="150" />
        <el-table-column prop="code" label="角色编码" width="150" />
        <el-table-column prop="description" label="描述" min-width="200" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
              {{ row.status === 1 ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="openRoleDialog(row as any)">编辑</el-button>
            <el-button size="small" link @click="openPermissionDialog(row as any)">权限</el-button>
            <el-button size="small" link type="danger" @click="deleteRole(row as any)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 角色编辑对话框 -->
    <el-dialog
      v-model="roleDialogVisible"
      :title="roleForm.id ? '编辑角色' : '新建角色'"
      width="600px"
    >
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleRules" label-width="100px">
        <el-form-item label="角色名称" prop="name">
          <el-input v-model="roleForm.name" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item label="角色编码" prop="code">
          <el-input v-model="roleForm.code" :disabled="!!roleForm.id" />
        </el-form-item>
        <el-form-item label="描述" prop="description">
          <el-input v-model="roleForm.description" type="textarea" :rows="3" />
        </el-form-item>
        <el-form-item v-if="roleForm.id" label="状态">
          <el-switch v-model="roleForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="roleSubmitLoading" @click="submitRole">确定</el-button>
      </template>
    </el-dialog>

    <!-- 权限配置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="`权限配置 - ${currentRoleName}`"
      width="600px"
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
        <el-button @click="permissionDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="permissionSubmitLoading" @click="submitPermissions">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listRoles,
  createRole,
  updateRole,
  deleteRole as deleteRoleApi,
  getRolePermissions,
  assignPermission,
  listPermissions,
  type Role,
  type Permission,
} from '@/api/role'

const roles = ref<Role[]>([])
const roleLoading = ref(false)

const fetchRoles = async () => {
  roleLoading.value = true
  try {
    const res = await listRoles()
    const d = res.data as any
    roles.value = d?.items || d?.data || d || []
  } catch (e: any) {
    ElMessage.error(e.message || '获取角色列表失败')
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
  name: [{ required: true, message: '请输入角色名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色编码', trigger: 'blur' }],
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
      ElMessage.success('更新成功')
    } else {
      await createRole({
        name: roleForm.name,
        code: roleForm.code,
        description: roleForm.description,
      })
      ElMessage.success('创建成功')
    }
    roleDialogVisible.value = false
    fetchRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    roleSubmitLoading.value = false
  }
}

const deleteRole = async (row: Role) => {
  try {
    await ElMessageBox.confirm(`确定删除角色 "${row.name}"?`, '删除确认', { type: 'warning' })
    await deleteRoleApi(row.id)
    ElMessage.success('删除成功')
    fetchRoles()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// 权限配置
const permissionDialogVisible = ref(false)
const permissionTreeRef = ref()
const currentRoleId = ref(0)
const currentRoleName = ref('')
const permissionTree = ref<any[]>([])
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
    const treeRes = await listPermissions()
    permissionTree.value = buildPermissionTree(treeRes.data || [])
    const roleRes = await getRolePermissions(roleId)
    checkedPermissions.value = (roleRes.data || []).map((p: Permission) => p.id)
  } catch (e) {
    console.error('获取权限失败:', e)
  } finally {
    permissionLoading.value = false
  }
}

const buildPermissionTree = (perms: Permission[]): any[] => {
  const map = new Map<number, any>()
  const tree: any[] = []
  perms.forEach((p) => map.set(p.id, { ...p, children: [] }))
  perms.forEach((p) => {
    const node = map.get(p.id)!
    p.parent_id && map.has(p.parent_id)
      ? map.get(p.parent_id)!.children.push(node)
      : tree.push(node)
  })
  return tree
}

const handlePermissionCheck = (_: any, { checkedKeys }: any) => {
  checkedPermissions.value = checkedKeys
}

const submitPermissions = async () => {
  permissionSubmitLoading.value = true
  try {
    await assignPermission(currentRoleId.value, { permission_ids: checkedPermissions.value })
    ElMessage.success('权限配置成功')
    permissionDialogVisible.value = false
  } catch (e: any) {
    ElMessage.error(e.message || '配置失败')
  } finally {
    permissionSubmitLoading.value = false
  }
}

onMounted(() => {
  fetchRoles()
})
</script>
