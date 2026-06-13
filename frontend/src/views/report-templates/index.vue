<template>
  <div class="report-templates-page">
    <div class="page-header">
      <h2 class="page-title">报表中心</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>
          新建模板
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="filter-container">
        <el-input
          v-model="listQuery.keyword"
          placeholder="搜索模板编号/名称"
          style="width: 200px"
          clearable
          @clear="fetchData"
          @keyup.enter="fetchData"
        />
        <el-select v-model="listQuery.category" placeholder="分类" clearable style="width: 120px">
          <el-option label="销售" value="sales" />
          <el-option label="库存" value="inventory" />
          <el-option label="财务" value="finance" />
          <el-option label="生产" value="production" />
          <el-option label="自定义" value="custom" />
        </el-select>
        <el-select v-model="listQuery.status" placeholder="状态" clearable style="width: 120px">
          <el-option label="启用" value="active" />
          <el-option label="停用" value="inactive" />
        </el-select>
        <el-button type="primary" @click="fetchData">
          <el-icon><Search /></el-icon>
          搜索
        </el-button>
      </div>

      <el-table v-loading="listLoading" :data="list" stripe>
        <el-table-column prop="template_code" label="模板编号" width="140" />
        <el-table-column prop="template_name" label="模板名称" min-width="180" />
        <el-table-column prop="category" label="分类" width="100">
          <template #default="{ row }">
            {{ categoryMap[row.category] }}
          </template>
        </el-table-column>
        <el-table-column prop="format" label="格式" width="80">
          <template #default="{ row }">
            {{ row.format.toUpperCase() }}
          </template>
        </el-table-column>
        <el-table-column prop="is_system" label="系统模板" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_system ? 'success' : 'info'" size="small">
              {{ row.is_system ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="250" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handlePreview(row)">预览</el-button>
            <el-button type="primary" link size="small" @click="handleGenerate(row)"
              >生成</el-button
            >
            <el-button type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-if="!row.is_system"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-container">
        <el-pagination
          v-model:current-page="listQuery.page"
          v-model:page-size="listQuery.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="fetchData"
          @current-change="fetchData"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑模板' : '新建模板'" width="800px">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="模板编号" prop="template_code">
              <el-input
                v-model="form.template_code"
                :disabled="!!form.id"
                placeholder="请输入模板编号"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="模板名称" prop="template_name">
              <el-input v-model="form.template_name" placeholder="请输入模板名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="分类" prop="category">
              <el-select v-model="form.category" placeholder="请选择分类" style="width: 100%">
                <el-option label="销售" value="sales" />
                <el-option label="库存" value="inventory" />
                <el-option label="财务" value="finance" />
                <el-option label="生产" value="production" />
                <el-option label="自定义" value="custom" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="格式" prop="format">
              <el-select v-model="form.format" placeholder="请选择格式" style="width: 100%">
                <el-option label="PDF" value="pdf" />
                <el-option label="Excel" value="excel" />
                <el-option label="Word" value="word" />
                <el-option label="HTML" value="html" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="3" placeholder="请输入描述" />
        </el-form-item>
        <el-form-item label="模板内容" prop="content">
          <el-input
            v-model="form.content"
            type="textarea"
            :rows="10"
            placeholder="请输入模板内容（支持HTML）"
          />
        </el-form-item>
        <el-form-item label="参数配置" prop="parameters">
          <el-input
            v-model="parametersText"
            type="textarea"
            :rows="4"
            placeholder='JSON格式参数，例如：{"date_range": true, "department": true}'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="previewVisible" title="报表预览" width="900px">
      <div v-loading="previewLoading" class="preview-container">
        <div v-if="previewData" v-html="previewData"></div>
        <div v-else class="no-preview">暂无预览数据</div>
      </div>
      <template #footer>
        <el-button @click="previewVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
import {
  listReportTemplates,
  createReportTemplate,
  updateReportTemplate,
  deleteReportTemplate,
  previewReportTemplate,
  generateReport,
  type ReportTemplate,
} from '@/api/report-templates'

const list = ref<ReportTemplate[]>([])
const total = ref(0)
const listLoading = ref(false)
const listQuery = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  category: '',
  status: '',
})

const categoryMap: Record<string, string> = {
  sales: '销售',
  inventory: '库存',
  finance: '财务',
  production: '生产',
  custom: '自定义',
}

const fetchData = async () => {
  listLoading.value = true
  try {
    const res = await listReportTemplates(listQuery)
    list.value = res.data || []
    total.value = res.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取数据失败')
  } finally {
    listLoading.value = false
  }
}

const dialogVisible = ref(false)
const formRef = ref<FormInstance>()
const submitLoading = ref(false)
const parametersText = ref('')
const form = reactive<Partial<ReportTemplate>>({
  id: undefined,
  template_code: '',
  template_name: '',
  description: '',
  category: 'custom',
  format: 'pdf',
  content: '',
  parameters: {},
  is_system: false,
  status: 'active',
})

const rules: FormRules = {
  template_code: [{ required: true, message: '请输入模板编号', trigger: 'blur' }],
  template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  category: [{ required: true, message: '请选择分类', trigger: 'change' }],
  format: [{ required: true, message: '请选择格式', trigger: 'change' }],
  content: [{ required: true, message: '请输入模板内容', trigger: 'blur' }],
}

const openDialog = (row?: ReportTemplate) => {
  if (row) {
    Object.assign(form, row)
    parametersText.value = JSON.stringify(row.parameters || {}, null, 2)
  } else {
    Object.assign(form, {
      id: undefined,
      template_code: '',
      template_name: '',
      description: '',
      category: 'custom',
      format: 'pdf',
      content: '',
      parameters: {},
      is_system: false,
      status: 'active',
    })
    parametersText.value = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (parametersText.value) {
        try {
          form.parameters = JSON.parse(parametersText.value)
        } catch (e) {
          ElMessage.error('参数格式错误，请检查JSON格式')
          return
        }
      }
      if (form.id) {
        await updateReportTemplate(form.id, form)
      } else {
        await createReportTemplate(form)
      }
      ElMessage.success('操作成功')
      dialogVisible.value = false
      fetchData()
    } catch (error: any) {
      ElMessage.error(error.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: ReportTemplate) => {
  try {
    await ElMessageBox.confirm('确定要删除此模板吗？', '确认删除', { type: 'warning' })
    await deleteReportTemplate(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') ElMessage.error(error.message || '删除失败')
  }
}

const previewVisible = ref(false)
const previewLoading = ref(false)
const previewData = ref('')

const handlePreview = async (row: ReportTemplate) => {
  previewLoading.value = true
  previewVisible.value = true
  try {
    const res = await previewReportTemplate(row.id)
    previewData.value = res.data || ''
  } catch (error: any) {
    ElMessage.error(error.message || '预览失败')
    previewData.value = ''
  } finally {
    previewLoading.value = false
  }
}

const handleGenerate = async (row: ReportTemplate) => {
  try {
    const blob = await generateReport(row.id, {})
    const link = document.createElement('a')
    link.href = URL.createObjectURL(blob)
    link.download = `${row.template_name}_${new Date().toISOString().split('T')[0]}.${row.format}`
    link.click()
    ElMessage.success('报表生成成功')
  } catch (error: any) {
    ElMessage.error(error.message || '生成失败')
  }
}

onMounted(() => {
  fetchData()
})
</script>

<style scoped>
.report-templates-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-container {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}
.preview-container {
  min-height: 300px;
  max-height: 500px;
  overflow-y: auto;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  padding: 16px;
}
.no-preview {
  text-align: center;
  color: #909399;
  padding: 40px;
}
</style>
