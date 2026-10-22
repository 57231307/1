<template>
  <div class="notification">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>{{ t('notification.index.pageTitle') }}</span>
          <div class="header-actions">
            <el-badge :value="unreadCount" :hidden="unreadCount === 0" class="unread-badge">
              <el-button link type="primary" @click="handleMarkAllRead">{{
                t('notification.index.buttonMarkAllRead')
              }}</el-button>
            </el-badge>
          </div>
        </div>
      </template>

      <div class="toolbar">
        <el-radio-group v-model="statusFilter" @change="handleStatusFilterChange">
          <el-radio-button value="">{{ t('notification.index.optionAll') }}</el-radio-button>
          <el-radio-button value="UNREAD">{{
            t('notification.index.optionUnread')
          }}</el-radio-button>
          <el-radio-button value="READ">{{ t('notification.index.optionRead') }}</el-radio-button>
        </el-radio-group>
        <el-button
          type="primary"
          plain
          size="small"
          :disabled="selectedIds.length === 0"
          @click="handleBatchRead"
        >
          批量已读（{{ selectedIds.length }}）
        </el-button>
        <el-button plain size="small" @click="openSettings">{{
          t('system.notification.title')
        }}</el-button>
      </div>

      <div class="notification-list">
        <div
          v-for="item in notificationList"
          :key="item.id"
          class="notification-item"
          :class="{ unread: item.status === 'UNREAD' }"
        >
          <div class="item-header">
            <el-checkbox :model-value="isSelected(item.id)" @change="toggleSelect(item.id)" />
            <div class="item-type">
              <el-tag v-if="item.notificationType === 'SYSTEM'" type="danger">{{
                t('notification.index.typeSystem')
              }}</el-tag>
              <el-tag v-else-if="item.notificationType === 'INTERNAL'" type="primary">{{
                t('notification.index.typeInternal')
              }}</el-tag>
              <el-tag v-else-if="item.notificationType === 'EMAIL'" type="success">{{
                t('notification.index.typeEmail')
              }}</el-tag>
              <el-tag v-else type="warning">{{ item.notificationType }}</el-tag>
            </div>
            <div class="item-time">{{ item.createdAt }}</div>
          </div>
          <div class="item-title" @click="handleView(item)">
            <span v-if="item.status === 'UNREAD'" class="unread-dot"></span>
            {{ item.title }}
          </div>
          <div class="item-content" @click="handleView(item)">
            {{ item.content }}
          </div>
          <div class="item-actions">
            <el-button link type="primary" size="small" @click="handleView(item)">{{
              t('notification.index.buttonViewDetail')
            }}</el-button>
            <el-button
              v-if="item.status === 'UNREAD'"
              link
              type="primary"
              size="small"
              @click="handleMarkRead(item)"
              >{{ t('notification.index.buttonMarkRead') }}</el-button
            >
            <el-button link type="danger" size="small" @click="handleDelete(item)">{{
              t('notification.index.buttonDelete')
            }}</el-button>
          </div>
        </div>

        <el-empty
          v-if="notificationList.length === 0"
          :description="t('notification.index.messageNoData')"
        />
      </div>

      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        layout="total, prev, pager, next, jumper"
        :aria-label="t('notification.index.ariaPagination')"
      />
    </el-card>

    <!-- 详情对话框 -->
    <el-dialog
      v-model="detailDialogVisible"
      :title="t('notification.index.titleDetail')"
      width="600px"
      :aria-label="t('notification.index.ariaDetailDialog')"
    >
      <div v-if="currentNotification" class="notification-detail">
        <div class="detail-info">
          <div class="info-item">
            <span class="label">{{ t('notification.index.labelTitle') }}</span>
            <span class="value">{{ currentNotification.title }}</span>
          </div>
          <div class="info-item">
            <span class="label">{{ t('notification.index.labelType') }}</span>
            <el-tag v-if="currentNotification.notificationType === 'SYSTEM'" type="danger">{{
              t('notification.index.typeSystem')
            }}</el-tag>
            <el-tag
              v-else-if="currentNotification.notificationType === 'INTERNAL'"
              type="primary"
              >{{ t('notification.index.typeInternal') }}</el-tag
            >
            <el-tag v-else-if="currentNotification.notificationType === 'EMAIL'" type="success">{{
              t('notification.index.typeEmail')
            }}</el-tag>
            <el-tag v-else type="warning">{{ currentNotification.notificationType }}</el-tag>
          </div>
          <div class="info-item">
            <span class="label">{{ t('notification.index.labelCreatedAt') }}</span>
            <span class="value">{{ currentNotification.createdAt }}</span>
          </div>
        </div>
        <div class="detail-content">
          <div class="content-label">{{ t('notification.index.labelContent') }}</div>
          <div class="content-text">{{ currentNotification.content }}</div>
        </div>
      </div>
    </el-dialog>

    <!-- 通知设置（读 GET user-notification-settings，写 PUT 同名端点，存当前登录用户的单条通知偏好） -->
    <el-dialog v-model="settingsVisible" :title="t('system.notification.title')" width="520">
      <el-form
        v-loading="settingsLoading"
        :model="settingForm"
        label-width="150px"
        :aria-label="t('system.notification.aria.form')"
      >
        <el-form-item :label="t('system.notification.label.email')">
          <el-switch v-model="settingForm.email_enabled" />
        </el-form-item>
        <el-form-item :label="t('system.notification.label.internal')">
          <el-switch v-model="settingForm.internal_enabled" />
        </el-form-item>
        <el-divider content-position="left">{{ t('system.notification.divider.type') }}</el-divider>
        <el-form-item :label="t('system.notification.label.order')">
          <el-select v-model="settingForm.order_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.approval')">
          <el-select v-model="settingForm.approval_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.inventory')">
          <el-select v-model="settingForm.inventory_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.purchase')">
          <el-select v-model="settingForm.purchase_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.finance')">
          <el-select v-model="settingForm.finance_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('system.notification.label.system')">
          <el-select v-model="settingForm.system_notification_type" style="width: 100%">
            <el-option
              v-for="opt in NOTIFICATION_CHANNEL_OPTIONS"
              :key="opt.value"
              :label="t(opt.labelKey)"
              :value="opt.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="settingsVisible = false">{{
          t('notification.index.buttonCancel')
        }}</el-button>
        <el-button type="primary" :loading="settingsSaving" @click="handleSaveSetting">{{
          t('system.notification.button.save')
        }}</el-button>
      </template>
    </el-dialog>

    <!-- 系统公告直发：走 POST /notifications/announcement（notifications:create），
         与 OA 公告管理页的"建草稿再发布"是两条不同链路，互不替代 -->
    <el-card v-permission="'notifications:create'" shadow="hover" class="announce-card">
      <template #header>{{ t('system.notification.announce.title') }}</template>
      <el-form :model="announcementForm" label-width="100px">
        <el-form-item :label="t('system.notification.announce.label.title')" required>
          <el-input v-model="announcementForm.title" :maxlength="100" show-word-limit />
        </el-form-item>
        <el-form-item :label="t('system.notification.announce.label.content')" required>
          <el-input
            v-model="announcementForm.content"
            type="textarea"
            :rows="5"
            :maxlength="2000"
            show-word-limit
          />
        </el-form-item>
        <el-form-item :label="t('system.notification.announce.label.recipients')" required>
          <el-select
            v-model="announcementForm.userIds"
            multiple
            filterable
            :placeholder="t('system.notification.announce.placeholder.recipients')"
            style="width: 100%"
            @visible-change="handleRecipientsDropdown"
          >
            <el-option v-for="u in userOptions" :key="u.id" :label="u.username" :value="u.id" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="announceSending" @click="sendAnnouncement">{{
            t('system.notification.announce.button.send')
          }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { isDialogDismissal } from '@/utils/monitor';
import { ref, reactive, onMounted } from 'vue';
import { logAuxLoadFailure, logger } from '@/utils/logger';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getNotification,
  markAsRead,
  markAllAsRead,
  batchMarkAsRead,
  getSettings,
  updateSetting,
  deleteNotification,
  getUnreadCount,
  createAnnouncement,
  type NotificationSetting,
  type Notification,
} from '@/api/notification';
import { getUserList, type User } from '@/api/user';
import { useTableApi } from '@/composables/useTableApi';

