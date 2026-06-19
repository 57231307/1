<!--
  PpForm.vue - 采购价格新建/编辑对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<!-- eslint-disable vue/no-mutating-props -->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="formData" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="产品" prop="product_id">
            <el-select
              :model-value="formData.product_id"
              placeholder="请选择产品"
              filterable
              @update:model-value="(v: number) => (formData.product_id = v)"
            >
              <el-option
                v-for="p in products"
                :key="p.id"
                :label="p.product_name"
                :value="p.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="供应商" prop="supplier_id">
            <el-select
              :model-value="formData.supplier_id"
              placeholder="请选择供应商"
              filterable
              @update:model-value="(v: number) => (formData.supplier_id = v)"
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
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="采购价格" prop="price">
            <el-input-number
              :model-value="formData.price"
              :precision="6"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (formData.price = v ?? 0)"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="币种" prop="currency">
            <el-select
              :model-value="formData.currency"
              placeholder="请选择币种"
              @update:model-value="(v: string) => (formData.currency = v)"
            >
              <el-option label="人民币" value="CNY" />
              <el-option label="美元" value="USD" />
              <el-option label="欧元" value="EUR" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="单位" prop="unit">
            <el-select
              :model-value="formData.unit"
              placeholder="请选择单位"
              @update:model-value="(v: string) => (formData.unit = v)"
            >
              <el-option label="米" value="meter" />
              <el-option label="公斤" value="kg" />
              <el-option label="件" value="piece" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="最小订购量" prop="min_order_qty">
            <el-input-number
              :model-value="formData.min_order_qty"
              :precision="2"
              :min="0"
              style="width: 100%"
              @update:model-value="(v: number) => (formData.min_order_qty = v ?? 0)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="价格类型" prop="price_type">
            <el-select
              :model-value="formData.price_type"
              placeholder="请选择价格类型"
              @update:model-value="(v: string) => (formData.price_type = v)"
            >
              <el-option label="标准价" value="STANDARD" />
              <el-option label="协议价" value="AGREED" />
              <el-option label="促销价" value="PROMOTION" />
            </el-select>
          </el-form-item>
        </el-col>
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
      </el-row>
      <el-row :gutter="20">
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
import type { Supplier } from '@/api/supplier'
import type { Product } from '@/api/product'

// 表单数据类型（所有字段可选，兼容 Partial<PurchasePrice>）
interface PpFormData {
  id?: number | undefined
  product_id?: number | undefined
  supplier_id?: number | undefined
  price?: number
  currency?: string
  unit?: string
  min_order_qty?: number
  price_type?: string
  effective_date?: string
  expiry_date?: string
  remarks?: string
}

/**
 * 采购价格新建/编辑对话框组件
 */
defineProps<{
  // 对话框可见性
  visible: boolean
  // 标题
  title: string
  // 表单数据
  formData: PpFormData
  // 供应商列表
  suppliers: Supplier[]
  // 产品列表
  products: Product[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: []
}>()

/** 关闭对话框 */
const onVisibleChange = (v: boolean) => {
  emit('update:visible', v)
}
</script>
