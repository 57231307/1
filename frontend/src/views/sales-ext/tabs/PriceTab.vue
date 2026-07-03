<!--
  PriceTab.vue - 销售价格 Tab
  来源：原 sales-ext/index.vue 中 销售价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="price-tab">
    <div class="page-header">
      <h2 class="page-title">销售价格管理</h2>
      <el-button type="primary" @click="openPriceDialog()">
        <el-icon><Plus /></el-icon> 新建价格
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="priceQuery">
        <el-form-item label="产品">
          <el-input v-model="priceQuery.productName" placeholder="产品名称" clearable />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="priceQuery.customerName" placeholder="客户名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="priceQuery.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchSalesPrices">查询</el-button>
          <el-button @click="resetPriceQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="priceLoading" :data="salesPrices" stripe>
        <el-table-column prop="productName" label="产品名称" min-width="150" />
        <el-table-column prop="productCode" label="产品编码" width="120" />
        <el-table-column prop="customerName" label="客户" min-width="150" />
        <el-table-column prop="price" label="价格" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="currency" label="货币" width="80" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="effectiveDate" label="生效日期" width="120" />
        <el-table-column prop="expiryDate" label="失效日期" width="120" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button v-permission="'sales_price:update'" size="small" link @click="openPriceDialog(row as unknown as SalesPrice)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approvePrice(row as unknown as SalesPrice)"
              >审批</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 扩展指令（批次 86）：补全价格编辑对话框，替换原占位符 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceForm.id ? '编辑销售价格' : '新建销售价格'"
      width="600px"
    >
      <el-form ref="priceFormRef" :model="priceForm" :rules="priceRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品名称" prop="product_name">
              <el-input v-model="priceForm.product_name" placeholder="产品名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="产品编码" prop="product_code">
              <el-input v-model="priceForm.product_code" placeholder="产品编码" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="客户" prop="customer_name">
          <el-input v-model="priceForm.customer_name" placeholder="客户名称" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="价格" prop="price">
              <el-input-number
                v-model="priceForm.price"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="货币" prop="currency">
              <el-select v-model="priceForm.currency" placeholder="货币" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单位" prop="unit">
              <el-input v-model="priceForm.unit" placeholder="单位" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="生效日期" prop="effective_date">
              <el-date-picker
                v-model="priceForm.effective_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="失效日期" prop="expiry_date">
              <el-date-picker
                v-model="priceForm.expiry_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="状态">
          <el-select v-model="priceForm.status" placeholder="状态" style="width: 100%">
            <el-option label="待审批" value="pending" />
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item label="备注" prop="remark">
          <el-input v-model="priceForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="priceSubmitLoading" @click="submitPrice"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  listSalesPrices,
  approveSalesPrice,
  createSalesPrice,
  updateSalesPrice,
  getSalesPrice,
  type SalesPrice,
} from '@/api/sales-price'

const salesPrices = ref<SalesPrice[]>([])
const priceLoading = ref(false)

const priceQuery = reactive({
  productName: '',
  customerName: '',
  status: '',
})

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchSalesPrices = async () => {
  priceLoading.value = true
  try {
    const res = await listSalesPrices(priceQuery)
    salesPrices.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取销售价格失败')
  } finally {
    priceLoading.value = false
  }
}

const resetPriceQuery = () => {
  priceQuery.productName = ''
  priceQuery.customerName = ''
  priceQuery.status = ''
  fetchSalesPrices()
}

// 扩展指令（批次 86）：补全价格编辑表单状态，替换原占位符
const priceDialogVisible = ref(false)
const priceFormRef = ref<FormInstance>()
const priceSubmitLoading = ref(false)
const priceForm = reactive({
  id: 0,
  product_id: 0,
  product_name: '',
  product_code: '',
  customer_id: 0,
  customer_name: '',
  price: 0,
  currency: 'CNY',
  unit: '',
  effective_date: '',
  expiry_date: '',
  status: 'pending' as SalesPrice['status'],
  remark: '',
})

const priceRules: FormRules = {
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  customer_name: [{ required: true, message: '请输入客户名称', trigger: 'blur' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
}

const openPriceDialog = async (row?: SalesPrice) => {
  if (row) {
    const res = await getSalesPrice(row.id)
    Object.assign(priceForm, res.data!)
  } else {
    Object.assign(priceForm, {
      id: 0,
      product_id: 0,
      product_name: '',
      product_code: '',
      customer_id: 0,
      customer_name: '',
      price: 0,
      currency: 'CNY',
      unit: '',
      effective_date: '',
      expiry_date: '',
      status: 'pending',
      remark: '',
    })
  }
  priceDialogVisible.value = true
}

const submitPrice = async () => {
  const valid = await priceFormRef.value?.validate()
  if (!valid) return

  priceSubmitLoading.value = true
  try {
    if (priceForm.id) {
      await updateSalesPrice(priceForm.id, priceForm)
      ElMessage.success('更新成功')
    } else {
      await createSalesPrice(priceForm)
      ElMessage.success('创建成功')
    }
    priceDialogVisible.value = false
    fetchSalesPrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    priceSubmitLoading.value = false
  }
}

const approvePrice = async (row: SalesPrice) => {
  try {
    await approveSalesPrice(row.id)
    ElMessage.success('审批成功')
    fetchSalesPrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  }
}

defineExpose({ refresh: fetchSalesPrices })

onMounted(() => {
  fetchSalesPrices()
})
</script>