const { t } = useI18n({ useScope: 'global' });

const unreadCount = ref(0);
const statusFilter = ref('');

// 批次 275：接入 useTableApi，消除手写 notificationList/pagination/fetchNotifications 重复
// useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
const {
  data: notificationList,
  page,
  pageSize,
  total,
  refresh: fetchNotifications,
  setQueryParam,
} = useTableApi<Notification>({
  url: '/notifications',
  // 钉到后端真实键（notification_handler::list_notifications → json!({ list, total })）
  listKey: 'list',
  pageSizeKey: 'page_size',
  onError: () => ElMessage.error(t('notification.index.messageLoadListFailed')),
});

// 批次 275：同步筛选条件到 useTableApi.queryParams
// statusFilter 变化时调用此函数（模板 @change="handleStatusFilterChange"）
const handleStatusFilterChange = () => {
  setQueryParam('status', statusFilter.value || undefined);
  page.value = 1;
  fetchNotifications();
};

const detailDialogVisible = ref(false);
const currentNotification = ref<Notification | null>(null);

const fetchUnreadCount = async () => {
  try {
    const res = await getUnreadCount();
    if (res.data !== undefined) {
      unreadCount.value = res.data;
    }
  } catch (e) {
    logAuxLoadFailure(t('notification.index.messageLoadUnreadCountFailed'), e);
  }
};

