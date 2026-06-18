<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
/**
 * ReceiveDlg - 采购收货对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 收货对话框）
 */
import type { Warehouse } from '@/api/warehouse'
import type { ReceiveFormData } from '../composables/usePurchRcv'

interface Props {
  modelValue: boolean
  form: ReceiveFormData
  warehouses: Warehouse[]
  onSubmit: () => void
  onCancel: () => void
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    title="采购收货"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form :model="form" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="采购单号">
            <el-input v-model="form.order_no" readonly />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="供应商">
            <el-input v-model="form.supplier_name" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="收货日期" required>
            <el-date-picker
              v-model="form.receive_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="仓库" required>
            <el-select
              v-model="form.warehouse_id"
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
        <el-table :data="form.items" border style="width: 100%">
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
