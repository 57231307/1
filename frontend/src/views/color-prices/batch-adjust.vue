<!--
  面料多色号定价扩展 - 批量调价页
  选择色号 + 调价模式 + 审批
  创建时间: 2026-06-18
-->
<template>
  <div class="batch-adjust">
    <el-page-header @back="$router.back()" :content="$t('colorPrices.batchAdjust.back')" />

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="14">
        <el-card :header="$t('colorPrices.batchAdjust.selectColor')">
          <el-form :inline="true" :model="filterForm" class="filter-form" :aria-label="$t('colorPrices.batchAdjust.filter.ariaLabel')">
            <el-form-item :label="$t('colorPrices.batchAdjust.filter.productId')">
              <el-input v-model.number="filterForm.product_id" :placeholder="$t('colorPrices.batchAdjust.filter.productId')" clearable style="width: 140px" />
            </el-form-item>
            <el-form-item :label="$t('colorPrices.batchAdjust.filter.customerLevel')">
              <el-select v-model="filterForm.customer_level" :placeholder="$t('colorPrices.common.all')" clearable style="width: 120px">
                <el-option :label="$t('colorPrices.customerLevel.VIP')" value="VIP" />
                <el-option :label="$t('colorPrices.customerLevel.NORMAL')" value="NORMAL" />
                <el-option :label="$t('colorPrices.customerLevel.GOLD')" value="GOLD" />
                <el-option :label="$t('colorPrices.customerLevel.SILVER')" value="SILVER" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="loadPrices">{{ $t('colorPrices.common.search') }}</el-button>
            </el-form-item>
          </el-form>
          <el-table
            :data="prices"
            v-loading="loading"
            border
            stripe
            @selection-change="handleSelectionChange"
            max-height="500"
            :aria-label="$t('colorPrices.batchAdjust.table.ariaLabel')"
          >
            <el-table-column type="selection" width="55" />
            <el-table-column prop="id" :label="$t('colorPrices.batchAdjust.table.id')" width="80" />
            <el-table-column prop="product_id" :label="$t('colorPrices.batchAdjust.table.product')" width="100" />
            <el-table-column prop="color_id" :label="$t('colorPrices.batchAdjust.table.color')" width="100" />
            <el-table-column :label="$t('colorPrices.batchAdjust.table.customerLevel')" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
                  {{ getLevelLabel(row.customer_level) }}
                </el-tag>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column :label="$t('colorPrices.batchAdjust.table.basePrice')" width="140">
              <template #default="{ row }">{{ formatPrice(row.base_price, row.currency) }}</template>
            </el-table-column>
            <el-table-column :label="$t('colorPrices.batchAdjust.table.currency')" width="80" prop="currency" />
          </el-table>
        </el-card>
      </el-col>

      <el-col :span="10">
        <el-card :header="$t('colorPrices.batchAdjust.adjustSetting')">
          <el-form :model="form" label-width="120px" :aria-label="$t('colorPrices.batchAdjust.form.ariaLabel')">
            <el-form-item :label="$t('colorPrices.batchAdjust.form.selectedCount')">
              <el-tag type="info">{{ $t('colorPrices.batchAdjust.form.countItems', { count: selectedRows.length }) }}</el-tag>
            </el-form-item>
            <el-form-item :label="$t('colorPrices.batchAdjust.form.mode')">
              <el-radio-group v-model="form.mode">
                <el-radio-button value="percentage">{{ $t('colorPrices.batchAdjust.form.modePercentage') }}</el-radio-button>
                <el-radio-button value="fixed">{{ $t('colorPrices.batchAdjust.form.modeFixed') }}</el-radio-button>
                <el-radio-button value="tier">{{ $t('colorPrices.batchAdjust.form.modeTier') }}</el-radio-button>
              </el-radio-group>
            </el-form-item>
            <el-form-item v-if="form.mode === 'percentage'" :label="$t('colorPrices.batchAdjust.form.adjustPercentage')">
              <el-input-number
                v-model="form.percentage"
                :min="-100"
                :max="100"
                :step="1"
                :precision="2"
                style="width: 200px"
              />
              <span style="margin-left: 8px">%</span>
            </el-form-item>
            <el-form-item v-if="form.mode === 'fixed'" :label="$t('colorPrices.batchAdjust.form.adjustAmount')">
              <el-input-number v-model="form.fixedAmount" :precision="2" style="width: 200px" />
              <span style="margin-left: 8px">{{ $t('colorPrices.batchAdjust.form.unitYuanPerMeter') }}</span>
            </el-form-item>
            <el-form-item :label="$t('colorPrices.batchAdjust.form.changeReason')">
              <el-input
                v-model="form.changeReason"
                type="textarea"
                :rows="3"
                :placeholder="$t('colorPrices.batchAdjust.form.changeReasonPlaceholder')"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleSubmit" :loading="submitting" :disabled="selectedRows.length === 0">
                {{ $t('colorPrices.batchAdjust.form.submit') }}
              </el-button>
              <el-button @click="handleCalculate">{{ $t('colorPrices.batchAdjust.form.calculateDemo') }}</el-button>
            </el-form-item>
          </el-form>

          <el-divider />

          <el-alert
            v-if="result"
            :title="$t('colorPrices.batchAdjust.result', { auto: result.auto_approved.length, pending: result.pending_approval.length })"
            type="success"
            :closable="false"
          />
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  getColorPriceList,
  batchAdjustColorPrices,
  calculateColorPrice,
  formatPrice,
  getLevelColor,
  type ColorPriceListItem,
  type ListColorPricesQuery,
} from '@/api/color-price'

