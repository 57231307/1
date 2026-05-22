<template>
  <div class="system-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="用户管理" name="user">
        <div class="page-header">
          <h2 class="page-title">用户管理</h2>
          <el-button type="primary" @click="openUserDialog()">
            <el-icon><Plus /></el-icon>
            新建用户
          </el-button>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="userQuery">
            <el-form-item label="关键词">
              <el-input v-model="userQuery.keyword" placeholder="用户名/姓名/手机号" clearable @keyup.enter="fetchUsers" />
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="userQuery.status" placeholder="选择状态" clearable>
                <el-option label="启用" :value="1" />
                <el-option label="禁用" :value="0" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchUsers">查询</el-button>
              <el-button @click="resetUserQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover">
          <el-table :data="users" v-loading="userLoading" stripe>
            <el-table-column prop="username" label="用户名" width="120" />
            <el-table-column prop="real_name" label="姓名" width="100" />
            <el-table-column prop="phone" label="手机号" width="130" />
            <el-table-column prop="email" label="邮箱" min-width="180" />
            <el-table-column prop="department_name" label="部门" width="120" />
            <el-table-column label="角色" width="150">
              <template #default="{ row }">
                <template v-if="row.role_names?.length">
                  <el-tag v-for="r in row.role_names" :key="r" size="small" class="mr-1">{{ r }}</el-tag>
                </template>
                <span v-else class="text-gray">未分配</span>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
                  {{ row.status === 1 ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openUserDialog(row)">编辑</el-button>
                <el-button type="danger" link size="small" @click="deleteUser(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-pagination
            v-model:current-page="userQuery.page"
            v-model:page-size="userQuery.page_size"
            :total="userTotal"
            :page-sizes="[10, 20, 50]"
            layout="total, sizes, prev, pager, next"
            class="mt-4"
            @change="fetchUsers"
          />
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="角色管理" name="role">
        <div class="page-header">
          <h2 class="page-title">角色管理</h2>
          <el-button type="primary" @click="openRoleDialog()">
            <el-icon><Plus /></el-icon>
            新建角色
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="roles" v-loading="roleLoading" stripe>
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
                <el-button type="primary" link size="small" @click="openRoleDialog(row)">编辑</el-button>
                <el-button type="primary" link size="small" @click="openPermissionDialog(row)">权限</el-button>
                <el-button type="danger" link size="small" @click="deleteRole(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="部门管理" name="department">
        <div class="page-header">
          <h2 class="page-title">部门管理</h2>
          <el-button type="primary" @click="openDeptDialog()">
            <el-icon><Plus /></el-icon>
            新建部门
          </el-button>
        </div>

        <el-card shadow="hover">
          <el-table :data="departments" v-loading="deptLoading" stripe row-key="id" default-expand-all>
            <el-table-column prop="name" label="部门名称" min-width="200" />
            <el-table-column prop="code" label="部门编码" width="120" />
            <el-table-column prop="manager_name" label="负责人" width="100" />
            <el-table-column prop="sort_order" label="排序" width="80" align="center" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">
                  {{ row.status === 1 ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openDeptDialog(row)">编辑</el-button>
                <el-button type="danger" link size="small" @click="deleteDept(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="公司信息" name="company">
        <div class="page-header">
          <h2 class="page-title">公司信息设置</h2>
        </div>

        <el-card shadow="hover">
          <el-form
            ref="companyFormRef"
            :model="companyForm"
            :rules="companyRules"
            label-width="120px"
            style="max-width: 800px"
          >
            <el-divider content-position="left">基本信息</el-divider>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="公司名称" prop="company_name">
                  <el-input v-model="companyForm.company_name" placeholder="请输入公司名称" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="公司简称" prop="company_short_name">
                  <el-input v-model="companyForm.company_short_name" placeholder="请输入公司简称" />
                </el-form-item>
              </el-col>
            </el-row>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="统一社会信用代码" prop="credit_code">
                  <el-input v-model="companyForm.credit_code" placeholder="请输入统一社会信用代码" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="法定代表人" prop="legal_representative">
                  <el-input v-model="companyForm.legal_representative" placeholder="请输入法定代表人" />
                </el-form-item>
              </el-col>
            </el-row>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="注册资本(万元)" prop="registered_capital">
                  <el-input-number v-model="companyForm.registered_capital" :min="0" :precision="2" style="width: 100%" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="成立日期" prop="establishment_date">
                  <el-date-picker
                    v-model="companyForm.establishment_date"
                    type="date"
                    placeholder="选择日期"
                    style="width: 100%"
                    value-format="YYYY-MM-DD"
                  />
                </el-form-item>
              </el-col>
            </el-row>

            <el-divider content-position="left">联系方式</el-divider>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="联系电话" prop="phone">
                  <el-input v-model="companyForm.phone" placeholder="请输入联系电话" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="传真" prop="fax">
                  <el-input v-model="companyForm.fax" placeholder="请输入传真" />
                </el-form-item>
              </el-col>
            </el-row>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="电子邮箱" prop="email">
                  <el-input v-model="companyForm.email" placeholder="请输入电子邮箱" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="公司网站" prop="website">
                  <el-input v-model="companyForm.website" placeholder="请输入公司网站" />
                </el-form-item>
              </el-col>
            </el-row>
            <el-form-item label="公司地址" prop="address">
              <el-input v-model="companyForm.address" placeholder="请输入公司地址" />
            </el-form-item>

            <el-divider content-position="left">银行信息</el-divider>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="开户银行" prop="bank_name">
                  <el-input v-model="companyForm.bank_name" placeholder="请输入开户银行" />
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="银行账号" prop="bank_account">
                  <el-input v-model="companyForm.bank_account" placeholder="请输入银行账号" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-divider content-position="left">税务信息</el-divider>
            <el-row :gutter="20">
              <el-col :span="12">
                <el-form-item label="纳税人类型" prop="taxpayer_type">
                  <el-select v-model="companyForm.taxpayer_type" placeholder="请选择" style="width: 100%">
                    <el-option label="一般纳税人" value="general" />
                    <el-option label="小规模纳税人" value="small" />
                  </el-select>
                </el-form-item>
              </el-col>
              <el-col :span="12">
                <el-form-item label="税务登记号" prop="tax_registration_number">
                  <el-input v-model="companyForm.tax_registration_number" placeholder="请输入税务登记号" />
                </el-form-item>
              </el-col>
            </el-row>

            <el-divider content-position="left">其他信息</el-divider>
            <el-form-item label="公司Logo" prop="logo">
              <el-input v-model="companyForm.logo" placeholder="请输入Logo URL" />
            </el-form-item>
            <el-form-item label="备注" prop="remarks">
              <el-input v-model="companyForm.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
            </el-form-item>

            <el-form-item>
              <el-button type="primary" :loading="companySubmitLoading" @click="saveCompanyInfo">
                保存
              </el-button>
              <el-button @click="resetCompanyForm">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="userDialogVisible" :title="userForm.id ? '编辑用户' : '新建用户'" width="500px">
      <el-form ref="userFormRef" :model="userForm" :rules="userRules" label-width="80px">
        <el-form-item label="用户名" prop="username">
          <el-input v-model="userForm.username" :disabled="!!userForm.id" placeholder="请输入用户名" />
        </el-form-item>
        <el-form-item v-if="!userForm.id" label="密码" prop="password">
          <el-input v-model="userForm.password" type="password" placeholder="请输入密码" show-password />
        </el-form-item>
        <el-form-item label="姓名" prop="real_name">
          <el-input v-model="userForm.real_name" placeholder="请输入姓名" />
        </el-form-item>
        <el-form-item label="手机号" prop="phone">
          <el-input v-model="userForm.phone" placeholder="请输入手机号" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="userForm.email" placeholder="请输入邮箱" />
        </el-form-item>
        <el-form-item label="部门">
          <el-tree-select
            v-model="userForm.department_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            placeholder="选择部门"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="userForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="userDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="userSubmitLoading" @click="submitUser">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="roleDialogVisible" :title="roleForm.id ? '编辑角色' : '新建角色'" width="500px">
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleRules" label-width="80px">
        <el-form-item label="角色名称" prop="name">
          <el-input v-model="roleForm.name" placeholder="请输入角色名称" />
        </el-form-item>
        <el-form-item label="角色编码" prop="code">
          <el-input v-model="roleForm.code" :disabled="!!roleForm.id" placeholder="请输入角色编码" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="roleForm.description" type="textarea" placeholder="请输入描述" />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="roleForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="roleSubmitLoading" @click="submitRole">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="deptDialogVisible" :title="deptForm.id ? '编辑部门' : '新建部门'" width="500px">
      <el-form ref="deptFormRef" :model="deptForm" :rules="deptRules" label-width="80px">
        <el-form-item label="部门名称" prop="name">
          <el-input v-model="deptForm.name" placeholder="请输入部门名称" />
        </el-form-item>
        <el-form-item label="部门编码" prop="code">
          <el-input v-model="deptForm.code" placeholder="请输入部门编码" />
        </el-form-item>
        <el-form-item label="上级部门">
          <el-tree-select
            v-model="deptForm.parent_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            placeholder="选择上级部门"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="排序">
          <el-input-number v-model="deptForm.sort_order" :min="0" />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="deptForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deptDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="deptSubmitLoading" @click="submitDept">确定</el-button>
      </template>
    </el-dialog>

    <!-- 权限配置对话框 -->
    <el-dialog v-model="permissionDialogVisible" :title="`权限配置 - ${currentRoleName}`" width="600px">
      <el-tree
        v-loading="permissionLoading"
        :data="permissionTree"
        show-checkbox
        node-key="id"
        :default-checked-keys="checkedPermissions"
        :props="{ label: 'name', children: 'children' }"
        @check="handlePermissionCheck"
      />
      <template #footer>
        <el-button @click="permissionDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitPermissions">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { listUsers, createUser, updateUser, deleteUser as deleteUserApi, type User } from '@/api/user'
import { listRoles, createRole, updateRole, deleteRole as deleteRoleApi, getRolePermissions, assignPermission, listPermissions, type Role, type Permission } from '@/api/role'
import { createDepartment, updateDepartment, deleteDepartment as deleteDeptApi, getDepartmentTree, type Department } from '@/api/department'

const activeTab = ref('user')

const users = ref<User[]>([])
const roles = ref<Role[]>([])
const departments = ref<Department[]>([])
const userTotal = ref(0)
const userLoading = ref(false)
const roleLoading = ref(false)
const deptLoading = ref(false)

const userQuery = reactive({
  keyword: '',
  status: undefined as number | undefined,
  page: 1,
  page_size: 10
})

const fetchUsers = async () => {
  userLoading.value = true
  try {
    const res = await listUsers(userQuery)
    users.value = res.data?.list || []
    userTotal.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取用户列表失败')
  } finally {
    userLoading.value = false
  }
}

const resetUserQuery = () => {
  userQuery.keyword = ''
  userQuery.status = undefined
  userQuery.page = 1
  fetchUsers()
}

const fetchRoles = async () => {
  roleLoading.value = true
  try {
    const res = await listRoles()
    roles.value = res.data! || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取角色列表失败')
  } finally {
    roleLoading.value = false
  }
}

const fetchDepartments = async () => {
  deptLoading.value = true
  try {
    const res = await getDepartmentTree()
    departments.value = res.data! || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取部门列表失败')
  } finally {
    deptLoading.value = false
  }
}

const deptTreeData = computed(() => departments.value)

const userDialogVisible = ref(false)
const userFormRef = ref<FormInstance>()
const userSubmitLoading = ref(false)
const userForm = reactive({
  id: 0,
  username: '',
  password: '',
  real_name: '',
  phone: '',
  email: '',
  department_id: undefined as number | undefined,
  status: 1
})

const validateEmail = (_rule: any, value: string, callback: any) => {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
  if (value && !emailRegex.test(value)) {
    callback(new Error('请输入有效的邮箱地址'))
  } else {
    callback()
  }
}

const validatePhone = (_rule: any, value: string, callback: any) => {
  const phoneRegex = /^1[3-9]\d{9}$/
  if (value && !phoneRegex.test(value)) {
    callback(new Error('请输入有效的手机号'))
  } else {
    callback()
  }
}

const validatePassword = (_rule: any, value: string, callback: any) => {
  if (value && value.length < 8) {
    callback(new Error('密码长度至少 8 位'))
  } else if (value && !/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).+$/.test(value)) {
    callback(new Error('密码需包含大小写字母和数字'))
  } else {
    callback()
  }
}

const userRules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '用户名长度 3-20 位', trigger: 'blur' },
    { pattern: /^[a-zA-Z0-9_]+$/, message: '用户名只能包含字母、数字和下划线', trigger: 'blur' }
  ],
  password: [
    { required: true, validator: validatePassword, trigger: 'blur' }
  ],
  real_name: [
    { required: true, message: '请输入姓名', trigger: 'blur' },
    { max: 50, message: '姓名长度不超过 50 位', trigger: 'blur' }
  ],
  email: [
    { validator: validateEmail, trigger: 'blur' }
  ],
  phone: [
    { validator: validatePhone, trigger: 'blur' }
  ]
}

