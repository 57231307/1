<template>
  <div class="bpm-templates-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('bpm.templates.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('bpm.templates.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.templates.breadcrumb.approval') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.templates.breadcrumb.templates') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="filterForm" class="filter-form" :aria-label="$t('bpm.templates.filter.ariaLabel')">
        <el-form-item :label="$t('bpm.templates.filter.category')">
          <el-select
            v-model="filterForm.category"
            :placeholder="$t('bpm.templates.filter.categoryPlaceholder')"
            clearable
            style="width: 160px"
            @change="handleSearch"
          >
            <el-option :label="$t('bpm.templates.category.sales')" value="sales" />
            <el-option :label="$t('bpm.templates.category.purchase')" value="purchase" />
            <el-option :label="$t('bpm.templates.category.finance')" value="finance" />
            <el-option :label="$t('bpm.templates.category.hr')" value="hr" />
            <el-option :label="$t('bpm.templates.category.production')" value="production" />
            <el-option :label="$t('bpm.templates.category.common')" value="common" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{ $t('bpm.templates.filter.query') }}</el-button>
          <el-button @click="handleResetFilter">{{ $t('bpm.templates.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-row v-loading="loading" :gutter="20">
      <el-col
        v-for="template in templates"
        :key="template.id"
        :xs="24"
        :sm="12"
        :md="8"
        :lg="6"
        class="template-col"
      >
        <el-card shadow="hover" class="template-card">
          <div class="template-header">
            <div class="template-icon" :class="`icon-${template.category}`">
              <el-icon><component :is="getCategoryIcon(template.category)" /></el-icon>
            </div>
            <el-dropdown trigger="click" class="template-actions">
              <el-icon><MoreFilled /></el-icon>
              <template #dropdown>
                <el-dropdown-menu>
                  <el-dropdown-item @click="handleViewDetail(template)">{{ $t('bpm.templates.card.viewDetail') }}</el-dropdown-item>
                  <el-dropdown-item @click="handleCreateFromTemplate(template)"
                    >{{ $t('bpm.templates.card.createFromTemplate') }}</el-dropdown-item
                  >
                  <el-dropdown-item
                    divided
                    style="color: #f56c6c"
                    @click="handleDeleteTemplate(template)"
                    >{{ $t('bpm.templates.card.deleteTemplate') }}</el-dropdown-item
                  >
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
          <div class="template-body">
            <h3 class="template-name">{{ template.template_name }}</h3>
            <p class="template-desc">{{ template.description || $t('bpm.templates.card.noDescription') }}</p>
            <div class="template-meta">
              <el-tag size="small">{{ getCategoryText(template.category) }}</el-tag>
              <span class="usage-count">{{ $t('bpm.templates.card.usageCount', { count: template.usage_count }) }}</span>
            </div>
          </div>
          <div class="template-footer">
            <span class="template-time">{{ template.created_at }}</span>
            <el-button type="primary" size="small" @click="handleCreateFromTemplate(template)"
              >{{ $t('bpm.templates.card.useThisTemplate') }}</el-button
            >
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-empty v-if="!loading && templates.length === 0" :description="$t('bpm.templates.empty')" />

    <div v-if="total > 0" class="pagination-wrapper">
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[8, 16, 32]"
        layout="total, sizes, prev, pager, next"
        :aria-label="$t('bpm.templates.paginationAriaLabel')"
        @size-change="handleSizeChange"
        @current-change="handlePageChange"
      />
    </div>

    <el-dialog v-model="detailDialogVisible" :title="$t('bpm.templates.detailDialog.title')" width="700px" destroy-on-close :aria-label="$t('bpm.templates.detailDialog.ariaLabel')">
      <div v-if="currentTemplate" class="template-detail">
        <el-descriptions :column="2" border :aria-label="$t('bpm.templates.detailDialog.descriptionsAriaLabel')">
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.templateName')">{{
            currentTemplate.template_name
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.templateCategory')">{{
            getCategoryText(currentTemplate.category)
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.templateKey')">{{
            currentTemplate.template_key
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.usageCount')">{{
            currentTemplate.usage_count
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.createdAt')">{{
            currentTemplate.created_at
          }}</el-descriptions-item>
          <el-descriptions-item :label="$t('bpm.templates.detailDialog.description')" :span="2">{{
            currentTemplate.description || '-'
          }}</el-descriptions-item>
        </el-descriptions>
        <div v-if="currentTemplate.process_definition" class="process-preview">
          <h4>{{ $t('bpm.templates.detailDialog.nodePreview') }}</h4>
          <el-table
            :data="currentTemplate.process_definition.nodes || []"
            size="small"
            style="margin-top: 12px"
            :aria-label="$t('bpm.templates.detailDialog.nodePreviewAriaLabel')"
          >
            <el-table-column prop="type" :label="$t('bpm.templates.detailDialog.nodeType')" width="120">
              <template #default="{ row }">
                <el-tag size="small">{{ getNodeTypeName(row.type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="name" :label="$t('bpm.templates.detailDialog.nodeName')" min-width="150" />
            <el-table-column prop="assignee_type" :label="$t('bpm.templates.detailDialog.assigneeType')" width="120">
              <template #default="{ row }">
                <span v-if="row.assignee_type">{{ getAssigneeTypeText(row.assignee_type) }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="assignee_value" :label="$t('bpm.templates.detailDialog.assigneeValue')" min-width="120" />
          </el-table>
        </div>
      </div>
      <template #footer>
        <el-button @click="detailDialogVisible = false">{{ $t('bpm.templates.detailDialog.close') }}</el-button>
        <el-button type="primary" @click="handleCreateFromTemplate(currentTemplate)"
          >{{ $t('bpm.templates.detailDialog.createFromTemplate') }}</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="createDialogVisible" :title="$t('bpm.templates.createDialog.title')" width="500px" destroy-on-close :aria-label="$t('bpm.templates.createDialog.ariaLabel')">
      <el-form :model="createForm" label-width="100px" :aria-label="$t('bpm.templates.createDialog.formAriaLabel')">
        <el-form-item :label="$t('bpm.templates.createDialog.templateName')">
          <span>{{ currentTemplate?.template_name }}</span>
        </el-form-item>
        <el-form-item :label="$t('bpm.templates.createDialog.processName')">
          <el-input v-model="createForm.process_name" :placeholder="$t('bpm.templates.createDialog.processNamePlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">{{ $t('bpm.templates.createDialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="confirmCreateFromTemplate"
          >{{ $t('bpm.templates.createDialog.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import type { Component } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  MoreFilled,
  Document,
  ShoppingBag,
  Money,
  User,
  TrendCharts,
  Connection,
} from '@element-plus/icons-vue'
// D14 Batch 5b：原 bpmEnhancedApi 对象已转风格 B 函数
import { createBpmFromTemplate, deleteBpmTemplate } from '@/api/bpm-enhanced'
import type { ProcessTemplate } from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const submitLoading = ref(false)
const currentTemplate = ref<ProcessTemplate | null>(null)

const filterForm = reactive({ category: '' })

const detailDialogVisible = ref(false)
const createDialogVisible = ref(false)
const createForm = reactive({ process_name: '' })

const getCategoryText = (category: string) => {
  const map: Record<string, string> = {
    sales: t('bpm.templates.category.sales'),
    purchase: t('bpm.templates.category.purchase'),
    finance: t('bpm.templates.category.finance'),
    hr: t('bpm.templates.category.hr'),
    production: t('bpm.templates.category.production'),
    common: t('bpm.templates.category.common'),
  }
  return map[category] || category
}

const getCategoryIcon = (category: string): Component => {
  const map: Record<string, Component> = {
    sales: TrendCharts,
    purchase: ShoppingBag,
    finance: Money,
    hr: User,
    production: Connection,
    common: Document,
  }
  return map[category] || Document
}

const getNodeTypeName = (type: string) => {
  const map: Record<string, string> = {
    start: t('bpm.templates.nodeType.start'),
    end: t('bpm.templates.nodeType.end'),
    approval: t('bpm.templates.nodeType.approval'),
    condition: t('bpm.templates.nodeType.condition'),
    notify: t('bpm.templates.nodeType.notify'),
  }
  return map[type] || type
}

const getAssigneeTypeText = (type: string) => {
  const map: Record<string, string> = {
    user: t('bpm.templates.assigneeType.user'),
    role: t('bpm.templates.assigneeType.role'),
    department: t('bpm.templates.assigneeType.department'),
    dynamic: t('bpm.templates.assigneeType.dynamic'),
  }
  return map[type] || type
}

// 批次 277：接入 useTableApi，消除手写 templates/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: templates,
  loading,
  page,
  pageSize,
  total,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<ProcessTemplate>({
  url: '/bpm/templates',
  defaultPageSize: 12,
  onError: (err: unknown) => logger.error(String(err)),
})

// 批次 277：同步筛选条件到 useTableApi.queryParams 并刷新
const syncQueryParams = () => {
  setQueryParam('category', filterForm.category || undefined)
}

const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

const handleResetFilter = () => {
  filterForm.category = ''
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 分页（useTableApi 自动 watch page/pageSize 变化触发重载）
const handlePageChange = (p: number) => {
  page.value = p
}

const handleSizeChange = (s: number) => {
  pageSize.value = s
  page.value = 1
}

const handleViewDetail = (row: ProcessTemplate) => {
  currentTemplate.value = row
  detailDialogVisible.value = true
}

const handleCreateFromTemplate = (row: ProcessTemplate | null) => {
  if (!row && currentTemplate.value) row = currentTemplate.value
  if (!row) return
  currentTemplate.value = row
  createForm.process_name = row.template_name
  detailDialogVisible.value = false
  createDialogVisible.value = true
}

const confirmCreateFromTemplate = async () => {
  if (!currentTemplate.value) return
  submitLoading.value = true
  try {
    const data =
      createForm.process_name !== currentTemplate.value.template_name
        ? { process_name: createForm.process_name }
        : undefined
    await createBpmFromTemplate(currentTemplate.value.id, data)
    ElMessage.success(t('bpm.templates.message.createSuccess'))
    createDialogVisible.value = false
  } catch (e) {
    logger.error(String(e))
  } finally {
    submitLoading.value = false
  }
}

const handleDeleteTemplate = async (row: ProcessTemplate) => {
  try {
    await ElMessageBox.confirm(t('bpm.templates.message.deleteConfirm', { name: row.template_name }), t('bpm.templates.message.deleteConfirmTitle'), {
      type: 'warning',
    })
    await deleteBpmTemplate(row.id)
    ElMessage.success(t('bpm.templates.message.deleteSuccess'))
    fetchData()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}
</script>

<style scoped>
.bpm-templates-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.filter-card {
  margin-bottom: 20px;
}
.filter-form {
  margin-bottom: 0;
}
.template-col {
  margin-bottom: 20px;
}
.template-card {
  border-radius: 12px;
  transition: all 0.3s ease;
  height: 100%;
}
.template-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.template-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}
.template-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  color: white;
}
.icon-sales {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.icon-purchase {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.icon-finance {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.icon-hr {
  background: linear-gradient(135deg, #fa709a 0%, #fee140 100%);
}
.icon-production {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.icon-common {
  background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
}
.template-actions {
  cursor: pointer;
  color: #909399;
}
.template-body {
  margin-bottom: 16px;
}
.template-name {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 8px 0;
}
.template-desc {
  font-size: 13px;
  color: #909399;
  margin: 0 0 12px 0;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
.template-meta {
  display: flex;
  align-items: center;
  gap: 12px;
}
.usage-count {
  font-size: 12px;
  color: #909399;
}
.template-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding-top: 12px;
  border-top: 1px solid #ebeef5;
}
.template-time {
  font-size: 12px;
  color: #c0c4cc;
}
.pagination-wrapper {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
.template-detail {
  padding: 8px 0;
}
.process-preview {
  margin-top: 20px;
}
.process-preview h4 {
  font-size: 14px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 8px 0;
}
</style>
