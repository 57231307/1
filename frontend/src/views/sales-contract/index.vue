<template>
  <div class="sales-contract-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售合同管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售合同</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建合同
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
        <el-form-item label="客户">
          <el-select
            v-model="queryParams.customer_id"
            placeholder="选择客户"
            clearable
            @change="handleQuery"
          >
            <el-option v-for="c in customers" :key="c.id" :label="c.name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="合同状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="草稿" value="DRAFT" />
            <el-option label="待审批" value="PENDING" />
            <el-option label="已审批" value="APPROVED" />
            <el-option label="执行中" value="EXECUTING" />
            <el-option label="已完成" value="COMPLETED" />
            <el-option label="已取消" value="CANCELLED" />
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
        <el-table-column prop="customer_name" label="客户" width="150" show-overflow-tooltip />
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
              v-if="row.status === 'DRAFT'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
              type="success"
              link
              size="small"
              @click="handleSubmit(row)"
              >提交</el-button
            >
            <el-button
              v-if="row.status === 'PENDING'"
              type="success"
              link
              size="small"
              @click="handleApprove(row)"
              >审批</el-button
            >
            <el-button
              v-if="row.status === 'APPROVED'"
              type="warning"
              link
              size="small"
              @click="handleExecute(row)"
              >执行</el-button
            >
            <el-button
              v-if="row.status === 'DRAFT'"
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
            <el-form-item label="客户" prop="customer_id">
              <el-select v-model="formData.customer_id" placeholder="请选择客户" filterable>
                <el-option v-for="c in customers" :key="c.id" :label="c.name" :value="c.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="合同类型" prop="contract_type">
              <el-select v-model="formData.contract_type" placeholder="请选择合同类型">
                <el-option label="销售合同" value="SALES" />
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Printer, Search, Refresh } from '@element-plus/icons-vue'
import {
  listSalesContracts,
  createSalesContract,
  updateSalesContract,
  deleteSalesContract,
  approveSalesContract,
  executeSalesContract,
} from '@/api/sales-contract'
import { customerApi } from '@/api/customer'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_id: '',
  status: '',
  date_range: [],
})

// 列表数据
const loading = ref(false)
const contractList = ref([])
const total = ref(0)

// 客户列表
const customers = ref([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 表单数据
const formData = reactive({
  id: null,
  contract_no: '',
  contract_name: '',
  customer_id: '',
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
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    const { data } = await listSalesContracts(queryParams)
    contractList.value = data.items || []
    total.value = data.total || 0
  } catch (error) {
    console.error('获取销售合同列表失败:', error)
  } finally {
    loading.value = false
  }
}

// 获取客户列表
const getCustomers = async () => {
  try {
    const { data } = await customerApi.list()
    customers.value = data.list || []
  } catch (error) {
    console.error('获取客户列表失败:', error)
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
  queryParams.customer_id = ''
  queryParams.status = ''
  queryParams.date_range = []
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建销售合同'
  Object.assign(formData, {
    id: null,
    contract_no: '',
    contract_name: '',
    customer_id: '',
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

// 查看
const handleView = (row: any) => {}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑销售合同'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 提交审批
const handleSubmit = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认提交该合同审批？', '提示', { type: 'warning' })
    await approveSalesContract(row.id)
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
    await approveSalesContract(row.id)
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
    await executeSalesContract(row.id)
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
    await deleteSalesContract(row.id)
    ElMessage.success('删除成功')
    getList()
  } catch (error) {
    console.error('删除失败:', error)
  }
}

// 打印
const handlePrint = () => {
  ElMessage.success('打印成功')
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
      await updateSalesContract(formData.id, formData)
    } else {
      await createSalesContract(formData)
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
    DRAFT: 'info',
    PENDING: 'warning',
    APPROVED: 'success',
    EXECUTING: 'primary',
    COMPLETED: 'success',
    CANCELLED: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    DRAFT: '草稿',
    PENDING: '待审批',
    APPROVED: '已审批',
    EXECUTING: '执行中',
    COMPLETED: '已完成',
    CANCELLED: '已取消',
  }
  return map[status] || status
}

onMounted(() => {
  getList()
  getCustomers()
})
</script>

<style scoped>
.sales-contract-page {
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
