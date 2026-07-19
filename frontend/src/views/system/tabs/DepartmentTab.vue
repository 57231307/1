<!--
  DepartmentTab.vue - 部门管理 Tab
  来源：原 system/index.vue 中 部门管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="department-tab">
    <div class="page-header">
      <h2 class="page-title">部门管理</h2>
      <el-button type="primary" @click="openDeptDialog()">
        <el-icon><Plus /></el-icon> 新建部门
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="deptLoading" :data="departments" stripe row-key="id" default-expand-all aria-label="部门列表">
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
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'department:update'" size="small" link @click="openDeptDialog(row as Department)">编辑</el-button>
            <el-button v-permission="'department:delete'" size="small" link type="danger" @click="deleteDept(row as Department)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="deptDialogVisible"
      :title="deptForm.id ? '编辑部门' : '新建部门'"
      width="500px"
      aria-label="部门编辑对话框"
    >
      <el-form ref="deptFormRef" :model="deptForm" :rules="deptRules" label-width="80px" aria-label="部门信息表单">
        <el-form-item label="部门名称" prop="name">
          <el-input v-model="deptForm.name" />
        </el-form-item>
        <el-form-item label="部门编码" prop="code">
          <el-input v-model="deptForm.code" />
        </el-form-item>
        <el-form-item label="上级部门">
          <el-tree-select
            v-model="deptForm.parent_id"
            :data="departments"
            :props="{ label: 'name', value: 'id' }"
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  createDepartment,
  updateDepartment,
  deleteDepartment as deleteDeptApi,
  getDepartmentTree,
  type Department,
} from '@/api/department'

const departments = ref<Department[]>([])
const deptLoading = ref(false)

const fetchDepartments = async () => {
  deptLoading.value = true
  try {
    const res = await getDepartmentTree()
    const d = res.data as { items?: Department[]; data?: Department[] } | Department[]
    departments.value =
      (d as { items?: Department[] })?.items ||
      (d as { data?: Department[] })?.data ||
      (d as Department[]) ||
      []
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取部门列表失败')
  } finally {
    deptLoading.value = false
  }
}

defineExpose({ refresh: fetchDepartments })

const deptDialogVisible = ref(false)
const deptFormRef = ref<FormInstance>()
const deptSubmitLoading = ref(false)
const deptForm = reactive({
  id: 0,
  name: '',
  code: '',
  parent_id: undefined as number | undefined,
  sort_order: 0,
  status: 1,
})

const deptRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入部门编码', trigger: 'blur' }],
}

const openDeptDialog = (row?: Department) => {
  deptFormRef.value?.resetFields()
  if (row) {
    Object.assign(deptForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      parent_id: row.parent_id,
      sort_order: row.sort_order,
      status: row.status,
    })
  } else {
    Object.assign(deptForm, {
      id: 0,
      name: '',
      code: '',
      parent_id: undefined,
      sort_order: 0,
      status: 1,
    })
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
        status: deptForm.status,
      })
      ElMessage.success('更新成功')
    } else {
      await createDepartment({
        name: deptForm.name,
        code: deptForm.code,
        parent_id: deptForm.parent_id,
        sort_order: deptForm.sort_order,
      })
      ElMessage.success('创建成功')
    }
    deptDialogVisible.value = false
    fetchDepartments()
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    deptSubmitLoading.value = false
  }
}

const deleteDept = async (row: Department) => {
  try {
    await ElMessageBox.confirm(`确定删除部门 "${row.name}"?`, '删除确认', { type: 'warning' })
    await deleteDeptApi(row.id)
    ElMessage.success('删除成功')
    fetchDepartments()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchDepartments()
})
</script>
