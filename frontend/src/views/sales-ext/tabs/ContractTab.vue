<!--
  ContractTab.vue - 销售合同 Tab
  来源：原 sales-ext/index.vue 中 销售合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="contract-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('salesExt.contractTab.pageTitle') }}</h2>
      <el-button type="primary" @click="openContractDialog()">
        <el-icon><Plus /></el-icon> {{ t('salesExt.contractTab.buttonCreate') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table
        v-loading="contractLoading"
        :data="salesContracts"
        stripe
        :aria-label="t('salesExt.contractTab.ariaLabelList')"
      >
        <el-table-column
          prop="contract_no"
          :label="t('salesExt.contractTab.columnContractNo')"
          width="140"
        />
        <el-table-column
          prop="customer_name"
          :label="t('salesExt.contractTab.columnCustomer')"
          min-width="150"
        />
        <el-table-column
          prop="contract_date"
          :label="t('salesExt.contractTab.columnContractDate')"
          width="120"
        />
        <el-table-column
          prop="start_date"
          :label="t('salesExt.contractTab.columnStartDate')"
          width="120"
        />
        <el-table-column
          prop="end_date"
          :label="t('salesExt.contractTab.columnEndDate')"
          width="120"
        />
        <el-table-column
          prop="total_amount"
          :label="t('salesExt.contractTab.columnTotalAmount')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="status"
          :label="t('salesExt.contractTab.columnStatus')"
          width="100"
          align="center"
        >
          <template #default="{ row }">
            <el-tag :type="getContractStatusType(row.status)" size="small">
              {{ getContractStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column
          prop="createdBy"
          :label="t('salesExt.contractTab.columnCreatedBy')"
          width="100"
        />
        <el-table-column :label="t('salesExt.contractTab.columnAction')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewContract(row as unknown as SalesContract)">{{
              t('salesExt.contractTab.buttonView')
            }}</el-button>
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'sales_contract:update'"
              size="small"
              link
              @click="openContractDialog(row as unknown as SalesContract)"
              >{{ t('salesExt.contractTab.buttonEdit') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approveContract(row as unknown as SalesContract)"
              >{{ t('salesExt.contractTab.buttonApprove') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              link
              type="warning"
              @click="executeContract(row as unknown as SalesContract)"
              >{{ t('salesExt.contractTab.buttonExecute') }}</el-button
            >
            <el-button
              v-if="['draft', 'pending'].includes(row.status)"
              size="small"
              link
              type="danger"
              @click="cancelContract(row as unknown as SalesContract)"
              >{{ t('salesExt.contractTab.buttonCancel') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="contractDialogVisible"
      :title="
        contractForm.id
          ? t('salesExt.contractTab.titleEdit')
          : t('salesExt.contractTab.titleCreate')
      "
      width="800px"
      :aria-label="t('salesExt.contractTab.ariaLabelDialog')"
    >
      <el-form
        ref="contractFormRef"
        :model="contractForm"
        :rules="contractRules"
        label-width="100px"
        :aria-label="t('salesExt.contractTab.ariaLabelForm')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.contractTab.labelContractNo')" prop="contract_no">
              <el-input v-model="contractForm.contract_no" :disabled="!!contractForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.contractTab.labelCustomer')" prop="customer_name">
              <el-input
                v-model="contractForm.customer_name"
                :placeholder="t('salesExt.contractTab.placeholderCustomer')"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('salesExt.contractTab.labelContractDate')" prop="contract_date">
              <el-date-picker
                v-model="contractForm.contract_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('salesExt.contractTab.labelStartDate')" prop="start_date">
              <el-date-picker
                v-model="contractForm.start_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('salesExt.contractTab.labelEndDate')" prop="end_date">
              <el-date-picker
                v-model="contractForm.end_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('salesExt.contractTab.labelCurrency')" prop="currency">
              <el-select
                v-model="contractForm.currency"
                :placeholder="t('salesExt.contractTab.placeholderCurrency')"
                style="width: 100%"
              >
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('salesExt.contractTab.labelTotalAmount')" prop="total_amount">
              <el-input-number
                v-model="contractForm.total_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider>{{ t('salesExt.contractTab.dividerItems') }}</el-divider>
        <el-table
          :data="contractForm.items"
          border
          style="width: 100%"
          :aria-label="t('salesExt.contractTab.ariaLabelItemsEdit')"
        >
          <el-table-column
            prop="product_name"
            :label="t('salesExt.contractTab.columnProductName')"
            min-width="150"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.product_name"
                :placeholder="t('salesExt.contractTab.placeholderProductName')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="product_code"
            :label="t('salesExt.contractTab.columnProductCode')"
            width="120"
          >
            <template #default="{ row }">
              <el-input
                v-model="row.product_code"
                :placeholder="t('salesExt.contractTab.placeholderProductCode')"
              />
            </template>
          </el-table-column>
          <el-table-column
            prop="quantity"
            :label="t('salesExt.contractTab.columnQuantity')"
            width="100"
          >
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unit" :label="t('salesExt.contractTab.columnUnit')" width="80">
            <template #default="{ row }">
              <el-input
                v-model="row.unit"
                :placeholder="t('salesExt.contractTab.placeholderUnit')"
              />
            </template>
          </el-table-column>
          <el-table-column prop="price" :label="t('salesExt.contractTab.columnPrice')" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column
            prop="amount"
            :label="t('salesExt.contractTab.columnAmount')"
            width="100"
          >
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.price || 0)) }}
            </template>
          </el-table-column>
          <el-table-column :label="t('salesExt.contractTab.columnAction')" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeContractItem($index)">{{
                t('salesExt.contractTab.buttonDelete')
              }}</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addContractItem">{{
          t('salesExt.contractTab.buttonAddProduct')
        }}</el-button>
        <el-form-item :label="t('salesExt.contractTab.labelPaymentTerms')" prop="payment_terms">
          <el-input v-model="contractForm.payment_terms" type="textarea" />
        </el-form-item>
        <el-form-item :label="t('salesExt.contractTab.labelDeliveryTerms')" prop="delivery_terms">
          <el-input v-model="contractForm.delivery_terms" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contractDialogVisible = false">{{
          t('salesExt.contractTab.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="contractSubmitLoading" @click="submitContract">{{
          t('salesExt.contractTab.buttonConfirm')
        }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="contractViewVisible"
      :title="t('salesExt.contractTab.titleDetail')"
      width="800px"
      :aria-label="t('salesExt.contractTab.ariaLabelDetail')"
    >
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('salesExt.contractTab.labelContractNo')">{{
          currentContract?.contract_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelCustomer')">{{
          currentContract?.customer_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelContractDate')">{{
          currentContract?.contract_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelValidDate')"
          >{{ currentContract?.start_date }} ~ {{ currentContract?.end_date }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('salesExt.contractTab.labelCurrency')">{{
          currentContract?.currency
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelTotalAmount')">{{
          formatMoney(currentContract?.total_amount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelStatus')">
          <el-tag :type="getContractStatusType(currentContract?.status)">
            {{ getContractStatusLabel(currentContract?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelCreatedBy')">{{
          currentContract?.created_by_name
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>{{ t('salesExt.contractTab.dividerItems') }}</el-divider>
      <el-table
        :data="currentContract?.items || []"
        stripe
        :aria-label="t('salesExt.contractTab.ariaLabelItemsList')"
      >
        <el-table-column
          prop="product_name"
          :label="t('salesExt.contractTab.columnProductName')"
          min-width="150"
        />
        <el-table-column
          prop="product_code"
          :label="t('salesExt.contractTab.columnProductCode')"
          width="120"
        />
        <el-table-column
          prop="quantity"
          :label="t('salesExt.contractTab.columnQuantity')"
          width="100"
          align="right"
        />
        <el-table-column prop="unit" :label="t('salesExt.contractTab.columnUnit')" width="80" />
        <el-table-column
          prop="price"
          :label="t('salesExt.contractTab.columnPrice')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="amount"
          :label="t('salesExt.contractTab.columnAmount')"
          width="100"
          align="right"
        >
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column
          prop="remark"
          :label="t('salesExt.contractTab.columnRemark')"
          min-width="120"
        />
      </el-table>
      <el-divider>{{ t('salesExt.contractTab.dividerTerms') }}</el-divider>
      <el-descriptions :column="1" border>
        <el-descriptions-item :label="t('salesExt.contractTab.labelPaymentTerms')">{{
          currentContract?.payment_terms
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('salesExt.contractTab.labelDeliveryTerms')">{{
          currentContract?.delivery_terms
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import {
  getSalesContractList,
  getSalesContract,
  createSalesContract,
  updateSalesContract,
  approveSalesContract,
  executeSalesContract,
  cancelSalesContract,
  type SalesContract,
  type ContractItem as SalesContractItem,
} from '@/api/sales-contract'

const { t } = useI18n({ useScope: 'global' })

const salesContracts = ref<SalesContract[]>([])
const contractLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getContractStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: t('salesExt.contractTab.statusDraft'),
    pending: t('salesExt.contractTab.statusPending'),
    active: t('salesExt.contractTab.statusActive'),
    completed: t('salesExt.contractTab.statusCompleted'),
    cancelled: t('salesExt.contractTab.statusCancelled'),
  }
  return map[status || ''] || status || ''
}

const getContractStatusType = (status?: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    active: 'primary',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status || ''] || 'info'
}

const fetchSalesContracts = async () => {
  contractLoading.value = true
  try {
    const res = await getSalesContractList()
    salesContracts.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('salesExt.contractTab.messageFetchFailed'))
  } finally {
    contractLoading.value = false
  }
}

const contractDialogVisible = ref(false)
const contractFormRef = ref<FormInstance>()
const contractSubmitLoading = ref(false)
const contractForm = reactive({
  id: 0,
  contract_no: '',
  customer_id: 0,
  customer_name: '',
  contract_date: '',
  start_date: '',
  end_date: '',
  total_amount: 0,
  currency: 'CNY',
  status: 'draft' as 'draft' | 'pending' | 'active' | 'completed' | 'cancelled',
  items: [] as SalesContractItem[],
  payment_terms: '',
  delivery_terms: '',
})

const contractRules: FormRules = {
  contract_no: [
    { required: true, message: t('salesExt.contractTab.ruleContractNo'), trigger: 'blur' },
  ],
  customer_name: [
    { required: true, message: t('salesExt.contractTab.ruleCustomerName'), trigger: 'blur' },
  ],
  contract_date: [
    { required: true, message: t('salesExt.contractTab.ruleContractDate'), trigger: 'change' },
  ],
  total_amount: [
    { required: true, message: t('salesExt.contractTab.ruleTotalAmount'), trigger: 'blur' },
  ],
}

const openContractDialog = async (row?: SalesContract) => {
  if (row) {
    const res = await getSalesContract(row.id)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(contractForm, res.data)
  } else {
    Object.assign(contractForm, {
      id: 0,
      contract_no: '',
      customer_id: 0,
      customer_name: '',
      contract_date: '',
      start_date: '',
      end_date: '',
      total_amount: 0,
      currency: 'CNY',
      status: 'draft',
      items: [
        {
          id: 0,
          contract_id: 0,
          product_id: 0,
          product_name: '',
          product_code: '',
          quantity: 0,
          unit: '',
          price: 0,
          amount: 0,
          remark: '',
        },
      ],
      payment_terms: '',
      delivery_terms: '',
    })
  }
  contractDialogVisible.value = true
}

const submitContract = async () => {
  const valid = await contractFormRef.value?.validate()
  if (!valid) return
  contractSubmitLoading.value = true
  try {
    if (contractForm.id) {
      await updateSalesContract(contractForm.id, contractForm)
      ElMessage.success(t('salesExt.contractTab.messageUpdateSuccess'))
    } else {
      await createSalesContract(contractForm)
      ElMessage.success(t('salesExt.contractTab.messageCreateSuccess'))
    }
    contractDialogVisible.value = false
    fetchSalesContracts()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('salesExt.contractTab.messageOperationFailed'))
  } finally {
    contractSubmitLoading.value = false
  }
}

const contractViewVisible = ref(false)
const currentContract = ref<SalesContract | null>(null)

const viewContract = async (row: SalesContract) => {
  const res = await getSalesContract(row.id)
  // 安全检查：防止后端返回 data 为 null 时崩溃
  if (res.data) currentContract.value = res.data
  contractViewVisible.value = true
}

const approveContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm(
      t('salesExt.contractTab.confirmApprove'),
      t('salesExt.contractTab.confirmTitle'),
      { type: 'info' }
    )
    await approveSalesContract(row.id)
    ElMessage.success(t('salesExt.contractTab.messageApproveSuccess'))
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('salesExt.contractTab.messageOperationFailed'))
    }
  }
}

const executeContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm(
      t('salesExt.contractTab.confirmExecute'),
      t('salesExt.contractTab.confirmTitle'),
      { type: 'info' }
    )
    await executeSalesContract(row.id)
    ElMessage.success(t('salesExt.contractTab.messageExecuteSuccess'))
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('salesExt.contractTab.messageOperationFailed'))
    }
  }
}

const cancelContract = async (row: SalesContract) => {
  try {
    await ElMessageBox.confirm(
      t('salesExt.contractTab.confirmCancel'),
      t('salesExt.contractTab.confirmTitle'),
      { type: 'warning' }
    )
    await cancelSalesContract(row.id)
    ElMessage.success(t('salesExt.contractTab.messageCancelSuccess'))
    fetchSalesContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('salesExt.contractTab.messageOperationFailed'))
    }
  }
}

const addContractItem = () => {
  contractForm.items.push({
    id: 0,
    contract_id: 0,
    product_id: 0,
    product_name: '',
    product_code: '',
    quantity: 0,
    unit: '',
    price: 0,
    amount: 0,
    remark: '',
  })
}

const removeContractItem = (index: number) => {
  if (contractForm.items.length > 1) {
    contractForm.items.splice(index, 1)
  }
}

defineExpose({ refresh: fetchSalesContracts })

onMounted(() => {
  fetchSalesContracts()
})
</script>
