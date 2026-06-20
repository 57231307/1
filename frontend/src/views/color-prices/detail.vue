<!--
  面料多色号定价扩展 - 详情页
  色号价格详情 + 历史图表 + 阶梯价管理
  创建时间: 2026-06-18
-->
<template>
  <div class="color-price-detail" v-loading="loading">
    <el-page-header @back="$router.back()" content="色号价格详情" />

    <el-row :gutter="20" style="margin-top: 20px">
      <el-col :span="12">
        <el-card header="基本信息">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="ID">{{ price?.id }}</el-descriptions-item>
            <el-descriptions-item label="产品 ID">{{ price?.product_id }}</el-descriptions-item>
            <el-descriptions-item label="色号 ID">{{ price?.color_id }}</el-descriptions-item>
            <el-descriptions-item label="币种">{{ price?.currency }}</el-descriptions-item>
            <el-descriptions-item label="基础价">
              <span v-if="price">{{ formatPrice(price.base_price, price.currency) }}</span>
            </el-descriptions-item>
            <el-descriptions-item label="客户等级">
              <el-tag v-if="price?.customer_level" :type="getLevelColor(price.customer_level) as any">
                {{ getLevelLabel(price.customer_level) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item label="季节">
              <el-tag v-if="price?.season" :type="getSeasonColor(price.season) as any">
                {{ getSeasonLabel(price.season) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item label="优先级">{{ price?.priority }}</el-descriptions-item>
            <el-descriptions-item label="生效日期">
              {{ price?.effective_from }} ~ {{ price?.effective_to || '长期' }}
            </el-descriptions-item>
            <el-descriptions-item label="状态">
              <el-tag :type="price?.is_active ? 'success' : 'info' as any">
                {{ price?.is_active ? '启用' : '禁用' }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="审批状态" :span="2">
              <el-tag :type="getApprovalColor(price?.approval_status || '') as any">
                {{ getApprovalLabel(price?.approval_status || '') }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="备注" :span="2">{{ price?.notes || '-' }}</el-descriptions-item>
          </el-descriptions>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card header="价格历史">
          <PriceHistoryChart
            v-if="price"
            :history-data="history"
            :currency="price.currency"
            :height="300"
          />
          <el-empty v-else description="暂无数据" />
        </el-card>
      </el-col>
    </el-row>

    <el-card header="阶梯价" style="margin-top: 20px">
      <template #header>
        <div class="card-header">
          <span>阶梯价</span>
          <el-button type="primary" :icon="Plus" @click="handleAddTier">添加阶梯</el-button>
        </div>
      </template>
      <el-table :data="tiers" border>
        <el-table-column prop="sequence" label="顺序" width="80" />
        <el-table-column prop="min_quantity" label="起订量" width="120" />
        <el-table-column prop="max_quantity" label="上限" width="120">
          <template #default="{ row }">{{ row.max_quantity || '无限' }}</template>
        </el-table-column>
        <el-table-column prop="tier_price" label="阶梯价" width="120" />
        <el-table-column label="客户等级" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level) as any">
              {{ getLevelLabel(row.customer_level) }}
            </el-tag>
            <span v-else>通用</span>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="120" fixed="right">
          <template #default="{ row }">
            <el-button link type="danger" @click="handleDeleteTier(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getColorPrice,
  getColorPriceHistory,
  listTiers,
  deleteTier,
  formatPrice,
  getLevelLabel,
  getLevelColor,
  getSeasonLabel,
  getSeasonColor,
  getApprovalLabel,
  getApprovalColor,
  type ColorPriceDetail,
  type PriceHistoryItem,
  type PriceTier,
} from '@/api/color-price'
import PriceHistoryChart from '@/components/PriceHistoryChart.vue'

const route = useRoute()
const loading = ref(false)
const price = ref<ColorPriceDetail | null>(null)
const history = ref<PriceHistoryItem[]>([])
const tiers = ref<PriceTier[]>([])

const priceId = Number(route.params.id)

const loadData = async () => {
  loading.value = true
  try {
    price.value = await getColorPrice(priceId)
    const h = await getColorPriceHistory(priceId)
    history.value = h.items
    const t = await listTiers(priceId)
    tiers.value = t.items
  } catch (e: any) {
    ElMessage.error('加载失败：' + (e?.message || '未知错误'))
  } finally {
    loading.value = false
  }
}

const handleAddTier = () => {
  ElMessage.info('请通过批量调价页或 API 创建阶梯价')
}

const handleDeleteTier = async (row: PriceTier) => {
  try {
    await ElMessageBox.confirm(`确定删除阶梯 #${row.id}？`, '确认', { type: 'warning' })
    await deleteTier(row.id)
    ElMessage.success('删除成功')
    loadData()
  } catch (e: any) {
    if (e === 'cancel') return
    ElMessage.error('删除失败：' + (e?.message || '未知错误'))
  }
}

onMounted(() => {
  loadData()
})
</script>

<style scoped>
.color-price-detail { padding: 20px; }
.card-header { display: flex; justify-content: space-between; align-items: center; }
</style>
