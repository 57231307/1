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
        <el-button v-permission="'suppliers:create'" type="primary" @click="handleCreate">
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
      @search="fetchData"
      @reset="handleReset"
      @update:query-params="(v: any) => Object.assign(queryParams, v)"
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
import { ref, reactive, onMounted, computed } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { supplierApi, type Supplier } from '@/api/supplier'
import { exportData } from '@/utils/export'
import { printData } from '@/utils/print'

const loading = ref(false)
const submitLoading = ref(false)
const suppliers = ref<Supplier[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const isEdit = ref(false)
const dialogRef = ref<InstanceType<typeof SupplierDialog>>()
// SupplierList 通过 dialog-mode prop 接收的当前模式（add/edit/view）
const dialogMode = ref<'add' | 'edit' | 'view'>('add')

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  grade: '',
  status: '',
})

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

const fetchData = async () => {
  loading.value = true
  try {
    const res = await supplierApi.list(queryParams)
    suppliers.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取供应商列表失败')
    suppliers.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.grade = ''
  queryParams.status = ''
  handleQuery()
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
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '删除失败')
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
  } catch (error: any) {
    ElMessage.error(error.message || '操作失败')
  } finally {
    submitLoading.value = false
  }
}

const handleExport = () => {
  exportData({
    filename: '供应商列表',
    columns: [
      { key: 'supplier_code', title: '供应商编码' },
      { key: 'supplier_name', title: '供应商名称' },
      { key: 'supplier_short_name', title: '简称' },
      { key: 'contact_phone', title: '联系电话' },
      { key: 'email', title: '邮箱' },
      { key: 'grade', title: '等级' },
      { key: 'supplier_type', title: '类型' },
      { key: 'status', title: '状态', formatter: v => (v === 'active' ? '启用' : '禁用') },
    ],
    data: suppliers.value,
  })
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
    data: suppliers.value,
  })
}

onMounted(() => {
  fetchData()
})
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
