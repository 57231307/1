<template>
  <div class="bpm-templates-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">流程模板库</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>审批管理</el-breadcrumb-item>
          <el-breadcrumb-item>模板库</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="filterForm" class="filter-form">
        <el-form-item label="模板分类">
          <el-select
            v-model="filterForm.category"
            placeholder="全部分类"
            clearable
            style="width: 160px"
            @change="handleSearch"
          >
            <el-option label="销售模板" value="sales" />
            <el-option label="采购模板" value="purchase" />
            <el-option label="财务模板" value="finance" />
            <el-option label="人事模板" value="hr" />
            <el-option label="生产模板" value="production" />
            <el-option label="通用模板" value="common" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleResetFilter">重置</el-button>
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
                  <el-dropdown-item @click="handleViewDetail(template)">查看详情</el-dropdown-item>
                  <el-dropdown-item @click="handleCreateFromTemplate(template)"
                    >从模板创建</el-dropdown-item
                  >
                  <el-dropdown-item
                    divided
                    style="color: #f56c6c"
                    @click="handleDeleteTemplate(template)"
                    >删除模板</el-dropdown-item
                  >
                </el-dropdown-menu>
              </template>
            </el-dropdown>
          </div>
          <div class="template-body">
            <h3 class="template-name">{{ template.template_name }}</h3>
            <p class="template-desc">{{ template.description || '暂无描述' }}</p>
            <div class="template-meta">
              <el-tag size="small">{{ getCategoryText(template.category) }}</el-tag>
              <span class="usage-count">使用 {{ template.usage_count }} 次</span>
            </div>
          </div>
          <div class="template-footer">
            <span class="template-time">{{ template.created_at }}</span>
            <el-button type="primary" size="small" @click="handleCreateFromTemplate(template)"
              >使用此模板</el-button
            >
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-empty v-if="!loading && templates.length === 0" description="暂无模板数据" />

    <div v-if="pagination.total > 0" class="pagination-wrapper">
      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.page_size"
        :total="pagination.total"
        :page-sizes="[8, 16, 32]"
        layout="total, sizes, prev, pager, next"
        @size-change="fetchData"
        @current-change="fetchData"
      />
    </div>

    <el-dialog v-model="detailDialogVisible" title="模板详情" width="700px" destroy-on-close>
      <div v-if="currentTemplate" class="template-detail">
        <el-descriptions :column="2" border>
          <el-descriptions-item label="模板名称">{{
            currentTemplate.template_name
          }}</el-descriptions-item>
          <el-descriptions-item label="模板分类">{{
            getCategoryText(currentTemplate.category)
          }}</el-descriptions-item>
          <el-descriptions-item label="模板标识">{{
            currentTemplate.template_key
          }}</el-descriptions-item>
          <el-descriptions-item label="使用次数">{{
            currentTemplate.usage_count
          }}</el-descriptions-item>
          <el-descriptions-item label="创建时间">{{
            currentTemplate.created_at
          }}</el-descriptions-item>
          <el-descriptions-item label="描述" :span="2">{{
            currentTemplate.description || '-'
          }}</el-descriptions-item>
        </el-descriptions>
        <div v-if="currentTemplate.process_definition" class="process-preview">
          <h4>流程节点预览</h4>
          <el-table
            :data="currentTemplate.process_definition.nodes || []"
            size="small"
            style="margin-top: 12px"
          >
            <el-table-column prop="type" label="节点类型" width="120">
              <template #default="{ row }">
                <el-tag size="small">{{ getNodeTypeName(row.type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="name" label="节点名称" min-width="150" />
            <el-table-column prop="assignee_type" label="审批人类型" width="120">
              <template #default="{ row }">
                <span v-if="row.assignee_type">{{ getAssigneeTypeText(row.assignee_type) }}</span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="assignee_value" label="审批人值" min-width="120" />
          </el-table>
        </div>
      </div>
      <template #footer>
        <el-button @click="detailDialogVisible = false">关闭</el-button>
        <el-button type="primary" @click="handleCreateFromTemplate(currentTemplate)"
          >从模板创建</el-button
        >
      </template>
    </el-dialog>

    <el-dialog v-model="createDialogVisible" title="从模板创建流程" width="500px" destroy-on-close>
      <el-form :model="createForm" label-width="100px">
        <el-form-item label="模板名称">
          <span>{{ currentTemplate?.template_name }}</span>
        </el-form-item>
        <el-form-item label="流程名称">
          <el-input v-model="createForm.process_name" placeholder="默认使用模板名称" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="createDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="confirmCreateFromTemplate"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import type { Component } from 'vue'
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
import { bpmEnhancedApi } from '@/api/bpm-enhanced'
import type { ProcessTemplate } from '@/api/bpm-enhanced'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const templates = ref<ProcessTemplate[]>([])
const currentTemplate = ref<ProcessTemplate | null>(null)

const filterForm = reactive({ category: '' })
const pagination = reactive({ page: 1, page_size: 12, total: 0 })

const detailDialogVisible = ref(false)
const createDialogVisible = ref(false)
const createForm = reactive({ process_name: '' })

const getCategoryText = (category: string) => {
  const map: Record<string, string> = {
    sales: '销售',
    purchase: '采购',
    finance: '财务',
    hr: '人事',
    production: '生产',
    common: '通用',
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
    start: '开始',
    end: '结束',
    approval: '审批',
    condition: '条件',
    notify: '通知',
  }
  return map[type] || type
}

const getAssigneeTypeText = (type: string) => {
  const map: Record<string, string> = {
    user: '指定用户',
    role: '角色',
    department: '部门',
    dynamic: '动态',
  }
  return map[type] || type
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await bpmEnhancedApi.listTemplates({
      page: pagination.page,
      page_size: pagination.page_size,
      category: filterForm.category || undefined,
    })
    templates.value = res.data.list
    pagination.total = res.data.total
  } catch (e) {
    logger.error(String(e))
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
  handleSearch()
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
    await bpmEnhancedApi.createFromTemplate(currentTemplate.value.id, data)
    ElMessage.success('创建成功')
    createDialogVisible.value = false
  } catch (e) {
    logger.error(String(e))
  } finally {
    submitLoading.value = false
  }
}

const handleDeleteTemplate = async (row: ProcessTemplate) => {
  try {
    await ElMessageBox.confirm(`确定删除模板「${row.template_name}」吗？`, '确认', {
      type: 'warning',
    })
    await bpmEnhancedApi.deleteTemplate(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

onMounted(() => {
  fetchData()
})
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
