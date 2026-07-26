<!--
  PurchasePriceTab.vue - 采购价格 Tab
  来源：原 trading/index.vue 中 采购价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="purchase-price-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('trading.purchasePriceTab.title') }}</h2>
      <el-button type="primary" @click="openPurchasePriceDialog()">
        <el-icon><Plus /></el-icon> {{ t('trading.purchasePriceTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="purchasePriceLoading"
        :data="purchasePrices"
        stripe
        :aria-label="t('trading.purchasePriceTab.tableAriaLabel')"
      >
        <el-table-column
          prop="product_name"
          :label="t('trading.purchasePriceTab.columnProduct')"
          width="150"
        />
        <el-table-column
          prop="supplier_name"
          :label="t('trading.purchasePriceTab.columnSupplier')"
          width="150"
        />
        <el-table-column
          prop="price"
          :label="t('trading.purchasePriceTab.columnPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">{{ formatMoney(row.price) }}</template>
        </el-table-column>
        <el-table-column
          prop="currency"
          :label="t('trading.purchasePriceTab.columnCurrency')"
          width="80"
        />
        <el-table-column prop="unit" :label="t('trading.purchasePriceTab.columnUnit')" width="80" />
        <el-table-column
          prop="effective_date"
          :label="t('trading.purchasePriceTab.columnEffectiveDate')"
          width="120"
        />
        <el-table-column
          prop="expiry_date"
          :label="t('trading.purchasePriceTab.columnExpiryDate')"
          width="120"
        />
        <el-table-column
          prop="status"
          :label="t('trading.purchasePriceTab.columnStatus')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ getPriceStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('trading.purchasePriceTab.columnActions')" width="120">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="openPurchasePriceDialog(row as unknown as TradingPrice)"
              >{{ t('trading.purchasePriceTab.buttonEdit') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 批次 157c P1-1 修复：采购价格编辑对话框 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceDialogTitle"
      width="520px"
      :aria-label="t('trading.purchasePriceTab.dialogAriaLabel')"
    >
      <el-form
        ref="priceFormRef"
        :model="priceForm"
        :rules="priceRules"
        label-width="100px"
        :aria-label="t('trading.purchasePriceTab.formAriaLabel')"
      >
        <el-form-item :label="t('trading.purchasePriceTab.fieldProductName')" prop="product_name">
          <el-input
            v-model="priceForm.product_name"
            :placeholder="t('trading.purchasePriceTab.placeholderProductName')"
          />
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldSupplierName')">
          <el-input
            v-model="priceForm.supplier_name"
            :placeholder="t('trading.purchasePriceTab.placeholderSupplierName')"
          />
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldPrice')" prop="price">
          <el-input-number v-model="priceForm.price" :min="0" :precision="4" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldCurrency')" prop="currency">
          <el-select v-model="priceForm.currency" style="width: 100%">
            <el-option :label="t('trading.purchasePriceTab.currencyCny')" value="CNY" />
            <el-option :label="t('trading.purchasePriceTab.currencyUsd')" value="USD" />
            <el-option :label="t('trading.purchasePriceTab.currencyEur')" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldUnit')" prop="unit">
          <el-input
            v-model="priceForm.unit"
            :placeholder="t('trading.purchasePriceTab.placeholderUnit')"
          />
        </el-form-item>
        <el-form-item
          :label="t('trading.purchasePriceTab.fieldEffectiveDate')"
          prop="effective_date"
        >
          <el-date-picker
            v-model="priceForm.effective_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldExpiryDate')">
          <el-date-picker
            v-model="priceForm.expiry_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('trading.purchasePriceTab.fieldStatus')" prop="status">
          <el-select v-model="priceForm.status" style="width: 100%">
            <el-option :label="t('trading.purchasePriceTab.statusActive')" value="active" />
            <el-option :label="t('trading.purchasePriceTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">{{
          t('trading.purchasePriceTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="priceSubmitting" @click="onSubmitPrice">{{
          t('trading.purchasePriceTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getTradingPriceList,
  getTradingPrice,
  createTradingPrice,
  updateTradingPrice,
  type TradingPrice,
} from '@/api/trading-price'

const { t } = useI18n({ useScope: 'global' })

const purchasePrices = ref<TradingPrice[]>([])
const purchasePriceLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

/** 采购价格状态 → i18n 标签（语言切换响应） */
const getPriceStatusLabel = (status: string): string => {
  switch (status) {
    case 'active':
      return t('trading.purchasePriceTab.statusActive')
    case 'inactive':
      return t('trading.purchasePriceTab.statusInactive')
    default:
      return status
  }
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
    ElMessage.error(err.message || t('trading.purchasePriceTab.messageFetchFailed'))
  } finally {
    purchasePriceLoading.value = false
  }
}

// 批次 157c P1-1 修复：采购价格编辑/新建对话框接入 updateTradingPrice/createTradingPrice
const priceDialogVisible = ref(false)
const priceSubmitting = ref(false)
const priceDialogTitle = ref(t('trading.purchasePriceTab.dialogTitleCreate'))
const priceFormRef = ref<FormInstance>()
const priceEditingId = ref<number | null>(null)
const priceForm = reactive<Omit<TradingPrice, 'id'>>({
  product_name: '',
  supplier_name: '',
  price: 0,
  currency: 'CNY',
  unit: t('trading.purchasePriceTab.defaultUnit'),
  effective_date: new Date().toISOString().slice(0, 10),
  expiry_date: '',
  status: 'active',
})
const priceRules: FormRules = {
  product_name: [
    { required: true, message: t('trading.purchasePriceTab.validateProductName'), trigger: 'blur' },
  ],
  price: [
    { required: true, message: t('trading.purchasePriceTab.validatePrice'), trigger: 'blur' },
  ],
  currency: [
    { required: true, message: t('trading.purchasePriceTab.validateCurrency'), trigger: 'change' },
  ],
  unit: [{ required: true, message: t('trading.purchasePriceTab.validateUnit'), trigger: 'blur' }],
  effective_date: [
    {
      required: true,
      message: t('trading.purchasePriceTab.validateEffectiveDate'),
      trigger: 'change',
    },
  ],
  status: [
    { required: true, message: t('trading.purchasePriceTab.validateStatus'), trigger: 'change' },
  ],
}

const resetPriceForm = () => {
  priceEditingId.value = null
  priceForm.product_name = ''
  priceForm.supplier_name = ''
  priceForm.price = 0
  priceForm.currency = 'CNY'
  priceForm.unit = t('trading.purchasePriceTab.defaultUnit')
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
      priceDialogTitle.value = t('trading.purchasePriceTab.dialogTitleEdit')
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.purchasePriceTab.messageFetchDetailFailed'))
      return
    }
  } else {
    priceDialogTitle.value = t('trading.purchasePriceTab.dialogTitleCreate')
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
        ElMessage.success(t('trading.purchasePriceTab.messageUpdateSuccess'))
      } else {
        await createTradingPrice({ ...priceForm, type: 'purchase' })
        ElMessage.success(t('trading.purchasePriceTab.messageCreateSuccess'))
      }
      priceDialogVisible.value = false
      fetchPurchasePrices()
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.purchasePriceTab.messageOperationFailed'))
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