const handleView = async (item: Notification) => {
  if (!item.id) return;

  try {
    const res = await getNotification(item.id);
    if (res.data) {
      currentNotification.value = res.data;
      detailDialogVisible.value = true;
      fetchNotifications();
      fetchUnreadCount();
    }
  } catch (e) {
    ElMessage.error(t('notification.index.messageLoadDetailFailed'));
  }
};

const handleMarkRead = async (item: Notification) => {
  if (!item.id) return;

  try {
    await markAsRead(item.id);
    ElMessage.success(t('notification.index.messageMarkReadSuccess'));
    fetchNotifications();
    fetchUnreadCount();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('notification.index.messageOperationFailed')
    );
  }
};

// ===== 批量已读（batchMarkAsRead） =====
const selectedIds = ref<number[]>([]);

const toggleSelect = (id?: number) => {
  if (!id) return;
  const idx = selectedIds.value.indexOf(id);
  if (idx >= 0) selectedIds.value.splice(idx, 1);
  else selectedIds.value.push(id);
};

// Notification.id 可为 undefined，模板勾选状态需要安全判断
const isSelected = (id?: number) => (id !== undefined ? selectedIds.value.includes(id) : false);

const handleBatchRead = async () => {
  if (selectedIds.value.length === 0) return;
  try {
    await batchMarkAsRead({ ids: selectedIds.value });
    ElMessage.success(t('notification.index.messageOperationSuccess'));
    selectedIds.value = [];
    fetchNotifications();
    fetchUnreadCount();
  } catch (e: unknown) {
    ElMessage.error(e instanceof Error ? e.message : String(e));
  }
};

// ===== 通知设置（读 GET user-notification-settings，写 PUT 同名端点，存当前登录用户单条偏好） =====
// 通知方式取值必须与后端 user_notification_setting 模型的 notification_type 词表逐字相同
// （email、internal、both、none）。投递侧 should_send_email 与 should_send_internal 只按这四个值判定，
// 写入词表外的值会让该类别的邮件与站内通知双双停发，属静默丢通知。
type NotificationChannel = 'email' | 'internal' | 'both' | 'none';

