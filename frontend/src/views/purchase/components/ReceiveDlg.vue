<!--
  ReceiveDlg - 采购收货对话框
  任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 收货对话框）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="采购收货"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="localForm" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="采购单号">
            <el-input v-model="localForm.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="供应商">
            <el-input v-model="localForm.supplier_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="收货日期" required>
            <el-date-picker
              v-model="localForm.receive_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="仓库" required>
            <el-select
              v-model="localForm.warehouse_id"
              placeholder="选择仓库"
              style="width: 100%"
            >
              <el-option
                v-for="w in warehouses"
                :key="w.id"
                :label="w.warehouse_name"
                :value="w.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="收货明细">
        <el-table :data="localForm.items" border style="width: 100%">
          <el-table-column prop="product_name" label="产品" width="150" />
          <el-table-column prop="ordered_quantity" label="订购数量" width="100" />
          <el-table-column prop="received_quantity" label="已收货" width="100" />
          <el-table-column label="本次收货" width="120">
            <template #default="{ row }">
              <el-input-number
                v-model="row.receive_quantity"
                :min="0"
                :max="row.ordered_quantity - row.received_quantity"
                size="small"
              />
            </template>
          </el-table-column>
          <el-table-column prop="unit_price" label="单价" width="100" />
          <el-table-column label="备注" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.remarks" size="small" placeholder="备注" />
            </template>
          </el-table-column>
        </el-table>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">取消</el-button>
        <el-button type="primary" @click="onSubmit">确定收货</el-button>
      </span>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, watch, nextTick } from 'vue'
import type { Warehouse } from '@/api/warehouse'
import type { ReceiveFormData } from '../composables/usePurchRcv'

const props = defineProps<{
  // 对话框可见性
  modelValue: boolean
  // 表单数据（由父组件管理，子组件通过 emit('update:form') 回写）
  form: ReceiveFormData
  // 仓库列表
  warehouses: Warehouse[]
  // 提交
  onSubmit: () => void
  // 取消
  onCancel: () => void
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
  // 整体回写表单（父组件监听此事件并回写到自己的 form.value）
  (e: 'update:form', form: ReceiveFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
// 注意：表单内有 items 数组，需要深拷贝以保证本地修改与父组件解耦
const localForm = ref<ReceiveFormData>(JSON.parse(JSON.stringify(props.form)))

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local（如父组件打开对话框时填充数据）
watch(
  () => props.form,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = JSON.parse(JSON.stringify(newForm))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件（用户输入）
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:form', JSON.parse(JSON.stringify(newForm)))
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<style scoped>
.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
