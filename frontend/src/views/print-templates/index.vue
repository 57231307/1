<!--
  print-templates/index.vue - 打印模板管理
  D05 Batch 2：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="print-templates-page">
    <div class="page-header">
      <h2 class="page-title">{{ $t('printTemplates.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>
          {{ $t('printTemplates.create') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="filter-container">
        <el-input
          v-model="listQuery.keyword"
          :placeholder="$t('printTemplates.searchPlaceholder')"
          style="width: 200px"
          clearable
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select v-model="listQuery.module" :placeholder="$t('printTemplates.module.placeholder')" clearable style="width: 120px">
          <el-option :label="$t('printTemplates.module.sales')" value="sales" />
          <el-option :label="$t('printTemplates.module.purchase')" value="purchase" />
          <el-option :label="$t('printTemplates.module.inventory')" value="inventory" />
          <el-option :label="$t('printTemplates.module.finance')" value="finance" />
          <el-option :label="$t('printTemplates.module.production')" value="production" />
          <el-option :label="$t('printTemplates.module.logistics')" value="logistics" />
        </el-select>
        <el-select v-model="listQuery.type" :placeholder="$t('printTemplates.type.placeholder')" clearable style="width: 120px">
          <el-option :label="$t('printTemplates.type.order')" value="order" />
          <el-option :label="$t('printTemplates.type.invoice')" value="invoice" />
          <el-option :label="$t('printTemplates.type.receipt')" value="receipt" />
          <el-option :label="$t('printTemplates.type.label')" value="label" />
          <el-option :label="$t('printTemplates.type.report')" value="report" />
          <el-option :label="$t('printTemplates.type.custom')" value="custom" />
        </el-select>
        <el-select v-model="listQuery.status" :placeholder="$t('printTemplates.status.placeholder')" clearable style="width: 120px">
          <el-option :label="$t('printTemplates.status.active')" value="active" />
          <el-option :label="$t('printTemplates.status.inactive')" value="inactive" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ $t('printTemplates.search') }}
        </el-button>
      </div>

      <el-table v-loading="loading" :data="list" stripe :aria-label="$t('printTemplates.table.ariaLabel')">
        <el-table-column prop="template_code" :label="$t('printTemplates.table.templateCode')" width="140" />
        <el-table-column prop="template_name" :label="$t('printTemplates.table.templateName')" min-width="180" />
        <el-table-column prop="module" :label="$t('printTemplates.table.module')" width="80">
          <template #default="{ row }">
            {{ getModuleLabel(row.module) }}
          </template>
        </el-table-column>
        <el-table-column prop="type" :label="$t('printTemplates.table.type')" width="80">
          <template #default="{ row }">
            {{ getTypeLabel(row.type) }}
          </template>
        </el-table-column>
        <el-table-column prop="paper_size" :label="$t('printTemplates.table.paperSize')" width="80" />
        <el-table-column prop="orientation" :label="$t('printTemplates.table.orientation')" width="80">
          <template #default="{ row }">
            {{ row.orientation === 'portrait' ? $t('printTemplates.orientation.portrait') : $t('printTemplates.orientation.landscape') }}
          </template>
        </el-table-column>
        <el-table-column prop="is_default" :label="$t('printTemplates.table.isDefault')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_default ? 'success' : 'info'" size="small">
              {{ row.is_default ? $t('printTemplates.yesNo.yes') : $t('printTemplates.yesNo.no') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('printTemplates.table.status')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? $t('printTemplates.status.active') : $t('printTemplates.status.inactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('printTemplates.table.operation')" width="300" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handlePreview(row)">{{ $t('printTemplates.table.preview') }}</el-button>
            <el-button type="primary" link size="small" @click="handleCopy(row)">{{ $t('printTemplates.table.copy') }}</el-button>
            <el-button v-permission="'print_template:update'" type="primary" link size="small" @click="openDialog(row)">{{ $t('printTemplates.table.edit') }}</el-button>
            <el-button
              v-if="!row.is_default"
              type="success"
              link
              size="small"
              @click="handleSetDefault(row)"
              >{{ $t('printTemplates.table.setDefault') }}</el-button
            >
            <el-button v-permission="'print_template:delete'" type="danger" link size="small" @click="handleDelete(row)">{{ $t('printTemplates.table.delete') }}</el-button>
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
          :aria-label="$t('printTemplates.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? $t('printTemplates.dialog.editTitle') : $t('printTemplates.dialog.createTitle')" width="900px" :aria-label="$t('printTemplates.dialog.ariaLabel')">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" :aria-label="$t('printTemplates.dialog.formAriaLabel')">
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.templateCode')" prop="template_code">
              <el-input
                v-model="form.template_code"
                :disabled="!!form.id"
                :placeholder="$t('printTemplates.dialog.templateCodePlaceholder')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.templateName')" prop="template_name">
              <el-input v-model="form.template_name" :placeholder="$t('printTemplates.dialog.templateNamePlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.module')" prop="module">
              <el-select v-model="form.module" :placeholder="$t('printTemplates.dialog.modulePlaceholder')" style="width: 100%">
                <el-option :label="$t('printTemplates.module.sales')" value="sales" />
                <el-option :label="$t('printTemplates.module.purchase')" value="purchase" />
                <el-option :label="$t('printTemplates.module.inventory')" value="inventory" />
                <el-option :label="$t('printTemplates.module.finance')" value="finance" />
                <el-option :label="$t('printTemplates.module.production')" value="production" />
                <el-option :label="$t('printTemplates.module.logistics')" value="logistics" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.type')" prop="type">
              <el-select v-model="form.type" :placeholder="$t('printTemplates.dialog.typePlaceholder')" style="width: 100%">
                <el-option :label="$t('printTemplates.type.order')" value="order" />
                <el-option :label="$t('printTemplates.type.invoice')" value="invoice" />
                <el-option :label="$t('printTemplates.type.receipt')" value="receipt" />
                <el-option :label="$t('printTemplates.type.label')" value="label" />
                <el-option :label="$t('printTemplates.type.report')" value="report" />
                <el-option :label="$t('printTemplates.type.custom')" value="custom" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.paperSize')" prop="paper_size">
              <el-select v-model="form.paper_size" :placeholder="$t('printTemplates.dialog.paperSizePlaceholder')" style="width: 100%">
                <el-option label="A4" value="A4" />
                <el-option label="A5" value="A5" />
                <el-option label="B5" value="B5" />
                <el-option label="Letter" value="Letter" />
                <el-option :label="$t('printTemplates.type.custom')" value="Custom" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="$t('printTemplates.dialog.orientation')" prop="orientation">
              <el-radio-group v-model="form.orientation">
                <el-radio label="portrait">{{ $t('printTemplates.orientation.portrait') }}</el-radio>
                <el-radio label="landscape">{{ $t('printTemplates.orientation.landscape') }}</el-radio>
              </el-radio-group>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="$t('printTemplates.dialog.description')" prop="description">
          <el-input v-model="form.description" type="textarea" :rows="2" :placeholder="$t('printTemplates.dialog.descriptionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('printTemplates.dialog.content')" prop="content">
          <el-input
            v-model="form.content"
            type="textarea"
            :rows="10"
            :placeholder="$t('printTemplates.dialog.contentPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('printTemplates.dialog.cssStyles')" prop="css_styles">
          <el-input
            v-model="form.css_styles"
            type="textarea"
            :rows="4"
            :placeholder="$t('printTemplates.dialog.cssStylesPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('printTemplates.dialog.variables')" prop="variables">
          <el-input
            v-model="variablesText"
            type="textarea"
            :rows="4"
            :placeholder="$t('printTemplates.dialog.variablesPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('printTemplates.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('printTemplates.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="previewVisible" :title="$t('printTemplates.previewDialog.title')" width="900px" :aria-label="$t('printTemplates.previewDialog.ariaLabel')">
      <div v-loading="previewLoading" class="preview-container">
        <!-- Wave B-2 修复（B3-2）：使用 DOMPurify 净化后端返回的 HTML，防止 XSS 注入 -->
        <div v-if="previewData" v-html="sanitizedPreview"></div>
        <div v-else class="no-preview">{{ $t('printTemplates.previewDialog.noData') }}</div>
      </div>
      <template #footer>
        <el-button @click="previewVisible = false">{{ $t('printTemplates.previewDialog.close') }}</el-button>
        <el-button type="primary" @click="handlePrint">{{ $t('printTemplates.previewDialog.print') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n({ useScope: 'global' })

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
    ElMessage.error((err instanceof Error ? err.message : String(err)) || t('printTemplates.message.loadFailed')),
})

// 批次 277：listQuery 仅保留筛选字段用于表单 v-model 绑定，分页字段由 useTableApi 管理
const listQuery = reactive({
  keyword: '',
  module: '',
  type: '',
  status: '',
})

// D05 Batch 2：moduleMap/typeMap 改为函数返回，使 t() 在每次渲染时响应式求值（参照 AssetListTab.vue 的 getCategoryLabel）
const getModuleLabel = (module: string) => {
  const map: Record<string, string> = {
    sales: t('printTemplates.module.sales'),
    purchase: t('printTemplates.module.purchase'),
    inventory: t('printTemplates.module.inventory'),
    finance: t('printTemplates.module.finance'),
    production: t('printTemplates.module.production'),
    logistics: t('printTemplates.module.logistics'),
  }
  return map[module] || module
}

const getTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    order: t('printTemplates.type.order'),
    invoice: t('printTemplates.type.invoice'),
    receipt: t('printTemplates.type.receipt'),
    label: t('printTemplates.type.label'),
    report: t('printTemplates.type.report'),
    custom: t('printTemplates.type.custom'),
  }
  return map[type] || type
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
  template_code: [{ required: true, message: t('printTemplates.validation.templateCodeRequired'), trigger: 'blur' }],
  template_name: [{ required: true, message: t('printTemplates.validation.templateNameRequired'), trigger: 'blur' }],
  module: [{ required: true, message: t('printTemplates.validation.moduleRequired'), trigger: 'change' }],
  type: [{ required: true, message: t('printTemplates.validation.typeRequired'), trigger: 'change' }],
  paper_size: [{ required: true, message: t('printTemplates.validation.paperSizeRequired'), trigger: 'change' }],
  orientation: [{ required: true, message: t('printTemplates.validation.orientationRequired'), trigger: 'change' }],
  content: [{ required: true, message: t('printTemplates.validation.contentRequired'), trigger: 'blur' }],
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
          ElMessage.error(t('printTemplates.message.variablesFormatError'))
          return
        }
      }
      if (form.id) {
        await updatePrintTemplate(form.id, form)
      } else {
        await createPrintTemplate(form)
      }
      ElMessage.success(t('printTemplates.message.operationSuccess'))
      dialogVisible.value = false
      fetchData()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.operationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: PrintTemplate) => {
  try {
    await ElMessageBox.confirm(
      t('printTemplates.message.deleteConfirm'),
      t('printTemplates.message.deleteConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    )
    await deletePrintTemplate(row.id)
    ElMessage.success(t('printTemplates.message.deleteSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.deleteFailed'))
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
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.previewFailed'))
    previewData.value = ''
  } finally {
    previewLoading.value = false
  }
}

const handlePrint = async () => {
  if (!currentPreviewTemplate.value) return
  try {
    await printTemplate(currentPreviewTemplate.value.id, {})
    ElMessage.success(t('printTemplates.message.printSent'))
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.printFailed'))
  }
}

const handleSetDefault = async (row: PrintTemplate) => {
  try {
    await ElMessageBox.confirm(
      t('printTemplates.message.setDefaultConfirm'),
      t('printTemplates.message.setDefaultConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    )
    await setDefaultPrintTemplate(row.id)
    ElMessage.success(t('printTemplates.message.setDefaultSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.setDefaultFailed'))
  }
}

const handleCopy = async (row: PrintTemplate) => {
  try {
    await copyPrintTemplate(row.id)
    ElMessage.success(t('printTemplates.message.copySuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('printTemplates.message.copyFailed'))
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
