<!--
  SalesContractForm.vue - 销售合同新建/编辑对话框
  拆分自 sales-contract/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
  P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="800px"
    :close-on-click-modal="false"
    :aria-label="title"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form
      :model="localFormData"
      label-width="100px"
      :aria-label="t('salesContract.form.ariaLabelForm')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelContractNo')" prop="contract_no">
            <el-input
              v-model="localFormData.contract_no"
              :placeholder="t('salesContract.form.placeholderContractNo')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelContractName')" prop="contract_name">
            <el-input
              v-model="localFormData.contract_name"
              :placeholder="t('salesContract.form.placeholderContractName')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelCustomer')" prop="customer_id">
            <el-select
              v-model="localFormData.customer_id"
              :placeholder="t('salesContract.form.placeholderCustomer')"
              filterable
            >
              <el-option
                v-for="c in customers"
                :key="c.id"
                :label="c.customer_name"
                :value="c.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelContractType')" prop="contract_type">
            <el-select
              v-model="localFormData.contract_type"
              :placeholder="t('salesContract.form.placeholderContractType')"
            >
              <el-option :label="t('salesContract.form.optionSales')" value="SALES" />
              <el-option :label="t('salesContract.form.optionFramework')" value="FRAMEWORK" />
              <el-option :label="t('salesContract.form.optionSupplement')" value="SUPPLEMENT" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelTotalAmount')" prop="total_amount">
            <el-input-number
              v-model="localFormData.total_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelSignedDate')" prop="signed_date">
            <el-date-picker
              v-model="localFormData.signed_date"
              type="date"
              :placeholder="t('salesContract.form.placeholderSignedDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelEffectiveDate')" prop="effective_date">
            <el-date-picker
              v-model="localFormData.effective_date"
              type="date"
              :placeholder="t('salesContract.form.placeholderEffectiveDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelExpiryDate')" prop="expiry_date">
            <el-date-picker
              v-model="localFormData.expiry_date"
              type="date"
              :placeholder="t('salesContract.form.placeholderExpiryDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelPaymentTerms')" prop="payment_terms">
            <el-input
              v-model="localFormData.payment_terms"
              :placeholder="t('salesContract.form.placeholderPaymentTerms')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelPaymentMethod')" prop="payment_method">
            <el-select
              v-model="localFormData.payment_method"
              :placeholder="t('salesContract.form.placeholderPaymentMethod')"
            >
              <el-option
                :label="t('salesContract.form.optionBankTransfer')"
                value="BANK_TRANSFER"
              />
              <el-option :label="t('salesContract.form.optionCheck')" value="CHECK" />
              <el-option :label="t('salesContract.form.optionCash')" value="CASH" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('salesContract.form.labelDeliveryDate')" prop="delivery_date">
            <el-date-picker
              v-model="localFormData.delivery_date"
              type="date"
              :placeholder="t('salesContract.form.placeholderDeliveryDate')"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('salesContract.form.labelDeliveryLocation')"
            prop="delivery_location"
          >
            <el-input
              v-model="localFormData.delivery_location"
              :placeholder="t('salesContract.form.placeholderDeliveryLocation')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('salesContract.form.labelRemarks')" prop="remarks">
        <el-input
          v-model="localFormData.remarks"
          type="textarea"
          :rows="3"
          :placeholder="t('salesContract.form.placeholderRemarks')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">{{
        t('salesContract.form.buttonCancel')
      }}</el-button>
      <el-button type="primary" @click="emit('submit')">{{
        t('salesContract.form.buttonConfirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { Customer } from '@/api/customer'

const { t } = useI18n({ useScope: 'global' })

interface ScFormData {
  id?: number
  contract_no: string
  contract_name: string
  customer_id: number | undefined
  contract_type: string
  total_amount: number
  signed_date: string
  effective_date: string
  expiry_date: string
  payment_terms: string
  payment_method: string
  delivery_date: string
  delivery_location: string
  remarks: string
}

/**
 * 销售合同新建/编辑对话框组件
 */
const props = defineProps<{
  visible: boolean
  title: string
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: ScFormData
  customers: Customer[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
  // 整体回写表单
  'update:formData': [formData: ScFormData]
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<ScFormData>({ ...props.formData })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.formData,
  newForm => {
    if (syncing) return
    syncing = true
    localFormData.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)

// 本地变化时通知父组件
watch(
  localFormData,
  newForm => {
    if (syncing) return
    syncing = true
    emit('update:formData', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true }
)
</script>
