<!--
  quality-standards/index.vue - 质量标准管理
  D05 Batch 5：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="quality-standards-page">
    <div class="page-header">
      <h2 class="page-title">{{ $t('qualityStandards.title') }}</h2>
      <div class="header-actions">
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>
          {{ $t('qualityStandards.create') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          {{ $t('qualityStandards.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <div class="filter-container">
        <el-input
          v-model="listQuery.keyword"
          :placeholder="$t('qualityStandards.filter.keywordPlaceholder')"
          style="width: 200px"
          clearable
          @clear="handleSearch"
          @keyup.enter="handleSearch"
        />
        <el-select v-model="listQuery.status" :placeholder="$t('qualityStandards.filter.statusPlaceholder')" clearable style="width: 120px">
          <el-option :label="$t('qualityStandards.status.draft')" value="draft" />
          <el-option :label="$t('qualityStandards.status.approved')" value="approved" />
          <el-option :label="$t('qualityStandards.status.published')" value="published" />
          <el-option :label="$t('qualityStandards.status.archived')" value="archived" />
        </el-select>
        <el-select v-model="listQuery.type" :placeholder="$t('qualityStandards.filter.typePlaceholder')" clearable style="width: 120px">
          <el-option :label="$t('qualityStandards.type.product')" value="product" />
          <el-option :label="$t('qualityStandards.type.process')" value="process" />
          <el-option :label="$t('qualityStandards.type.safety')" value="safety" />
          <el-option :label="$t('qualityStandards.type.environmental')" value="environmental" />
        </el-select>
        <el-button type="primary" @click="handleSearch">
          <el-icon><Search /></el-icon>
          {{ $t('qualityStandards.filter.search') }}
        </el-button>
      </div>

      <el-table v-loading="loading" :data="list" stripe :aria-label="$t('qualityStandards.table.ariaLabel')">
        <el-table-column prop="standard_code" :label="$t('qualityStandards.table.standardCode')" width="140" />
        <el-table-column prop="standard_name" :label="$t('qualityStandards.table.standardName')" min-width="180" />
        <el-table-column prop="type" :label="$t('qualityStandards.table.type')" width="100">
          <template #default="{ row }">
            {{ getTypeLabel(row.type) }}
          </template>
        </el-table-column>
        <el-table-column prop="version" :label="$t('qualityStandards.table.version')" width="80" />
        <el-table-column prop="status" :label="$t('qualityStandards.table.status')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="statusTypeMap[row.status]" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" :label="$t('qualityStandards.table.createdBy')" width="100" />
        <el-table-column prop="approved_by_name" :label="$t('qualityStandards.table.approvedBy')" width="100">
          <template #default="{ row }">
            {{ row.approved_by_name || '-' }}
          </template>
        </el-table-column>
        <el-table-column prop="created_at" :label="$t('qualityStandards.table.createdAt')" width="160" />
        <el-table-column :label="$t('qualityStandards.table.operation')" width="280" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">{{ $t('qualityStandards.table.edit') }}</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >{{ $t('qualityStandards.table.approve') }}</el-button
            >
            <el-button
              v-if="row.status === 'approved'"
              type="warning"
              link
              size="small"
              @click="handlePublish(row)"
              >{{ $t('qualityStandards.table.publish') }}</el-button
            >
            <el-button
              v-if="row.status === 'published'"
              type="info"
              link
              size="small"
              @click="handleArchive(row)"
              >{{ $t('qualityStandards.table.archive') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="danger"
              link
              size="small"
              @click="handleDelete(row)"
              >{{ $t('qualityStandards.table.delete') }}</el-button
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
          :aria-label="$t('qualityStandards.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="form.id ? $t('qualityStandards.dialog.editTitle') : $t('qualityStandards.dialog.createTitle')"
      width="700px"
      :aria-label="form.id ? $t('qualityStandards.dialog.editAriaLabel') : $t('qualityStandards.dialog.createAriaLabel')"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" :aria-label="$t('qualityStandards.dialog.formAriaLabel')">
        <el-form-item :label="$t('qualityStandards.dialog.standardCode')" prop="standard_code">
          <el-input
            v-model="form.standard_code"
            :disabled="!!form.id"
            :placeholder="$t('qualityStandards.dialog.standardCodePlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('qualityStandards.dialog.standardName')" prop="standard_name">
          <el-input v-model="form.standard_name" :placeholder="$t('qualityStandards.dialog.standardNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('qualityStandards.dialog.type')" prop="type">
          <el-select v-model="form.type" :placeholder="$t('qualityStandards.dialog.typePlaceholder')" style="width: 100%">
            <el-option :label="$t('qualityStandards.type.product')" value="product" />
            <el-option :label="$t('qualityStandards.type.process')" value="process" />
            <el-option :label="$t('qualityStandards.type.safety')" value="safety" />
            <el-option :label="$t('qualityStandards.type.environmental')" value="environmental" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('qualityStandards.dialog.version')" prop="version">
          <el-input v-model="form.version" :placeholder="$t('qualityStandards.dialog.versionPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('qualityStandards.dialog.content')" prop="content">
          <el-input v-model="form.content" type="textarea" :rows="6" :placeholder="$t('qualityStandards.dialog.contentPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('qualityStandards.dialog.attachments')" prop="attachments">
          <el-input
            v-model="attachmentsText"
            type="textarea"
            :placeholder="$t('qualityStandards.dialog.attachmentsPlaceholder')"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('qualityStandards.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('qualityStandards.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download, Search } from '@element-plus/icons-vue'
import {
  createQualityStandard,
  updateQualityStandard,
  deleteQualityStandard,
  approveQualityStandard,
  publishQualityStandard,
  archiveQualityStandard,
  type QualityStandard,
} from '@/api/quality-standards'
// V15 P0-S12 修复（Batch 475d）：导出改用后端带水印 xlsx 接口
// 后端 GET /quality-standards/export 已就绪（含异步审计日志 + 水印）
import { exportFromBackend } from '@/utils/export'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

// 批次 277：listQuery 仅保留筛选字段，page/page_size 交给 useTableApi 管理
const listQuery = reactive({
  keyword: '',
  status: '',
  type: '',
})

// 批次 277：接入 useTableApi，消除手写 list/total/listLoading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// getQualityStandardList 返回 ApiResponse<QualityStandard[]>（{ data: T[], total: number }），
// useTableApi detectList 支持 data 字段、detectTotal 支持 res 外层 total，已兼容
const {
  data: list,
  total,
  loading,
  page,
  pageSize,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<QualityStandard>({
  url: '/quality-standards',
  onError: (err: unknown) =>
    // 批次 98 P2-D 修复（v5 复审）：unknown 类型守卫
    ElMessage.error((err instanceof Error ? err.message : String(err)) || t('qualityStandards.message.loadFailed')),
})

// 批次 277：同步 listQuery 筛选条件到 useTableApi.queryParams
const syncQueryParams = () => {
  setQueryParam('keyword', listQuery.keyword || undefined)
  setQueryParam('status', listQuery.status || undefined)
  setQueryParam('type', listQuery.type || undefined)
}

// 批次 277：搜索/重置统一入口：同步筛选条件 + 回到首页 + 拉取
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

// D05 Batch 5：typeMap/statusMap 改为函数，使 t() 在每次渲染时响应式求值
const getTypeLabel = (type: string) => {
  const labels: Record<string, string> = {
    product: t('qualityStandards.type.product'),
    process: t('qualityStandards.type.process'),
    safety: t('qualityStandards.type.safety'),
    environmental: t('qualityStandards.type.environmental'),
  }
  return labels[type] || type
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    draft: t('qualityStandards.status.draft'),
    approved: t('qualityStandards.status.approved'),
    published: t('qualityStandards.status.published'),
    archived: t('qualityStandards.status.archived'),
  }
  return labels[status] || status
}

const statusTypeMap: Record<string, string> = {
  draft: 'info',
  approved: 'warning',
  published: 'success',
  archived: 'info',
}

const dialogVisible = ref(false)
const formRef = ref<FormInstance>()
const submitLoading = ref(false)
const attachmentsText = ref('')
const form = reactive<Partial<QualityStandard>>({
  id: undefined,
  standard_code: '',
  standard_name: '',
  version: '1.0',
  type: 'product',
  content: '',
  attachments: [],
})

const rules: FormRules = {
  standard_code: [{ required: true, message: t('qualityStandards.validation.standardCodeRequired'), trigger: 'blur' }],
  standard_name: [{ required: true, message: t('qualityStandards.validation.standardNameRequired'), trigger: 'blur' }],
  type: [{ required: true, message: t('qualityStandards.validation.typeRequired'), trigger: 'change' }],
  version: [{ required: true, message: t('qualityStandards.validation.versionRequired'), trigger: 'blur' }],
  content: [{ required: true, message: t('qualityStandards.validation.contentRequired'), trigger: 'blur' }],
}

const openDialog = (row?: QualityStandard) => {
  if (row) {
    Object.assign(form, row)
    attachmentsText.value = JSON.stringify(row.attachments || [], null, 2)
  } else {
    Object.assign(form, {
      id: undefined,
      standard_code: '',
      standard_name: '',
      version: '1.0',
      type: 'product',
      content: '',
      attachments: [],
    })
    attachmentsText.value = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (attachmentsText.value) {
        try {
          form.attachments = JSON.parse(attachmentsText.value)
        } catch (e) {
          ElMessage.error(t('qualityStandards.message.attachmentsFormatError'))
          return
        }
      }
      if (form.id) {
        await updateQualityStandard(form.id, form)
      } else {
        await createQualityStandard(form)
      }
      ElMessage.success(t('qualityStandards.message.operationSuccess'))
      dialogVisible.value = false
      fetchData()
    } catch (error: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
      ElMessage.error((error instanceof Error ? error.message : String(error)) || t('qualityStandards.message.operationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(t('qualityStandards.message.deleteConfirm'), t('qualityStandards.message.deleteConfirmTitle'), { type: 'warning' })
    await deleteQualityStandard(row.id)
    ElMessage.success(t('qualityStandards.message.deleteSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('qualityStandards.message.deleteFailed'))
  }
}

const handleApprove = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(t('qualityStandards.message.approveConfirm'), t('qualityStandards.message.approveConfirmTitle'), { type: 'warning' })
    await approveQualityStandard(row.id)
    ElMessage.success(t('qualityStandards.message.approveSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('qualityStandards.message.approveFailed'))
  }
}

const handlePublish = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(t('qualityStandards.message.publishConfirm'), t('qualityStandards.message.publishConfirmTitle'), {
      type: 'warning',
    })
    await publishQualityStandard(row.id)
    ElMessage.success(t('qualityStandards.message.publishSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('qualityStandards.message.publishFailed'))
  }
}

const handleArchive = async (row: QualityStandard) => {
  try {
    await ElMessageBox.confirm(t('qualityStandards.message.archiveConfirm'), t('qualityStandards.message.archiveConfirmTitle'), { type: 'warning' })
    await archiveQualityStandard(row.id)
    ElMessage.success(t('qualityStandards.message.archiveSuccess'))
    fetchData()
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    if (error !== 'cancel') ElMessage.error((error instanceof Error ? error.message : String(error)) || t('qualityStandards.message.archiveFailed'))
  }
}

// 导出 Excel（V15 P0-S12 修复 Batch 475d）
// 规则 3：导出统一使用 xlsx 格式（禁止 CSV 作为最终交付格式）
// 改为调用后端 GET /quality-standards/export，后端注入水印 + 异步审计日志
// 传入当前筛选条件：listQuery.type 映射为后端 standard_type 字段，status 与后端一致
const handleExport = async () => {
  await exportFromBackend(
    '/quality-standards/export',
    {
      standard_type: listQuery.type || undefined,
      status: listQuery.status || undefined,
    },
    'quality_standards_export'
  )
}
</script>

<style scoped>
.quality-standards-page {
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
</style>
