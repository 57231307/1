<!--
  PpForm.vue - 采购价格新建/编辑对话框
  拆分自 purchase-price/index.vue（P14 批 2 I-3 第 3 批）
  P9-3 批次 F Pattern A 重构：本地 ref 镜像 + watch 防循环 + emit 整体覆盖父组件
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    @update:model-value="onVisibleChange"
  >
    <el-form :model="localFormData" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="产品" prop="product_id">
            <el-select
              v-model="localFormData.product_id"
              placeholder="请选择产品"
              filterable
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
              v-model="localFormData.supplier_id"
              placeholder="请选择供应商"
              filterable
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
              v-model="localFormData.price"
              :precision="6"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="币种" prop="currency">
            <el-select
              v-model="localFormData.currency"
              placeholder="请选择币种"
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
            <el-select v-model="localFormData.unit" placeholder="请选择单位">
              <el-option label="米" value="meter" />
              <el-option label="公斤" value="kg" />
              <el-option label="件" value="piece" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="最小订购量" prop="min_order_qty">
            <el-input-number
              v-model="localFormData.min_order_qty"
              :precision="2"
              :min="0"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="价格类型" prop="price_type">
            <el-select v-model="localFormData.price_type" placeholder="请选择价格类型">
              <el-option label="标准价" value="STANDARD" />
              <el-option label="协议价" value="AGREED" />
              <el-option label="促销价" value="PROMOTION" />
            </el-select>
          </el-form-item>
        </el-col>
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
      </el-row>
      <el-row :gutter="20">
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
const props = defineProps<{
  // 对话框可见性
  visible: boolean
  // 标题
  title: string
  // 表单数据（由父组件管理，子组件通过 emit('update:formData') 回写）
  formData: PpFormData
  // 供应商列表
  suppliers: Supplier[]
  // 产品列表
  products: Product[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', v: boolean): void
  (e: 'submit'): void
  // 整体回写表单数据（父组件监听此事件并 Object.assign 到自己的 formData）
  (e: 'update:formData', formData: PpFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localFormData = ref<PpFormData>({ ...props.formData })

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
