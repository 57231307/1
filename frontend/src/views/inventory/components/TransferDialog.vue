<!--
  TransferDialog.vue - 新建调拨单对话框
  任务编号: P14 批 2 I-3 第 8 批
  拆分原 inventory/index.vue 的新建调拨单对话框
  行为完全保持一致（仅结构重构）
  使用 props.initialForm 初始化 + 内部 localForm
  submit 时 emit submitWithForm(localForm) 把当前 form 回传
-->
<template>
  <el-dialog
    :model-value="visible"
    title="新建调拨单"
    width="700px"
    :close-on-click-modal="false"
    aria-label="新建调拨单对话框"
    @update:model-value="onClose"
  >
    <el-form :model="localForm" label-width="100px" aria-label="新建调拨单表单">
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item label="调出仓库">
            <el-select
              v-model="localForm.from_warehouse_id"
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
              v-model="localForm.to_warehouse_id"
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
        v-for="(item, index) in localForm.items"
        :key="index"
        style="display: flex; gap: 10px; margin-bottom: 10px"
      >
        <el-input-number v-model="item.quantity" :min="1" placeholder="数量" style="flex: 1" />
        <el-button
          type="danger"
          :icon="Delete"
          circle
          :disabled="localForm.items.length <= 1"
          @click="emit('removeItem', index)"
        />
      </div>
      <el-button type="primary" link @click="emit('addItem')">
        <el-icon><Plus /></el-icon>
        添加产品
      </el-button>
      <el-form-item label="备注" style="margin-top: 16px">
        <el-input
          v-model="localForm.remark"
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
import { deepClone } from '@/utils'
import { Delete, Plus } from '@element-plus/icons-vue'
import { reactive, watch } from 'vue'
import { getWarehouseLabel } from '../composables/invFmts'
// v11 批次 160 P2-7 修复：导入 Warehouse 接口替代 any[]
import type { Warehouse } from '@/api/warehouse'

/** 调拨单明细行 */
interface TransferFormItem {
  product_id: number | null
  quantity: number
}

/** 调拨单表单数据 */
interface TransferForm {
  from_warehouse_id: number | null
  to_warehouse_id: number | null
  items: TransferFormItem[]
  remark: string
}

const props = defineProps<{
  visible: boolean
  initialForm: TransferForm
  warehouses: Warehouse[]
}>()

const emit = defineEmits<{
  (e: 'update:visible', val: boolean): void
  (e: 'submit', data: TransferForm): void
  (e: 'addItem'): void
  (e: 'removeItem', index: number): void
}>()

// 浅拷贝 initialForm 同步初始值（不直接突变 prop）
const localForm = reactive<TransferForm>({
  from_warehouse_id: null,
  to_warehouse_id: null,
  items: [],
  remark: '',
})
watch(
  () => props.initialForm,
  newVal => {
    // TransferForm 字段固定，直接 Object.assign 覆盖即可（无需逐键 delete）
    Object.assign(localForm, deepClone(newVal))
  },
  { immediate: true, deep: true }
)

const onClose = (val: boolean) => {
  emit('update:visible', val)
}

const onSubmit = () => {
  emit('submit', deepClone(localForm))
}
</script>
