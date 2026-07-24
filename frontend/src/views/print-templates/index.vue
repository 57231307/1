<template>
  <div class="print-templates-page">
    <div class="page-header">
      <h2 class="page-title">打印模板管理</h2>
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
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select v-model="listQuery.module" placeholder="模块" clearable style="width: 120px">
          <el-option label="销售" value="sales" />
          <el-option label="采购" value="purchase" />
          <el-option label="库存" value="inventory" />
          <el-option label="财务" value="finance" />
          <el-option label="生产" value="production" />
          <el-option label="物流" value="logistics" />
        </el-select>
        <el-select v-model="listQuery.type" placeholder="类型" clearable style="width: 120px">
          <el-option label="订单" value="order" />
          <el-option label="发票" value="invoice" />
          <el-option label="收据" value="receipt" />
          <el-option label="标签" value="label" />
          <el-option label="报表" value="report" />
          <el-option label="自定义" value="custom" />
        </el-select>
        <el-select v-model="listQuery.status" placeholder="状态" clearable style="width: 120px">
          <el-option label="启用" value="active" />
          <el-option label="停用" value="inactive" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          搜索
        </el-button>
      </div>

      <el-table v-loading="loading" :data="list" stripe aria-label="打印模板列表">
        <el-table-column prop="template_code" label="模板编号" width="140" />
        <el-table-column prop="template_name" label="模板名称" min-width="180" />
        <el-table-column prop="module" label="模块" width="80">
          <template #default="{ row }">
            {{ moduleMap[row.module] }}
          </template>
        </el-table-column>
        <el-table-column prop="type" label="类型" width="80">
          <template #default="{ row }">
            {{ typeMap[row.type] }}
          </template>
        </el-table-column>
        <el-table-column prop="paper_size" label="纸张" width="80" />
        <el-table-column prop="orientation" label="方向" width="80">
          <template #default="{ row }">
            {{ row.orientation === 'portrait' ? '纵向' : '横向' }}
          </template>
        </el-table-column>
        <el-table-column prop="is_default" label="默认" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_default ? 'success' : 'info'" size="small">
              {{ row.is_default ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="300" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handlePreview(row)">预览</el-button>
            <el-button type="primary" link size="small" @click="handleCopy(row)">复制</el-button>
            <el-button v-permission="'print_template:update'" type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-if="!row.is_default"
              type="success"
              link
              size="small"
              @click="handleSetDefault(row)"
              >设为默认</el-button
            >
            <el-button v-permission="'print_template:delete'" type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
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
          aria-label="打印模板列表分页"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? '编辑模板' : '新建模板'" width="900px" aria-label="打印模板编辑对话框">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" aria-label="打印模板表单">
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="模板编号" prop="template_code">
              <el-input
                v-model="form.template_code"
                :disabled="!!form.id"
                placeholder="请输入模板编号"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="模板名称" prop="template_name">
              <el-input v-model="form.template_name" placeholder="请输入模板名称" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="模块" prop="module">
              <el-select v-model="form.module" placeholder="请选择模块" style="width: 100%">
                <el-option label="销售" value="sales" />
                <el-option label="采购" value="purchase" />
                <el-option label="库存" value="inventory" />
                <el-option label="财务" value="finance" />
                <el-option label="生产" value="production" />
                <el-option label="物流" value="logistics" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="类型" prop="type">
              <el-select v-model="form.type" placeholder="请选择类型" style="width: 100%">
                <el-option label="订单" value="order" />
                <el-option label="发票" value="invoice" />
                <el-option label="收据" value="receipt" />
                <el-option label="标签" value="label" />
                <el-option label="报表" value="report" />
                <el-option label="自定义" value="custom" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="纸张大小" prop="paper_size">
              <el-select v-model="form.paper_size" placeholder="请选择纸张" style="width: 100%">
                <el-option label="A4" value="A4" />
                <el-option label="A5" value="A5" />
                <el-option label="B5" value="B5" />
                <el-option label="Letter" value="Letter" />
                <el-option label="自定义" value="Custom" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="方向" prop="orientation">
              <el-radio-group v-model="form.orientation">
                <el-radio label="portrait">纵向</el-radio>
                <el-radio label="landscape">横向</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="描述" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="2" placeholder="请输入描述" />
        </el-form-item>
        <el-form-item label="模板内容" prop="content">
          <el-input
            v-model="form.content"
            type="textarea"
            :rows="10"
            placeholder="请输入HTML模板内容"
          />
        </el-form-item>
        <el-form-item label="CSS样式" prop="css_styles">
          <el-input
            v-model="form.css_styles"
            type="textarea"
            :rows="4"
            placeholder="请输入CSS样式"
          />
        </el-form-item>
        <el-form-item label="变量配置" prop="variables">
          <el-input
            v-model="variablesText"
            type="textarea"
            :rows="4"
            placeholder='JSON格式变量，例如：{"company_name": "公司名称", "date": "日期"}'
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="previewVisible" title="模板预览" width="900px" aria-label="模板预览对话框">
      <div v-loading="previewLoading" class="preview-container">
        <!-- Wave B-2 修复（B3-2）：使用 DOMPurify 净化后端返回的 HTML，防止 XSS 注入 -->
        <div v-if="previewData" v-html="sanitizedPreview"></div>
        <div v-else class="no-preview">暂无预览数据</div>
      </div>
      <template #footer>
        <el-button @click="previewVisible = false">关闭</el-button>
        <el-button type="primary" @click="handlePrint">打印</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
// Wave B-2 修复（B3-2）：引入 DOMPurify 用于净化后端返回的 HTML 模板，防止 XSS
import DOMPurify from 'dompurify'
import {
  createPrintTemplate,
  updatePrintTemplate,
  deletePrintTemplate,
  previewPrintTemplate,
  setDefaultPrintTemplate,
  copyPrintTemplate,
  printTemplate,
  type PrintTemplate,
} from '@/api/print-templates'
// 批次 277：接入 useTableApi，消除手写 list/total/listLoading/fetchData 重复
import { useTableApi } from '@/composables/useTableApi'

// 批次 277：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// getPrintTemplateList 返回 ApiResponse<PrintTemplate[]>（{ data: T[], total: number }），
// useTableApi detectList 会 fallback 到 obj.data 取裸数组，detectTotal 取外层 total
const {
  data: list,
  loading,
  total,
  page,
  pageSize,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<PrintTemplate>({
  url: '/print-templates',
  onError: (err: unknown) =>
    // 批次 98 P2-D 修复（v5 复审）：unknown + 类型守卫
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取数据失败'),
})

// 批次 277：listQuery 仅保留筛选字段用于表单 v-model 绑定，分页字段由 useTableApi 管理
const listQuery = reactive({
  keyword: '',
  module: '',
  type: '',
  status: '',
})

const moduleMap: Record<string, string> = {
  sales: '销售',
  purchase: '采购',
  inventory: '库存',
  finance: '财务',
  production: '生产',
  logistics: '物流',
}

const typeMap: Record<string, string> = {
  order: '订单',
  invoice: '发票',
  receipt: '收据',
  label: '标签',
  report: '报表',
  custom: '自定义',
}

const dialogVisible = ref(false)
const formRef = ref<FormInstance>()
const submitLoading = ref(false)
const variablesText = ref('')
const form = reactive<Partial<PrintTemplate>>({
  id: undefined,
  template_code: '',
  template_name: '',
  description: '',
  module: 'sales',
  type: 'order',
  paper_size: 'A4',
  orientation: 'portrait',
  content: '',
  css_styles: '',
  variables: {},
  status: 'active',
  is_default: false,
})

const rules: FormRules = {
  template_code: [{ required: true, message: '请输入模板编号', trigger: 'blur' }],
  template_name: [{ required: true, message: '请输入模板名称', trigger: 'blur' }],
  module: [{ required: true, message: '请选择模块', trigger: 'change' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
  paper_size: [{ required: true, message: '请选择纸张大小', trigger: 'change' }],
  orientation: [{ required: true, message: '请选择方向', trigger: 'change' }],
  content: [{ required: true, message: '请输入模板内容', trigger: 'blur' }],
}

const openDialog = (row?: PrintTemplate) => {
  if (row) {
    Object.assign(form, row)
    variablesText.value = JSON.stringify(row.variables || {}, null, 2)
  } else {
    Object.assign(form, {
      id: undefined,
      template_code: '',
      template_name: '',
      description: '',
      module: 'sales',
      type: 'order',
      paper_size: 'A4',
      orientation: 'portrait',
      content: '',
      css_styles: '',
      variables: {},
      status: 'active',
      is_default: false,
    })
    variablesText.value = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (variablesText.value) {
        try {
          form.variables = JSON.parse(variablesText.value)
        } catch (e) {
          ElMessage.error('变量配置格式错误，请检查JSON格式')
          return
        }
      }
      if (form.id) {
        await updatePrintTemplate(form.id, form)
      } else {
        await createPrintTemplate(form)
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

const handleDelete = async (row: PrintTemplate) => {
  try {
    await ElMessageBox.confirm('确定要删除此模板吗？', '确认删除', { type: 'warning' })
    await deletePrintTemplate(row.id)
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
const currentPreviewTemplate = ref<PrintTemplate | null>(null)

// Wave B-2 修复（B3-2）：使用 DOMPurify.sanitize 净化预览 HTML 内容
// 安全原因：v-html 默认不转义，后端返回的打印模板内容若包含恶意脚本（<script>、onerror 等），
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

const handlePreview = async (row: PrintTemplate) => {
  previewLoading.value = true
  previewVisible.value = true
  currentPreviewTemplate.value = row
  try {
    const res = await previewPrintTemplate(row.id)
    // P2-16 修复回归（批次 86）：res.data 是 PrintTemplatePreviewResult，取 html 字段
    previewData.value = res.data?.html || ''
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '预览失败')
    previewData.value = ''
  } finally {
    previewLoading.value = false
  }
}

const handlePrint = async () => {
  if (!currentPreviewTemplate.value) return
  try {
    await printTemplate(currentPreviewTemplate.value.id, {})
    ElMessage.success('打印任务已发送')
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '打印失败')
  }
}

const handleSetDefault = async (row: PrintTemplate) => {
  try {
    await ElMessageBox.confirm('确定要将此模板设为默认吗？', '确认设置', { type: 'warning' })
    await setDefaultPrintTemplate(row.id)
    ElMessage.success('设置成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || '设置失败')
  }
}

const handleCopy = async (row: PrintTemplate) => {
  try {
    await copyPrintTemplate(row.id)
    ElMessage.success('复制成功')
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '复制失败')
  }
}

// 批次 277：同步筛选条件到 useTableApi.queryParams，再触发刷新
const syncQueryParams = () => {
  setQueryParam('keyword', listQuery.keyword || undefined)
  setQueryParam('module', listQuery.module || undefined)
  setQueryParam('type', listQuery.type || undefined)
  setQueryParam('status', listQuery.status || undefined)
}

// 批次 277：搜索前先同步筛选条件，重置到首页再加载
const handleSearch = () => {
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

// 批次 277：useTableApi 构造时自动初始加载，无需 onMounted 调用 fetchData
</script>

<style scoped>
.print-templates-page {
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
