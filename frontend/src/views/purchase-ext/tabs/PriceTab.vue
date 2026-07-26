<!--
  PriceTab.vue - 采购价格 Tab
  来源：原 purchase-ext/index.vue 中 采购价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="price-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('purchaseExt.priceTab.title') }}</h2>
      <el-button type="primary" @click="openPriceDialog()">
        <el-icon><Plus /></el-icon> {{ t('purchaseExt.priceTab.create') }}
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="priceQuery" :aria-label="t('purchaseExt.priceTab.filterAria')">
        <el-form-item :label="t('purchaseExt.priceTab.product')">
          <el-input v-model="priceQuery.product_name" :placeholder="t('purchaseExt.priceTab.productNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="t('purchaseExt.priceTab.supplier')">
          <el-input v-model="priceQuery.supplier_name" :placeholder="t('purchaseExt.priceTab.supplierNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="t('purchaseExt.priceTab.status')">
          <el-select v-model="priceQuery.status" :placeholder="t('purchaseExt.priceTab.statusPlaceholder')" clearable>
            <el-option :label="t('purchaseExt.priceTab.statusActive')" value="active" />
            <el-option :label="t('purchaseExt.priceTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchPurchasePrices">{{ t('purchaseExt.priceTab.query') }}</el-button>
          <el-button @click="resetPriceQuery">{{ t('purchaseExt.priceTab.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table v-loading="priceLoading" :data="purchasePrices" stripe :aria-label="t('purchaseExt.priceTab.listAria')">
        <el-table-column prop="product_name" :label="t('purchaseExt.priceTab.colProductName')" min-width="150" />
        <el-table-column prop="product_code" :label="t('purchaseExt.priceTab.colProductCode')" width="120" />
        <el-table-column prop="supplier_name" :label="t('purchaseExt.priceTab.colSupplier')" min-width="150" />
        <el-table-column prop="price" :label="t('purchaseExt.priceTab.colPrice')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="currency" :label="t('purchaseExt.priceTab.colCurrency')" width="80" />
        <el-table-column prop="unit" :label="t('purchaseExt.priceTab.colUnit')" width="80" />
        <el-table-column prop="effective_date" :label="t('purchaseExt.priceTab.colEffectiveDate')" width="120" />
        <el-table-column prop="expiry_date" :label="t('purchaseExt.priceTab.colExpiryDate')" width="120" />
        <el-table-column prop="status" :label="t('purchaseExt.priceTab.colStatus')" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ row.status === 'active' ? t('purchaseExt.priceTab.statusActive') : t('purchaseExt.priceTab.statusInactive') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('purchaseExt.priceTab.colOperation')" width="180" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-permission="'purchase_price:update'"
              size="small"
              link
              @click="openPriceDialog(row as unknown as PurchasePrice)"
              >{{ t('purchaseExt.priceTab.edit') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 价格编辑对话框 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceForm.id ? t('purchaseExt.priceTab.editTitle') : t('purchaseExt.priceTab.createTitle')"
      width="600px"
      :aria-label="t('purchaseExt.priceTab.dialogAria')"
    >
      <el-form ref="priceFormRef" :model="priceForm" :rules="priceRules" label-width="100px" :aria-label="t('purchaseExt.priceTab.formAria')">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.priceTab.colProductName')" prop="product_name">
              <el-input v-model="priceForm.product_name" :placeholder="t('purchaseExt.priceTab.productNamePlaceholder')" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.priceTab.colProductCode')" prop="product_code">
              <el-input v-model="priceForm.product_code" :placeholder="t('purchaseExt.priceTab.colProductCode')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('purchaseExt.priceTab.colSupplier')" prop="supplier_name">
          <el-input v-model="priceForm.supplier_name" :placeholder="t('purchaseExt.priceTab.supplierNamePlaceholder')" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.priceTab.colPrice')" prop="price">
              <el-input-number
                v-model="priceForm.price"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.priceTab.colCurrency')" prop="currency">
              <el-select v-model="priceForm.currency" :placeholder="t('purchaseExt.priceTab.currencyPlaceholder')" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.priceTab.colUnit')" prop="unit">
              <el-input v-model="priceForm.unit" :placeholder="t('purchaseExt.priceTab.unitPlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.priceTab.colEffectiveDate')" prop="effective_date">
              <el-date-picker
                v-model="priceForm.effective_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.priceTab.colExpiryDate')" prop="expiry_date">
              <el-date-picker
                v-model="priceForm.expiry_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('purchaseExt.priceTab.colStatus')">
          <el-select v-model="priceForm.status" :placeholder="t('purchaseExt.priceTab.statusPlaceholder')" style="width: 100%">
            <el-option :label="t('purchaseExt.priceTab.statusActive')" value="active" />
            <el-option :label="t('purchaseExt.priceTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('purchaseExt.priceTab.remark')" prop="remark">
          <el-input v-model="priceForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">{{ t('purchaseExt.priceTab.cancelBtn') }}</el-button>
        <el-button type="primary" :loading="priceSubmitLoading" @click="submitPrice"
          >{{ t('purchaseExt.priceTab.confirmBtn') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
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

const { t } = useI18n({ useScope: 'global' })

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
    ElMessage.error(err.message || t('purchaseExt.priceTab.fetchFailed'))
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
  product_name: [{ required: true, message: t('purchaseExt.priceTab.ruleProductName'), trigger: 'blur' }],
  supplier_name: [{ required: true, message: t('purchaseExt.priceTab.ruleSupplierName'), trigger: 'blur' }],
  price: [{ required: true, message: t('purchaseExt.priceTab.rulePrice'), trigger: 'blur' }],
  effective_date: [{ required: true, message: t('purchaseExt.priceTab.ruleEffectiveDate'), trigger: 'change' }],
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
      ElMessage.success(t('purchaseExt.priceTab.updateSuccess'))
    } else {
      await createPurchasePrice(priceForm)
      ElMessage.success(t('purchaseExt.priceTab.createSuccess'))
    }
    priceDialogVisible.value = false
    fetchPurchasePrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('purchaseExt.priceTab.operationFailed'))
  } finally {
    priceSubmitLoading.value = false
  }
}

defineExpose({ refresh: fetchPurchasePrices })

onMounted(() => {
  fetchPurchasePrices()
})
</script>
