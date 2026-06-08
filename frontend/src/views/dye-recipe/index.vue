<template>
  <div class="dye-recipe-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">染色配方管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>面料行业</el-breadcrumb-item>
          <el-breadcrumb-item>染色配方</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建配方
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="配方名称/配方编号"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="色号">
          <el-input
            v-model="queryParams.color_no"
            placeholder="请输入色号"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="草稿" value="DRAFT" />
            <el-option label="待审批" value="PENDING" />
            <el-option label="已审批" value="APPROVED" />
            <el-option label="已停用" value="INACTIVE" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="recipeList" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="recipe_no" label="配方编号" width="120" show-overflow-tooltip />
        <el-table-column
          prop="recipe_name"
          label="配方名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="color_no" label="色号" width="100" show-overflow-tooltip />
        <el-table-column prop="color_name" label="颜色名称" width="120" show-overflow-tooltip />
        <el-table-column prop="version" label="版本" width="80" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180" align="center" />
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row as any)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as any)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              type="success"
              link
              size="small"
              @click="handleSubmit(row as any)"
              >提交</el-button
            >
            <el-button
              v-if="row.status === 'PENDING'"
              type="success"
              link
              size="small"
              @click="handleApprove(row as any)"
              >审批</el-button
            >
            <el-button type="info" link size="small" @click="handleVersion(row as any)"
              >版本</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="800px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="配方编号" prop="recipe_no">
              <el-input v-model="formData.recipe_no" placeholder="请输入配方编号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="配方名称" prop="recipe_name">
              <el-input v-model="formData.recipe_name" placeholder="请输入配方名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="色号" prop="color_no">
              <el-input v-model="formData.color_no" placeholder="请输入色号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="颜色名称" prop="color_name">
              <el-input v-model="formData.color_name" placeholder="请输入颜色名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="配方内容" prop="content">
          <el-input
            v-model="formData.content"
            type="textarea"
            :rows="10"
            placeholder="请输入配方内容"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 版本历史对话框 -->
    <el-dialog v-model="versionVisible" title="版本历史" width="800px">
      <el-table :data="versionList" border stripe>
        <el-table-column prop="version" label="版本" width="80" align="center" />
        <el-table-column
          prop="recipe_name"
          label="配方名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180" align="center" />
        <el-table-column label="操作" width="100" align="center">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleViewVersion(row as any)"
              >查看</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import {
  listDyeRecipes,
  createDyeRecipe,
  updateDyeRecipe,
  approveDyeRecipe,
  submitDyeRecipe,
  getRecipeVersions,
  exportDyeRecipes,
} from '@/api/dye-recipe'
import type { DyeRecipe } from '@/api/dye-recipe'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  color_no: '',
  status: '',
})

// 列表数据
const loading = ref(false)
const recipeList = ref<DyeRecipe[]>([])
const total = ref(0)

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 版本历史
const versionVisible = ref(false)
const versionList = ref<DyeRecipe[]>([])

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  recipe_no: '',
  recipe_name: '',
  color_no: '',
  color_name: '',
  content: '',
  remarks: '',
})

// 表单验证规则
const formRules = {
  recipe_no: [{ required: true, message: '请输入配方编号', trigger: 'blur' }],
  recipe_name: [{ required: true, message: '请输入配方名称', trigger: 'blur' }],
  color_no: [{ required: true, message: '请输入色号', trigger: 'blur' }],
  color_name: [{ required: true, message: '请输入颜色名称', trigger: 'blur' }],
  content: [{ required: true, message: '请输入配方内容', trigger: 'blur' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    const res = await listDyeRecipes(queryParams)
    recipeList.value = res.data || []
    total.value = res.total || 0
  } catch (error) {
    console.error('获取染色配方列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 查询
const handleQuery = () => {
  queryParams.page = 1
  getList()
}

// 重置
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.color_no = ''
  queryParams.status = ''
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建染色配方'
  Object.assign(formData, {
    id: undefined,
    recipe_no: '',
    recipe_name: '',
    color_no: '',
    color_name: '',
    content: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看
const handleView = (_row: any) => {}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑染色配方'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 提交审批
const handleSubmit = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认提交该配方审批？', '提示', { type: 'warning' })
    await submitDyeRecipe(row.id)
    ElMessage.success('提交成功')
    getList()
  } catch (error) {
    console.error('提交失败:', error)
  }
}

// 审批
const handleApprove = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认审批通过该配方？', '提示', { type: 'warning' })
    await approveDyeRecipe(row.id)
    ElMessage.success('审批成功')
    getList()
  } catch (error) {
    console.error('审批失败:', error)
  }
}

// 版本历史
const handleVersion = async (row: any) => {
  try {
    const res = await getRecipeVersions(row.id)
    versionList.value = res.data || []
    versionVisible.value = true
  } catch (error) {
    console.error('获取版本历史失败:', error)
  }
}

// 查看版本
const handleViewVersion = (_row: any) => {}

// 导出
const handleExport = async () => {
  try {
    const res = await exportDyeRecipes(queryParams)
    const url = window.URL.createObjectURL(new Blob([res]))
    const link = document.createElement('a')
    link.href = url
    link.setAttribute('download', '染色配方.xlsx')
    document.body.appendChild(link)
    link.click()
    document.body.removeChild(link)
    window.URL.revokeObjectURL(url)
    ElMessage.success('导出成功')
  } catch (error) {
    console.error('导出失败:', error)
  }
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    if (formData.id) {
      await updateDyeRecipe(formData.id, formData)
    } else {
      await createDyeRecipe(formData)
    }
    ElMessage.success('保存成功')
    dialogVisible.value = false
    getList()
  } catch (error) {
    console.error('表单验证失败:', error)
  }
}

// 分页
const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getList()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getList()
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: 'info',
    PENDING: 'warning',
    APPROVED: 'success',
    INACTIVE: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: '草稿',
    PENDING: '待审批',
    APPROVED: '已审批',
    INACTIVE: '已停用',
  }
  return map[status] || status
}

onMounted(() => {
  getList()
})
</script>

<style scoped>
.dye-recipe-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
