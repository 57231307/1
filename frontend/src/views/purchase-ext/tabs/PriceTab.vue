<!--
  PriceTab.vue - 采购价格 Tab
  来源：原 purchase-ext/index.vue 中 采购价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="price-tab">
    <div class="page-header">
      <h2 class="page-title">采购价格管理</h2>
      <el-button type="primary" @click="openPriceDialog()">
        <el-icon><Plus /></el-icon> 新建价格
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="priceQuery" aria-label="采购价格筛选表单">
        <el-form-item label="产品">
          <el-input v-model="priceQuery.product_name" placeholder="产品名称" clearable />
        </el-form-item>
        <el-form-item label="供应商">
          <el-input v-model="priceQuery.supplier_name" placeholder="供应商名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="priceQuery.status" placeholder="选择状态" clearable>
            <el-option label="启用" value="active" />
            <el-option label="禁用" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchPurchasePrices">查询</el-button>
          <el-button @click="resetPriceQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="priceLoading" :data="purchasePrices" stripe aria-label="采购价格列表">
        <el-table-column prop="product_name" label="产品名称" min-width="150" />
        <el-table-column prop="product_code" label="产品编码" width="120" />
        <el-table-column prop="supplier_name" label="供应商" min-width="150" />
        <el-table-column prop="price" label="价格" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="currency" label="货币" width="80" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="effective_date" label="生效日期" width="120" />
        <el-table-column prop="expiry_date" label="失效日期" width="120" />
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
            <el-button
              v-permission="'purchase_price:update'"
              size="small"
              link
              @click="openPriceDialog(row as unknown as PurchasePrice)"
              >编辑</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 价格编辑对话框 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceForm.id ? '编辑采购价格' : '新建采购价格'"
      width="600px"
      aria-label="采购价格编辑对话框"
    >
      <el-form ref="priceFormRef" :model="priceForm" :rules="priceRules" label-width="100px" aria-label="采购价格表单">
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
        <el-form-item label="供应商" prop="supplier_name">
          <el-input v-model="priceForm.supplier_name" placeholder="供应商名称" />
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
  getPurchasePriceList,
  getPurchasePrice,
  createPurchasePrice,
  updatePurchasePrice,
  type PurchasePrice,
} from '@/api/purchase-price'

const purchasePrices = ref<PurchasePrice[]>([])
const priceLoading = ref(false)

const priceQuery = reactive({
  product_name: '',
  supplier_name: '',
  status: '',
})

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchPurchasePrices = async () => {
  priceLoading.value = true
  try {
    const res = await getPurchasePriceList(priceQuery)
    purchasePrices.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '获取采购价格失败')
  } finally {
    priceLoading.value = false
  }
}

const resetPriceQuery = () => {
  priceQuery.product_name = ''
  priceQuery.supplier_name = ''
  priceQuery.status = ''
  fetchPurchasePrices()
}

const priceDialogVisible = ref(false)
const priceFormRef = ref<FormInstance>()
const priceSubmitLoading = ref(false)
const priceForm = reactive({
  id: 0,
  product_id: 0,
  product_name: '',
  product_code: '',
  supplier_id: 0,
  supplier_name: '',
  price: 0,
  currency: 'CNY',
  unit: '',
  effective_date: '',
  expiry_date: '',
  status: 'active' as 'active' | 'inactive',
  remark: '',
})

const priceRules: FormRules = {
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  supplier_name: [{ required: true, message: '请输入供应商名称', trigger: 'blur' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
}

const openPriceDialog = async (row?: PurchasePrice) => {
  if (row) {
    const res = await getPurchasePrice(row.id)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(priceForm, res.data)
  } else {
    Object.assign(priceForm, {
      id: 0,
      product_id: 0,
      product_name: '',
      product_code: '',
      supplier_id: 0,
      supplier_name: '',
      price: 0,
      currency: 'CNY',
      unit: '',
      effective_date: '',
      expiry_date: '',
      status: 'active',
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
      await updatePurchasePrice(priceForm.id, priceForm)
      ElMessage.success('更新成功')
    } else {
      await createPurchasePrice(priceForm)
      ElMessage.success('创建成功')
    }
    priceDialogVisible.value = false
    fetchPurchasePrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    priceSubmitLoading.value = false
  }
}

defineExpose({ refresh: fetchPurchasePrices })

onMounted(() => {
  fetchPurchasePrices()
})
</script>
