<!--
  PiForm.vue - 采购验货新建/编辑表单对话框
  拆分自 purchase-inspection/index.vue（P14 批 2 I-3 第 5 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="isEdit ? '编辑检验单' : '新建检验单'"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-form ref="formRef" :model="localFormData" :rules="rules" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="入库单号" prop="receipt_id">
            <el-select
              :model-value="localFormData.receipt_id"
              placeholder="选择入库单"
              filterable
              @update:model-value="(v: number) => emit('receipt-change', v)"
            >
              <el-option
                v-for="receipt in receipts"
                :key="receipt.id"
                :label="receipt.receipt_no"
                :value="receipt.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="检验日期" prop="inspection_date">
            <el-date-picker
              :model-value="localFormData.inspection_date"
              type="date"
              placeholder="选择检验日期"
              value-format="YYYY-MM-DD"
              @update:model-value="(v: string) => (localFormData.inspection_date = v)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注">
        <el-input
          :model-value="localFormData.remark"
          type="textarea"
          :rows="3"
          placeholder="请输入备注"
          @update:model-value="(v: string) => (localFormData.remark = v)"
        />
      </el-form-item>

      <!-- 检验明细 -->
      <el-divider content-position="left">检验明细</el-divider>
      <el-table :data="localFormData.items" border>
        <el-table-column prop="product_name" label="产品名称" min-width="150" />
        <el-table-column prop="expected_quantity" label="预期数量" width="100" />
        <el-table-column prop="inspected_quantity" label="检验数量" width="120">
          <template #default="{ row }">
            <el-input-number
              :model-value="row.inspected_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.inspected_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="passed_quantity" label="合格数量" width="120">
          <template #default="{ row }">
            <el-input-number
              :model-value="row.passed_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.passed_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="failed_quantity" label="不合格数量" width="120">
          <template #default="{ row }">
            <el-input-number
              :model-value="row.failed_quantity"
              :min="0"
              size="small"
              @update:model-value="(v: number) => (row.failed_quantity = v)"
            />
          </template>
        </el-table-column>
        <el-table-column prop="defect_reason" label="缺陷原因" min-width="150">
          <template #default="{ row }">
            <el-input
              :model-value="row.defect_reason"
              size="small"
              @update:model-value="(v: string) => (row.defect_reason = v)"
            />
          </template>
        </el-table-column>
      </el-table>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:visible', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import { type FormInstance, type FormRules } from 'element-plus'
import type { PurchaseInspectionItem } from '@/api/purchase-inspection'

// 表单数据类型（所有字段可选，兼容父组件 reactive）
interface PiFormData {
  id?: number
  receipt_id?: number
  inspection_date: string
  remark: string
  items: Partial<PurchaseInspectionItem>[]
}

const props = defineProps<{
  // 可见性
  visible: boolean
  // 是否编辑
  isEdit: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: PiFormData
  // 验证规则
  rules: FormRules
  // 提交加载
  submitLoading: boolean
  // 入库单列表
  receipts: { id: number; receipt_no: string }[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  // 入库单变化（父组件加载明细）
  (e: 'receipt-change', receiptId: number): void
  // 提交（父组件处理 API）
  (e: 'submit'): void
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: PiFormData): void
}>()

// 表单 ref
const formRef = ref<FormInstance>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localFormData = ref<PiFormData>(JSON.parse(JSON.stringify(props.formData)))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件编辑/新建时填充数据）
watch(
  () => props.formData,
  (newData) => {
    if (syncing) return
    syncing = true
    localFormData.value = JSON.parse(JSON.stringify(newData))
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
    emit('update:formData', JSON.parse(JSON.stringify(newData)))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 暴露给父组件
defineExpose({ formRef })

/** 提交（先校验，再通知父组件） */
const handleSubmit = async () => {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
    emit('submit')
  } catch {
    // 校验失败
  }
}
</script>
