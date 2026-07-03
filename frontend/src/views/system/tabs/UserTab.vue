<!--
  UserTab.vue - 用户管理 Tab
  来源：原 system/index.vue 第 10-84 行（template）+ 732-869 行（script）
  拆分日期：2026-06-05
  说明：本文件由 system/index.vue 拆分而来，逻辑完整可独立运行
-->
<template>
  <div class="user-tab">
    <div class="page-header">
      <h2 class="page-title">用户管理</h2>
      <el-button type="primary" @click="openUserDialog()">
        <el-icon><Plus /></el-icon> 新建用户
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="userQuery">
        <el-form-item label="关键词">
          <el-input
            v-model="userQuery.keyword"
            placeholder="用户名/姓名/手机号"
            clearable
            @keyup.enter="fetchUsers"
          />
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
      <el-table v-loading="userLoading" :data="users" stripe>
        <el-table-column prop="username" label="用户名" width="120" />
        <el-table-column prop="real_name" label="姓名" width="100" />
        <el-table-column prop="phone" label="手机号" width="130" />
        <el-table-column prop="email" label="邮箱" min-width="180" />
        <el-table-column prop="department_name" label="部门" width="120" />
        <el-table-column label="角色" width="150">
          <template #default="{ row }">
            <template v-if="row.role_names?.length">
              <el-tag v-for="r in row.role_names" :key="r" size="small" class="mr-1">{{
                r
              }}</el-tag>
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
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'user:update'" size="small" link @click="openUserDialog(row as any)">编辑</el-button>
            <el-button v-permission="'user:delete'" size="small" link type="danger" @click="deleteUser(row as any)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="userQuery.page"
        v-model:page-size="userQuery.page_size"
        :total="userTotal"
        layout="total, sizes, prev, pager, next, jumper"
        style="margin-top: 16px; justify-content: flex-end"
        @current-change="fetchUsers"
        @size-change="fetchUsers"
      />
    </el-card>

    <!-- 用户编辑对话框 -->
    <el-dialog
      v-model="userDialogVisible"
      :title="userForm.id ? '编辑用户' : '新建用户'"
      width="600px"
    >
      <el-form ref="userFormRef" :model="userForm" :rules="userRules" label-width="100px">
        <el-form-item label="用户名" prop="username">
          <el-input v-model="userForm.username" :disabled="!!userForm.id" />
        </el-form-item>
        <el-form-item v-if="!userForm.id" label="密码" prop="password">
          <el-input v-model="userForm.password" type="password" show-password />
        </el-form-item>
        <el-form-item label="姓名" prop="real_name">
          <el-input v-model="userForm.real_name" />
        </el-form-item>
        <el-form-item label="手机号" prop="phone">
          <el-input v-model="userForm.phone" />
        </el-form-item>
        <el-form-item label="邮箱" prop="email">
          <el-input v-model="userForm.email" />
        </el-form-item>
        <el-form-item v-if="userForm.id" label="状态">
          <el-switch v-model="userForm.status" :active-value="1" :inactive-value="0" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="userDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="userSubmitLoading" @click="submitUser">确定</el-button>
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
  listUsers,
  createUser,
  updateUser,
  deleteUser as deleteUserApi,
  type User,
} from '@/api/user'

// 批次 32 v7 P0-2：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const users = ref<User[]>([])
const userTotal = ref(0)
const userLoading = ref(false)
const userQuery = reactive({
  keyword: '',
  status: undefined as number | undefined,
  page: 1,
  page_size: 10,
})

const fetchUsers = async () => {
  userLoading.value = true
  try {
    const res = await listUsers(userQuery)
    users.value = res.data?.list || []
    userTotal.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取用户列表失败')
  } finally {
    userLoading.value = false
  }
}

defineExpose({ refresh: fetchUsers })

const resetUserQuery = () => {
  userQuery.keyword = ''
  userQuery.status = undefined
  userQuery.page = 1
  fetchUsers()
}

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
  status: 1,
})

const validateEmail = (_: any, v: string, cb: any) => {
  v && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v) ? cb(new Error('邮箱格式错误')) : cb()
}
const validatePhone = (_: any, v: string, cb: any) => {
  v && !/^1[3-9]\d{9}$/.test(v) ? cb(new Error('手机号格式错误')) : cb()
}
const validatePassword = (_: any, v: string, cb: any) => {
  if (userForm.id && !v) {
    cb()
    return
  }
  v && v.length < 8
    ? cb(new Error('密码至少8位'))
    : v && !/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).+$/.test(v)
      ? cb(new Error('需含大小写字母和数字'))
      : cb()
}

const userRules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '3-20位', trigger: 'blur' },
  ],
  password: [{ required: true, validator: validatePassword, trigger: 'blur' }],
  real_name: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  email: [{ validator: validateEmail, trigger: 'blur' }],
  phone: [{ validator: validatePhone, trigger: 'blur' }],
}

const openUserDialog = (row?: User) => {
  userFormRef.value?.resetFields()
  if (row) {
    Object.assign(userForm, {
      id: row.id,
      username: row.username,
      real_name: row.real_name,
      phone: row.phone || '',
      email: row.email || '',
      department_id: row.department_id,
      status: row.status,
    })
  } else {
    Object.assign(userForm, {
      id: 0,
      username: '',
      password: '',
      real_name: '',
      phone: '',
      email: '',
      department_id: undefined,
      status: 1,
    })
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
        status: userForm.status,
      })
      ElMessage.success(t('system.user.updateSuccess'))
    } else {
      await createUser({
        username: userForm.username,
        password: userForm.password,
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
      })
      ElMessage.success(t('system.user.createSuccess'))
    }
    userDialogVisible.value = false
    fetchUsers()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    userSubmitLoading.value = false
  }
}

const deleteUser = async (row: User) => {
  try {
    await ElMessageBox.confirm(`确定删除用户 "${row.username}"?`, '删除确认', { type: 'warning' })
    await deleteUserApi(row.id)
    ElMessage.success(t('system.user.deleteSuccess'))
    fetchUsers()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

onMounted(() => {
  fetchUsers()
})
</script>
