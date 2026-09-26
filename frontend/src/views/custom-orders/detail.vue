<!--
  定制订单详情页
  - Tab 切换：基本信息 / 工艺节点 / 质量异常 / 售后
  - 操作：编辑（草稿）/ 取消（草稿）/ 推进状态
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <div v-loading="loading" class="custom-order-detail">
    <el-card v-if="order">
      <template #header>
        <div class="card-header">
          <div>
            <span class="title">{{
              t('customOrders.detail.title', { orderNo: order.order_no })
            }}</span>
            <el-tag :type="STATUS_COLORS[order.status] || 'info'" style="margin-left: 12px">
              {{ getStatusLabel(order.status) }}
            </el-tag>
          </div>
          <div>
            <el-button @click="$router.push('/custom-orders')">{{
              t('customOrders.detail.buttonBack')
            }}</el-button>
            <el-button
              v-if="order.status === 'draft'"
              type="primary"
              @click="$router.push(`/custom-orders/${order.id}/edit`)"
            >
              {{ t('customOrders.detail.buttonEdit') }}
            </el-button>
            <el-button
              v-if="order.status !== 'completed' && order.status !== 'cancelled'"
              type="success"
              @click="handleAdvance"
            >
              {{ t('customOrders.detail.buttonAdvance') }}
            </el-button>
            <el-button v-if="order.status === 'draft'" type="danger" @click="handleCancel">
              {{ t('customOrders.detail.buttonCancel') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <!-- 基本信息 -->
        <el-tab-pane :label="t('customOrders.detail.tabInfo')" name="info">
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="t('customOrders.detail.labelOrderNo')">{{
              order.order_no
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelCustomerId')">{{
              order.customer_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelProductId')">{{
              order.product_id
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelColorId')">{{
              order.color_id || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelSpec')" :span="2">{{
              order.spec
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelQuantity')"
              >{{ order.quantity }} {{ order.unit }}</el-descriptions-item
            >
            <el-descriptions-item :label="t('customOrders.detail.labelAmount')">
              {{ order.currency }} {{ order.total_amount || '-' }}
            </el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelYarnSpec')">{{
              order.yarn_spec || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelDyeMethod')">{{
              order.dye_method || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelFinishingMethod')">{{
              order.finishing_method || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelExpectedDelivery')">{{
              order.expected_delivery_date || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelActualDelivery')">{{
              order.actual_delivery_date || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelSalesOrder')">{{
              order.sales_order_id || '-'
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelCreatedAt')">{{
              order.created_at
            }}</el-descriptions-item>
            <el-descriptions-item :label="t('customOrders.detail.labelUpdatedAt')" :span="2">{{
              order.updated_at
            }}</el-descriptions-item>
            <!-- v3 复审 P1-5：展示订单备注 -->
            <el-descriptions-item :label="t('customOrders.detail.labelNotes')" :span="2">{{
              order.notes || '-'
            }}</el-descriptions-item>
          </el-descriptions>
        </el-tab-pane>

        <!-- 工艺节点 -->
        <el-tab-pane :label="tabProcessNodesLabel" name="nodes">
          <div class="node-toolbar">
            <el-button type="primary" plain size="small" @click="nodeDialogVisible = true">
              {{ t('customOrders.detail.addNode') }}
            </el-button>
          </div>
          <ProcessFlow :nodes="order.process_nodes || []" />

          <!-- 节点操作：更新状态 / 添加日志（updateProcessNode / createNodeLog） -->
          <div class="node-ops">
            <el-select
              v-model="selectedNodeId"
              :placeholder="t('customOrders.detail.selectNode')"
              style="width: 260px"
            >
              <el-option
                v-for="node in order.process_nodes || []"
                :key="node.id"
                :label="`#${node.sequence} ${node.node_name}`"
                :value="node.id"
              />
            </el-select>
            <el-button type="primary" plain :disabled="!selectedNodeId" @click="openUpdateNode">
              {{ t('customOrders.detail.updateNodeStatus') }}
            </el-button>
            <el-button plain :disabled="!selectedNodeId" @click="openNodeLog">
              {{ t('customOrders.detail.addNodeLog') }}
            </el-button>
          </div>
        </el-tab-pane>

        <!-- 质量异常 -->
        <el-tab-pane :label="tabQualityIssuesLabel" name="issues">
          <QualityCheck
            :order-id="order.id"
            :issues="order.quality_issues || []"
            @refresh="loadData"
          />
        </el-tab-pane>

        <!-- 售后 -->
        <el-tab-pane :label="tabAfterSalesLabel" name="aftersales">
          <AfterSalesPanel
            :order-id="order.id"
            :after-sales="order.after_sales || []"
            @refresh="loadData"
          />
        </el-tab-pane>
      </el-tabs>
    </el-card>
    <!-- 空态：订单不存在或加载失败时给出可见提示（避免整页空白） -->
    <el-empty v-else-if="!loading" :description="t('customOrders.detail.notFound')" />

    <!-- 新增节点（createProcessNode） -->
    <el-dialog v-model="nodeDialogVisible" :title="t('customOrders.detail.addNode')" width="480">
      <el-form :model="nodeForm" label-width="110px">
        <el-form-item :label="t('customOrders.detail.nodeType')" required>
          <el-select v-model="nodeForm.node_type" style="width: 100%">
            <el-option label="裁剪" value="cutting" />
            <el-option label="缝制" value="sewing" />
            <el-option label="染色" value="dyeing" />
            <el-option label="后整理" value="finishing" />
            <el-option label="检验" value="inspection" />
            <el-option label="包装" value="packing" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.nodeName')" required>
          <el-input v-model="nodeForm.node_name" />
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.nodeSequence')" required>
          <el-input-number v-model="nodeForm.sequence" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.plannedStart')">
          <el-input v-model="nodeForm.planned_start_date" placeholder="2026-01-01" />
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.plannedEnd')">
          <el-input v-model="nodeForm.planned_end_date" placeholder="2026-01-31" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="nodeDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="nodeSaving" @click="handleCreateNode">
          {{ t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 更新节点状态（updateProcessNode） -->
    <el-dialog
      v-model="nodeUpdateVisible"
      :title="t('customOrders.detail.updateNodeStatus')"
      width="480"
    >
      <el-form :model="nodeUpdateForm" label-width="110px">
        <el-form-item :label="t('customOrders.detail.nodeStatus')" required>
          <el-select v-model="nodeUpdateForm.status" style="width: 100%">
            <el-option label="待开始" value="pending" />
            <el-option label="进行中" value="in_progress" />
            <el-option label="已完成" value="completed" />
            <el-option label="已跳过" value="skipped" />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.nodeNotes')">
          <el-input v-model="nodeUpdateForm.notes" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="nodeUpdateVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="nodeSaving" @click="handleUpdateNode">
          {{ t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>

    <!-- 添加节点日志（createNodeLog） -->
    <el-dialog v-model="nodeLogVisible" :title="t('customOrders.detail.addNodeLog')" width="480">
      <el-form :model="nodeLogForm" label-width="110px">
        <el-form-item :label="t('customOrders.detail.logAction')" required>
          <el-input v-model="nodeLogForm.action" placeholder="如：start / pause / complete" />
        </el-form-item>
        <el-form-item :label="t('customOrders.detail.logContent')">
          <el-input v-model="nodeLogForm.log_content" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="nodeLogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" :loading="nodeSaving" @click="handleCreateNodeLog">
          {{ t('common.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted, watch } from 'vue';
import { useUserStore } from '@/store/user';
import { useRoute } from 'vue-router';
import { useI18n } from 'vue-i18n';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getCustomOrder,
  advanceCustomOrder,
  cancelCustomOrder,
  createProcessNode,
  updateProcessNode,
  createNodeLog,
  CUSTOM_ORDER_STATUS_COLORS as STATUS_COLORS,
} from '@/api/custom-order';
import type { CustomOrderDetail } from '@/api/custom-order';
import ProcessFlow from '@/components/ProcessFlow.vue';
import QualityCheck from '@/components/QualityCheck.vue';
import logger from '@/utils/logger';
import AfterSalesPanel from '@/components/AfterSalesPanel.vue';

// v11 批次 181 P2-1 修复：CustomOrderDetail 已声明 quality_issues 和 after_sales 字段
// 不再需要本地扩展类型，直接使用 CustomOrderDetail

const route = useRoute();
const { t } = useI18n({ useScope: 'global' });
const loading = ref(false);
const order = ref<CustomOrderDetail | null>(null);
const activeTab = ref('info');

// Tab label 计算属性（避免模板中跨行 :label 导致 ESLint 解析错误）
const tabProcessNodesLabel = computed(() =>
  t('customOrders.detail.tabProcessNodes', {
    count: (order.value?.process_nodes || []).length,
  })
);
const tabQualityIssuesLabel = computed(() =>
  t('customOrders.detail.tabQualityIssues', {
    count: (order.value?.quality_issues || []).length,
  })
);
const tabAfterSalesLabel = computed(() =>
  t('customOrders.detail.tabAfterSales', {
    count: (order.value?.after_sales || []).length,
  })
);

// 状态标签映射函数（i18n）
const getStatusLabel = (status: string): string => {
  const map: Record<string, string> = {
    draft: t('customOrders.status.draft'),
    yarn_purchasing: t('customOrders.status.yarnPurchasing'),
    dyeing: t('customOrders.status.dyeing'),
    finishing: t('customOrders.status.finishing'),
    delivery: t('customOrders.status.delivery'),
    after_sales: t('customOrders.status.afterSales'),
    completed: t('customOrders.status.completed'),
    cancelled: t('customOrders.status.cancelled'),
  };
  return map[status] || status;
};

async function loadData() {
  const id = Number(route.params.id);
  if (!id) return;
  loading.value = true;
  try {
    const res = await getCustomOrder(id);
    order.value = (res.data || res) as unknown as CustomOrderDetail;
  } catch (e) {
    logger.error(t('customOrders.detail.messageLoadFailed'), e);
    ElMessage.error(t('customOrders.detail.messageLoadFailed'));
  } finally {
    loading.value = false;
  }
}

async function handleAdvance() {
  if (!order.value) return;
  try {
    await ElMessageBox.confirm(
      t('customOrders.detail.messageAdvanceConfirm'),
      t('customOrders.detail.messageAdvanceTitle'),
      { type: 'warning' }
    );
    // 与 list.vue 同源：操作人必须取当前登录用户，不得硬编码
    const userStore = useUserStore();
    const operatorId = userStore.userInfo?.id;
    if (!operatorId) {
      ElMessage.warning(t('customOrders.detail.operatorMissing'));
      return;
    }
    await advanceCustomOrder(order.value.id, {
      operator_id: operatorId,
      notes: t('customOrders.detail.messageAdvanceNotes'),
    });
    ElMessage.success(t('customOrders.detail.messageAdvanceSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.detail.messageAdvanceFailed'));
    }
  }
}

async function handleCancel() {
  if (!order.value) return;
  try {
    const { value: reason } = await ElMessageBox.prompt(
      t('customOrders.detail.messageCancelPrompt'),
      t('customOrders.detail.messageCancelTitle'),
      {
        inputPattern: /\S+/,
        inputErrorMessage: t('customOrders.detail.messageReasonRequired'),
      }
    );
    await cancelCustomOrder(order.value.id, reason);
    ElMessage.success(t('customOrders.detail.messageCancelSuccess'));
    loadData();
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const msg = e instanceof Error ? e.message : String(e);
      ElMessage.error(msg || t('customOrders.detail.messageCancelFailed'));
    }
  }
}

watch(() => route.params.id, loadData);
onMounted(loadData);

// ===== 节点维护（createProcessNode / updateProcessNode / createNodeLog） =====
const nodeDialogVisible = ref(false);
const nodeUpdateVisible = ref(false);
const nodeLogVisible = ref(false);
const nodeSaving = ref(false);
const selectedNodeId = ref<number | undefined>(undefined);

const nodeForm = reactive({
  node_type: 'cutting',
  node_name: '',
  sequence: 1,
  planned_start_date: '',
  planned_end_date: '',
});
const nodeUpdateForm = reactive({ status: 'in_progress', notes: '' });
const nodeLogForm = reactive({ action: '', log_content: '' });

const handleCreateNode = async () => {
  if (!order.value || !nodeForm.node_name || !nodeForm.node_type) {
    ElMessage.warning(t('customOrders.detail.nodeRequired'));
    return;
  }
  nodeSaving.value = true;
  try {
    await createProcessNode(order.value.id, {
      node_type: nodeForm.node_type,
      node_name: nodeForm.node_name,
      sequence: nodeForm.sequence,
      planned_start_date: nodeForm.planned_start_date || undefined,
      planned_end_date: nodeForm.planned_end_date || undefined,
    });
    ElMessage.success(t('customOrders.detail.nodeCreated'));
    nodeDialogVisible.value = false;
    await loadData();
  } catch (e) {
    ElMessage.error((e as Error).message || t('customOrders.detail.nodeFailed'));
  } finally {
    nodeSaving.value = false;
  }
};

const openUpdateNode = () => {
  const node = (order.value?.process_nodes || []).find(n => n.id === selectedNodeId.value);
  nodeUpdateForm.status = node?.status || 'in_progress';
  nodeUpdateForm.notes = '';
  nodeUpdateVisible.value = true;
};

const handleUpdateNode = async () => {
  if (!order.value || !selectedNodeId.value) return;
  nodeSaving.value = true;
  try {
    await updateProcessNode(order.value.id, selectedNodeId.value, {
      status: nodeUpdateForm.status,
      notes: nodeUpdateForm.notes || undefined,
    });
    ElMessage.success(t('customOrders.detail.nodeUpdated'));
    nodeUpdateVisible.value = false;
    await loadData();
  } catch (e) {
    ElMessage.error((e as Error).message || t('customOrders.detail.nodeFailed'));
  } finally {
    nodeSaving.value = false;
  }
};

const openNodeLog = () => {
  nodeLogForm.action = '';
  nodeLogForm.log_content = '';
  nodeLogVisible.value = true;
};

const handleCreateNodeLog = async () => {
  if (!order.value || !selectedNodeId.value || !nodeLogForm.action.trim()) {
    ElMessage.warning(t('customOrders.detail.logActionRequired'));
    return;
  }
  const userStore = useUserStore();
  const operatorId = userStore.userInfo?.id;
  if (!operatorId) {
    ElMessage.warning(t('customOrders.detail.operatorMissing'));
    return;
  }
  nodeSaving.value = true;
  try {
    await createNodeLog(order.value.id, selectedNodeId.value, {
      action: nodeLogForm.action,
      operator_id: operatorId,
      log_content: nodeLogForm.log_content || undefined,
    });
    ElMessage.success(t('customOrders.detail.logCreated'));
    nodeLogVisible.value = false;
    await loadData();
  } catch (e) {
    ElMessage.error((e as Error).message || t('customOrders.detail.nodeFailed'));
  } finally {
    nodeSaving.value = false;
  }
};
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
.node-toolbar {
  margin-bottom: 8px;
}
.node-ops {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}
</style>
