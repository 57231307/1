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
      </div>

      <div class="notification-list">
        <div
          v-for="item in notificationList"
          :key="item.id"
          class="notification-item"
          :class="{ unread: item.status === 'UNREAD' }"
        >
          <div class="item-header">
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
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getNotification,
  markAsRead,
  markAllAsRead,
  deleteNotification,
  getUnreadCount,
  type Notification,
} from '@/api/notification';
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
    // 忽略错误
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
    if (e !== 'cancel') {
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
</script>

<style scoped>
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
