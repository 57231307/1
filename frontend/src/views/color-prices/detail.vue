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
              <el-tag v-if="price?.customer_level" :type="getLevelColor(price.customer_level)">
                {{ getLevelLabel(price.customer_level) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item label="季节">
              <el-tag v-if="price?.season" :type="getSeasonColor(price.season)">
                {{ getSeasonLabel(price.season) }}
              </el-tag>
              <span v-else>-</span>
            </el-descriptions-item>
            <el-descriptions-item label="优先级">{{ price?.priority }}</el-descriptions-item>
            <el-descriptions-item label="生效日期">
              {{ price?.effective_from }} ~ {{ price?.effective_to || '长期' }}
            </el-descriptions-item>
            <el-descriptions-item label="状态">
              <el-tag :type="price?.is_active ? 'success' : 'info'">
                {{ price?.is_active ? '启用' : '禁用' }}
              </el-tag>
            </el-descriptions-item>
            <el-descriptions-item label="审批状态" :span="2">
              <el-tag :type="getApprovalColor(price?.approval_status || '')">
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
      <el-table :data="tiers" border aria-label="色卡阶梯价列表">
        <el-table-column prop="sequence" label="顺序" width="80" />
        <el-table-column prop="min_quantity" label="起订量" width="120" />
        <el-table-column prop="max_quantity" label="上限" width="120">
          <template #default="{ row }">{{ row.max_quantity || '无限' }}</template>
        </el-table-column>
        <el-table-column prop="tier_price" label="阶梯价" width="120" />
        <el-table-column label="客户等级" width="120">
          <template #default="{ row }">
            <el-tag v-if="row.customer_level" :type="getLevelColor(row.customer_level)">
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

    <!-- 批次 157c P1-1 修复：添加阶梯价对话框 -->
    <el-dialog v-model="tierDialogVisible" title="添加阶梯价" width="480px" aria-label="添加阶梯价对话框">
      <el-form ref="tierFormRef" :model="tierForm" :rules="tierRules" label-width="100px" aria-label="阶梯价表单">
        <el-form-item label="起订量" prop="min_quantity">
          <el-input-number v-model="tierForm.min_quantity" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="上限">
          <el-input-number v-model="tierForm.max_quantity" :min="0" style="width: 100%" placeholder="0 表示无限" />
        </el-form-item>
        <el-form-item label="阶梯价" prop="tier_price">
          <el-input-number v-model="tierForm.tier_price" :min="0" :precision="4" style="width: 100%" />
        </el-form-item>
        <el-form-item label="客户等级">
          <el-select v-model="tierForm.customer_level" placeholder="通用（留空）" clearable style="width: 100%">
            <el-option label="VIP" value="VIP" />
            <el-option label="A" value="A" />
            <el-option label="B" value="B" />
            <el-option label="C" value="C" />
          </el-select>
        </el-form-item>
        <el-form-item label="顺序">
          <el-input-number v-model="tierForm.sequence" :min="1" style="width: 100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tierDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="tierSubmitting" @click="onSubmitTier">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  getColorPrice,
  getColorPriceHistory,
  listTiers,
  createTier,
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
  type CreatePriceTierDto,
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
  } catch (e: unknown) {
    // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error('加载失败：' + (e instanceof Error ? e.message : String(e)))
  } finally {
    loading.value = false
  }
}

// 批次 157c P1-1 修复：添加阶梯价对话框接入 createTier API
const tierDialogVisible = ref(false)
const tierSubmitting = ref(false)
const tierFormRef = ref<FormInstance>()
const tierForm = reactive<{
  min_quantity: number
  max_quantity: number | null
  tier_price: number
  customer_level: string | null
  sequence: number
}>({
  min_quantity: 1,
  max_quantity: null,
  tier_price: 0,
  customer_level: null,
  sequence: 1,
})
const tierRules: FormRules = {
  min_quantity: [{ required: true, message: '请输入起订量', trigger: 'blur' }],
  tier_price: [{ required: true, message: '请输入阶梯价', trigger: 'blur' }],
}

const handleAddTier = () => {
  tierForm.min_quantity = 1
  tierForm.max_quantity = null
  tierForm.tier_price = 0
  tierForm.customer_level = null
  tierForm.sequence = (tiers.value.length || 0) + 1
  tierDialogVisible.value = true
}

const onSubmitTier = async () => {
  if (!tierFormRef.value) return
  await tierFormRef.value.validate(async valid => {
    if (!valid) return
    tierSubmitting.value = true
    try {
      const payload: CreatePriceTierDto = {
        product_color_price_id: priceId,
        min_quantity: tierForm.min_quantity,
        max_quantity: tierForm.max_quantity && tierForm.max_quantity > 0 ? tierForm.max_quantity : null,
        tier_price: tierForm.tier_price,
        customer_level: tierForm.customer_level || null,
        sequence: tierForm.sequence,
      }
      await createTier(payload)
      ElMessage.success('阶梯价添加成功')
      tierDialogVisible.value = false
      loadData()
    } catch (e: unknown) {
      // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error('添加失败：' + (e instanceof Error ? e.message : String(e)))
    } finally {
      tierSubmitting.value = false
    }
  })
}

const handleDeleteTier = async (row: PriceTier) => {
  try {
    await ElMessageBox.confirm(`确定删除阶梯 #${row.id}？`, '确认', { type: 'warning' })
    await deleteTier(row.id)
    ElMessage.success('删除成功')
    loadData()
  } catch (e: unknown) {
    // v11 批次 174 P2-1 修复：catch (e: any) 改为 unknown + 类型守卫
    if (e === 'cancel') return
    ElMessage.error('删除失败：' + (e instanceof Error ? e.message : String(e)))
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
