<!--
  ProductFormDialogTab.vue - 产品新建/编辑对话框
  来源：原 product/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="title"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="t('product.productFormDialogTab.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelProductCode')"
            prop="product_code"
          >
            <el-input
              v-model="formData.product_code"
              :placeholder="t('product.productFormDialogTab.placeholderProductCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelProductName')"
            prop="product_name"
          >
            <el-input
              v-model="formData.product_name"
              :placeholder="t('product.productFormDialogTab.placeholderProductName')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelCategory')" prop="category_id">
            <el-select
              v-model="formData.category_id"
              :placeholder="t('product.productFormDialogTab.placeholderCategory')"
              style="width: 100%"
            >
              <el-option
                v-for="item in categories"
                :key="item.id"
                :label="item.name"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelSpecification')"
            prop="specification"
          >
            <el-input
              v-model="formData.specification"
              :placeholder="t('product.productFormDialogTab.placeholderSpecification')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelUnit')" prop="unit">
            <el-input
              v-model="formData.unit"
              :placeholder="t('product.productFormDialogTab.placeholderUnit')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelBarcode')" prop="barcode">
            <el-input
              v-model="formData.barcode"
              :placeholder="t('product.productFormDialogTab.placeholderBarcode')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelPrice')" prop="price">
            <el-input-number v-model="formData.price" :min="0" :precision="2" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelCostPrice')" prop="cost_price">
            <el-input-number
              v-model="formData.cost_price"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('product.productFormDialogTab.labelDescription')" prop="description">
        <el-input
          v-model="formData.description"
          type="textarea"
          :rows="3"
          :placeholder="t('product.productFormDialogTab.placeholderDescription')"
        />
      </el-form-item>
      <el-form-item :label="t('product.productFormDialogTab.labelStatus')" prop="is_active">
        <el-switch
          v-model="formData.is_active"
          :active-text="t('product.productFormDialogTab.statusActive')"
          :inactive-text="t('product.productFormDialogTab.statusInactive')"
        />
      </el-form-item>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('product.productFormDialogTab.buttonCancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('product.productFormDialogTab.buttonSave')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { Product, ProductCategory } from '@/api/product'
import { createProduct, updateProduct } from '@/api/product'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

interface Props {
  modelValue: boolean
  title: string
  rowData: Product | null
  categories: ProductCategory[]
  mode: 'create' | 'edit' | 'view'
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const visible = ref(props.modelValue)
const submitLoading = ref(false)
const formRef = ref<FormInstance>()

const formData = reactive({
  id: undefined as number | undefined,
  product_code: '',
  product_name: '',
  category_id: undefined as number | undefined,
  specification: '',
  unit: '',
  barcode: '',
  price: 0,
  cost_price: 0,
  description: '',
  is_active: true,
})

const formRules: FormRules = {
  product_code: [
    {
      required: true,
      message: t('product.productFormDialogTab.validateProductCodeRequired'),
      trigger: 'blur',
    },
    { max: 50, message: t('product.productFormDialogTab.validateProductCodeMax'), trigger: 'blur' },
  ],
  product_name: [
    {
      required: true,
      message: t('product.productFormDialogTab.validateProductNameRequired'),
      trigger: 'blur',
    },
    {
      max: 200,
      message: t('product.productFormDialogTab.validateProductNameMax'),
      trigger: 'blur',
    },
  ],
  category_id: [
    {
      required: true,
      message: t('product.productFormDialogTab.validateCategoryRequired'),
      trigger: 'change',
    },
  ],
  unit: [
    {
      required: true,
      message: t('product.productFormDialogTab.validateUnitRequired'),
      trigger: 'blur',
    },
  ],
}

const resetForm = () => {
  formData.id = undefined
  formData.product_code = ''
  formData.product_name = ''
  formData.category_id = undefined
  formData.specification = ''
  formData.unit = ''
  formData.barcode = ''
  formData.price = 0
  formData.cost_price = 0
  formData.description = ''
  formData.is_active = true
  formRef.value?.clearValidate()
}

watch(
  () => props.modelValue,
  val => {
    visible.value = val
    if (val) {
      resetForm()
      if (props.rowData) {
        Object.assign(formData, props.rowData)
      }
    }
  }
)

watch(visible, val => {
  emit('update:modelValue', val)
})

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (props.mode === 'create') {
        await createProduct(formData)
        ElMessage.success(t('product.productFormDialogTab.messageCreateSuccess'))
      } else {
        await updateProduct(formData.id as number, formData)
        ElMessage.success(t('product.productFormDialogTab.messageUpdateSuccess'))
      }
      emit('update:modelValue', false)
      emit('submitted')
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || t('product.productFormDialogTab.messageOperationFailed'))
      logger.error(t('product.productFormDialogTab.messageSaveFailed'), err.message)
    } finally {
      submitLoading.value = false
    }
  })
}
</script>