const { t } = useI18n({ useScope: 'global' })

// 状态码 → 本地化标签（响应式：随语言切换自动更新）
const getLevelLabel = (level: string | null | undefined) => t(`colorPrices.customerLevel.${level || 'default'}`)

const route = useRoute()
const loading = ref(false)
const submitting = ref(false)
const prices = ref<ColorPriceListItem[]>([])
const selectedRows = ref<ColorPriceListItem[]>([])
const result = ref<{ auto_approved: number[]; pending_approval: number[]; total: number } | null>(null)

const filterForm = reactive<ListColorPricesQuery>({
  page: 1,
  page_size: 50,
  is_active: true,
})

const form = reactive({
  mode: 'percentage' as 'percentage' | 'fixed' | 'tier',
  percentage: 5,
  fixedAmount: 1.0,
  changeReason: '',
})

const loadPrices = async () => {
  loading.value = true
  try {
    const res = await getColorPriceList(filterForm)
    prices.value = res.items
    // 处理 query 参数 ids
    const ids = (route.query.ids as string)?.split(',').map(Number).filter(Boolean) || []
    if (ids.length > 0) {
      selectedRows.value = res.items.filter((p) => ids.includes(p.id))
    }
  } catch (e: unknown) {
    ElMessage.error(t('colorPrices.message.loadFailed', { msg: e instanceof Error ? e.message : t('colorPrices.message.unknownError') }))
  } finally {
    loading.value = false
  }
}

const handleSelectionChange = (rows: ColorPriceListItem[]) => {
  selectedRows.value = rows
}

const handleSubmit = async () => {
  if (selectedRows.value.length === 0) {
    ElMessage.warning(t('colorPrices.message.selectColorFirst'))
    return
  }
  submitting.value = true
  try {
    const items = selectedRows.value.map((row) => ({
      price_id: row.id,
      adjustment_type: form.mode === 'tier' ? 'percentage' : (form.mode as 'percentage' | 'fixed'),
      adjustment_value: form.mode === 'percentage' ? form.percentage / 100 : form.fixedAmount,
    }))
    const r = await batchAdjustColorPrices({
      items,
      change_reason: form.changeReason,
    })
    result.value = r
    ElMessage.success(t('colorPrices.message.batchAdjustSuccess', { auto: r.auto_approved.length, pending: r.pending_approval.length }))
  } catch (e: unknown) {
    ElMessage.error(t('colorPrices.message.batchAdjustFailed', { msg: e instanceof Error ? e.message : t('colorPrices.message.unknownError') }))
  } finally {
    submitting.value = false
  }
}

const handleCalculate = async () => {
  if (selectedRows.value.length === 0) {
    ElMessage.warning(t('colorPrices.message.selectColorFirst'))
    return
  }
  const sample = selectedRows.value[0]
  try {
    const r = await calculateColorPrice({
      product_id: sample.product_id,
      color_id: sample.color_id,
      customer_level: sample.customer_level || 'NORMAL',
      quantity: 100,
      season: sample.season || undefined,
      currency: sample.currency,
    })
    ElMessage.success(
      t('colorPrices.message.priceCalc', {
        base: r.base_price,
        final: formatPrice(r.final_price, r.currency),
        rule: r.applied_rule,
      }),
    )
  } catch (e: unknown) {
    ElMessage.error(t('colorPrices.message.calcFailed', { msg: e instanceof Error ? e.message : t('colorPrices.message.unknownError') }))
  }
}

onMounted(() => {
  loadPrices()
})
</script>

<style scoped>
.batch-adjust { padding: 20px; }
.filter-form { margin-bottom: 16px; }
</style>