// 六个类别下拉共用的候选集，避免在同一页里各写一套取值
const NOTIFICATION_CHANNEL_OPTIONS: { value: NotificationChannel; labelKey: string }[] = [
  { value: 'email', labelKey: 'system.notification.option.emailOnly' },
  { value: 'internal', labelKey: 'system.notification.option.internalOnly' },
  { value: 'both', labelKey: 'system.notification.option.all' },
  { value: 'none', labelKey: 'system.notification.option.none' },
];

const settingsVisible = ref(false);
const settingsLoading = ref(false);
const settingsSaving = ref(false);
// 初值与后端 get_or_create_default 落库的默认偏好一致，仅在 GET 失败时作为可见兜底；GET 成功后被出参覆盖
const settingForm = reactive<NotificationSetting>({
  email_enabled: true,
  internal_enabled: true,
  order_notification_type: 'both',
  approval_notification_type: 'both',
  inventory_notification_type: 'both',
  purchase_notification_type: 'both',
  finance_notification_type: 'both',
  system_notification_type: 'internal',
});

const openSettings = async () => {
  settingsVisible.value = true;
  settingsLoading.value = true;
  try {
    const res = await getSettings();
    if (!res.data) {
      logAuxLoadFailure(t('system.notification.message.loadFailed'), res);
      ElMessage.error(t('system.notification.message.loadFailed'));
      return;
    }
    Object.assign(settingForm, res.data);
  } catch (e: unknown) {
    logAuxLoadFailure(t('system.notification.message.loadFailed'), e);
    ElMessage.error(t('system.notification.message.loadFailed'));
  } finally {
    settingsLoading.value = false;
  }
};

const handleSaveSetting = async () => {
  settingsSaving.value = true;
  try {
    await updateSetting({
      email_enabled: settingForm.email_enabled,
      internal_enabled: settingForm.internal_enabled,
      order_notification_type: settingForm.order_notification_type,
      approval_notification_type: settingForm.approval_notification_type,
      inventory_notification_type: settingForm.inventory_notification_type,
      purchase_notification_type: settingForm.purchase_notification_type,
      finance_notification_type: settingForm.finance_notification_type,
      system_notification_type: settingForm.system_notification_type,
    });
    ElMessage.success(t('system.notification.message.saveSuccess'));
    settingsVisible.value = false;
  } catch (e: unknown) {
    logger.error(t('system.notification.message.saveFailed'), e);
    ElMessage.error(t('system.notification.message.saveFailed'));
  } finally {
    settingsSaving.value = false;
  }
};

const handleMarkAllRead = async () => {
  try {
    await markAllAsRead();
    ElMessage.success(t('notification.index.messageOperationSuccess'));
    fetchNotifications();
    fetchUnreadCount();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('notification.index.messageOperationFailed')
    );
  }
};

const handleDelete = async (item: Notification) => {
  if (!item.id) return;

  try {
    await ElMessageBox.confirm(
      t('notification.index.messageConfirmDelete'),
      t('notification.index.titleConfirm'),
      {
        confirmButtonText: t('notification.index.buttonConfirm'),
        cancelButtonText: t('notification.index.buttonCancel'),
        type: 'warning',
      }
    );

    await deleteNotification(item.id);
    ElMessage.success(t('notification.index.messageDeleteSuccess'));
    fetchNotifications();
    fetchUnreadCount();
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (!isDialogDismissal(e)) {
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('notification.index.messageDeleteFailed')
      );
    }
  }
};

const hasLoaded = createLazyLoader();

onMounted(() => {
  // 批次 275：useTableApi 构造时自动初始加载通知列表，无需手动调用 fetchNotifications
  loadIfNot('unreadCount', fetchUnreadCount, hasLoaded);
});

