<template>
  <div class="data-permission">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>数据权限管理</span>
        </div>
      </template>

      <div class="layout">
        <div class="role-panel">
          <h3>角色列表</h3>
          <el-menu :default-active="selectedRoleId" class="role-menu" @select="handleSelectRole">
            <el-menu-item v-for="role in roleList" :key="role.id" :index="String(role.id)">
              {{ role.name }}
            </el-menu-item>
          </el-menu>
        </div>

        <div class="permission-panel">
          <div class="panel-header">
            <h3>{{ currentRoleName }} - 数据权限</h3>
            <el-button type="primary" @click="handleAddPermission">添加权限</el-button>
          </div>

          <el-table :data="permissionList" border stripe>
            <el-table-column prop="resourceType" label="资源类型" />
            <el-table-column prop="scopeType" label="数据范围">
              <template #default="{ row }">
                <el-tag v-if="row.scopeType === 'ALL'" type="success">全部数据</el-tag>
                <el-tag v-else-if="row.scopeType === 'DEPT'" type="primary">本部门数据</el-tag>
                <el-tag v-else-if="row.scopeType === 'DEPT_AND_BELOW'" type="warning"
                  >本部门及以下</el-tag
                >
                <el-tag v-else-if="row.scopeType === 'SELF'" type="info">仅本人数据</el-tag>
                <el-tag v-else type="danger">自定义</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="isEnabled" label="状态" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.isEnabled" type="success">启用</el-tag>
                <el-tag v-else type="danger">禁用</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150">
              <template #default="{ row }">
                <el-button link type="primary" @click="handleEditPermission(row)">编辑</el-button>
                <el-button link type="danger" @click="handleDeletePermission(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </div>
      </div>
    </el-card>

    <!-- 权限设置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="isEdit ? '编辑数据权限' : '添加数据权限'"
      width="600px"
    >
      <el-form
        ref="permissionFormRef"
        :model="permissionForm"
        :rules="permissionRules"
        label-width="120px"
      >
        <el-form-item label="资源类型" prop="resourceType">
          <el-select v-model="permissionForm.resourceType" placeholder="请选择" style="width: 100%">
            <el-option label="客户" value="customer" />
            <el-option label="供应商" value="supplier" />
            <el-option label="销售订单" value="sales_order" />
            <el-option label="采购订单" value="purchase_order" />
            <el-option label="库存" value="inventory" />
            <el-option label="财务" value="finance" />
          </el-select>
        </el-form-item>
        <el-form-item label="数据范围" prop="scopeType">
          <el-select v-model="permissionForm.scopeType" placeholder="请选择" style="width: 100%">
            <el-option
              v-for="scope in scopeTypeList"
              :key="scope.value"
              :label="scope.label"
              :value="scope.value"
            >
              <span>{{ scope.label }}</span>
              <span style="color: #909399; font-size: 12px; margin-left: 8px">{{
                scope.description
              }}</span>
            </el-option>
          </el-select>
        </el-form-item>
        <el-form-item
          v-if="permissionForm.scopeType === 'CUSTOM'"
          label="自定义条件"
          prop="customCondition"
        >
          <el-input
            v-model="permissionForm.customCondition"
            type="textarea"
            :rows="4"
            placeholder="输入自定义SQL条件或JSON配置"
          />
        </el-form-item>
        <el-form-item label="允许字段" prop="allowedFields">
          <el-input
            v-model="permissionForm.allowedFields"
            type="textarea"
            :rows="2"
            placeholder="输入允许的字段，多个字段用逗号分隔"
          />
        </el-form-item>
        <el-form-item label="隐藏字段" prop="hiddenFields">
          <el-input
            v-model="permissionForm.hiddenFields"
            type="textarea"
            :rows="2"
            placeholder="输入隐藏的字段，多个字段用逗号分隔"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="permissionDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSavePermission"
          >保存</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listRoleDataPermissions,
  setDataPermission,
  deleteDataPermissionByRole,
  listScopeTypes,
  type DataPermissionRole,
  type ScopeType,
} from '@/api/data-permission'

const roleList = ref([
  { id: 1, name: '超级管理员' },
  { id: 2, name: '财务主管' },
  { id: 3, name: '销售主管' },
  { id: 4, name: '采购员' },
  { id: 5, name: '普通员工' },
])