const openUserDialog = (row?: User) => {
  userFormRef.value?.resetFields()
  if (row) {
    userForm.id = row.id
    userForm.username = row.username
    userForm.real_name = row.real_name
    userForm.phone = row.phone || ''
    userForm.email = row.email || ''
    userForm.department_id = row.department_id
    userForm.status = row.status
  } else {
    userForm.id = 0
    userForm.username = ''
    userForm.password = ''
    userForm.real_name = ''
    userForm.phone = ''
    userForm.email = ''
    userForm.department_id = undefined
    userForm.status = 1
  }
  userDialogVisible.value = true
}

const submitUser = async () => {
  const valid = await userFormRef.value?.validate()
  if (!valid) return
  
  userSubmitLoading.value = true
  try {
    if (userForm.id) {
      await updateUser(userForm.id, {
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
        status: userForm.status
      })
      ElMessage.success('更新成功')
    } else {
      await createUser({
        username: userForm.username,
        password: userForm.password,
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id
      })
      ElMessage.success('创建成功')
    }
    userDialogVisible.value = false
    fetchUsers()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    userSubmitLoading.value = false
  }
}

const deleteUser = async (row: User) => {
  try {
    await ElMessageBox.confirm(`确定删除用户 "${row.username}" 吗？`, '删除确认', { type: 'warning' })
    await deleteUserApi(row.id)
    ElMessage.success('删除成功')
    fetchUsers()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const roleDialogVisible = ref(false)
const roleFormRef = ref<FormInstance>()
const roleSubmitLoading = ref(false)
const roleForm = reactive({
  id: 0,
  name: '',
  code: '',
  description: '',
  status: 1
})

const roleRules: FormRules = {
  name: [{ required: true, message: '请输入角色名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色编码', trigger: 'blur' }]
}

const openRoleDialog = (row?: Role) => {
  roleFormRef.value?.resetFields()
  if (row) {
    roleForm.id = row.id
    roleForm.name = row.name
    roleForm.code = row.code
    roleForm.description = row.description || ''
    roleForm.status = row.status
  } else {
    roleForm.id = 0
    roleForm.name = ''
    roleForm.code = ''
    roleForm.description = ''
    roleForm.status = 1
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
        status: roleForm.status
      })
      ElMessage.success('更新成功')
    } else {
      await createRole({
        name: roleForm.name,
        code: roleForm.code,
        description: roleForm.description
      })
      ElMessage.success('创建成功')
    }
    roleDialogVisible.value = false
    fetchRoles()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    roleSubmitLoading.value = false
  }
}

