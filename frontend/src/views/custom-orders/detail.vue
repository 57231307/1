<!--
  定制订单详情页
  - Tab 切换：基本信息 / 工艺节点 / 质量异常 / 售后
  - 操作：编辑（草稿）/ 取消（草稿）/ 推进状态
-->
<template>
  <div class="custom-order-detail" v-loading="loading">
    <el-card v-if="order">
      <template #header>
        <div class="card-header">
          <div>
            <span class="title">定制订单 {{ order.order_no }}</span>
            <el-tag :type="STATUS_COLORS[order.status] || 'info'" style="margin-left: 12px">
              {{ STATUS_LABELS[order.status] || order.status }}
            </el-tag>
          </div>
          <div>
            <el-button @click="$router.push('/custom-orders')">返回</el-button>
            <el-button
              v-if="order.status === 'draft'"
              type="primary"
              @click="$router.push(`/custom-orders/${order.id}/edit`)"
            >
              编辑
            </el-button>
            <el-button
              v-if="order.status !== 'completed' && order.status !== 'cancelled'"
              type="success"
              @click="handleAdvance"
            >
              推进状态
            </el-button>
            <el-button
              v-if="order.status === 'draft'"
              type="danger"
              @click="handleCancel"
            >
              取消
            </el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane label="基本信息" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="订单号">{{ order.order_no }}</el-descriptions-item>
            <el-descriptions-item label="客户 ID">{{ order.customer_id }}</el-descriptions-item>
            <el-descriptions-item label="产品 ID">{{ order.product_id }}</el-descriptions-item>
            <el-descriptions-item label="色号 ID">{{ order.color_id || '-' }}</el-descriptions-item>
            <el-descriptions-item label="规格" :span="2">{{ order.spec }}</el-descriptions-item>
            <el-descriptions-item label="数量">{{ order.quantity }} {{ order.unit }}</el-descriptions-item>
            <el-descriptions-item label="金额">
              {{ order.currency }} {{ order.total_amount || '-' }}
            </el-descriptions-item>
            <el-descriptions-item label="纱线规格">{{ order.yarn_spec || '-' }}</el-descriptions-item>
            <el-descriptions-item label="染色方法">{{ order.dye_method || '-' }}</el-descriptions-item>
            <el-descriptions-item label="后整理">{{ order.finishing_method || '-' }}</el-descriptions-item>
            <el-descriptions-item label="期望交付">{{ order.expected_delivery_date || '-' }}</el-descriptions-item>
            <el-descriptions-item label="实际交付">{{ order.actual_delivery_date || '-' }}</el-descriptions-item>
            <el-descriptions-item label="关联销售订单">{{ order.sales_order_id || '-' }}</el-descriptions-item>
            <el-descriptions-item label="创建时间">{{ order.created_at }}</el-descriptions-item>
            <el-descriptions-item label="更新时间" :span="2">{{ order.updated_at }}</el-descriptions-item>
            <!-- v3 复审 P1-5：展示订单备注 -->
            <el-descriptions-item label="备注" :span="2">{{ order.notes || '-' }}</el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 工艺节点 -->
        <el-tab-pane :label="`工艺节点（${(order.process_nodes || []).length}）`" name="nodes">
          <ProcessFlow :nodes="order.process_nodes || []" />
        </el-tab-pane>

        <!-- 质量异常 -->
        <el-tab-pane :label="`质量异常（${(order.quality_issues || []).length}）`" name="issues">
          <QualityCheck :order-id="order.id" :issues="order.quality_issues || []" @refresh="loadData" />
        </el-tab-pane>

        <!-- 售后 -->
        <el-tab-pane :label="`售后（${(order.after_sales || []).length}）`" name="aftersales">
          <AfterSalesPanel :order-id="order.id" :after-sales="order.after_sales || []" @refresh="loadData" />
        </el-tab-pane>
      </el-tabs>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  getCustomOrder,
  advanceCustomOrder,
  cancelCustomOrder,
  CUSTOM_ORDER_STATUS as STATUS_LABELS,
  CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS,
} from '@/api/custom-order'
import type { CustomOrderDetail, AfterSales } from '@/api/custom-order'
import ProcessFlow from '@/components/ProcessFlow.vue'
import QualityCheck from '@/components/QualityCheck.vue'
import logger from '@/utils/logger'
import AfterSalesPanel from '@/components/AfterSalesPanel.vue'

// P2-5：详情页 order 类型，扩展模板使用但 CustomOrderDetail 未声明的关联字段
// v11 批次 180 P2-1 修复：quality_issues 类型从 unknown[] 改为具体接口，与 QualityCheck.vue 的 QualityIssue 结构兼容
interface QualityIssueItem {
  id: number
  issue_type: string
  severity: string
  description?: string
  discovered_at?: string | number | Date
  status: string
  resolution?: string
  [key: string]: unknown
}

type CustomOrderDetailWithRelations = CustomOrderDetail & {
  quality_issues?: QualityIssueItem[]
  after_sales?: AfterSales[]
}

const route = useRoute()
const loading = ref(false)
const order = ref<CustomOrderDetailWithRelations | null>(null)
const activeTab = ref('info')

async function loadData() {
  const id = Number(route.params.id)
  if (!id) return
  loading.value = true
  try {
    const res = await getCustomOrder(id)
    order.value = (res.data || res) as unknown as CustomOrderDetailWithRelations
  } catch (e) {
    logger.error('加载订单失败', e)
    ElMessage.error('加载订单失败')
  } finally {
    loading.value = false
  }
}

async function handleAdvance() {
  if (!order.value) return
  try {
    await ElMessageBox.confirm('确定推进到下一阶段？', '确认推进', { type: 'warning' })
    await advanceCustomOrder(order.value.id, { operator_id: 1, notes: '状态推进' })
    ElMessage.success('推进成功')
    loadData()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e)
      ElMessage.error(msg || '推进失败')
    }
  }
}

async function handleCancel() {
  if (!order.value) return
  try {
    const { value: reason } = await ElMessageBox.prompt('请输入取消原因', '取消订单', {
      inputPattern: /\S+/,
      inputErrorMessage: '原因不能为空',
    })
    await cancelCustomOrder(order.value.id, reason)
    ElMessage.success('取消成功')
    loadData()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e)
      ElMessage.error(msg || '取消失败')
    }
  }
}

watch(() => route.params.id, loadData)
onMounted(loadData)
</script>

<style scoped>
.custom-order-detail {
  padding: 16px;
}
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.title {
  font-size: 18px;
  font-weight: 600;
}
</style>
