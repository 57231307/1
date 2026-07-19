<!--
  PcForm.vue - 采购合同新建/编辑对话框
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
    <el-form :model="localFormData" label-width="100px" aria-label="采购合同表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="合同编号" prop="contract_no">
            <el-input
              :model-value="localFormData.contract_no"
              placeholder="请输入合同编号"
              @update:model-value="(v: string) => (localFormData.contract_no = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="合同名称" prop="contract_name">
            <el-input
              :model-value="localFormData.contract_name"
              placeholder="请输入合同名称"
              @update:model-value="(v: string) => (localFormData.contract_name = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商" prop="supplier_id">
            <el-select
              :model-value="localFormData.supplier_id"
              placeholder="请选择供应商"
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
          <el-form-item label="合同类型" prop="contract_type">
            <el-select
              :model-value="localFormData.contract_type"
              placeholder="请选择合同类型"
              @update:model-value="(v: string) => (localFormData.contract_type = v)"
            >
              <el-option label="采购合同" value="PURCHASE" />
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
              :model-value="localFormData.total_amount"
              :precision="2"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (localFormData.total_amount = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="签订日期" prop="signed_date">
            <el-date-picker
              :model-value="localFormData.signed_date"
              type="date"
              placeholder="请选择签订日期"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.signed_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="生效日期" prop="effective_date">
            <el-date-picker
              :model-value="localFormData.effective_date"
              type="date"
              placeholder="请选择生效日期"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.effective_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="到期日期" prop="expiry_date">
            <el-date-picker
              :model-value="localFormData.expiry_date"
              type="date"
              placeholder="请选择到期日期"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.expiry_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="付款条件" prop="payment_terms">
            <el-input
              :model-value="localFormData.payment_terms"
              placeholder="请输入付款条件"
              @update:model-value="(v: string) => (localFormData.payment_terms = v)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="付款方式" prop="payment_method">
            <el-select
              :model-value="localFormData.payment_method"
              placeholder="请选择付款方式"
              @update:model-value="(v: string) => (localFormData.payment_method = v)"
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
              :model-value="localFormData.delivery_date"
              type="date"
              placeholder="请选择交货日期"
              style="width: 100%"
              @update:model-value="(v: string) => (localFormData.delivery_date = v ?? '')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="交货地点" prop="delivery_location">
            <el-input
              :model-value="localFormData.delivery_location"
              placeholder="请输入交货地点"
              @update:model-value="(v: string) => (localFormData.delivery_location = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注" prop="remarks">
        <el-input
          :model-value="localFormData.remarks"
          type="textarea"
          :rows="3"
          placeholder="请输入备注"
          @update:model-value="(v: string) => (localFormData.remarks = v)"
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
import type { Supplier } from '@/api/supplier'

// 表单数据类型（所有字段可选，兼容 Partial<PurchaseContract>）
interface PcFormData {
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
  formData: PcFormData
  // 供应商列表
  suppliers: Supplier[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'submit'): void
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: PcFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<PcFormData>({ ...props.formData })

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
