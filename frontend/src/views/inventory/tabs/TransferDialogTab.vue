<!--
  TransferDialogTab.vue - 新建调拨单对话框
  来源：原 inventory/index.vue 中 新建调拨单对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    title="新建调拨单"
    width="700px"
    :close-on-click-modal="false"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form :model="form" label-width="100px">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="调出仓库">
            <el-select
              v-model="form.from_warehouse_id"
              placeholder="请选择调出仓库"
              style="width: 100%"
            >
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="调入仓库">
            <el-select
              v-model="form.to_warehouse_id"
              placeholder="请选择调入仓库"
              style="width: 100%"
            >
              <el-option
                v-for="wh in warehouses"
                :key="wh.id"
                :label="wh.warehouse_name"
                :value="wh.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-divider content-position="left">调拨产品</el-divider>
      <div
        v-for="(item, index) in form.items"
        :key="index"
        style="display: flex; gap: 10px; margin-bottom: 10px"
      >
        <el-input v-model="item.product_name" placeholder="产品名称" style="flex: 2" />
        <el-input-number v-model="item.quantity" :min="1" placeholder="数量" style="flex: 1" />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="form.items.length <= 1"
          @click="handleRemoveItem(index)"
        />
      </div>
      <el-button type="primary" link @click="handleAddItem">
        <el-icon><Plus /></el-icon>
        添加产品
      </el-button>
      <el-form-item label="备注" style="margin-top: 16px">
        <el-input v-model="form.remark" type="textarea" :rows="2" placeholder="请输入备注" />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">取消</el-button>
      <el-button type="primary" @click="handleSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, watch } from 'vue'
import { ElMessage } from 'element-plus'
import { Plus, Delete } from '@element-plus/icons-vue'
import { logger } from '@/utils/logger'

interface Warehouse {
  id: number
  warehouse_name?: string
  name?: string
}

interface TransferItem {
  product_id: number | null
  product_name: string
  quantity: number
}

interface Props {
  modelValue: boolean
  warehouses: Warehouse[]
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void
  (e: 'submitted'): void
}

const props = defineProps<Props>()
const emit = defineEmits<Emits>()

const form = reactive({
  from_warehouse_id: null as number | null,
  to_warehouse_id: null as number | null,
  items: [{ product_id: null, product_name: '', quantity: 0 }] as TransferItem[],
  remark: '',
})

const resetForm = () => {
  form.from_warehouse_id = null
  form.to_warehouse_id = null
  form.items = [{ product_id: null, product_name: '', quantity: 0 }]
  form.remark = ''
}

watch(
  () => props.modelValue,
  val => {
    if (val) {
      resetForm()
    }
  }
)

const handleAddItem = () => {
  form.items.push({ product_id: null, product_name: '', quantity: 0 })
}

const handleRemoveItem = (index: number) => {
  if (form.items.length > 1) {
    form.items.splice(index, 1)
  }
}

const handleSubmit = async () => {
  if (!form.from_warehouse_id) {
    ElMessage.warning('请选择调出仓库')
    return
  }
  if (!form.to_warehouse_id) {
    ElMessage.warning('请选择调入仓库')
    return
  }
  if (form.from_warehouse_id === form.to_warehouse_id) {
    ElMessage.warning('调出仓库和调入仓库不能相同')
    return
  }

  const validItems = form.items
    .filter(item => item.product_id && item.quantity > 0)
    .map(item => ({
      product_id: item.product_id as number,
      quantity: item.quantity,
    }))
  if (validItems.length === 0) {
    ElMessage.warning('请添加至少一个调拨产品')
    return
  }

  try {
    const { inventoryApi } = await import('@/api/inventory')
    await inventoryApi.createTransfer({
      from_warehouse_id: form.from_warehouse_id,
      to_warehouse_id: form.to_warehouse_id,
      items: validItems,
      remark: form.remark,
    })
    ElMessage.success('调拨单创建成功')
    emit('update:modelValue', false)
    emit('submitted')
  } catch (error) {
    const err = error as Error
    ElMessage.error(err.message || '创建调拨单失败')
    logger.error('创建调拨单失败', err.message)
  }
}
</script>
