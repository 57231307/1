<template>
  <div class="sales-price-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售价格管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售价格</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建价格
        </el-button>
        <el-button @click="handleStrategy">
          <el-icon><Setting /></el-icon>
          价格策略
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
            placeholder="产品名称/客户名称"
            clearable
            @clear="handleQuery"
          />
        </el-form-item>
        <el-form-item label="客户">
          <el-select
            v-model="queryParams.customer_id"
            placeholder="选择客户"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="产品">
          <el-select
            v-model="queryParams.product_id"
            placeholder="选择产品"
            clearable
            filterable
            @change="handleQuery"
          >
            <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="价格状态">
          <el-select
            v-model="queryParams.status"
            placeholder="选择状态"
            clearable
            @change="handleQuery"
          >
            <el-option label="待审批" value="pending" />
            <el-option label="已生效" value="active" />
            <el-option label="已过期" value="expired" />
            <el-option label="已停用" value="inactive" />
          </el-select>
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
      <el-table v-loading="loading" :data="priceList" border stripe>
        <el-table-column type="index" label="序号" width="60" align="center" />
        <el-table-column
          prop="product_name"
          label="产品名称"
          min-width="150"
          show-overflow-tooltip
        />
        <el-table-column prop="customer_name" label="客户" width="150" show-overflow-tooltip />
        <el-table-column prop="price" label="销售价格" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="currency" label="币种" width="80" align="center" />
        <el-table-column prop="unit" label="单位" width="80" align="center" />
        <el-table-column prop="min_order_qty" label="最小订购量" width="100" align="right" />
        <el-table-column prop="price_type" label="价格类型" width="100" align="center">
          <template #default="{ row }">
            <el-tag>{{ getPriceTypeLabel(row.price_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="price_level" label="价格等级" width="100" align="center" />
        <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
        <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" align="center" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="handleView(row as any)"
              >查看</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="primary"
              link
              size="small"
              @click="handleEdit(row as any)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="handleApprove(row as any)"
              >审批</el-button
            >
            <el-button type="info" link size="small" @click="handleHistory(row as any)"
              >历史</el-button
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
      width="700px"
      :close-on-click-modal="false"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品" prop="product_id">
              <el-select v-model="formData.product_id" placeholder="请选择产品" filterable>
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select
                v-model="formData.customer_id"
                placeholder="请选择客户"
                filterable
                clearable
              >
                <el-option
                  v-for="c in customers"
                  :key="c.id"
                  :label="c.customer_name"
                  :value="c.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="销售价格" prop="price">
              <el-input-number
                v-model="formData.price"
                :precision="6"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="币种" prop="currency">
              <el-select v-model="formData.currency" placeholder="请选择币种">
                <el-option label="人民币" value="CNY" />
                <el-option label="美元" value="USD" />
                <el-option label="欧元" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="单位" prop="unit">
              <el-select v-model="formData.unit" placeholder="请选择单位">
                <el-option label="米" value="meter" />
                <el-option label="公斤" value="kg" />
                <el-option label="件" value="piece" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最小订购量" prop="min_order_qty">
              <el-input-number
                v-model="formData.min_order_qty"
                :precision="2"
                :min="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="价格类型" prop="price_type">
              <el-select v-model="formData.price_type" placeholder="请选择价格类型">
                <el-option label="标准价" value="STANDARD" />
                <el-option label="协议价" value="AGREED" />
                <el-option label="促销价" value="PROMOTION" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="价格等级" prop="price_level">
              <el-select v-model="formData.price_level" placeholder="请选择价格等级">
                <el-option label="A级" value="A" />
                <el-option label="B级" value="B" />
                <el-option label="C级" value="C" />
                <el-option label="D级" value="D" />
              </el-select>
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
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="formData.remarks" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmitForm">确定</el-button>
      </template>
    </el-dialog>

    <!-- 历史记录对话框 -->
    <el-dialog v-model="historyVisible" title="价格历史" width="800px">
      <el-table :data="historyList" border stripe>
        <el-table-column prop="price" label="销售价格" width="120" align="right">
          <template #default="{ row }">
            {{ formatCurrency(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="effective_date" label="生效日期" width="120" align="center" />
        <el-table-column prop="expiry_date" label="到期日期" width="120" align="center" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" width="180" align="center" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Download, Setting, Search, Refresh } from '@element-plus/icons-vue'
import {
  listSalesPrices,
  createSalesPrice,
  updateSalesPrice,
  approveSalesPrice,
  getPriceHistory,
} from '@/api/sales-price'
import type { SalesPrice } from '@/api/sales-price'
import { customerApi } from '@/api/customer'
import type { Customer } from '@/api/customer'
import { productApi } from '@/api/product'
import type { Product } from '@/api/product'
import { logger } from '@/utils/logger'

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_id: undefined as number | undefined,
  product_id: undefined as number | undefined,
  status: '',
})

// 列表数据
const loading = ref(false)
const priceList = ref<SalesPrice[]>([])
const total = ref(0)

// 客户和产品列表
const customers = ref<Customer[]>([])
const products = ref<Product[]>([])

// 对话框
const dialogVisible = ref(false)
const dialogTitle = ref('')
const formRef = ref()

// 历史记录
const historyVisible = ref(false)
const historyList = ref<SalesPrice[]>([])

// 表单数据
const formData = reactive({
  id: undefined as number | undefined,
  product_id: undefined as number | undefined,
  customer_id: undefined as number | undefined,
  price: 0,
  currency: 'CNY',
  unit: 'meter',
  min_order_qty: 0,
  price_type: 'STANDARD',
  price_level: '',
  effective_date: '',
  expiry_date: '',
  remarks: '',
})

// 表单验证规则
const formRules = {
  product_id: [{ required: true, message: '请选择产品', trigger: 'change' }],
  price: [{ required: true, message: '请输入销售价格', trigger: 'blur' }],
  currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
  unit: [{ required: true, message: '请选择单位', trigger: 'change' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
  price_type: [{ required: true, message: '请选择价格类型', trigger: 'change' }],
}

// 获取列表数据
const getList = async () => {
  loading.value = true
  try {
    const res = await listSalesPrices(queryParams)
    priceList.value = res.data?.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取销售价格列表失败')
  } finally {
    loading.value = false
  }
}

// 获取客户列表
const getCustomers = async () => {
  try {
    const res = await customerApi.list({ page: 1, page_size: 1000 })
    customers.value = res.data?.list || []
  } catch (error) {
    logger.error('获取客户列表失败:', error)
  }
}

// 获取产品列表
const getProducts = async () => {
  try {
    const res = await productApi.list({ page: 1, page_size: 1000 })
    products.value = res.data?.list || []
  } catch (error) {
    logger.error('获取产品列表失败:', error)
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
  queryParams.customer_id = undefined
  queryParams.product_id = undefined
  queryParams.status = ''
  handleQuery()
}

// 新建
const handleCreate = () => {
  dialogTitle.value = '新建销售价格'
  Object.assign(formData, {
    id: undefined,
    product_id: undefined,
    customer_id: undefined,
    price: 0,
    currency: 'CNY',
    unit: 'meter',
    min_order_qty: 0,
    price_type: 'STANDARD',
    price_level: '',
    effective_date: '',
    expiry_date: '',
    remarks: '',
  })
  dialogVisible.value = true
}

// 查看
const handleView = (row: any) => {
  ElMessageBox.alert(
    `<div>
      <p><strong>产品名称：</strong>${row.product_name}</p>
      <p><strong>客户：</strong>${row.customer_name || '-'}</p>
      <p><strong>销售价格：</strong>${formatCurrency(row.price)}</p>
      <p><strong>币种：</strong>${row.currency}</p>
      <p><strong>单位：</strong>${row.unit}</p>
      <p><strong>最小订购量：</strong>${row.min_order_qty || '-'}</p>
      <p><strong>价格类型：</strong>${getPriceTypeLabel(row.price_type)}</p>
      <p><strong>价格等级：</strong>${row.price_level || '-'}</p>
      <p><strong>生效日期：</strong>${row.effective_date || '-'}</p>
      <p><strong>到期日期：</strong>${row.expiry_date || '-'}</p>
      <p><strong>备注：</strong>${row.remarks || '-'}</p>
    </div>`,
    '价格详情',
    { dangerouslyUseHTMLString: true, confirmButtonText: '关闭' }
  )
}

// 编辑
const handleEdit = (row: any) => {
  dialogTitle.value = '编辑销售价格'
  Object.assign(formData, row)
  dialogVisible.value = true
}

// 审批
const handleApprove = async (row: any) => {
  try {
    await ElMessageBox.confirm('确认审批通过该价格？', '提示', { type: 'warning' })
    await approveSalesPrice(row.id)
    ElMessage.success('审批成功')
    getList()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '审批失败')
    }
  }
}

// 历史记录
const handleHistory = async (row: any) => {
  try {
    const res = await getPriceHistory(row.product_id)
    historyList.value = res.data || []
    historyVisible.value = true
  } catch (error: any) {
    ElMessage.error(error.message || '获取历史记录失败')
  }
}

// 价格策略
const handleStrategy = () => {
  ElMessage.info('价格策略功能开发中')
}

// 导出
const handleExport = () => {
  const csvContent = [
    [
      '产品名称',
      '客户',
      '价格',
      '币种',
      '单位',
      '最小订购量',
      '价格类型',
      '价格等级',
      '生效日期',
      '到期日期',
      '状态',
    ],
    ...priceList.value.map((item: any) => [
      item.product_name,
      item.customer_name || '',
      item.price,
      item.currency,
      item.unit,
      item.min_order_qty || '',
      getPriceTypeLabel(item.price_type),
      item.price_level || '',
      item.effective_date || '',
      item.expiry_date || '',
      getStatusLabel(item.status),
    ]),
  ]
    .map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob(['\ufeff' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `销售价格_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

// 提交表单
const handleSubmitForm = async () => {
  try {
    await formRef.value?.validate()
    if (formData.id) {
      await updateSalesPrice(formData.id, formData)
    } else {
      await createSalesPrice(formData)
    }
    ElMessage.success('保存成功')
    dialogVisible.value = false
    getList()
  } catch (error: any) {
    if (error.message) {
      ElMessage.error(error.message || '操作失败')
    }
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
  return value ? `¥${value.toFixed(6)}` : '¥0.000000'
}

// 获取价格类型标签
const getPriceTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    STANDARD: '标准价',
    AGREED: '协议价',
    PROMOTION: '促销价',
  }
  return map[type] || type
}

// 获取状态类型
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'warning',
    active: 'success',
    expired: 'info',
    inactive: 'danger',
  }
  return map[status] || 'info'
}

// 获取状态标签
const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待审批',
    active: '已生效',
    expired: '已过期',
    inactive: '已停用',
  }
  return map[status] || status
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  getList()
  loadIfNot('customers', getCustomers, hasLoaded)
  loadIfNot('products', getProducts, hasLoaded)
})
</script>

<style scoped>
.sales-price-page {
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
