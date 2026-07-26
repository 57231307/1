<!--
  ContractTab.vue - 采购合同 Tab
  来源：原 purchase-ext/index.vue 中 采购合同 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="contract-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('purchaseExt.contractTab.title') }}</h2>
      <el-button type="primary" @click="openContractDialog()">
        <el-icon><Plus /></el-icon> {{ t('purchaseExt.contractTab.create') }}
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="contractLoading" :data="purchaseContracts" stripe :aria-label="t('purchaseExt.contractTab.listAria')">
        <el-table-column prop="contract_no" :label="t('purchaseExt.contractTab.colContractNo')" width="140" />
        <el-table-column prop="supplier_name" :label="t('purchaseExt.contractTab.colSupplier')" min-width="150" />
        <el-table-column prop="contract_date" :label="t('purchaseExt.contractTab.colContractDate')" width="120" />
        <el-table-column prop="start_date" :label="t('purchaseExt.contractTab.colStartDate')" width="120" />
        <el-table-column prop="end_date" :label="t('purchaseExt.contractTab.colEndDate')" width="120" />
        <el-table-column prop="total_amount" :label="t('purchaseExt.contractTab.colTotalAmount')" width="120" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.total_amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="status" :label="t('purchaseExt.contractTab.colStatus')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getContractStatusType(row.status)" size="small">
              {{ getContractStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" :label="t('purchaseExt.contractTab.colCreator')" width="100" />
        <el-table-column :label="t('purchaseExt.contractTab.colOperation')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button size="small" link @click="viewContract(row as unknown as PurchaseContract)"
              >{{ t('purchaseExt.contractTab.view') }}</el-button
            >
            <!-- P2-17 修复（批次 86 v2 复审）：编辑按钮补齐 v-permission -->
            <el-button
              v-if="row.status === 'draft'"
              v-permission="'purchase_contract:update'"
              size="small"
              link
              @click="openContractDialog(row as unknown as PurchaseContract)"
              >{{ t('purchaseExt.contractTab.edit') }}</el-button
            >
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              link
              type="success"
              @click="approveContract(row as unknown as PurchaseContract)"
              >{{ t('purchaseExt.contractTab.approve') }}</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              link
              type="warning"
              @click="executeContract(row as unknown as PurchaseContract)"
              >{{ t('purchaseExt.contractTab.execute') }}</el-button
            >
            <el-button
              v-if="['draft', 'pending'].includes(row.status)"
              size="small"
              link
              type="danger"
              @click="cancelContract(row as unknown as PurchaseContract)"
              >{{ t('purchaseExt.contractTab.cancel') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <!-- 合同编辑对话框 -->
    <el-dialog
      v-model="contractDialogVisible"
      :title="contractForm.id ? t('purchaseExt.contractTab.editTitle') : t('purchaseExt.contractTab.createTitle')"
      width="800px"
      :aria-label="t('purchaseExt.contractTab.dialogAria')"
    >
      <el-form
        ref="contractFormRef"
        :model="contractForm"
        :rules="contractRules"
        label-width="100px"
        :aria-label="t('purchaseExt.contractTab.formAria')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.contractTab.contractNo')" prop="contract_no">
              <el-input v-model="contractForm.contract_no" :disabled="!!contractForm.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.contractTab.supplier')" prop="supplier_name">
              <el-input v-model="contractForm.supplier_name" :placeholder="t('purchaseExt.contractTab.supplierPlaceholder')" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.contractTab.contractDate')" prop="contract_date">
              <el-date-picker
                v-model="contractForm.contract_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.contractTab.startDate')" prop="start_date">
              <el-date-picker
                v-model="contractForm.start_date"
                type="date"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item :label="t('purchaseExt.contractTab.endDate')" prop="end_date">
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
            <el-form-item :label="t('purchaseExt.contractTab.currency')" prop="currency">
              <el-select v-model="contractForm.currency" :placeholder="t('purchaseExt.contractTab.currencyPlaceholder')" style="width: 100%">
                <el-option label="CNY" value="CNY" />
                <el-option label="USD" value="USD" />
                <el-option label="EUR" value="EUR" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('purchaseExt.contractTab.totalAmount')" prop="total_amount">
              <el-input-number
                v-model="contractForm.total_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-divider>{{ t('purchaseExt.contractTab.detailDivider') }}</el-divider>
        <el-table :data="contractForm.items" border style="width: 100%" :aria-label="t('purchaseExt.contractTab.detailEditAria')">
          <el-table-column prop="product_name" :label="t('purchaseExt.contractTab.colProductName')" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.product_name" :placeholder="t('purchaseExt.contractTab.productNamePlaceholder')" />
            </template>
          </el-table-column>
          <el-table-column prop="product_code" :label="t('purchaseExt.contractTab.colProductCode')" width="120">
            <template #default="{ row }">
              <el-input v-model="row.product_code" :placeholder="t('purchaseExt.contractTab.productCodePlaceholder')" />
            </template>
          </el-table-column>
          <el-table-column prop="quantity" :label="t('purchaseExt.contractTab.colQuantity')" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="0" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="unit" :label="t('purchaseExt.contractTab.colUnit')" width="80">
            <template #default="{ row }">
              <el-input v-model="row.unit" :placeholder="t('purchaseExt.contractTab.unitPlaceholder')" />
            </template>
          </el-table-column>
          <el-table-column prop="price" :label="t('purchaseExt.contractTab.colPrice')" width="100">
            <template #default="{ row }">
              <el-input-number v-model="row.price" :min="0" :precision="2" style="width: 100%" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" :label="t('purchaseExt.contractTab.colAmount')" width="100">
            <template #default="{ row }">
              {{ formatMoney((row.quantity || 0) * (row.price || 0)) }}
            </template>
          </el-table-column>
          <el-table-column :label="t('purchaseExt.contractTab.colOperation')" width="80">
            <template #default="{ $index }">
              <el-button size="small" link type="danger" @click="removeContractItem($index)"
                >{{ t('purchaseExt.contractTab.delete') }}</el-button
              >
            </template>
          </el-table-column>
        </el-table>
        <el-button type="primary" link style="margin-top: 8px" @click="addContractItem"
          >{{ t('purchaseExt.contractTab.addProduct') }}</el-button
        >
        <el-form-item :label="t('purchaseExt.contractTab.paymentTerms')" prop="payment_terms">
          <el-input v-model="contractForm.payment_terms" type="textarea" />
        </el-form-item>
        <el-form-item :label="t('purchaseExt.contractTab.deliveryTerms')" prop="delivery_terms">
          <el-input v-model="contractForm.delivery_terms" type="textarea" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="contractDialogVisible = false">{{ t('purchaseExt.contractTab.cancelBtn') }}</el-button>
        <el-button type="primary" :loading="contractSubmitLoading" @click="submitContract"
          >{{ t('purchaseExt.contractTab.confirmBtn') }}</el-button
        >
      </template>
    </el-dialog>

    <!-- 合同详情对话框 -->
    <el-dialog v-model="contractViewVisible" :title="t('purchaseExt.contractTab.viewTitle')" width="800px" :aria-label="t('purchaseExt.contractTab.viewDialogAria')">
      <el-descriptions :column="2" border>
        <el-descriptions-item :label="t('purchaseExt.contractTab.contractNo')">{{
          currentContract?.contract_no
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.supplier')">{{
          currentContract?.supplier_name
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.colContractDate')">{{
          currentContract?.contract_date
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.viewEffectiveDate')"
          >{{ currentContract?.start_date }} ~ {{ currentContract?.end_date }}</el-descriptions-item
        >
        <el-descriptions-item :label="t('purchaseExt.contractTab.currency')">{{ currentContract?.currency }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.colTotalAmount')">{{
          formatMoney(currentContract?.total_amount || 0)
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.colStatus')">
          <el-tag :type="getContractStatusType(currentContract?.status)">
            {{ getContractStatusLabel(currentContract?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.colCreator')">{{
          currentContract?.created_by_name
        }}</el-descriptions-item>
      </el-descriptions>
      <el-divider>{{ t('purchaseExt.contractTab.detailDivider') }}</el-divider>
      <el-table :data="currentContract?.items || []" stripe :aria-label="t('purchaseExt.contractTab.viewDetailAria')">
        <el-table-column prop="product_name" :label="t('purchaseExt.contractTab.colProductName')" min-width="150" />
        <el-table-column prop="product_code" :label="t('purchaseExt.contractTab.colProductCode')" width="120" />
        <el-table-column prop="quantity" :label="t('purchaseExt.contractTab.colQuantity')" width="100" align="right" />
        <el-table-column prop="unit" :label="t('purchaseExt.contractTab.colUnit')" width="80" />
        <el-table-column prop="price" :label="t('purchaseExt.contractTab.colPrice')" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.price) }}
          </template>
        </el-table-column>
        <el-table-column prop="amount" :label="t('purchaseExt.contractTab.colAmount')" width="100" align="right">
          <template #default="{ row }">
            {{ formatMoney(row.amount) }}
          </template>
        </el-table-column>
        <el-table-column prop="remark" :label="t('purchaseExt.contractTab.colRemark')" min-width="120" />
      </el-table>
      <el-divider>{{ t('purchaseExt.contractTab.termsDivider') }}</el-divider>
      <el-descriptions :column="1" border>
        <el-descriptions-item :label="t('purchaseExt.contractTab.paymentTerms')">{{
          currentContract?.payment_terms
        }}</el-descriptions-item>
        <el-descriptions-item :label="t('purchaseExt.contractTab.deliveryTerms')">{{
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
  getPurchaseContractList,
  getPurchaseContract,
  createPurchaseContract,
  updatePurchaseContract,
  approvePurchaseContract,
  executePurchaseContract,
  cancelPurchaseContract,
  type PurchaseContract,
  type ContractItem as PurchaseContractItem,
} from '@/api/purchase-contract'

const { t } = useI18n({ useScope: 'global' })

const purchaseContracts = ref<PurchaseContract[]>([])
const contractLoading = ref(false)

const formatMoney = (amount: number | undefined) => {
  return amount?.toLocaleString('zh-CN', { minimumFractionDigits: 2 }) || '0.00'
}

const getContractStatusLabel = (status?: string) => {
  const map: Record<string, string> = {
    draft: t('purchaseExt.contractTab.statusDraft'),
    pending: t('purchaseExt.contractTab.statusPending'),
    active: t('purchaseExt.contractTab.statusActive'),
    completed: t('purchaseExt.contractTab.statusCompleted'),
    cancelled: t('purchaseExt.contractTab.statusCancelled'),
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

const fetchPurchaseContracts = async () => {
  contractLoading.value = true
  try {
    const res = await getPurchaseContractList()
    purchaseContracts.value = res.data?.list || []
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('purchaseExt.contractTab.fetchFailed'))
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
  supplier_id: 0,
  supplier_name: '',
  contract_date: '',
  start_date: '',
  end_date: '',
  total_amount: 0,
  currency: 'CNY',
  status: 'draft' as 'draft' | 'pending' | 'active' | 'completed' | 'cancelled',
  items: [] as PurchaseContractItem[],
  payment_terms: '',
  delivery_terms: '',
})

const contractRules: FormRules = {
  contract_no: [{ required: true, message: t('purchaseExt.contractTab.ruleContractNo'), trigger: 'blur' }],
  supplier_name: [{ required: true, message: t('purchaseExt.contractTab.ruleSupplierName'), trigger: 'blur' }],
  contract_date: [{ required: true, message: t('purchaseExt.contractTab.ruleContractDate'), trigger: 'change' }],
  total_amount: [{ required: true, message: t('purchaseExt.contractTab.ruleTotalAmount'), trigger: 'blur' }],
}

const openContractDialog = async (row?: PurchaseContract) => {
  if (row) {
    const res = await getPurchaseContract(row.id)
    // 安全检查：防止后端返回 data 为 null 时崩溃
    if (res.data) Object.assign(contractForm, res.data)
  } else {
    Object.assign(contractForm, {
      id: 0,
      contract_no: '',
      supplier_id: 0,
      supplier_name: '',
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
      await updatePurchaseContract(contractForm.id, contractForm)
      ElMessage.success(t('purchaseExt.contractTab.updateSuccess'))
    } else {
      await createPurchaseContract(contractForm)
      ElMessage.success(t('purchaseExt.contractTab.createSuccess'))
    }
    contractDialogVisible.value = false
    fetchPurchaseContracts()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || t('purchaseExt.contractTab.operationFailed'))
  } finally {
    contractSubmitLoading.value = false
  }
}

const contractViewVisible = ref(false)
const currentContract = ref<PurchaseContract | null>(null)

const viewContract = async (row: PurchaseContract) => {
  const res = await getPurchaseContract(row.id)
  // 安全检查：防止后端返回 data 为 null 时崩溃
  if (res.data) currentContract.value = res.data
  contractViewVisible.value = true
}

const approveContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm(t('purchaseExt.contractTab.approveConfirm'), t('purchaseExt.contractTab.confirmTitle'), { type: 'info' })
    await approvePurchaseContract(row.id)
    ElMessage.success(t('purchaseExt.contractTab.approveSuccess'))
    fetchPurchaseContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('purchaseExt.contractTab.operationFailed'))
    }
  }
}

const executeContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm(t('purchaseExt.contractTab.executeConfirm'), t('purchaseExt.contractTab.confirmTitle'), { type: 'info' })
    await executePurchaseContract(row.id)
    ElMessage.success(t('purchaseExt.contractTab.executeSuccess'))
    fetchPurchaseContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('purchaseExt.contractTab.operationFailed'))
    }
  }
}

const cancelContract = async (row: PurchaseContract) => {
  try {
    await ElMessageBox.confirm(t('purchaseExt.contractTab.cancelConfirm'), t('purchaseExt.contractTab.confirmTitle'), { type: 'warning' })
    await cancelPurchaseContract(row.id)
    ElMessage.success(t('purchaseExt.contractTab.cancelSuccess'))
    fetchPurchaseContracts()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || t('purchaseExt.contractTab.operationFailed'))
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

defineExpose({ refresh: fetchPurchaseContracts })

onMounted(() => {
  fetchPurchaseContracts()
})
</script>
