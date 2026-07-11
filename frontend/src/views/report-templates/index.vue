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
          v-model="queryParams.keyword"
          placeholder="搜索模板编号/名称"
          style="width: 200px"
          clearable
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select v-model="queryParams.category" placeholder="分类" clearable style="width: 120px">
          <el-option label="销售" value="sales" />
          <el-option label="库存" value="inventory" />
          <el-option label="财务" value="finance" />
          <el-option label="生产" value="production" />
          <el-option label="自定义" value="custom" />
        </el-select>
        <el-select v-model="queryParams.status" placeholder="状态" clearable style="width: 120px">
          <el-option label="启用" value="active" />
          <el-option label="停用" value="inactive" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          搜索
        </el-button>
      </div>

      <el-table v-loading="loading" :data="list" stripe>
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
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
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
        <!-- Wave B-2 修复（B3-1）：使用 DOMPurify 净化后端返回的 HTML，防止 XSS 注入 -->
        <div v-if="previewData" v-html="sanitizedPreview"></div>
        <div v-else class="no-preview">暂无预览数据</div>
      </div>
      <template #footer>
        <el-button @click="previewVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
// Wave B-2 修复（B3-1）：引入 DOMPurify 用于净化后端返回的 HTML 模板，防止 XSS
import DOMPurify from 'dompurify'
// v14 中风险安全修复（批次 243）：引入 escapeHtml 对单元格值做 HTML 转义，
// 防止后端数据中包含的恶意 HTML/脚本经 v-html 渲染后误导用户（DOMPurify 默认允许 <img>/<a> 等标签）
import { escapeHtml } from '@/utils/print'
import {
  createReportTemplate,
  updateReportTemplate,
  deleteReportTemplate,
  previewReportTemplate,
  generateReport,
  type ReportTemplate,
} from '@/api/report-templates'
// 批次 277：接入 useTableApi composable，统一表格分页/加载/查询逻辑
import { useTableApi } from '@/composables/useTableApi'

const queryParams = reactive({
  keyword: '',
  category: '',
  status: '',
})

// 批次 277：接入 useTableApi，消除手写 list/total/listLoading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: list,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<ReportTemplate>({
  url: '/report-templates',
  onError: (err: unknown) =>
    // 批次 98 P2-D 修复（v5 复审）：类型守卫提取错误信息
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取数据失败'),
})

// 批次 277：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('category', queryParams.category || undefined)
  setQueryParam('status', queryParams.status || undefined)
}

const categoryMap: Record<string, string> = {
  sales: '销售',
  inventory: '库存',
  finance: '财务',
  production: '生产',
  custom: '自定义',
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
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '操作失败')
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
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
  }
}

const previewVisible = ref(false)
const previewLoading = ref(false)
const previewData = ref('')

// Wave B-2 修复（B3-1）：使用 DOMPurify.sanitize 净化预览 HTML 内容
// 安全原因：v-html 默认不转义，后端返回的模板内容若包含恶意脚本（<script>、onerror 等），
// 会在浏览器中执行导致 XSS 攻击。DOMPurify 通过白名单过滤危险标签和属性。
const sanitizedPreview = computed(() => {
  if (!previewData.value) return ''
  return DOMPurify.sanitize(previewData.value, {
    USE_PROFILES: { html: true },
    // 禁止危险标签（脚本/iframe/object/embed），即使 DOMPurify 默认也会过滤，作为双保险
    FORBID_TAGS: ['script', 'iframe', 'object', 'embed', 'form'],
    FORBID_ATTR: ['onerror', 'onload', 'onclick', 'onmouseover'],
  })
})

const handlePreview = async (row: ReportTemplate) => {
  previewLoading.value = true
  previewVisible.value = true
  try {
    const res = await previewReportTemplate(row.id)
    // P2-16 修复回归（批次 86）：res.data 是 ReportTemplatePreviewResult（结构化），
    // 渲染为 HTML 表格字符串供 v-html + DOMPurify 使用
    if (res.data && res.data.fields && res.data.rows) {
      // v14 中风险安全修复（批次 243）：表头与单元格值均经 escapeHtml 转义，
      // 防止后端返回的字段名/数据中包含 <img onerror> 等危险标签误导用户
      const headerHtml = res.data.fields.map(f => `<th>${escapeHtml(f)}</th>`).join('')
      const bodyHtml = res.data.rows
        .map((r: Record<string, unknown>) => `<tr>${(res.data?.fields ?? []).map(f => `<td>${escapeHtml(r[f])}</td>`).join('')}</tr>`)
        .join('')
      previewData.value = `<table border="1" cellpadding="4" cellspacing="0" style="border-collapse:collapse;width:100%"><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table>`
    } else {
      previewData.value = ''
    }
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '预览失败')
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
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '生成失败')
  }
}

// 批次 277：搜索/重置（同步筛选条件到 useTableApi 后重置到第一页并刷新）
const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.category = ''
  queryParams.status = ''
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 批次 277：分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}
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
