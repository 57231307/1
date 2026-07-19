<!--
  ProductFormDialogTab.vue - 产品新建/编辑对话框
  来源：原 product/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-4
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
      aria-label="产品表单"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="产品编码" prop="product_code">
            <el-input v-model="formData.product_code" placeholder="请输入产品编码" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="产品名称" prop="product_name">
            <el-input v-model="formData.product_name" placeholder="请输入产品名称" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="产品分类" prop="category_id">
            <el-select v-model="formData.category_id" placeholder="请选择分类" style="width: 100%">
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
          <el-form-item label="规格" prop="specification">
            <el-input v-model="formData.specification" placeholder="请输入规格" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="单位" prop="unit">
            <el-input v-model="formData.unit" placeholder="请输入单位" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="条形码" prop="barcode">
            <el-input v-model="formData.barcode" placeholder="请输入条形码" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="售价" prop="price">
            <el-input-number v-model="formData.price" :min="0" :precision="2" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="成本价" prop="cost_price">
            <el-input-number
              v-model="formData.cost_price"
              :min="0"
              :precision="2"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="描述" prop="description">
        <el-input
          v-model="formData.description"
          type="textarea"
          :rows="3"
          placeholder="请输入描述"
        />
      </el-form-item>
      <el-form-item label="状态" prop="is_active">
        <el-switch v-model="formData.is_active" active-text="启用" inactive-text="禁用" />
      </el-form-item>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import type { Product, ProductCategory } from '@/api/product'
import { productApi } from '@/api/product'
import { logger } from '@/utils/logger'

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
    { required: true, message: '请输入产品编码', trigger: 'blur' },
    { max: 50, message: '长度不能超过50个字符', trigger: 'blur' },
  ],
  product_name: [
    { required: true, message: '请输入产品名称', trigger: 'blur' },
    { max: 200, message: '长度不能超过200个字符', trigger: 'blur' },
  ],
  category_id: [{ required: true, message: '请选择产品分类', trigger: 'change' }],
  unit: [{ required: true, message: '请输入单位', trigger: 'blur' }],
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
        await productApi.create(formData)
        ElMessage.success('创建成功')
      } else {
        await productApi.update(formData.id as number, formData)
        ElMessage.success('更新成功')
      }
      emit('update:modelValue', false)
      emit('submitted')
    } catch (error) {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
      logger.error('产品保存失败', err.message)
    } finally {
      submitLoading.value = false
    }
  })
}
</script>
