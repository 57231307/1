<!--
  TransferDialog.vue - 新建调拨单对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的新建调拨单对话框
-->
<template>
  <el-dialog
    :model-value="visible"
    title="新建调拨单"
    width="700px"
    :close-on-click-modal="false"
    @update:model-value="onClose"
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
                :label="getWarehouseLabel(wh)"
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
                :label="getWarehouseLabel(wh)"
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
        <el-input-number v-model="item.quantity" :min="1" placeholder="数量" style="flex: 1" />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="form.items.length <= 1"
          @click="emit('removeItem', index)"
        />
      </div>
      <el-button type="primary" link @click="emit('addItem')">
        <el-icon><Plus /></el-icon>
        添加产品
      </el-button>
      <el-form-item label="备注" style="margin-top: 16px">
        <el-input
          v-model="form.remark"
          type="textarea"
          :rows="2"
          placeholder="请输入备注"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="onClose(false)">取消</el-button>
      <el-button type="primary" @click="onSubmit">确定</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { Delete, Plus } from '@element-plus/icons-vue'
import { getWarehouseLabel } from '../composables/invFmts'

defineProps<{
  visible: boolean
  form: any
  warehouses: any[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit'): void
  (e: 'addItem'): void
  (e: 'removeItem', index: number): void
}>()

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit')
}
</script>
