<template>
  <div class="system-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane label="用户管理" name="user">
        <div class="page-header">
          <div class="header-left">
            <h2 class="page-title">用户管理</h2>
          </div>
          <div class="header-actions">
            <el-button type="primary" @click="handleCreateUser">
              <el-icon><Plus /></el-icon>
              新建用户
            </el-button>
          </div>
        </div>

        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="userQuery" class="filter-form">
            <el-form-item label="关键词">
              <el-input v-model="userQuery.keyword" placeholder="用户名/姓名/手机号" clearable />
            </el-form-item>
            <el-form-item label="角色">
              <el-select v-model="userQuery.role_id" placeholder="选择角色" clearable>
                <el-option v-for="r in roles" :key="r.id" :label="r.name" :value="r.id" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchUsers">查询</el-button>
              <el-button @click="resetUserQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>

        <el-card shadow="hover" class="table-card">
          <el-table :data="users" stripe>
            <el-table-column prop="username" label="用户名" width="150" />
            <el-table-column prop="real_name" label="姓名" width="120" />
            <el-table-column prop="phone" label="手机号" width="130" />
            <el-table-column prop="email" label="邮箱" width="180" />
            <el-table-column prop="role_name" label="角色" width="120">
              <template #default="{ row }">
                <el-tag size="small">{{ row.role_name || '未分配' }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="dept_name" label="部门" width="120" />
            <el-table-column prop="is_active" label="状态" width="80">
              <template #default="{ row }">
                <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
                  {{ row.is_active ? '启用' : '禁用' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleEditUser(row)">编辑</el-button>
                <el-button type="danger" link size="small" @click="handleDeleteUser(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="角色管理" name="role">
        <div class="page-header">
          <div class="header-left">
            <h2 class="page-title">角色管理</h2>
          </div>
          <div class="header-actions">
            <el-button type="primary" @click="handleCreateRole">
              <el-icon><Plus /></el-icon>
              新建角色
            </el-button>
          </div>
        </div>

        <el-card shadow="hover" class="table-card">
          <el-table :data="roles" stripe>
            <el-table-column prop="name" label="角色名称" width="150" />
            <el-table-column prop="code" label="角色编码" width="150" />
            <el-table-column prop="description" label="描述" min-width="200" />
            <el-table-column prop="user_count" label="用户数" width="80" align="center" />
            <el-table-column label="操作" width="200">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleEditRole(row)">编辑</el-button>
                <el-button type="primary" link size="small" @click="handleAssignPermissions(row)">权限</el-button>
                <el-button type="danger" link size="small" @click="handleDeleteRole(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="部门管理" name="department">
        <div class="page-header">
          <div class="header-left">
            <h2 class="page-title">部门管理</h2>
          </div>
          <div class="header-actions">
            <el-button type="primary" @click="handleCreateDept">
              <el-icon><Plus /></el-icon>
              新建部门
            </el-button>
          </div>
        </div>

        <el-card shadow="hover" class="table-card">
          <el-table :data="departments" stripe row-key="id" default-expand-all>
            <el-table-column prop="name" label="部门名称" width="200" />
            <el-table-column prop="code" label="部门编码" width="150" />
            <el-table-column prop="manager_name" label="负责人" width="120" />
            <el-table-column prop="parent_name" label="上级部门" width="150" />
            <el-table-column prop="sort_order" label="排序" width="80" align="center" />
            <el-table-column label="操作" width="180">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleEditDept(row)">编辑</el-button>
                <el-button type="danger" link size="small" @click="handleDeleteDept(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'

const activeTab = ref('user')

const users = ref<any[]>([])
const roles = ref<any[]>([])
const departments = ref<any[]>([])

const userQuery = reactive({
  keyword: '',
  role_id: undefined as number | undefined
})

const fetchUsers = () => {
  users.value = [
    { id: 1, username: 'admin', real_name: '系统管理员', phone: '13800000000', email: 'admin@example.com', role_name: '超级管理员', dept_name: '信息中心', is_active: true },
    { id: 2, username: 'zhangsan', real_name: '张三', phone: '13800000001', email: 'zhangsan@example.com', role_name: '销售经理', dept_name: '销售部', is_active: true },
    { id: 3, username: 'lisi', real_name: '李四', phone: '13800000002', email: 'lisi@example.com', role_name: '采购员', dept_name: '采购部', is_active: true }
  ]
  ElMessage.info('使用演示数据')
}

const resetUserQuery = () => {
  userQuery.keyword = ''
  userQuery.role_id = undefined
  fetchUsers()
}

const fetchRoles = () => {
  roles.value = [
    { id: 1, name: '超级管理员', code: 'super_admin', description: '系统最高权限', user_count: 1 },
    { id: 2, name: '销售经理', code: 'sales_manager', description: '销售部门管理', user_count: 5 },
    { id: 3, name: '采购员', code: 'purchaser', description: '采购操作人员', user_count: 8 }
  ]
}

const fetchDepartments = () => {
  departments.value = [
    { id: 1, name: '信息中心', code: 'IT', manager_name: '系统管理员', parent_name: '-', sort_order: 1 },
    { id: 2, name: '销售部', code: 'SALES', manager_name: '张经理', parent_name: '-', sort_order: 2 },
    { id: 3, name: '采购部', code: 'PURCHASE', manager_name: '李经理', parent_name: '-', sort_order: 3 },
    { id: 4, name: '华东销售组', code: 'SALES_EAST', manager_name: '王经理', parent_name: '销售部', sort_order: 1 },
    { id: 5, name: '华南销售组', code: 'SALES_SOUTH', manager_name: '赵经理', parent_name: '销售部', sort_order: 2 }
  ]
}

const handleCreateUser = () => { ElMessage.info('新建用户功能开发中') }
const handleEditUser = (row: any) => { ElMessage.info(`编辑用户 ${row.username}`) }
const handleDeleteUser = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除用户 "${row.username}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
  } catch {}
}

const handleCreateRole = () => { ElMessage.info('新建角色功能开发中') }
const handleEditRole = (row: any) => { ElMessage.info(`编辑角色 ${row.name}`) }
const handleAssignPermissions = (row: any) => { ElMessage.info(`分配权限 ${row.name}`) }
const handleDeleteRole = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除角色 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
  } catch {}
}

const handleCreateDept = () => { ElMessage.info('新建部门功能开发中') }
const handleEditDept = (row: any) => { ElMessage.info(`编辑部门 ${row.name}`) }
const handleDeleteDept = async (row: any) => {
  try {
    await ElMessageBox.confirm(`确定删除部门 "${row.name}" 吗？`, '删除确认', { type: 'warning' })
    ElMessage.success('删除成功')
  } catch {}
}

onMounted(() => {
  fetchUsers()
  fetchRoles()
  fetchDepartments()
})
</script>

<style scoped>
.system-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; }
.page-title { font-size: 20px; font-weight: 600; color: #303133; margin: 0; }
.header-actions { display: flex; gap: 12px; }
.filter-card { margin-bottom: 20px; }
.table-card { margin-bottom: 20px; }
:deep(.el-tabs__header) { margin-bottom: 20px; }
</style>
