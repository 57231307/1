<script setup lang="ts">
/* eslint-disable vue/no-mutating-props */
/**
 * CreateDlg - 新建采购单对话框（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 purchase/index.vue 新建采购单对话框）
 */
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'
import type { CreateFormData } from '../composables/useCreate'

interface Props {
  modelValue: boolean
  form: CreateFormData
  rules: any
  suppliers: Supplier[]
  products: Product[]
  formRef: any
  onSubmit: () => void
  onCancel: () => void
  onAddItem: () => void
  onRemoveItem: (index: number) => void
  onProductSelect: (index: number) => void
  onCalculateSubtotal: (item: any) => void
  calculateTotal: () => number
}

defineProps<Props>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()
</script>

<template>
  <el-dialog
    :model-value="modelValue"
    title="新建采购单"
    width="800px"
    @update:model-value="(v: boolean) => emit('update:modelValue', v)"
  >
    <el-form
      ref="formRef"
      :model="form"
      :rules="rules"
      label-width="100px"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="供应商" required>
            <el-select
              v-model="form.supplier_id"
              placeholder="选择供应商"
              style="width: 100%"
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
          <el-form-item label="订单日期" required>
            <el-date-picker
              v-model="form.order_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="要求交货日期">
            <el-date-picker
              v-model="form.required_date"
              type="date"
              placeholder="选择日期"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="备注">
            <el-input v-model="form.remark" placeholder="请输入备注" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="采购明细">
        <div class="items-table">
          <div class="items-header">
            <span class="col-product">产品</span>
            <span class="col-qty">数量</span>
            <span class="col-price">单价</span>
            <span class="col-amount">金额</span>
            <span class="col-action">操作</span>
          </div>
          <div v-for="(item, index) in form.items" :key="index" class="items-row">
            <el-select
              v-model="item.product_id"
              placeholder="选择产品"
              class="col-product"
              @change="onProductSelect(index)"
            >
              <el-option
                v-for="p in products"
                :key="p.id"
                :label="p.product_name"
                :value="p.id"
              />
            </el-select>
            <el-input-number
              v-model="item.quantity"
              :min="1"
              class="col-qty"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number
              v-model="item.unit_price"
              :min="0"
              :precision="2"
              class="col-price"
              @change="onCalculateSubtotal(item)"
            />
            <el-input-number v-model="item.subtotal" :precision="2" class="col-amount" readonly />
            <el-button
              v-if="form.items.length > 1"
              size="small"
              type="danger"
              @click="onRemoveItem(index)"
              >删除</el-button
            >
          </div>
          <el-button type="text" @click="onAddItem">+ 添加明细</el-button>
        </div>
      </el-form-item>
      <el-form-item label="合计金额">
        <span class="total-amount">¥{{ calculateTotal().toLocaleString() }}</span>
      </el-form-item>
    </el-form>
    <template #footer>
      <span class="dialog-footer">
        <el-button @click="onCancel">取消</el-button>
        <el-button type="primary" @click="onSubmit">确定</el-button>
      </span>
    </template>
  </el-dialog>
</template>
