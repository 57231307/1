<!--
  面料多色号定价扩展 - 批量调价页
  选择色号 + 调价模式 + 审批
  创建时间: 2026-06-18
-->
<template>
  <div class="batch-adjust">
    <el-page-header @back="$router.back()" content="批量调价" />

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="14">
        <el-card header="选择色号">
          <el-form :inline="true" :model="filterForm" class="filter-form">
            <el-form-item label="产品 ID">
              <el-input v-model.number="filterForm.product_id" placeholder="产品 ID" clearable style="width: 140px" />
            </el-form-item>
            <el-form-item label="客户等级">
              <el-select v-model="filterForm.customer_level" placeholder="全部" clearable style="width: 120px">
                <el-option label="VIP" value="VIP" />
                <el-option label="NORMAL" value="NORMAL" />
                <el-option label="GOLD" value="GOLD" />
                <el-option label="SILVER" value="SILVER" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="loadPrices">查询</el-button>
            </el-form-item>
          </el-form>
          <el-table
            :data="prices"
            v-loading="loading"
            border
            stripe
            @selection-change="handleSelectionChange"
            max-height="500"
          >
            <el-table-column type="selection" width="55" />
            <el-table-column prop="id" label="ID" width="80" />
            <el-table-column prop="product_id" label="产品" width="100" />
            <el-table-column prop="color_id" label="色号" width="100" />
            <el-table-column label="客户等级" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
                  {{ getLevelLabel(row.customer_level) }}
                </el-tag>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column label="基础价" width="140">
              <template #default="{ row }">{{ formatPrice(row.base_price, row.currency) }}</template>
            </el-table-column>
            <el-table-column label="币种" width="80" prop="currency" />
          </el-table>
        </el-card>
      </el-col>

      <el-col :span="10">
        <el-card header="调价设置">
          <el-form :model="form" label-width="120px">
            <el-form-item label="已选色号">
              <el-tag type="info">共 {{ selectedRows.length }} 条</el-tag>
            </el-form-item>
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
            </el-form-item>
            <el-form-item v-if="form.mode === 'fixed'" label="调价金额">
              <el-input-number v-model="form.fixedAmount" :precision="2" style="width: 200px" />
              <span style="margin-left: 8px">元/米</span>
            </el-form-item>
            <el-form-item label="调价原因">
              <el-input
                v-model="form.changeReason"
                type="textarea"
                :rows="3"
                placeholder="请说明调价原因"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="handleSubmit" :loading="submitting" :disabled="selectedRows.length === 0">
                提交批量调价
              </el-button>
              <el-button @click="handleCalculate">价格计算演示</el-button>
            </el-form-item>
          </el-form>

          <el-divider />

          <el-alert
            v-if="result"
            :title="`调价结果：自动通过 ${result.auto_approved.length} 条，待审批 ${result.pending_approval.length} 条`"
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
import { useRoute } from 'vue-router'
import { ElMessage } from 'element-plus'
import {
  listColorPrices,
  batchAdjustColorPrices,
  calculateColorPrice,
  formatPrice,
  getLevelLabel,
  getLevelColor,
  type ColorPriceListItem,
  type ListColorPricesQuery,
} from '@/api/color-price'

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
    const res = await listColorPrices(filterForm)
    prices.value = res.items
    // 处理 query 参数 ids
    const ids = (route.query.ids as string)?.split(',').map(Number).filter(Boolean) || []
    if (ids.length > 0) {
      selectedRows.value = res.items.filter((p) => ids.includes(p.id))
    }
  } catch (e: unknown) {
    ElMessage.error('加载失败：' + (e instanceof Error ? e.message : '未知错误'))
  } finally {
    loading.value = false
  }
}

const handleSelectionChange = (rows: ColorPriceListItem[]) => {
  selectedRows.value = rows
}

const handleSubmit = async () => {
  if (selectedRows.value.length === 0) {
    ElMessage.warning('请先选择色号')
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
    ElMessage.success(`调价提交完成：自动通过 ${r.auto_approved.length}，待审批 ${r.pending_approval.length}`)
  } catch (e: unknown) {
    ElMessage.error('提交失败：' + (e instanceof Error ? e.message : '未知错误'))
  } finally {
    submitting.value = false
  }
}

const handleCalculate = async () => {
  if (selectedRows.value.length === 0) {
    ElMessage.warning('请先选择色号')
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
      `价格计算：基础价 ${r.base_price} → 最终价 ${formatPrice(r.final_price, r.currency)}（${r.applied_rule}）`,
    )
  } catch (e: unknown) {
    ElMessage.error('计算失败：' + (e instanceof Error ? e.message : '未知错误'))
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
