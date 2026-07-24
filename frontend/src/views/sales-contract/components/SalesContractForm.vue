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
    <el-form :model="localFormData" label-width="100px" aria-label="销售合同表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="合同编号" prop="contract_no">
            <el-input
              v-model="localFormData.contract_no"
              placeholder="请输入合同编号"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="合同名称" prop="contract_name">
            <el-input
              v-model="localFormData.contract_name"
              placeholder="请输入合同名称"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="客户" prop="customer_id">
            <el-select
              v-model="localFormData.customer_id"
              placeholder="请选择客户"
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
          <el-form-item label="合同类型" prop="contract_type">
            <el-select
              v-model="localFormData.contract_type"
              placeholder="请选择合同类型"
            >
              <el-option label="销售合同" value="SALES" />
              <el-option label="框架合同" value="FRAMEWORK" />
              <el-option label="补充协议" value="SUPPLEMENT" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="合同金额" prop="total_amount">
            <el-input-number
              v-model="localFormData.total_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="签订日期" prop="signed_date">
            <el-date-picker
              v-model="localFormData.signed_date"
              type="date"
              placeholder="请选择签订日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="生效日期" prop="effective_date">
            <el-date-picker
              v-model="localFormData.effective_date"
              type="date"
              placeholder="请选择生效日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="到期日期" prop="expiry_date">
            <el-date-picker
              v-model="localFormData.expiry_date"
              type="date"
              placeholder="请选择到期日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="付款条件" prop="payment_terms">
            <el-input
              v-model="localFormData.payment_terms"
              placeholder="请输入付款条件"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="付款方式" prop="payment_method">
            <el-select
              v-model="localFormData.payment_method"
              placeholder="请选择付款方式"
            >
              <el-option label="银行转账" value="BANK_TRANSFER" />
              <el-option label="支票" value="CHECK" />
              <el-option label="现金" value="CASH" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="交货日期" prop="delivery_date">
            <el-date-picker
              v-model="localFormData.delivery_date"
              type="date"
              placeholder="请选择交货日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="交货地点" prop="delivery_location">
            <el-input
              v-model="localFormData.delivery_location"
              placeholder="请输入交货地点"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注" prop="remarks">
        <el-input
          v-model="localFormData.remarks"
          type="textarea"
          :rows="3"
          placeholder="请输入备注"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" @click="emit('submit')">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { Customer } from '@/api/customer'

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
  (newForm) => {
    if (syncing) return
    syncing = true
    localFormData.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件
watch(
  localFormData,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:formData', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>
