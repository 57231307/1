<!--
  报价单明细编辑器
  - 通过 v-model 双向绑定 items 数组
  - 支持添加/删除明细行
  - 自动计算含税单价（按 13% 增值税）
  - 加载产品下拉与色号下拉（按产品过滤）
-->
<template>
  <div class="item-editor">
    <el-button type="primary" plain @click="handleAdd">
      <el-icon><Plus /></el-icon>
      {{ t('quotations.itemEditor.addProduct') }}
    </el-button>

    <el-table :data="modelValue" border style="margin-top: 10px" :empty-text="t('quotations.itemEditor.emptyText')" :aria-label="t('quotations.itemEditor.tableAriaLabel')">
      <el-table-column type="index" label="#" width="50" align="center" />

      <el-table-column :label="t('quotations.itemEditor.colProduct')" min-width="180">
        <template #default="{ row }">
          <el-select
            v-model="row.product_id"
            filterable
            :placeholder="t('quotations.itemEditor.selectProduct')"
            style="width: 100%"
            @change="(v: number | undefined) => handleProductChange(row, v)"
          >
            <el-option
              v-for="p in products"
              :key="p.id"
              :label="p.product_name || p.name"
              :value="p.id"
            />
          </el-select>
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colColor')" min-width="120">
        <template #default="{ row }">
          <el-select
            v-model="row.color_id"
            clearable
            :placeholder="t('quotations.itemEditor.noColor')"
            style="width: 100%"
            :disabled="!row.product_id"
          >
            <el-option
              v-for="c in row._colors || []"
              :key="c.id"
              :label="c.color_code || c.color_name"
              :value="c.id"
            />
          </el-select>
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colSpec')" min-width="140">
        <template #default="{ row }">
          <el-input v-model="row.specification" :placeholder="t('quotations.itemEditor.specPlaceholder')" />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnit')" min-width="90">
        <template #default="{ row }">
          <el-select v-model="row.unit" style="width: 100%">
            <el-option :label="t('quotations.itemEditor.unitMeter')" value="米" />
            <el-option :label="t('quotations.itemEditor.unitRoll')" value="卷" />
            <el-option :label="t('quotations.itemEditor.unitKg')" value="kg" />
            <el-option :label="t('quotations.itemEditor.unitPiece')" value="件" />
          </el-select>
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colQuantity')" min-width="120">
        <template #default="{ row }">
          <el-input-number v-model="row.quantity" :min="0" :precision="2" style="width: 100%" />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnitPrice')" min-width="130">
        <template #default="{ row }">
          <el-input-number
            v-model="row.unit_price"
            :min="0"
            :precision="2"
            style="width: 100%"
            @change="(v: number | undefined) => recalcTax(row, v)"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colUnitPriceWithTax')" min-width="130">
        <template #default="{ row }">
          <el-input-number
            v-model="row.unit_price_with_tax"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colAmount')" min-width="130" align="right">
        <template #default="{ row }"> {{ currency }} {{ formatAmount(row.amount) }} </template>
      </el-table-column>

      <el-table-column :label="t('quotations.itemEditor.colAction')" width="80" fixed="right" align="center">
        <template #default="{ $index }">
          <el-button link type="danger" @click="handleRemove($index)">{{ t('quotations.itemEditor.delete') }}</el-button>
        </template>
      </el-table-column>
    </el-table>
  </div>
</template>

<script setup lang="ts">
// 报价单明细编辑器脚本
// - v-model 双向绑定
// - 加载产品/色号
// - 含税单价 = 单价 × 1.13
import { ref, onMounted, watch } from 'vue'
import { useI18n } from 'vue-i18n'
import { Plus } from '@element-plus/icons-vue'
import { getProductColorList, getProductList } from '@/api/product'
import type { ProductColor } from '@/api/product'
import type { CreateQuotationItemDto } from '@/api/quotation'

// QuotationItemRow 覆盖 product_id 为可选（创建空明细时 product_id 为 undefined，用户选择后才有值）
interface QuotationItemRow extends Omit<CreateQuotationItemDto, 'product_id'> {
  product_id?: number
  _colors?: Array<{ id: number; color_code?: string; color_name?: string }>
  amount?: number
}

const { t } = useI18n({ useScope: 'global' })

const props = defineProps<{
  modelValue: QuotationItemRow[]
  currency: string
}>()
const emit = defineEmits<{
  (e: 'update:modelValue', value: QuotationItemRow[]): void
}>()

const products = ref<Array<{ id: number; product_name?: string; name?: string }>>([])

/** 创建一行空明细 */
function createBlankItem(): QuotationItemRow {
  return {
    product_id: undefined,
    color_id: undefined,
    unit: '米',
    quantity: 0,
    unit_price: 0,
    unit_price_with_tax: 0,
    specification: '',
    _colors: [],
  }
}

/** 添加明细 */
function handleAdd() {
  emit('update:modelValue', [...props.modelValue, createBlankItem()])
}

/** 删除明细 */
function handleRemove(idx: number) {
  const arr = [...props.modelValue]
  arr.splice(idx, 1)
  emit('update:modelValue', arr)
}

/** 产品变化时加载色号 */
async function handleProductChange(row: QuotationItemRow, productId: number | undefined) {
  // 重置色号与本地色号列表
  row.color_id = undefined
  row._colors = []
  if (!productId) return
  try {
    const res = await getProductColorList(productId)
    const data: ProductColor[] = res.data || []
    // 直接修改 row（因为是父组件 v-model 持有的对象引用）
    row._colors = data
    // 强制响应式：发出新数组
    emit('update:modelValue', [...props.modelValue])
  } catch {
    // 静默失败：色号可选
  }
}

/** 单价变化时自动重算含税单价（13%） */
function recalcTax(row: QuotationItemRow, unitPrice: number | undefined) {
  if (unitPrice === undefined) return
  row.unit_price = unitPrice
  row.unit_price_with_tax = +(unitPrice * 1.13).toFixed(2)
  // 触发金额更新
  emit('update:modelValue', [...props.modelValue])
}

/** 数量或单价变化时同步金额字段（金额仅用于显示） */
watch(
  () => props.modelValue.map(i => [i.quantity, i.unit_price]),
  () => {
    props.modelValue.forEach((i: QuotationItemRow) => {
      i.amount = (i.quantity || 0) * (i.unit_price || 0)
    })
  },
  { deep: true }
)

/** 金额格式化 */
function formatAmount(value?: number): string {
  if (value === undefined || value === null) return '0.00'
  return Number(value).toLocaleString('zh-CN', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })
}

onMounted(async () => {
  try {
    const res = await getProductList({ page: 1, page_size: 1000, is_active: true })
    const data = res.data
    if (data && typeof data === 'object' && 'list' in data) {
      products.value = data.list || []
    } else {
      products.value = []
    }
  } catch {
    products.value = []
  }
})
</script>

<style scoped>
.item-editor {
  width: 100%;
}
</style>