// ===== 系统公告直发（POST /notifications/announcement，权限 notifications:create）=====
// 收件人候选按下拉首次展开时懒加载：本页对全部登录用户可达，而公告卡只对持
// notifications:create 的角色渲染，无条件取用户列表会给无权限角色白打一次请求。
const userOptions = ref<User[]>([]);
const announceSending = ref(false);
const announcementForm = reactive({
  title: '',
  content: '',
  userIds: [] as number[],
});

const fetchUserOptions = async () => {
  try {
    const res = await getUserList({ page: 1, page_size: 200 });
    userOptions.value = res.data.users;
  } catch (error) {
    logAuxLoadFailure(t('system.notification.message.loadUsersFailed'), error);
  }
};

const handleRecipientsDropdown = (visible: boolean) => {
  if (visible) {
    loadIfNot('announceUsers', fetchUserOptions, hasLoaded);
  }
};

const sendAnnouncement = async () => {
  if (!announcementForm.title.trim()) {
    ElMessage.warning(t('system.notification.announce.message.titleRequired'));
    return;
  }
  if (!announcementForm.content.trim()) {
    ElMessage.warning(t('system.notification.announce.message.contentRequired'));
    return;
  }
  if (announcementForm.userIds.length === 0) {
    ElMessage.warning(t('system.notification.announce.message.recipientsRequired'));
    return;
  }
  announceSending.value = true;
  try {
    const res = await createAnnouncement({
      userIds: announcementForm.userIds,
      title: announcementForm.title,
      content: announcementForm.content,
    });
    const count = res.data?.delivered_count;
    if (typeof count !== 'number') {
      // 后端投递结果恒出 delivered_count 键，取到非数值即契约异常，
      // 抛错交外层 catch 显式提示，绝不静默兜底成 0（掩盖投递数会让用户误信无人收到）
      throw new Error(t('system.notification.announce.message.deliveryCountMissing'));
    }
    ElMessage.success(t('system.notification.announce.message.sendSuccess', { count }));
    announcementForm.title = '';
    announcementForm.content = '';
    announcementForm.userIds = [];
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) ||
        t('system.notification.announce.message.sendFailed')
    );
  } finally {
    announceSending.value = false;
  }
};
</script>

<style scoped>
.notification .announce-card {
  margin-top: 20px;
}

.notification .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.notification .card-header .header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
}

.notification .toolbar {
  margin-bottom: 16px;
}

.notification .notification-list .notification-item {
  padding: 16px;
  border: 1px solid #ebeef5;
  border-radius: 4px;
  margin-bottom: 12px;
  background: #fff;
}

.notification .notification-list .notification-item.unread {
  border-left: 4px solid #409eff;
  background: #f5f7fa;
}

.notification .notification-list .notification-item .item-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.notification .notification-list .notification-item .item-header .item-time {
  font-size: 12px;
  color: #909399;
}

.notification .notification-list .notification-item .item-title {
  font-size: 15px;
  font-weight: 500;
  margin-bottom: 8px;
  cursor: pointer;
  display: flex;
  align-items: center;
}

.notification .notification-list .notification-item .item-title .unread-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #f56c6c;
  margin-right: 8px;
}

.notification .notification-list .notification-item .item-content {
  color: #606266;
  margin-bottom: 12px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1.6;
}

.notification .notification-list .notification-item .item-actions {
  display: flex;
  gap: 8px;
}

.notification .notification-detail .detail-info {
  margin-bottom: 20px;
}

.notification .notification-detail .detail-info .info-item {
  margin-bottom: 12px;
  display: flex;
  align-items: center;
}

.notification .notification-detail .detail-info .info-item .label {
  color: #909399;
  min-width: 80px;
}

.notification .notification-detail .detail-info .info-item .value {
  font-weight: 500;
}

.notification .notification-detail .detail-content {
  border-top: 1px solid #ebeef5;
  padding-top: 20px;
}

.notification .notification-detail .detail-content .content-label {
  color: #909399;
  margin-bottom: 8px;
}

.notification .notification-detail .detail-content .content-text {
  line-height: 1.8;
  color: #303133;
}
</style>
