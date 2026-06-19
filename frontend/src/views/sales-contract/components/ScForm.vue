<!--
  ScForm.vue - 销售合同新建/编辑对话框
  拆分自 sales-contract/index.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="800px"
    :close-on-click-modal="false"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form :model="formData" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="合同编号" prop="contract_no">
            <el-input
              :model-value="formData.contract_no"
              placeholder="请输入合同编号"
              @update:model-value="(v: string) => (formData.contract_no = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="合同名称" prop="contract_name">
            <el-input
              :model-value="formData.contract_name"
              placeholder="请输入合同名称"
              @update:model-value="(v: string) => (formData.contract_name = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="客户" prop="customer_id">
            <el-select
              :model-value="formData.customer_id"
              placeholder="请选择客户"
              filterable
              @update:model-value="(v: number) => (formData.customer_id = v)"
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
              :model-value="formData.contract_type"
              placeholder="请选择合同类型"
              @update:model-value="(v: string) => (formData.contract_type = v)"
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
              :model-value="formData.total_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (formData.total_amount = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="签订日期" prop="signed_date">
            <el-date-picker
              :model-value="formData.signed_date"
              type="date"
              placeholder="请选择签订日期"
              style="width: 100%"
              @update:model-value="(v: string) => (formData.signed_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="生效日期" prop="effective_date">
            <el-date-picker
              :model-value="formData.effective_date"
              type="date"
              placeholder="请选择生效日期"
              style="width: 100%"
              @update:model-value="(v: string) => (formData.effective_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="到期日期" prop="expiry_date">
            <el-date-picker
              :model-value="formData.expiry_date"
              type="date"
              placeholder="请选择到期日期"
              style="width: 100%"
              @update:model-value="(v: string) => (formData.expiry_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="付款条件" prop="payment_terms">
            <el-input
              :model-value="formData.payment_terms"
              placeholder="请输入付款条件"
              @update:model-value="(v: string) => (formData.payment_terms = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="付款方式" prop="payment_method">
            <el-select
              :model-value="formData.payment_method"
              placeholder="请选择付款方式"
              @update:model-value="(v: string) => (formData.payment_method = v)"
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
              :model-value="formData.delivery_date"
              type="date"
              placeholder="请选择交货日期"
              style="width: 100%"
              @update:model-value="(v: string) => (formData.delivery_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="交货地点" prop="delivery_location">
            <el-input
              :model-value="formData.delivery_location"
              placeholder="请输入交货地点"
              @update:model-value="(v: string) => (formData.delivery_location = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注" prop="remarks">
        <el-input
          :model-value="formData.remarks"
          type="textarea"
          :rows="3"
          placeholder="请输入备注"
          @update:model-value="(v: string) => (formData.remarks = v)"
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
/* eslint-disable vue/no-mutating-props */
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
  formData: ScFormData
  customers: Customer[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
}>()

void props
</script>
