<!--
  PriceTab.vue - 销售价格 Tab
  来源：原 sales-ext/index.vue 中 销售价格 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="price-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('salesExt.priceTab.pageTitle') }}</h2>
      <el-button type="primary" @click="openPriceDialog()">
        <el-icon><Plus /></el-icon> {{ t('salesExt.priceTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover" class="filter-card">
      <el-form
        :inline="true"
        :model="priceQuery"
        :aria-label="t('salesExt.priceTab.ariaLabelFilter')"
      >
        <el-form-item :label="t('salesExt.priceTab.labelProduct')">
          <el-input
            v-model="priceQuery.productName"
            :placeholder="t('salesExt.priceTab.placeholderProductName')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('salesExt.priceTab.labelCustomer')">
          <el-input
            v-model="priceQuery.customerName"
            :placeholder="t('salesExt.priceTab.placeholderCustomerName')"
            clearable
          />
        </el-form-item>
        <el-form-item :label="t('salesExt.priceTab.labelStatus')">
          <el-select
            v-model="priceQuery.status"
            :placeholder="t('salesExt.priceTab.placeholderStatus')"
            clearable
          >
            <el-option :label="t('salesExt.priceTab.optionActive')" value="active" />
            <el-option :label="t('salesExt.priceTab.optionInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchSalesPrices">{{
            t('salesExt.priceTab.buttonSearch')
          }}</el-button>
          <el-button @click="resetPriceQuery">{{ t('salesExt.priceTab.buttonReset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
    <el-card shadow="hover">
      <el-table
        v-loading="priceLoading"
        :data="salesPrices"
        stripe
        :aria-label="t('salesExt.priceTab.ariaLabelList')"
      >
        <el-table-column
          prop="productName"
          :label="t('salesExt.priceTab.columnProductName')"
          min-width="150"
        />
        <el-table-column
          prop="productCode"
          :label="t('salesExt.priceTab.columnProductCode')"
          width="120"
        />
        <el-table-column
          prop="customerName"
          :label="t('salesExt.priceTab.columnCustomer')"
          min-width="150"
        />
        <el-table-column
          prop="price"
          :label="t('salesExt.priceTab.columnPrice')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="currency"
          :label="t('salesExt.priceTab.columnCurrency')"
          width="80"
        />
        <el-table-column prop="unit" :label="t('salesExt.priceTab.columnUnit')" width="80" />
        <el-table-column
          prop="effectiveDate"
          :label="t('salesExt.priceTab.columnEffectiveDate')"
          width="120"
        />
        <el-table-column
          prop="expiryDate"
          :label="t('salesExt.priceTab.columnExpiryDate')"
          width="120"
        />
        <el-table-column
          prop="status"
          :label="t('salesExt.priceTab.columnStatus')"
          width="80"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="row.status === 'active' ? 'success' : 'info'" size="small">
              {{
                row.status === 'active'
                  ? t('salesExt.priceTab.statusActive')
                  : t('salesExt.priceTab.statusInactive')
              }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="t('salesExt.priceTab.columnAction')" width="180" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-permission="PERMISSIONS.SALES_PRICE_UPDATE"
              size="small"
              link
              @click="openPriceDialog(row as unknown as SalesPrice)"
              >{{ t('salesExt.priceTab.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approvePrice(row as unknown as SalesPrice)"
              >{{ t('salesExt.priceTab.buttonApprove') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 扩展指令（批次 86）：补全价格编辑对话框，替换原占位符 -->
    <el-dialog
      v-model="priceDialogVisible"
      :title="priceForm.id ? t('salesExt.priceTab.titleEdit') : t('salesExt.priceTab.titleCreate')"
      width="600px"
      :aria-label="t('salesExt.priceTab.ariaLabelDialog')"
    >
      <el-form
        ref="priceFormRef"
        :model="priceForm"
        :rules="priceRules"
        label-width="100px"
        :aria-label="t('salesExt.priceTab.ariaLabelForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.priceTab.labelProductName')" prop="product_name">
              <el-input
                v-model="priceForm.product_name"
                :placeholder="t('salesExt.priceTab.placeholderProductName')"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.priceTab.labelProductCode')" prop="product_code">
              <el-input
                v-model="priceForm.product_code"
                :placeholder="t('salesExt.priceTab.placeholderProductCode')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('salesExt.priceTab.labelCustomerName')" prop="customer_name">
          <el-input
            v-model="priceForm.customer_name"
            :placeholder="t('salesExt.priceTab.placeholderCustomerName')"
          />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('salesExt.priceTab.labelPrice')" prop="price">
              <el-input-number
                v-model="priceForm.price"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('salesExt.priceTab.labelCurrency')" prop="currency">
              <el-select
                v-model="priceForm.currency"
                :placeholder="t('salesExt.priceTab.placeholderCurrency')"
                style="width: 100%"
              >
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('salesExt.priceTab.labelUnit')" prop="unit">
              <el-input
                v-model="priceForm.unit"
                :placeholder="t('salesExt.priceTab.placeholderUnit')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.priceTab.labelEffectiveDate')" prop="effective_date">
              <el-date-picker
                v-model="priceForm.effective_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.priceTab.labelExpiryDate')" prop="expiry_date">
              <el-date-picker
                v-model="priceForm.expiry_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item :label="t('salesExt.priceTab.labelStatus')">
          <el-select
            v-model="priceForm.status"
            :placeholder="t('salesExt.priceTab.placeholderStatusSelect')"
            style="width: 100%"
          >
            <el-option :label="t('salesExt.priceTab.optionPending')" value="pending" />
            <el-option :label="t('salesExt.priceTab.optionActive')" value="active" />
            <el-option :label="t('salesExt.priceTab.optionInactive')" value="inactive" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('salesExt.priceTab.labelRemark')" prop="remark">
          <el-input v-model="priceForm.remark" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="priceDialogVisible = false">{{
          t('salesExt.priceTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="priceSubmitLoading" @click="submitPrice">{{
          t('salesExt.priceTab.buttonConfirm')
        }}</el-button>
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
  getSalesPriceList,
  approveSalesPrice,
  createSalesPrice,
  updateSalesPrice,
  getSalesPrice,
  type SalesPrice,
} from '@/api/sales-price'
// Batch 462 P0-S24：引入权限码常量，与后端 sales-prices 资源对齐
import { PERMISSIONS } from '@/constants/permissions'

const { t } = useI18n({ useScope: 'global' })

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
    const res = await getSalesPriceList(priceQuery)
    salesPrices.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('salesExt.priceTab.messageFetchFailed'))
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
  product_name: [
    { required: true, message: t('salesExt.priceTab.ruleProductName'), trigger: 'blur' },
  ],
  customer_name: [
    { required: true, message: t('salesExt.priceTab.ruleCustomerName'), trigger: 'blur' },
  ],
  price: [{ required: true, message: t('salesExt.priceTab.rulePrice'), trigger: 'blur' }],
  effective_date: [
    { required: true, message: t('salesExt.priceTab.ruleEffectiveDate'), trigger: 'change' },
  ],
}

const openPriceDialog = async (row?: SalesPrice) => {
  if (row) {
    const res = await getSalesPrice(row.id)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(priceForm, res.data)
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
      ElMessage.success(t('salesExt.priceTab.messageUpdateSuccess'))
    } else {
      await createSalesPrice(priceForm)
      ElMessage.success(t('salesExt.priceTab.messageCreateSuccess'))
    }
    priceDialogVisible.value = false
    fetchSalesPrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('salesExt.priceTab.messageOperationFailed'))
  } finally {
    priceSubmitLoading.value = false
  }
}

const approvePrice = async (row: SalesPrice) => {
  try {
    await approveSalesPrice(row.id)
    ElMessage.success(t('salesExt.priceTab.messageApproveSuccess'))
    fetchSalesPrices()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('salesExt.priceTab.messageOperationFailed'))
  }
}

defineExpose({ refresh: fetchSalesPrices })

onMounted(() => {
  fetchSalesPrices()
})
</script>