const selectedRoleId = ref('1')
const permissionList = ref<DataPermissionRole[]>([])
const scopeTypeList = ref<ScopeType[]>([])

const permissionDialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const permissionFormRef = ref()

const permissionForm = reactive({
  roleId: undefined as number | undefined,
  resourceType: '',
  scopeType: '',
  customCondition: '',
  allowedFields: '',
  hiddenFields: '',
})

const permissionRules = {
  resourceType: [{ required: true, message: '请选择资源类型', trigger: 'change' }],
  scopeType: [{ required: true, message: '请选择数据范围', trigger: 'change' }],
}

const currentRoleName = computed(() => {
  const role = roleList.value.find((r) => String(r.id) === selectedRoleId.value)
  return role ? role.name : ''
})

const fetchPermissions = async () => {
  try {
    const res: any = await listRoleDataPermissions(parseInt(selectedRoleId.value))
    if (res.data) {
      permissionList.value = res.data! || []
    }
  } catch (e) {
    ElMessage.error('获取权限列表失败')
  }
}

const fetchScopeTypes = async () => {
  try {
    const res: any = await listScopeTypes()
    if (res.data) {
      scopeTypeList.value = res.data! || []
    }
  } catch (e) {
    // 使用默认值
    scopeTypeList.value = [
      { value: 'ALL', label: '全部数据', description: '可以查看所有数据' },
      { value: 'DEPT', label: '本部门数据', description: '只能查看本部门的数据' },
      {
        value: 'DEPT_AND_BELOW',
        label: '本部门及以下',
        description: '可以查看本部门及下级部门的数据',
      },
      { value: 'SELF', label: '仅本人数据', description: '只能查看自己创建的数据' },
      { value: 'CUSTOM', label: '自定义', description: '通过自定义条件过滤数据' },
    ]
  }
}

const handleSelectRole = (roleId: string) => {
  selectedRoleId.value = roleId
  fetchPermissions()
}

const handleAddPermission = () => {
  isEdit.value = false
  Object.assign(permissionForm, {
    roleId: parseInt(selectedRoleId.value),
    resourceType: '',
    scopeType: '',
    customCondition: '',
    allowedFields: '',
    hiddenFields: '',
  })
  permissionDialogVisible.value = true
}

const handleEditPermission = (row: DataPermissionRole) => {
  isEdit.value = true
  Object.assign(permissionForm, {
    roleId: row.roleId,
    resourceType: row.resourceType,
    scopeType: row.scopeType,
    customCondition: row.customCondition || '',
    allowedFields: row.allowedFields || '',
    hiddenFields: row.hiddenFields || '',
  })
  permissionDialogVisible.value = true
}

const handleSavePermission = async () => {
  if (!permissionFormRef.value) return

  await permissionFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      await setDataPermission({
        roleId: permissionForm.roleId!,
        resourceType: permissionForm.resourceType,
        scopeType: permissionForm.scopeType,
        customCondition: permissionForm.customCondition || undefined,
        allowedFields: permissionForm.allowedFields || undefined,
        hiddenFields: permissionForm.hiddenFields || undefined,
      })
      ElMessage.success('保存成功')
      permissionDialogVisible.value = false
      fetchPermissions()
    } catch (e: any) {
      ElMessage.error(e.message || '保存失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDeletePermission = async (row: DataPermissionRole) => {
  if (!row.roleId || !row.resourceType) return

  try {
    await ElMessageBox.confirm('确认删除该数据权限？', '提示', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning',
    })

    await deleteDataPermissionByRole(row.roleId, row.resourceType)
    ElMessage.success('删除成功')
    fetchPermissions()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchPermissions()
  fetchScopeTypes()
})
</script>

<style scoped>
.data-permission .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.data-permission .layout {
  display: flex;
  gap: 20px;
}

.data-permission .layout .role-panel {
  width: 240px;
  flex-shrink: 0;
}

.data-permission .layout .role-panel h3 {
  margin-bottom: 12px;
  font-size: 16px;
}

.data-permission .layout .role-panel .role-menu {
  border: 1px solid #ebeef5;
  border-radius: 4px;
}

.data-permission .layout .permission-panel {
  flex: 1;
}

.data-permission .layout .permission-panel .panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.data-permission .layout .permission-panel .panel-header h3 {
  font-size: 16px;
  margin: 0;
}
</style>