const deleteRole = async (row: Role) => {
  try {
    await ElMessageBox.confirm(`确定删除角色 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteRoleApi(row.id)
    ElMessage.success('删除成功')
    fetchRoles()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

const openPermissionDialog = (row: Role) => {
  currentRoleId.value = row.id
  currentRoleName.value = row.name
  // 获取角色权限
  fetchRolePermissions(row.id)
  permissionDialogVisible.value = true
}

const permissionDialogVisible = ref(false)
const currentRoleId = ref(0)
const currentRoleName = ref('')
const permissionTree = ref<any[]>([])
const checkedPermissions = ref<number[]>([])
const permissionLoading = ref(false)

const fetchRolePermissions = async (roleId: number) => {
  permissionLoading.value = true
  try {
    // 获取所有权限树
    const treeRes = await listPermissions()
    permissionTree.value = buildPermissionTree(treeRes.data || [])
    
    // 获取角色已有的权限
    const roleRes = await getRolePermissions(roleId)
    checkedPermissions.value = (roleRes.data || []).map((p: Permission) => p.id)
  } catch (error) {
    console.error('获取权限失败:', error)
  } finally {
    permissionLoading.value = false
  }
}

const buildPermissionTree = (permissions: Permission[]): any[] => {
  const map = new Map<number, any>()
  const tree: any[] = []
  
  permissions.forEach(p => {
    map.set(p.id, { ...p, children: [] })
  })
  
  permissions.forEach(p => {
    const node = map.get(p.id)!
    if (p.parent_id && map.has(p.parent_id)) {
      map.get(p.parent_id)!.children.push(node)
    } else {
      tree.push(node)
    }
  })
  
  return tree
}

const submitPermissions = async () => {
  try {
    await assignPermission(currentRoleId.value, { permission_ids: checkedPermissions.value })
    ElMessage.success('权限配置成功')
    permissionDialogVisible.value = false
  } catch (error: any) {
    ElMessage.error(error.message || '配置失败')
  }
}

const handlePermissionCheck = (_data: any, { checkedKeys }: any) => {
  checkedPermissions.value = checkedKeys
}

// 公司信息表单
const companyFormRef = ref<FormInstance>()
const companySubmitLoading = ref(false)
const companyForm = reactive({
  company_name: '',
  company_short_name: '',
  credit_code: '',
  legal_representative: '',
  registered_capital: 0,
  establishment_date: '',
  phone: '',
  fax: '',
  email: '',
  website: '',
  address: '',
  bank_name: '',
  bank_account: '',
  taxpayer_type: 'general',
  tax_registration_number: '',
  logo: '',
  remarks: ''
})

const companyRules: FormRules = {
  company_name: [
    { required: true, message: '请输入公司名称', trigger: 'blur' }
  ],
  credit_code: [
    { pattern: /^[0-9A-HJ-NPQRTUWXY]{2}\d{6}[0-9A-HJ-NPQRTUWXY]{10}$/, message: '请输入正确的统一社会信用代码', trigger: 'blur' }
  ],
  phone: [
    { pattern: /^1[3-9]\d{9}$/, message: '请输入正确的联系电话', trigger: 'blur' }
  ],
  email: [
    { type: 'email', message: '请输入正确的邮箱地址', trigger: 'blur' }
  ]
}

const fetchCompanyInfo = async () => {
  try {
    // 从本地存储或API获取公司信息
    const savedInfo = localStorage.getItem('company_info')
    if (savedInfo) {
      Object.assign(companyForm, JSON.parse(savedInfo))
    }
  } catch (error) {
    console.error('获取公司信息失败:', error)
  }
}

const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return
  
  await companyFormRef.value.validate(async (valid) => {
    if (!valid) return
    
    companySubmitLoading.value = true
    try {
      // 保存到本地存储（实际项目中应该调用API）
      localStorage.setItem('company_info', JSON.stringify(companyForm))
      ElMessage.success('保存成功')
    } catch (error: any) {
      ElMessage.error(error.message || '保存失败')
    } finally {
      companySubmitLoading.value = false
    }
  })
}

const resetCompanyForm = () => {
  companyFormRef.value?.resetFields()
}

const deptDialogVisible = ref(false)
const deptFormRef = ref<FormInstance>()
const deptSubmitLoading = ref(false)
const deptForm = reactive({
  id: 0,
  name: '',
  code: '',
  parent_id: undefined as number | undefined,
  sort_order: 0,
  status: 1
})

const deptRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入部门编码', trigger: 'blur' }]
}

const openDeptDialog = (row?: Department) => {
  deptFormRef.value?.resetFields()
  if (row) {
    deptForm.id = row.id
    deptForm.name = row.name
    deptForm.code = row.code
    deptForm.parent_id = row.parent_id
    deptForm.sort_order = row.sort_order
    deptForm.status = row.status
  } else {
    deptForm.id = 0
    deptForm.name = ''
    deptForm.code = ''
    deptForm.parent_id = undefined
    deptForm.sort_order = 0
    deptForm.status = 1
  }
  deptDialogVisible.value = true
}

const submitDept = async () => {
  const valid = await deptFormRef.value?.validate()
  if (!valid) return
  
  deptSubmitLoading.value = true
  try {
    if (deptForm.id) {
      await updateDepartment(deptForm.id, {
        name: deptForm.name,
        sort_order: deptForm.sort_order,
        status: deptForm.status
      })
      ElMessage.success('更新成功')
    } else {
      await createDepartment({
        name: deptForm.name,
        code: deptForm.code,
        parent_id: deptForm.parent_id,
        sort_order: deptForm.sort_order
      })
      ElMessage.success('创建成功')
    }
    deptDialogVisible.value = false
    fetchDepartments()
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    deptSubmitLoading.value = false
  }
}

const deleteDept = async (row: Department) => {
  try {
    await ElMessageBox.confirm(`确定删除部门 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    await deleteDeptApi(row.id)
    ElMessage.success('删除成功')
    fetchDepartments()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchUsers()
  fetchRoles()
  fetchDepartments()
  fetchCompanyInfo()
})
</script>

<style scoped>
.system-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.filter-card { margin-bottom: 20px; }
.mr-1 { margin-right: 4px; }
.mt-4 { margin-top: 16px; }
.text-gray { color: #909399; }
</style>
