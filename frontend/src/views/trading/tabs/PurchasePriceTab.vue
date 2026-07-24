<!--
  PurchasePriceTab.vue - 采购价格 Tab
  来源：原 trading/index.vue 中 采购价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="purchase-price-tab">
    <div class="page-header">
      <h2 class="page-title">采购价格管理</h2>
      <el-button type="primary" @click="openPurchasePriceDialog()">
        <el-icon><Plus /></el-icon> 新建价格
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="purchasePriceLoading" :data="purchasePrices" stripe aria-label="采购价格列表">
        <el-table-column prop="product_name" label="产品" width="150" />
        <el-table-column prop="supplier_name" label="供应商" width="150" />
        <el-table-column prop="price" label="价格" width="100" align="right">
          <template #default="{ row }">{{ formatMoney(row.price) }}</template>
        </el-table-column>
        <el-table-column prop="currency" label="币种" width="80" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="effective_date" label="生效日期" width="120" />
        <el-table-column prop="expiry_date" label="失效日期" width="120" />
        <el-table-column prop="status" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? '有效' : '无效' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="openPurchasePriceDialog(row as unknown as TradingPrice)"
              >编辑</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 批次 157c P1-1 修复：采购价格编辑对话框 -->
    <el-dialog v-model="priceDialogVisible" :title="priceDialogTitle" width="520px" aria-label="采购价格编辑对话框">
      <el-form ref="priceFormRef" :model="priceForm" :rules="priceRules" label-width="100px" aria-label="采购价格表单">
        <el-form-item label="产品名称" prop="product_name">
          <el-input v-model="priceForm.product_name" placeholder="请输入产品名称" />
        </el-form-item>
        <el-form-item label="供应商名称">
          <el-input v-model="priceForm.supplier_name" placeholder="请输入供应商名称" />
        </el-form-item>
        <el-form-item label="价格" prop="price">
          <el-input-number v-model="priceForm.price" :min="0" :precision="4" style="width: 100%" />
        </el-form-item>
        <el-form-item label="币种" prop="currency">
          <el-select v-model="priceForm.currency" style="width: 100%">
            <el-option label="人民币 (CNY)" value="CNY" />
            <el-option label="美元 (USD)" value="USD" />
            <el-option label="欧元 (EUR)" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item label="单位" prop="unit">
          <el-input v-model="priceForm.unit" placeholder="如：米、千克" />
        </el-form-item>
        <el-form-item label="生效日期" prop="effective_date">
          <el-date-picker v-model="priceForm.effective_date" type="date" value-format="YYYY-MM-DD" style="width: 100%" />
        </el-form-item>
        <el-form-item label="失效日期">
          <el-date-picker v-model="priceForm.expiry_date" type="date" value-format="YYYY-MM-DD" style="width: 100%" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="priceForm.status" style="width: 100%">
            <el-option label="有效" value="active" />
            <el-option label="无效" value="inactive" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="priceSubmitting" @click="onSubmitPrice">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getTradingPriceList,
  getTradingPrice,
  createTradingPrice,
  updateTradingPrice,
  type TradingPrice,
} from '@/api/trading-price'

const purchasePrices = ref<TradingPrice[]>([])
const purchasePriceLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const fetchPurchasePrices = async () => {
  purchasePriceLoading.value = true
  try {
    const res = await getTradingPriceList({ type: 'purchase' })
    const d = res.data as
      | { list?: TradingPrice[]; items?: TradingPrice[] }
      | TradingPrice[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      purchasePrices.value = d.list || d.items || []
    } else {
      purchasePrices.value = (d as TradingPrice[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '获取采购价格失败')
  } finally {
    purchasePriceLoading.value = false
  }
}

// 批次 157c P1-1 修复：采购价格编辑/新建对话框接入 updateTradingPrice/createTradingPrice
const priceDialogVisible = ref(false)
const priceSubmitting = ref(false)
const priceDialogTitle = ref('新建采购价格')
const priceFormRef = ref<FormInstance>()
const priceEditingId = ref<number | null>(null)
const priceForm = reactive<Omit<TradingPrice, 'id'>>({
  product_name: '',
  supplier_name: '',
  price: 0,
  currency: 'CNY',
  unit: '米',
  effective_date: new Date().toISOString().slice(0, 10),
  expiry_date: '',
  status: 'active',
})
const priceRules: FormRules = {
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  price: [{ required: true, message: '请输入价格', trigger: 'blur' }],
  currency: [{ required: true, message: '请选择币种', trigger: 'change' }],
  unit: [{ required: true, message: '请输入单位', trigger: 'blur' }],
  effective_date: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const resetPriceForm = () => {
  priceEditingId.value = null
  priceForm.product_name = ''
  priceForm.supplier_name = ''
  priceForm.price = 0
  priceForm.currency = 'CNY'
  priceForm.unit = '米'
  priceForm.effective_date = new Date().toISOString().slice(0, 10)
  priceForm.expiry_date = ''
  priceForm.status = 'active'
}

const openPurchasePriceDialog = async (row?: TradingPrice) => {
  resetPriceForm()
  if (row) {
    try {
      const res = await getTradingPrice(row.id)
      const d = res.data
      if (d) {
        priceEditingId.value = d.id
        priceForm.product_name = d.product_name
        priceForm.supplier_name = d.supplier_name || ''
        priceForm.price = d.price
        priceForm.currency = d.currency
        priceForm.unit = d.unit
        priceForm.effective_date = d.effective_date
        priceForm.expiry_date = d.expiry_date || ''
        priceForm.status = d.status
      }
      priceDialogTitle.value = '编辑采购价格'
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || '获取价格详情失败')
      return
    }
  } else {
    priceDialogTitle.value = '新建采购价格'
  }
  priceDialogVisible.value = true
}

const onSubmitPrice = async () => {
  if (!priceFormRef.value) return
  await priceFormRef.value.validate(async valid => {
    if (!valid) return
    priceSubmitting.value = true
    try {
      if (priceEditingId.value !== null) {
        await updateTradingPrice(priceEditingId.value, { ...priceForm })
        ElMessage.success('更新成功')
      } else {
        await createTradingPrice({ ...priceForm, type: 'purchase' })
        ElMessage.success('创建成功')
      }
      priceDialogVisible.value = false
      fetchPurchasePrices()
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    } finally {
      priceSubmitting.value = false
    }
  })
}

defineExpose({ refresh: fetchPurchasePrices })

onMounted(() => {
  fetchPurchasePrices()
})
</script>

<style scoped>
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
</style>
