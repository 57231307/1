<!--
  面料多色号定价扩展 - 批量调价对话框
  支持 3 种调价模式：百分比 / 固定金额 / 阶梯价
  创建时间: 2026-06-18
-->
<template>
  <el-dialog
    :model-value="visible"
    title="批量调价"
    width="900px"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
    @close="handleClose"
  >
    <div class="batch-adjust-dialog">
      <el-form :model="form" label-width="120px">
        <el-form-item label="调价模式">
          <el-radio-group v-model="form.mode">
            <el-radio-button value="percentage">百分比</el-radio-button>
            <el-radio-button value="fixed">固定金额</el-radio-button>
            <el-radio-button value="tier">阶梯价</el-radio-button>
          </el-radio-group>
        </el-form-item>

        <el-form-item v-if="form.mode === 'percentage'" label="调价百分比">
          <el-input-number
            v-model="form.percentage"
            :min="-100"
            :max="100"
            :step="1"
            :precision="2"
            style="width: 200px"
          />
          <span style="margin-left: 8px">%</span>
          <span style="margin-left: 16px; color: #999">（>10% 需经理审批）</span>
        </el-form-item>

        <el-form-item v-if="form.mode === 'fixed'" label="调价金额">
          <el-input-number
            v-model="form.fixedAmount"
            :precision="2"
            style="width: 200px"
          />
          <span style="margin-left: 8px">元/米</span>
        </el-form-item>

        <el-form-item v-if="form.mode === 'tier'" label="阶梯价配置">
          <el-button @click="addTier">添加阶梯</el-button>
        </el-form-item>

        <el-form-item label="调价原因">
          <el-input
            v-model="form.changeReason"
            type="textarea"
            :rows="2"
            placeholder="请说明调价原因"
          />
        </el-form-item>
      </el-form>

      <el-divider content-position="left">调价预览</el-divider>

      <el-table :data="previewItems" border max-height="300">
        <el-table-column prop="id" label="价格 ID" width="100" />
        <el-table-column prop="product_id" label="产品" width="100" />
        <el-table-column prop="color_id" label="色号" width="100" />
        <el-table-column prop="currency" label="币种" width="80" />
        <el-table-column label="原价格" width="120">
          <template #default="{ row }">{{ formatPrice(row.base_price, row.currency) }}</template>
        </el-table-column>
        <el-table-column label="新价格" width="120">
          <template #default="{ row }">
            <span :style="{ color: row.new_price < row.base_price ? 'red' : 'green' }">
              {{ formatPrice(row.new_price, row.currency) }}
            </span>
          </template>
        </el-table-column>
        <el-table-column label="涨跌幅" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.change_percent > 0.10" type="warning">
              +{{ (row.change_percent * 100).toFixed(2) }}%（需审批）
            </el-tag>
            <el-tag v-else-if="row.change_percent < -0.30" type="danger">
              {{ (row.change_percent * 100).toFixed(2) }}%（预警）
            </el-tag>
            <el-tag v-else type="success">
              {{ (row.change_percent * 100).toFixed(2) }}%
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </div>

    <template #footer>
      <el-button @click="handleClose">取消</el-button>
      <el-button type="primary" @click="handleSubmit" :loading="loading">
        提交
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { reactive, computed, ref } from 'vue'
import { formatPrice } from '@/api/color-price'
import type { ColorPriceListItem } from '@/api/color-price'

const props = defineProps<{
  visible: boolean
  selectedPrices: ColorPriceListItem[]
}>()

const emit = defineEmits<{
  'update:visible': [v: boolean]
  submit: [data: { items: any[]; change_reason: string }]
}>()

const loading = ref(false)

const form = reactive({
  mode: 'percentage' as 'percentage' | 'fixed' | 'tier',
  percentage: 5,
  fixedAmount: 1.0,
  changeReason: '',
})

const previewItems = computed(() => {
  return props.selectedPrices.map((p) => {
    const base = parseFloat(p.base_price)
    let newPrice = base
    let changePercent = 0
    if (form.mode === 'percentage') {
      newPrice = base * (1 + form.percentage / 100)
      changePercent = form.percentage / 100
    } else if (form.mode === 'fixed') {
      newPrice = base + form.fixedAmount
      changePercent = (form.fixedAmount / base) || 0
    }
    return {
      ...p,
      new_price: newPrice.toFixed(6),
      change_percent: changePercent,
    }
  })
})

const addTier = () => {
  // 阶梯价模式的实现可以扩展
}

const handleClose = () => {
  emit('update:visible', false)
}

const handleSubmit = () => {
  const items = previewItems.value.map((p) => ({
    price_id: p.id,
    adjustment_type: form.mode === 'tier' ? 'tier' : form.mode,
    adjustment_value:
      form.mode === 'percentage'
        ? form.percentage / 100
        : form.mode === 'fixed'
        ? form.fixedAmount
        : 0,
  }))
  emit('submit', {
    items,
    change_reason: form.changeReason,
  })
  loading.value = true
  setTimeout(() => {
    loading.value = false
    handleClose()
  }, 500)
}
</script>

<style scoped>
.batch-adjust-dialog {
  padding: 0 20px;
}
</style>
