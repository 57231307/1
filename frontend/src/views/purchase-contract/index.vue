<template>
  <div class="purchase-contract-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">采购合同管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>采购管理</el-breadcrumb-item>
          <el-breadcrumb-item>采购合同</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建合同
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input
            v-model="queryParams.keyword"
            placeholder="合同编号/合同名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="供应商">
          <el-select
            v-model="queryParams.supplier_id"
            placeholder="选择供应商"
            clearable
            @change="handleQuery"
          >
            <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="合同状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="草稿" value="draft" />
            <el-option label="待审批" value="pending" />
            <el-option label="已生效" value="active" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item label="签订日期">
          <el-date-picker
            v-model="queryParams.date_range"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            @change="handleQuery"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="contractList" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column prop="contract_no" label="合同编号" width="150" show-overflow-tooltip />
        <el-table-column
          prop="contract_name"
          label="合同名称"
          min-width="200"
          show-overflow-tooltip
        />
        <el-table-column prop="supplier_name" label="供应商" width="150" show-overflow-tooltip />
        <el-table-column prop="total_amount" label="合同金额" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="signed_date" label="签订日期" width="120" align="center" />
        <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
        <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="250" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row)">查看</el-button>
            <el-button
              v-if="row.status === 'draft'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              type="success"
              link
              size="small"
              @click="handleSubmit(row)"
              >提交</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="handleExecute(row)"
              >执行</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
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
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handleCurrentChange"
        />
      </div>
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogTitle"
      width="800px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="合同编号" prop="contract_no">
              <el-input v-model="formData.contract_no" placeholder="请输入合同编号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="合同名称" prop="contract_name">
              <el-input v-model="formData.contract_name" placeholder="请输入合同名称" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商" prop="supplier_id">
              <el-select v-model="formData.supplier_id" placeholder="请选择供应商" filterable>
                <el-option
                  v-for="s in suppliers"
                  :key="s.id"
                  :label="s.supplier_name"
                  :value="s.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="合同类型" prop="contract_type">
              <el-select v-model="formData.contract_type" placeholder="请选择合同类型">
                <el-option label="采购合同" value="PURCHASE" />
                <el-option label="框架合同" value="FRAMEWORK" />
                <el-option label="补充协议" value="SUPPLEMENT" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="合同金额" prop="total_amount">
              <el-input-number
                v-model="formData.total_amount"
                :precision="2"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="签订日期" prop="signed_date">
              <el-date-picker
                v-model="formData.signed_date"
                type="date"
                placeholder="请选择签订日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="生效日期" prop="effective_date">
              <el-date-picker
                v-model="formData.effective_date"
                type="date"
                placeholder="请选择生效日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="到期日期" prop="expiry_date">
              <el-date-picker
                v-model="formData.expiry_date"
                type="date"
                placeholder="请选择到期日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="付款条件" prop="payment_terms">
              <el-input v-model="formData.payment_terms" placeholder="请输入付款条件" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="付款方式" prop="payment_method">
              <el-select v-model="formData.payment_method" placeholder="请选择付款方式">
                <el-option label="银行转账" value="BANK_TRANSFER" />
                <el-option label="支票" value="CHECK" />
                <el-option label="现金" value="CASH" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="交货日期" prop="delivery_date">
              <el-date-picker
                v-model="formData.delivery_date"
                type="date"
                placeholder="请选择交货日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="交货地点" prop="delivery_location">
              <el-input v-model="formData.delivery_location" placeholder="请输入交货地点" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 查看详情对话框 -->
    <el-dialog v-model="viewDialogVisible" title="合同详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="合同编号">{{ viewData.contract_no }}</el-descriptions-item>
        <el-descriptions-item label="合同名称">{{ viewData.contract_name }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ viewData.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="合同类型">{{ viewData.contract_type }}</el-descriptions-item>
        <el-descriptions-item label="合同金额">{{
          formatCurrency(viewData.total_amount)
        }}</el-descriptions-item>
        <el-descriptions-item label="签订日期">{{ viewData.signed_date }}</el-descriptions-item>
        <el-descriptions-item label="生效日期">{{ viewData.effective_date }}</el-descriptions-item>
        <el-descriptions-item label="到期日期">{{ viewData.expiry_date }}</el-descriptions-item>
        <el-descriptions-item label="付款条件">{{
          viewData.payment_terms || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="付款方式">{{
          viewData.payment_method || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="交货日期">{{
          viewData.delivery_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="交货地点">{{
          viewData.delivery_location || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getStatusType(viewData.status)">{{
            getStatusLabel(viewData.status)
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          viewData.remarks || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Search, Refresh } from '@element-plus/icons-vue'
import {
  listPurchaseContracts,
  createPurchaseContract,
  updatePurchaseContract,
  deletePurchaseContract,
  approvePurchaseContract,
  executePurchaseContract,
} from '@/api/purchase-contract'
import type { PurchaseContract } from '@/api/purchase-contract'
import { supplierApi } from '@/api/supplier'
import type { Supplier } from '@/api/supplier'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  supplier_id: undefined as number | undefined,
  status: '',
  date_range: [] as string[],
})

// 列表数据
const loading = ref(false)
const contractList = ref<PurchaseContract[]>([])
const total = ref(0)

// 供应商列表
const suppliers = ref<Supplier[]>([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  contract_no: '',
  contract_name: '',
  supplier_id: undefined as number | undefined,
  contract_type: '',
  total_amount: 0,
  signed_date: '',
  effective_date: '',
  expiry_date: '',
  payment_terms: '',
  payment_method: '',
  delivery_date: '',
  delivery_location: '',
  remarks: '',
})

// 表单验证规则
const formRules = {
  contract_no: [{ required: true, message: '请输入合同编号', trigger: 'blur' }],
  contract_name: [{ required: true, message: '请输入合同名称', trigger: 'blur' }],
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    const res = await listPurchaseContracts(queryParams)
    contractList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error) {
    console.error('获取采购合同列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取供应商列表
const getSuppliers = async () => {
  try {
    const res = await supplierApi.list()
    suppliers.value = res.data?.list || []
  } catch (error) {
    console.error('获取供应商列表失败:', error)
  }
}

// 查询
const handleQuery = () => {
  queryParams.page = 1
  getList()
}

// 重置
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.supplier_id = undefined
  queryParams.status = ''
  queryParams.date_range = []
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建采购合同'
  Object.assign(formData, {
    id: undefined,
    contract_no: '',
    contract_name: '',
    supplier_id: undefined,
    contract_type: '',
    total_amount: 0,
    signed_date: '',
    effective_date: '',
    expiry_date: '',
    payment_terms: '',
    payment_method: '',
    delivery_date: '',
    delivery_location: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看详情对话框
const viewDialogVisible = ref(false)
const viewData = ref<any>({})

const handleView = (row: any) => {
  viewData.value = row
  viewDialogVisible.value = true
}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑采购合同'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 提交审批
const handleSubmit = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认提交该合同审批？', '提示', { type: 'warning' })
    await approvePurchaseContract(row.id)
    ElMessage.success('提交成功')
    getList()
  } catch (error) {
    console.error('提交失败:', error)
  }
}

// 审批
const handleApprove = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认审批通过该合同？', '提示', { type: 'warning' })
    await approvePurchaseContract(row.id)
    ElMessage.success('审批成功')
    getList()
  } catch (error) {
    console.error('审批失败:', error)
  }
}

// 执行
const handleExecute = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认执行该合同？', '提示', { type: 'warning' })
    await executePurchaseContract(row.id)
    ElMessage.success('执行成功')
    getList()
  } catch (error) {
    console.error('执行失败:', error)
  }
}

// 删除
const handleDelete = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认删除该合同？', '提示', { type: 'warning' })
    await deletePurchaseContract(row.id)
    ElMessage.success('删除成功')
    getList()
  } catch (error) {
    console.error('删除失败:', error)
  }
}

// 导出
const handleExport = () => {
  ElMessage.success('导出成功')
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    if (formData.id) {
      await updatePurchaseContract(formData.id, formData)
    } else {
      await createPurchaseContract(formData)
    }
    ElMessage.success('保存成功')
    dialogVisible.value = false
    getList()
  } catch (error) {
    console.error('表单验证失败:', error)
  }
}

// 分页
const handleSizeChange = (val: number) => {
  queryParams.page_size = val
  getList()
}

const handleCurrentChange = (val: number) => {
  queryParams.page = val
  getList()
}

// 格式化货币
const formatCurrency = (value: number) => {
  return value ? `¥${value.toFixed(2)}` : '¥0.00'
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    active: 'success',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审批',
    active: '已生效',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

onMounted(() => {
  getList()
  getSuppliers()
})
</script>

<style scoped>
.purchase-contract-page {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  margin: 0;
  font-size: 24px;
  font-weight: 600;
}

.header-actions {
  display: flex;
  gap: 10px;
}

.filter-card {
  margin-bottom: 20px;
}

.filter-form {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-container {
  display: flex;
  justify-content: flex-end;
  margin-top: 20px;
}
</style>
