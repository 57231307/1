<template>
  <div class="departments-page">
    <div class="header">
      <h2>部门管理</h2>
      <el-button type="primary" @click="handleCreate">新建部门</el-button>
    </div>

    <el-table v-loading="loading" :data="departmentList" border>
      <el-table-column prop="name" label="部门名称" />
      <el-table-column prop="code" label="部门编码" />
      <el-table-column label="上级部门">
        <template #default="{ row }">
          {{ getParentName(row.parent_id) }}
        </template>
      </el-table-column>
      <el-table-column prop="manager_name" label="负责人" />
      <el-table-column prop="sort_order" label="排序" width="80" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="row.status === 1 ? 'success' : 'danger'">
            {{ row.status === 1 ? '启用' : '禁用' }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="200">
        <template #default="{ row }">
          <el-button v-permission="'department:update'" size="small" @click="handleEdit(row as any)">编辑</el-button>
          <el-button v-permission="'department:delete'" size="small" type="danger" @click="handleDelete(row as any)">删除</el-button>
        </template>
      </el-table-column>
    </el-table>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogMode === 'create' ? '新建部门' : '编辑部门'"
      width="600px"
      @close="handleDialogClose"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="部门名称" prop="name">
          <el-input v-model="formData.name" placeholder="请输入部门名称" />
        </el-form-item>
        <el-form-item label="部门编码" prop="code">
          <el-input v-model="formData.code" placeholder="请输入部门编码" />
        </el-form-item>
        <el-form-item label="上级部门" prop="parent_id">
          <el-tree-select
            v-model="formData.parent_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            placeholder="请选择上级部门"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="负责人" prop="manager_name">
          <el-input v-model="formData.manager_name" placeholder="请输入负责人" />
        </el-form-item>
        <el-form-item label="排序" prop="sort_order">
          <el-input-number v-model="formData.sort_order" :min="0" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="formData.status" placeholder="请选择状态">
            <el-option label="启用" :value="1" />
            <el-option label="禁用" :value="0" />
          </el-select>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit"> 确定 </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listDepartments,
  createDepartment,
  updateDepartment,
  deleteDepartment,
  getDepartmentTree,
} from '@/api/department'

const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
const departmentList = ref<any[]>([])
const deptTreeData = ref<any[]>([])

const formData = reactive<any>({
  name: '',
  code: '',
  parent_id: null,
  manager_name: '',
  sort_order: 0,
  status: 1,
})

const formRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入部门编码', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const loadDepartments = async () => {
  loading.value = true
  try {
    const [listRes, treeRes] = await Promise.all([listDepartments(), getDepartmentTree()])
    departmentList.value = listRes.data || []
    deptTreeData.value = treeRes.data || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '加载部门列表失败')
  } finally {
    loading.value = false
  }
}

const getParentName = (parentId: number | null): string => {
  if (!parentId) return '-'
  const dept = departmentList.value.find((d: any) => d.id === parentId)
  return dept?.name || '-'
}

const handleCreate = () => {
  dialogMode.value = 'create'
  Object.assign(formData, {
    id: null,
    name: '',
    code: '',
    parent_id: null,
    manager_name: '',
    sort_order: 0,
    status: 1,
  })
  dialogVisible.value = true
}

const handleEdit = (row: any) => {
  dialogMode.value = 'edit'
  Object.assign(formData, {
    id: row.id,
    name: row.name,
    code: row.code,
    parent_id: row.parent_id,
    manager_name: row.manager_name,
    sort_order: row.sort_order || 0,
    status: row.status,
  })
  dialogVisible.value = true
}

const handleDelete = async (row: any) => {
  if (!row.id) return

  try {
    await ElMessageBox.confirm(`确定删除部门 "${row.name}"?`, '删除确认', { type: 'warning' })
    await deleteDepartment(row.id)
    ElMessage.success('删除成功')
    await loadDepartments()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') {
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
    }
  }
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (dialogMode.value === 'create') {
        await createDepartment(formData)
        ElMessage.success('创建成功')
      } else {
        await updateDepartment(formData.id, formData)
        ElMessage.success('更新成功')
      }
      dialogVisible.value = false
      await loadDepartments()
    } catch (error) {
      ElMessage.error(dialogMode.value === 'create' ? '创建失败' : '更新失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDialogClose = () => {
  formRef.value?.resetFields()
}

onMounted(() => {
  loadDepartments()
})
</script>

<style scoped>
.departments-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
</style>
