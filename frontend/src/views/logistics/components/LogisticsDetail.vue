<!--
  LogisticsDetail.vue - 物流管理运单详情
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="t('logistics.detail.title')"
    width="600px"
    :aria-label="t('logistics.detail.aria.dialog')"
    @update:model-value="(v: boolean) => emit('update:visible', v)"
  >
    <el-descriptions :column="2" border>
      <el-descriptions-item :label="t('logistics.detail.label.signedAt')">{{
        detail.signed_at || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.relatedOrder')">{{
        detail.order_no
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.logisticsCompany')">{{
        detail.logistics_company
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.trackingNumber')">{{
        detail.tracking_number
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.driverName')">{{
        detail.driver_name || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.driverPhone')">{{
        detail.driver_phone || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.freight')"
        >¥{{ detail.freight_fee || 0 }}</el-descriptions-item
      >
      <el-descriptions-item :label="t('logistics.detail.label.status')">
        <el-tag :type="getStatusTypeFmt(detail.status)">
          {{ statusTextFmt(detail.status) }}
        </el-tag>
      </el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.expectedArrival')">{{
        detail.expected_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.actualArrival')">{{
        detail.actual_arrival || '-'
      }}</el-descriptions-item>
      <el-descriptions-item :label="t('logistics.detail.label.notes')" :span="2">{{
        detail.notes || '-'
      }}</el-descriptions-item>
    </el-descriptions>

    <!-- 轨迹事件（getTrackingEvents / recordTrackingEvent） -->
    <div class="events-section">
      <div class="events-toolbar">
        <h4>运输轨迹</h4>
        <el-button type="primary" plain size="small" @click="openEventDialog"> 登记事件 </el-button>
        <el-button plain size="small" @click="openLinkPo">关联采购订单</el-button>
      </div>
      <el-table v-loading="eventsLoading" :data="events" border size="small" max-height="240">
        <el-table-column prop="id" label="ID" width="60" />
        <el-table-column prop="event_time" label="时间" width="160" />
        <el-table-column prop="event_type" label="类型" width="100" />
        <el-table-column prop="location" label="地点" width="120">
          <template #default="{ row }">{{ row.location || '-' }}</template>
        </el-table-column>
        <el-table-column prop="description" label="描述" min-width="150">
          <template #default="{ row }">{{ row.description || '-' }}</template>
        </el-table-column>
      </el-table>
    </div>

    <!-- 登记轨迹事件对话框 -->
    <el-dialog v-model="eventDialogVisible" title="登记轨迹事件" width="460" append-to-body>
      <el-form :model="eventForm" label-width="90px">
        <el-form-item label="事件类型" required>
          <el-select v-model="eventForm.event_type" style="width: 100%">
            <el-option label="提货" value="pickup" />
            <el-option label="运输中" value="in_transit" />
            <el-option label="到达" value="arrived" />
            <el-option label="签收" value="delivered" />
          </el-select>
        </el-form-item>
        <el-form-item label="发生时间" required>
          <el-input v-model="eventForm.event_time" placeholder="2026-01-01T08:00:00Z" />
        </el-form-item>
        <el-form-item label="地点"><el-input v-model="eventForm.location" /></el-form-item>
        <el-form-item label="描述" required>
          <el-input v-model="eventForm.description" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="eventDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="eventSaving" @click="handleSaveEvent">保存</el-button>
      </template>
    </el-dialog>

    <!-- 关联采购订单对话框 -->
    <el-dialog v-model="linkPoVisible" title="关联采购订单" width="380" append-to-body>
      <el-input-number v-model="linkPoId" :min="1" style="width: 100%" placeholder="采购订单 ID" />
      <template #footer>
        <el-button @click="linkPoVisible = false">取消</el-button>
        <el-button type="primary" :loading="linkSaving" @click="handleLinkPo">确定</el-button>
      </template>
    </el-dialog>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import {
  getTrackingEvents,
  recordTrackingEvent,
  linkPurchaseOrder,
  type LogisticsWaybill,
} from '@/api/logistics';
import { getStatusText, getStatusType } from '../composables/lgsFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 物流运单详情组件
 */
const props = defineProps<{
  // 对话框可见性
  visible: boolean;
  // 详情数据
  detail: LogisticsWaybill;
}>();

const emit = defineEmits<{
  'update:visible': [v: boolean];
}>();

// ===== 轨迹事件（getTrackingEvents / recordTrackingEvent / linkPurchaseOrder） =====
const events = ref<Array<Record<string, unknown>>>([]);
const eventsLoading = ref(false);
const eventDialogVisible = ref(false);
const eventSaving = ref(false);
const linkPoVisible = ref(false);
const linkSaving = ref(false);
const linkPoId = ref<number | undefined>(undefined);
const eventForm = reactive({
  event_type: 'in_transit',
  event_time: '',
  location: '',
  description: '',
});

const fetchEvents = async () => {
  if (!props.detail?.id) return;
  eventsLoading.value = true;
  try {
    const res = await getTrackingEvents(props.detail.id);
    const d = res.data as unknown;
    events.value = Array.isArray(d)
      ? d
      : ((d as { items?: Array<Record<string, unknown>> })?.items ?? []);
  } catch (e) {
    ElMessage.error((e as Error).message || '获取轨迹失败');
  } finally {
    eventsLoading.value = false;
  }
};

watch(
  () => props.visible,
  val => {
    if (val) void fetchEvents();
  }
);

const openEventDialog = () => {
  Object.assign(eventForm, {
    event_type: 'in_transit',
    event_time: new Date().toISOString(),
    location: '',
    description: '',
  });
  eventDialogVisible.value = true;
};

const handleSaveEvent = async () => {
  if (!props.detail?.id || !eventForm.description || !eventForm.event_time) {
    ElMessage.warning('请填写事件类型/时间/描述');
    return;
  }
  eventSaving.value = true;
  try {
    await recordTrackingEvent(props.detail.id, {
      event_time: eventForm.event_time,
      location: eventForm.location || undefined,
      description: eventForm.description,
      event_type: eventForm.event_type,
      data_source: 'manual',
    });
    ElMessage.success('事件已登记');
    eventDialogVisible.value = false;
    await fetchEvents();
  } catch (e) {
    ElMessage.error((e as Error).message || '登记失败');
  } finally {
    eventSaving.value = false;
  }
};

const openLinkPo = () => {
  linkPoId.value = undefined;
  linkPoVisible.value = true;
};

const handleLinkPo = async () => {
  if (!props.detail?.id || !linkPoId.value) return;
  linkSaving.value = true;
  try {
    await linkPurchaseOrder(props.detail.id, { po_id: linkPoId.value });
    ElMessage.success('已关联采购订单');
    linkPoVisible.value = false;
  } catch (e) {
    ElMessage.error((e as Error).message || '关联失败');
  } finally {
    linkSaving.value = false;
  }
};

// 状态标签与文本统一走 lgsFmts，保证列表 / 详情 / 对话框口径一致
const getStatusTypeFmt = getStatusType;
const statusTextFmt = getStatusText;
</script>
