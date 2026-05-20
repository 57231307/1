<template>
  <div class="bpm-definitions-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">流程定义管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>审批管理</el-breadcrumb-item>
          <el-breadcrumb-item>流程定义</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建流程定义
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="filterForm" class="filter-form">
        <el-form-item label="流程分类">
          <el-select v-model="filterForm.category" placeholder="全部分类" clearable style="width: 160px">
            <el-option label="销售流程" value="sales" />
            <el-option label="采购流程" value="purchase" />
            <el-option label="财务流程" value="finance" />
            <el-option label="人事流程" value="hr" />
            <el-option label="生产流程" value="production" />
          </el-select>
        </el-form-item>
        <el-form-item label="关键词">
          <el-input v-model="filterForm.keyword" placeholder="搜索流程名称" clearable style="width: 200px" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleResetFilter">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table :data="definitions" stripe v-loading="loading">
        <el-table-column prop="process_key" label="流程标识" width="150" />
        <el-table-column prop="process_name" label="流程名称" min-width="180" />
        <el-table-column prop="category" label="分类" width="120">
          <template #default="{ row }">
            <el-tag size="small">{{ getCategoryText(row.category) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="version" label="版本" width="80" align="center" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="200" show-overflow-tooltip />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button type="success" link size="small" @click="handleVersions(row)">版本</el-button>
            <el-button type="warning" link size="small" @click="handleSaveTemplate(row)">保存为模板</el-button>
            <el-button type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="pagination.page"
          v-model:page-size="pagination.page_size"
          :total="pagination.total"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          @size-change="fetchData"
          @current-change="fetchData"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="700px" destroy-on-close>
      <el-form :model="formData" :rules="formRules" ref="formRef" label-width="100px">
        <el-form-item label="流程标识" prop="process_key">
          <el-input v-model="formData.process_key" placeholder="例如: sales_order_approval" :disabled="isEdit" />
        </el-form-item>
        <el-form-item label="流程名称" prop="process_name">
          <el-input v-model="formData.process_name" placeholder="请输入流程名称" />
        </el-form-item>
        <el-form-item label="流程分类" prop="category">
          <el-select v-model="formData.category" placeholder="请选择分类" style="width: 100%">
            <el-option label="销售流程" value="sales" />
            <el-option label="采购流程" value="purchase" />
            <el-option label="财务流程" value="finance" />
            <el-option label="人事流程" value="hr" />
            <el-option label="生产流程" value="production" />
          </el-select>
        </el-form-item>
        <el-form-item label="流程描述">
          <el-input v-model="formData.description" type="textarea" :rows="3" placeholder="请输入流程描述" />
        </el-form-item>
        <el-form-item label="节点配置">
          <el-button type="primary" plain size="small" @click="handleAddNode">
            <el-icon><Plus /></el-icon>
            添加节点
          </el-button>
          <el-table :data="formData.nodes || []" style="margin-top: 12px" size="small">
            <el-table-column label="节点类型" width="120">
              <template #default="{ row }">
                <el-select v-model="row.type" size="small">
                  <el-option label="开始" value="start" />
                  <el-option label="审批" value="approval" />
                  <el-option label="条件" value="condition" />
                  <el-option label="通知" value="notify" />
                  <el-option label="结束" value="end" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="节点名称" min-width="150">
              <template #default="{ row }">
                <el-input v-model="row.name" size="small" placeholder="节点名称" />
              </template>
            </el-table-column>
            <el-table-column label="审批人类型" width="130">
              <template #default="{ row }">
                <el-select v-model="row.assignee_type" size="small" :disabled="row.type !== 'approval'">
                  <el-option label="指定用户" value="user" />
                  <el-option label="角色" value="role" />
                  <el-option label="部门" value="department" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="审批人值" min-width="150">
              <template #default="{ row }">
                <el-input v-model="row.assignee_value" size="small" :disabled="row.type !== 'approval'" placeholder="用户ID/角色/部门" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ $index }">
                <el-button type="danger" link size="small" @click="handleRemoveNode($index)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="versionDialogVisible" title="版本管理" width="800px" destroy-on-close>
      <div class="version-header">
        <span>当前流程：<strong>{{ currentDefinition?.process_name }}</strong></span>
        <el-button type="primary" size="small" @click="handleCreateVersion">
          <el-icon><Plus /></el-icon>
          创建新版本
        </el-button>
      </div>
      <el-table :data="versions" stripe style="margin-top: 16px">
        <el-table-column prop="version" label="版本号" width="100" align="center" />
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : row.status === 'draft' ? 'info' : 'warning'" size="small">
              {{ getVersionStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="change_log" label="变更说明" min-width="200" show-overflow-tooltip />
        <el-table-column prop="created_by" label="创建人" width="120" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="150">
          <template #default="{ row }">
            <el-button v-if="row.status !== 'active'" type="success" link size="small" @click="handleActivateVersion(row)">
              激活
            </el-button>
            <span v-else class="active-tag">当前激活</span>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>

    <el-dialog v-model="templateDialogVisible" title="保存为模板" width="500px" destroy-on-close>
      <el-form :model="templateForm" :rules="templateRules" ref="templateFormRef" label-width="100px">
        <el-form-item label="模板名称" prop="template_name">
          <el-input v-model="templateForm.template_name" placeholder="请输入模板名称" />
        </el-form-item>
        <el-form-item label="模板分类" prop="category">
          <el-select v-model="templateForm.category" placeholder="请选择分类" style="width: 100%">
            <el-option label="销售模板" value="sales" />
            <el-option label="采购模板" value="purchase" />
            <el-option label="财务模板" value="finance" />
            <el-option label="人事模板" value="hr" />
            <el-option label="生产模板" value="production" />
            <el-option label="通用模板" value="common" />
          </el-select>
        </el-form-item>
        <el-form-item label="模板描述">
          <el-input v-model="templateForm.description" type="textarea" :rows="3" placeholder="请输入模板描述" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="templateDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleConfirmSaveTemplate" :loading="submitLoading">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { bpmEnhancedApi } from '@/api/bpm-enhanced'
import type { ProcessDefinition, ProcessVersion, ProcessNode } from '@/api/bpm-enhanced'

const loading = ref(false)
const submitLoading = ref(false)
const definitions = ref<ProcessDefinition[]>([])
const versions = ref<ProcessVersion[]>([])
const currentDefinition = ref<ProcessDefinition | null>(null)

const filterForm = reactive({
  category: '',
  keyword: ''
})

const pagination = reactive({
  page: 1,
  page_size: 10,
  total: 0
})

const dialogVisible = ref(false)
const isEdit = ref(false)
const formRef = ref<FormInstance>()
const formData = reactive<Partial<ProcessDefinition>>({
  process_key: '',
  process_name: '',
  category: '',
  description: '',
  nodes: []
})

const formRules: FormRules = {
  process_key: [{ required: true, message: '请输入流程标识', trigger: 'blur' }],
  process_name: [{ required: true, message: '请输入流程名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择流程分类', trigger: 'change' }]
}

const versionDialogVisible = ref(false)
const templateDialogVisible = ref(false)
const templateFormRef = ref<FormInstance>()
const templateForm = reactive({
  template_name: '',
  category: '',
  description: ''
})

const templateRules: FormRules = {
  template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择模板分类', trigger: 'change' }]
}

const dialogTitle = ref('新建流程定义')

const getStatusType = (status: string) => {
  const map: Record<string, any> = { draft: 'info', active: 'success', suspended: 'warning', deprecated: 'danger' }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', active: '已激活', suspended: '已暂停', deprecated: '已废弃' }
  return map[status] || status
}

const getCategoryText = (category?: string) => {
  const map: Record<string, string> = { sales: '销售', purchase: '采购', finance: '财务', hr: '人事', production: '生产' }
  return category ? (map[category] || category) : '-'
}

const getVersionStatusText = (status: string) => {
  const map: Record<string, string> = { draft: '草稿', active: '已激活', deprecated: '已废弃' }
  return map[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await bpmEnhancedApi.listDefinitions({
      page: pagination.page,
      page_size: pagination.page_size,
      category: filterForm.category || undefined,
      keyword: filterForm.keyword || undefined
    })
    definitions.value = res.data.list
    pagination.total = res.data.total
  } catch (e) {
    console.error(e)
  } finally {
    loading.value = false
  }
}

const handleSearch = () => {
  pagination.page = 1
  fetchData()
}

const handleResetFilter = () => {
  filterForm.category = ''
  filterForm.keyword = ''
  handleSearch()
}

const handleCreate = () => {
  isEdit.value = false
  dialogTitle.value = '新建流程定义'
  formData.process_key = ''
  formData.process_name = ''
  formData.category = ''
  formData.description = ''
  formData.nodes = []
  dialogVisible.value = true
}

const handleEdit = (row: ProcessDefinition) => {
  isEdit.value = true
  dialogTitle.value = '编辑流程定义'
  Object.assign(formData, {
    process_key: row.process_key,
    process_name: row.process_name,
    category: row.category,
    description: row.description,
    nodes: row.nodes || []
  })
  dialogVisible.value = true
}

const handleAddNode = () => {
  if (!formData.nodes) formData.nodes = []
  formData.nodes.push({
    id: `node_${Date.now()}`,
    type: 'approval',
    name: '新审批节点',
    assignee_type: 'user',
    assignee_value: ''
  })
}

const handleRemoveNode = (index: number) => {
  formData.nodes?.splice(index, 1)
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (isEdit.value && formData.id) {
        await bpmEnhancedApi.updateDefinition(formData.id, formData)
        ElMessage.success('更新成功')
      } else {
        await bpmEnhancedApi.createDefinition(formData)
        ElMessage.success('创建成功')
      }
      dialogVisible.value = false
      fetchData()
    } catch (e) {
      console.error(e)
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: ProcessDefinition) => {
  try {
    await ElMessageBox.confirm(`确定删除流程定义「${row.process_name}」吗？`, '确认', { type: 'warning' })
    await bpmEnhancedApi.deleteDefinition(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const handleVersions = async (row: ProcessDefinition) => {
  currentDefinition.value = row
  versionDialogVisible.value = true
  try {
    const res = await bpmEnhancedApi.listVersions(row.id)
    versions.value = res.data
  } catch (e) {
    console.error(e)
  }
}

const handleCreateVersion = async () => {
  if (!currentDefinition.value) return
  try {
    const { value: changeLog } = await ElMessageBox.prompt('请输入变更说明', '创建新版本', {
      inputPlaceholder: '例如：新增财务审批节点'
    })
    await bpmEnhancedApi.createVersion(currentDefinition.value.id, { change_log: changeLog })
    ElMessage.success('新版本创建成功')
    handleVersions(currentDefinition.value)
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const handleActivateVersion = async (row: ProcessVersion) => {
  try {
    await ElMessageBox.confirm(`确定激活版本 ${row.version} 吗？`, '确认', { type: 'info' })
    await bpmEnhancedApi.activateVersion(row.id)
    ElMessage.success('版本激活成功')
    if (currentDefinition.value) handleVersions(currentDefinition.value)
  } catch (e) {
    if (e !== 'cancel') console.error(e)
  }
}

const handleSaveTemplate = (row: ProcessDefinition) => {
  currentDefinition.value = row
  templateForm.template_name = `${row.process_name} 模板`
  templateForm.category = row.category || 'common'
  templateForm.description = row.description || ''
  templateDialogVisible.value = true
}

const handleConfirmSaveTemplate = async () => {
  if (!templateFormRef.value || !currentDefinition.value) return
  await templateFormRef.value.validate(async (valid) => {
    if (!valid) return
    submitLoading.value = true
    try {
      await bpmEnhancedApi.saveAsTemplate(currentDefinition.value!.id, templateForm)
      ElMessage.success('保存为模板成功')
      templateDialogVisible.value = false
    } catch (e) {
      console.error(e)
    } finally {
      submitLoading.value = false
    }
  })
}

onMounted(() => { fetchData() })
</script>

<style scoped>
.bpm-definitions-page { padding: 24px; background-color: #f5f7fa; min-height: 100%; }
.page-header { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.header-left .page-title { font-size: 28px; font-weight: 600; color: #303133; margin: 0 0 12px 0; }
.filter-card { margin-bottom: 20px; }
.filter-form { margin-bottom: 0; }
.table-card { margin-bottom: 20px; }
.pagination-wrapper { display: flex; justify-content: flex-end; margin-top: 20px; }
.version-header { display: flex; justify-content: space-between; align-items: center; }
.active-tag { color: #67c23a; font-weight: 600; }
</style>
