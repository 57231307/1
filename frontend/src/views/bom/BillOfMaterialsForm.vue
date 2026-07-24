<template>
  <div class="bom-form">
    <el-form ref="formRef" :model="localFormData" :rules="formRules" label-width="100px" aria-label="BOM 表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="产品名称" prop="product_name">
            <el-input v-model="localFormData.product_name" placeholder="请输入产品名称" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="版本号" prop="version">
            <el-input v-model="localFormData.version" placeholder="请输入版本号" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="是否默认" prop="is_default">
            <el-switch v-model="localFormData.is_default" active-text="是" inactive-text="否" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="状态" prop="status">
            <el-select v-model="localFormData.status" placeholder="请选择状态" style="width: 100%">
              <el-option label="草稿" value="draft" />
              <el-option label="启用" value="active" />
              <el-option label="归档" value="archived" />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item label="备注" prop="remark">
        <el-input
          v-model="localFormData.remark"
          type="textarea"
          :rows="2"
          placeholder="请输入备注"
        />
      </el-form-item>
    </el-form>

    <div class="items-section">
      <div class="items-header">
        <h3 class="items-title">BOM 明细</h3>
        <el-button type="primary" size="small" @click="handleAddItem">
          <el-icon><Plus /></el-icon>
          添加明细
        </el-button>
      </div>

      <el-table :data="localFormData.items" border size="small" class="items-table" aria-label="BOM 明细列表">
        <el-table-column label="物料名称" min-width="180">
          <template #default="{ row }">
            <el-input v-model="row.material_name" placeholder="请输入物料名称" />
          </template>
        </el-table-column>
        <el-table-column label="数量" width="120">
          <template #default="{ row }">
            <el-input-number
              v-model="row.quantity"
              :min="0"
              :precision="2"
              controls-position="right"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column label="单位" width="100">
          <template #default="{ row }">
            <el-input v-model="row.unit" placeholder="单位" />
          </template>
        </el-table-column>
        <el-table-column label="损耗率(%)" width="130">
          <template #default="{ row }">
            <el-input-number
              v-model="row.loss_rate"
              :min="0"
              :max="100"
              :precision="2"
              controls-position="right"
              style="width: 100%"
            />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="80" fixed="right">
          <template #default="{ $index }">
            <el-button type="danger" link size="small" @click="handleRemoveItem($index)">
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <div class="form-footer">
      <el-button @click="handleCancel">取消</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">保存</el-button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { Bom } from '@/api/bom'

const props = defineProps<{
  formData: {
    id?: number
    product_id?: number
    product_name: string
    version: string
    is_default: boolean
    status: 'draft' | 'active' | 'archived'
    remark: string
    items: Array<{
      material_name: string
      quantity: number
      unit: string
      loss_rate: number
    }>
  }
  mode: 'create' | 'edit'
}>()

// v11 批次 169 P2-1 修复：emit submit data: any 改为 Partial<Bom>
const emit = defineEmits<{
  submit: [data: Partial<Bom>]
  cancel: []
}>()

const formRef = ref<FormInstance>()
const submitLoading = ref(false)

const localFormData = ref({
  product_name: props.formData.product_name,
  version: props.formData.version,
  is_default: props.formData.is_default,
  status: props.formData.status,
  remark: props.formData.remark,
  items: [...props.formData.items.map(item => ({ ...item }))],
})

watch(
  () => props.formData,
  newVal => {
    localFormData.value = {
      product_name: newVal.product_name,
      version: newVal.version,
      is_default: newVal.is_default,
      status: newVal.status,
      remark: newVal.remark,
      items: [...newVal.items.map(item => ({ ...item }))],
    }
  },
  { deep: true }
)

const formRules: FormRules = {
  product_name: [{ required: true, message: '请输入产品名称', trigger: 'blur' }],
  version: [{ required: true, message: '请输入版本号', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
}

const handleAddItem = () => {
  localFormData.value.items.push({
    material_name: '',
    quantity: 1,
    unit: '',
    loss_rate: 0,
  })
}

const handleRemoveItem = (index: number) => {
  localFormData.value.items.splice(index, 1)
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async valid => {
    if (!valid) return

    const hasEmptyItems = localFormData.value.items.some(item => !item.material_name || !item.unit)
    if (hasEmptyItems) {
      ElMessage.warning('请填写完整的物料明细')
      return
    }

    submitLoading.value = true
    try {
      emit('submit', {
        product_name: localFormData.value.product_name,
        version: localFormData.value.version,
        is_default: localFormData.value.is_default,
        status: localFormData.value.status,
        remark: localFormData.value.remark,
        items: localFormData.value.items,
      })
    } finally {
      submitLoading.value = false
    }
  })
}

const handleCancel = () => {
  emit('cancel')
}
</script>

<style scoped>
.bom-form {
  padding: 10px 0;
}
.items-section {
  margin-top: 24px;
}
.items-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.items-title {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.items-table {
  margin-bottom: 20px;
}
.form-footer {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
  margin-top: 24px;
  padding-top: 20px;
  border-top: 1px solid #ebeef5;
}
</style>
