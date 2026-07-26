<!--
  PurchaseContractForm.vue - 采购合同新建/编辑对话框
  拆分自 purchase-contract/index.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="800px"
    :close-on-click-modal="false"
    :aria-label="title"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="localFormData" label-width="100px" :aria-label="t('purchaseContract.form.ariaLabel')">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.contractNo')" prop="contract_no">
            <el-input
              :model-value="localFormData.contract_no"
              :placeholder="t('purchaseContract.form.contractNoPlaceholder')"
              @update:model-value="(v: string) => (localFormData.contract_no = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.contractName')" prop="contract_name">
            <el-input
              :model-value="localFormData.contract_name"
              :placeholder="t('purchaseContract.form.contractNamePlaceholder')"
              @update:model-value="(v: string) => (localFormData.contract_name = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.supplier')" prop="supplier_id">
            <el-select
              :model-value="localFormData.supplier_id"
              :placeholder="t('purchaseContract.form.supplierPlaceholder')"
              filterable
              @update:model-value="(v: number) => (localFormData.supplier_id = v)"
            >
              <el-option
                v-for="s in suppliers"
                :key="s.id"
                :label="s.supplier_name"
                :value="s.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.contractType')" prop="contract_type">
            <el-select
              :model-value="localFormData.contract_type"
              :placeholder="t('purchaseContract.form.contractTypePlaceholder')"
              @update:model-value="(v: string) => (localFormData.contract_type = v)"
            >
              <el-option :label="t('purchaseContract.form.typePurchase')" value="PURCHASE" />
              <el-option :label="t('purchaseContract.form.typeFramework')" value="FRAMEWORK" />
              <el-option :label="t('purchaseContract.form.typeSupplement')" value="SUPPLEMENT" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.totalAmount')" prop="total_amount">
            <el-input-number
              :model-value="localFormData.total_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (localFormData.total_amount = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.signedDate')" prop="signed_date">
            <el-date-picker
              :model-value="localFormData.signed_date"
              type="date"
              :placeholder="t('purchaseContract.form.signedDatePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.signed_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.effectiveDate')" prop="effective_date">
            <el-date-picker
              :model-value="localFormData.effective_date"
              type="date"
              :placeholder="t('purchaseContract.form.effectiveDatePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.effective_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.expiryDate')" prop="expiry_date">
            <el-date-picker
              :model-value="localFormData.expiry_date"
              type="date"
              :placeholder="t('purchaseContract.form.expiryDatePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.expiry_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.paymentTerms')" prop="payment_terms">
            <el-input
              :model-value="localFormData.payment_terms"
              :placeholder="t('purchaseContract.form.paymentTermsPlaceholder')"
              @update:model-value="(v: string) => (localFormData.payment_terms = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.paymentMethod')" prop="payment_method">
            <el-select
              :model-value="localFormData.payment_method"
              :placeholder="t('purchaseContract.form.paymentMethodPlaceholder')"
              @update:model-value="(v: string) => (localFormData.payment_method = v)"
            >
              <el-option :label="t('purchaseContract.form.methodBankTransfer')" value="BANK_TRANSFER" />
              <el-option :label="t('purchaseContract.form.methodCheck')" value="CHECK" />
              <el-option :label="t('purchaseContract.form.methodCash')" value="CASH" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.deliveryDate')" prop="delivery_date">
            <el-date-picker
              :model-value="localFormData.delivery_date"
              type="date"
              :placeholder="t('purchaseContract.form.deliveryDatePlaceholder')"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.delivery_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('purchaseContract.form.deliveryLocation')" prop="delivery_location">
            <el-input
              :model-value="localFormData.delivery_location"
              :placeholder="t('purchaseContract.form.deliveryLocationPlaceholder')"
              @update:model-value="(v: string) => (localFormData.delivery_location = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('purchaseContract.form.remarks')" prop="remarks">
        <el-input
          :model-value="localFormData.remarks"
          type="textarea"
          :rows="3"
          :placeholder="t('purchaseContract.form.remarksPlaceholder')"
          @update:model-value="(v: string) => (localFormData.remarks = v)"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{ t('purchaseContract.form.cancel') }}</el-button>
      <el-button type="primary" @click="emit('submit')">{{ t('purchaseContract.form.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Supplier } from '@/api/supplier'

const { t } = useI18n({ useScope: 'global' })

// 表单数据类型（所有字段可选，兼容 Partial<PurchaseContract>）
interface PurchaseContractFormData {
  id?: number | undefined
  contract_no?: string
  contract_name?: string
  supplier_id?: number | undefined
  contract_type?: string
  total_amount?: number
  signed_date?: string
  effective_date?: string
  expiry_date?: string
  payment_terms?: string
  payment_method?: string
  delivery_date?: string
  delivery_location?: string
  remarks?: string
}

const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 标题
  title: string
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: PurchaseContractFormData
  // 供应商列表
  suppliers: Supplier[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'submit'): void
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: PurchaseContractFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<PurchaseContractFormData>({ ...props.formData })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件编辑/新建时填充数据）
watch(
  () => props.formData,
  (newData) => {
    if (syncing) return
    syncing = true
    localFormData.value = { ...newData }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localFormData,
  (newData) => {
    if (syncing) return
    syncing = true
    emit('update:formData', { ...newData })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>
