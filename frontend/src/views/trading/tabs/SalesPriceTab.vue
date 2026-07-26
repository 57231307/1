<!--
  SalesPriceTab.vue - 销售价格 Tab
  来源：原 trading/index.vue 中 销售价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="sales-price-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('trading.salesPriceTab.title') }}</h2>
      <el-button type="primary" @click="openSalesPriceDialog()">
        <el-icon><Plus /></el-icon> {{ t('trading.salesPriceTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="salesPriceLoading"
        :data="salesPrices"
        stripe
        :aria-label="t('trading.salesPriceTab.tableAriaLabel')"
      >
        <el-table-column
          prop="product_name"
          :label="t('trading.salesPriceTab.columnProduct')"
          width="150"
        />
        <el-table-column
          prop="customer_name"
          :label="t('trading.salesPriceTab.columnCustomer')"
          width="150"
        />
        <el-table-column
          prop="price"
          :label="t('trading.salesPriceTab.columnPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">{{ formatMoney(row.price) }}</template>
        </el-table-column>
        <el-table-column
          prop="currency"
          :label="t('trading.salesPriceTab.columnCurrency')"
          width="80"
        />
        <el-table-column prop="unit" :label="t('trading.salesPriceTab.columnUnit')" width="80" />
        <el-table-column
          prop="effective_date"
          :label="t('trading.salesPriceTab.columnEffectiveDate')"
          width="120"
        />
        <el-table-column
          prop="expiry_date"
          :label="t('trading.salesPriceTab.columnExpiryDate')"
          width="120"
        />
        <el-table-column
          prop="status"
          :label="t('trading.salesPriceTab.columnStatus')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{ getPriceStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('trading.salesPriceTab.columnActions')" width="120">
          <template #default="{ row }">
            <el-button
              type="primary"
              link
              size="small"
              @click="openSalesPriceDialog(row as unknown as TradingPrice)"
              >{{ t('trading.salesPriceTab.buttonEdit') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 批次 157c P1-1 修复：销售价格编辑对话框 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceDialogTitle"
      width="520px"
      :aria-label="t('trading.salesPriceTab.dialogAriaLabel')"
    >
      <el-form
        ref="priceFormRef"
        :model="priceForm"
        :rules="priceRules"
        label-width="100px"
        :aria-label="t('trading.salesPriceTab.formAriaLabel')"
      >
        <el-form-item :label="t('trading.salesPriceTab.fieldProductName')" prop="product_name">
          <el-input
            v-model="priceForm.product_name"
            :placeholder="t('trading.salesPriceTab.placeholderProductName')"
          />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldCustomerName')">
          <el-input
            v-model="priceForm.customer_name"
            :placeholder="t('trading.salesPriceTab.placeholderCustomerName')"
          />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldPrice')" prop="price">
          <el-input-number v-model="priceForm.price" :min="0" :precision="4" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldCurrency')" prop="currency">
          <el-select v-model="priceForm.currency" style="width: 100%">
            <el-option :label="t('trading.salesPriceTab.currencyCny')" value="CNY" />
            <el-option :label="t('trading.salesPriceTab.currencyUsd')" value="USD" />
            <el-option :label="t('trading.salesPriceTab.currencyEur')" value="EUR" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldUnit')" prop="unit">
          <el-input
            v-model="priceForm.unit"
            :placeholder="t('trading.salesPriceTab.placeholderUnit')"
          />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldEffectiveDate')" prop="effective_date">
          <el-date-picker
            v-model="priceForm.effective_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldExpiryDate')">
          <el-date-picker
            v-model="priceForm.expiry_date"
            type="date"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('trading.salesPriceTab.fieldStatus')" prop="status">
          <el-select v-model="priceForm.status" style="width: 100%">
            <el-option :label="t('trading.salesPriceTab.statusActive')" value="active" />
            <el-option :label="t('trading.salesPriceTab.statusInactive')" value="inactive" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">{{
          t('trading.salesPriceTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="priceSubmitting" @click="onSubmitPrice">{{
          t('trading.salesPriceTab.buttonConfirm')
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

const salesPrices = ref<TradingPrice[]>([])
const salesPriceLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

/** 销售价格状态 → i18n 标签（语言切换响应） */
const getPriceStatusLabel = (status: string): string => {
  switch (status) {
    case 'active':
      return t('trading.salesPriceTab.statusActive')
    case 'inactive':
      return t('trading.salesPriceTab.statusInactive')
    default:
      return status
  }
}

const fetchSalesPrices = async () => {
  salesPriceLoading.value = true
  try {
    const res = await getTradingPriceList({ type: 'sales' })
    const d = res.data as
      | { list?: TradingPrice[]; items?: TradingPrice[] }
      | TradingPrice[]
      | undefined
    if (d && typeof d === 'object' && !Array.isArray(d)) {
      salesPrices.value = d.list || d.items || []
    } else {
      salesPrices.value = (d as TradingPrice[]) || []
    }
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || t('trading.salesPriceTab.messageFetchFailed'))
  } finally {
    salesPriceLoading.value = false
  }
}

// 批次 157c P1-1 修复：销售价格编辑/新建对话框接入 updateTradingPrice/createTradingPrice
const priceDialogVisible = ref(false)
const priceSubmitting = ref(false)
const priceDialogTitle = ref(t('trading.salesPriceTab.dialogTitleCreate'))
const priceFormRef = ref<FormInstance>()
const priceEditingId = ref<number | null>(null)
const priceForm = reactive<Omit<TradingPrice, 'id'>>({
  product_name: '',
  customer_name: '',
  price: 0,
  currency: 'CNY',
  unit: t('trading.salesPriceTab.defaultUnit'),
  effective_date: new Date().toISOString().slice(0, 10),
  expiry_date: '',
  status: 'active',
})
const priceRules: FormRules = {
  product_name: [
    { required: true, message: t('trading.salesPriceTab.validateProductName'), trigger: 'blur' },
  ],
  price: [{ required: true, message: t('trading.salesPriceTab.validatePrice'), trigger: 'blur' }],
  currency: [
    { required: true, message: t('trading.salesPriceTab.validateCurrency'), trigger: 'change' },
  ],
  unit: [{ required: true, message: t('trading.salesPriceTab.validateUnit'), trigger: 'blur' }],
  effective_date: [
    {
      required: true,
      message: t('trading.salesPriceTab.validateEffectiveDate'),
      trigger: 'change',
    },
  ],
  status: [
    { required: true, message: t('trading.salesPriceTab.validateStatus'), trigger: 'change' },
  ],
}

const resetPriceForm = () => {
  priceEditingId.value = null
  priceForm.product_name = ''
  priceForm.customer_name = ''
  priceForm.price = 0
  priceForm.currency = 'CNY'
  priceForm.unit = t('trading.salesPriceTab.defaultUnit')
  priceForm.effective_date = new Date().toISOString().slice(0, 10)
  priceForm.expiry_date = ''
  priceForm.status = 'active'
}

const openSalesPriceDialog = async (row?: TradingPrice) => {
  resetPriceForm()
  if (row) {
    try {
      const res = await getTradingPrice(row.id)
      const d = res.data
      if (d) {
        priceEditingId.value = d.id
        priceForm.product_name = d.product_name
        priceForm.customer_name = d.customer_name || ''
        priceForm.price = d.price
        priceForm.currency = d.currency
        priceForm.unit = d.unit
        priceForm.effective_date = d.effective_date
        priceForm.expiry_date = d.expiry_date || ''
        priceForm.status = d.status
      }
      priceDialogTitle.value = t('trading.salesPriceTab.dialogTitleEdit')
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.salesPriceTab.messageFetchDetailFailed'))
      return
    }
  } else {
    priceDialogTitle.value = t('trading.salesPriceTab.dialogTitleCreate')
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
        ElMessage.success(t('trading.salesPriceTab.messageUpdateSuccess'))
      } else {
        await createTradingPrice({ ...priceForm, type: 'sales' })
        ElMessage.success(t('trading.salesPriceTab.messageCreateSuccess'))
      }
      priceDialogVisible.value = false
      fetchSalesPrices()
    } catch (e) {
      const err = e as { message?: string }
      ElMessage.error(err.message || t('trading.salesPriceTab.messageOperationFailed'))
    } finally {
      priceSubmitting.value = false
    }
  })
}

defineExpose({ refresh: fetchSalesPrices })

onMounted(() => {
  fetchSalesPrices()
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
