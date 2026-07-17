<!--
  supplier/index.vue - 供应商管理主页
  P9-3 批次 E 样板 2 拆分：458 行 → ~200 行
  - SupplierList.vue: 列表 + 过滤 + 分页（P1-3-Batch-6 已拆）
  - SupplierDialog.vue: 新建/编辑/查看弹窗（本次拆）
  行为完全保持一致（仅结构重构）
-->
<template>
  <div class="supplier-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">供应商管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>基础数据</el-breadcrumb-item>
          <el-breadcrumb-item>供应商管理</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
        <el-button v-permission="PERMISSIONS.SUPPLIER_CREATE" type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建供应商
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <SupplierList
      :suppliers="suppliers"
      :total="total"
      :loading="loading"
      :query-params="queryParams"
      :dialog-mode="dialogMode"
      @search="handleSearch"
      @reset="handleReset"
      @update:query-params="handleQueryParamsUpdate"
      @add="handleAdd"
      @view="handleView"
      @edit="handleEdit"
      @delete="handleDelete"
    />

    <!-- 新增/编辑/查看对话框 -->
    <SupplierDialog
      ref="dialogRef"
      v-model:visible="dialogVisible"
      :title="dialogTitle"
      :mode="dialogMode"
      :form-data="formData"
      :submit-loading="submitLoading"
      @update:form-data="(v) => Object.assign(formData, v)"
      @close="resetForm"
      @submit="handleSubmit"
    />
  </div>
</template>

<script setup lang="ts">
import SupplierList from './SupplierList.vue'
import SupplierDialog from './SupplierDialog.vue'
import { ref, reactive, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { supplierApi, type Supplier, type SupplierQueryParams } from '@/api/supplier'
// V15 P0-S12 + P0-S15 修复（Batch 474）：供应商导出改用后端带水印 xlsx 接口
import { exportFromBackend } from '@/utils/export'
import { printData } from '@/utils/print'
import { useTableApi } from '@/composables/useTableApi'
// Batch 468 P0-S28：引入权限码常量，与后端 suppliers 资源对齐
import { PERMISSIONS } from '@/constants/permissions'

const submitLoading = ref(false)
const dialogVisible = ref(false)
const isEdit = ref(false)
const dialogRef = ref<InstanceType<typeof SupplierDialog>>()
// SupplierList 通过 dialog-mode prop 接收的当前模式（add/edit/view）
const dialogMode = ref<'add' | 'edit' | 'view'>('add')

const queryParams = reactive({
  keyword: '',
  grade: '',
  status: '',
})

// 批次 277：接入 useTableApi，消除手写 suppliers/total/loading/fetchData 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: suppliers,
  loading,
  total,
  page,
  pageSize,
  refresh: fetchData,
  setQueryParam,
} = useTableApi<Supplier>({
  url: '/purchase/suppliers',
  onError: (err: unknown) =>
    ElMessage.error((err instanceof Error ? err.message : String(err)) || '获取供应商列表失败'),
})

// 批次 277：同步筛选条件到 useTableApi.queryParams 并刷新（SupplierList 仍通过 props 接收 queryParams）
const syncQueryParams = () => {
  setQueryParam('keyword', queryParams.keyword || undefined)
  setQueryParam('grade', queryParams.grade || undefined)
  setQueryParam('status', queryParams.status || undefined)
}

// 批次 277：搜索时先同步筛选条件再刷新
const handleSearch = () => {
  syncQueryParams()
  page.value = 1
  fetchData()
}

// 批次 277：SupplierList 子组件分页变化时同步到 useTableApi 的 page/pageSize
// SupplierList 通过 update:query-params emit 包含 page/page_size 的 queryParams
const handleQueryParamsUpdate = (v: SupplierQueryParams) => {
  Object.assign(queryParams, v)
  if (v.page) page.value = v.page
  if (v.page_size) pageSize.value = v.page_size
}

// 表单数据由父组件维护（避免 SupplierDialog 子组件直接 mutation prop）
// SupplierDialog 接收 formData prop + 通过 ref.resetForm() 同步
const formData = reactive({
  id: undefined as number | undefined,
  supplier_code: '',
  supplier_name: '',
  supplier_short_name: '',
  supplier_type: '',
  credit_code: '',
  registered_address: '',
  business_address: '',
  legal_representative: '',
  registered_capital: 0,
  contact_phone: '',
  fax: '',
  website: '',
  email: '',
  main_business: '',
  taxpayer_type: '',
  bank_name: '',
  bank_account: '',
  grade: '',
  status: 'active',
  remarks: '',
})

const dialogTitle = computed(() => (isEdit.value ? '编辑供应商' : '新建供应商'))

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.grade = ''
  queryParams.status = ''
  syncQueryParams()
  page.value = 1
  fetchData()
}

/** 重置表单（通过 ref 调用 SupplierDialog.resetForm） */
const resetForm = () => {
  dialogRef.value?.resetForm()
}

const handleCreate = () => {
  resetForm()
  isEdit.value = false
  dialogMode.value = 'add'
  dialogVisible.value = true
}

const handleAdd = () => {
  handleCreate()
}

const handleView = (row: Supplier) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = false
  dialogMode.value = 'view'
  dialogVisible.value = true
}

const handleEdit = (row: Supplier) => {
  resetForm()
  Object.assign(formData, row)
  isEdit.value = true
  dialogMode.value = 'edit'
  dialogVisible.value = true
}

const handleDelete = async (row: Supplier) => {
  try {
    await ElMessageBox.confirm(`确定删除供应商 "${row.supplier_name}" 吗？`, '删除确认', {
      type: 'warning',
    })
    await supplierApi.delete(row.id)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error: unknown) {
    if (error !== 'cancel') {
      ElMessage.error((error instanceof Error ? error.message : String(error)) || '删除失败')
    }
  }
}

const handleSubmit = async () => {
  submitLoading.value = true
  try {
    if (isEdit.value) {
      await supplierApi.update(formData.id!, formData)
      ElMessage.success('更新成功')
    } else {
      await supplierApi.create(formData)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    fetchData()
  } catch (error: unknown) {
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '操作失败')
  } finally {
    submitLoading.value = false
  }
}

const handleExport = async () => {
  // V15 P0-S12 + P0-S15 修复（Batch 474）：改用后端带水印 xlsx 接口
  // - 后端 GET /purchase/suppliers/export 已注入水印（操作员/导出时间/导出条数）
  // - 行级数据权限与 list 一致
  // - 异步记录审计日志（OperationType::Export）
  // queryParams 字段与 SupplierQueryParams 对齐（keyword/grade/status）
  const params: Record<string, unknown> = {
    keyword: queryParams.keyword || undefined,
    grade: queryParams.grade || undefined,
    status: queryParams.status || undefined,
  }
  await exportFromBackend('/purchase/suppliers/export', params, 'suppliers_export')
}

const handlePrint = () => {
  printData({
    title: '供应商列表',
    columns: [
      { key: 'supplier_code', title: '供应商编码', width: '100px' },
      { key: 'supplier_name', title: '供应商名称' },
      { key: 'contact_phone', title: '联系电话', width: '120px' },
      { key: 'grade', title: '等级', width: '60px' },
      { key: 'supplier_type', title: '类型', width: '80px' },
      {
        key: 'status',
        title: '状态',
        width: '60px',
        formatter: v => (v === 'active' ? '启用' : '禁用'),
      },
    ],
    data: suppliers.value as unknown as Record<string, unknown>[],
  })
}
</script>

<style scoped>
.supplier-page {
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
.header-actions {
  display: flex;
  gap: 12px;
}
.filter-card {
  margin-bottom: 20px;
}
.table-card {
  margin-bottom: 20px;
}
.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
