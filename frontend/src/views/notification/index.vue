<template>
  <div class="notification">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>通知中心</span>
          <div class="header-actions">
            <el-badge :value="unreadCount" :hidden="unreadCount === 0" class="unread-badge">
              <el-button link type="primary" @click="handleMarkAllRead">全部标为已读</el-button>
            </el-badge>
          </div>
        </div>
      </template>

      <div class="toolbar">
        <el-radio-group v-model="statusFilter" @change="fetchNotifications">
          <el-radio-button value="">全部</el-radio-button>
          <el-radio-button value="UNREAD">未读</el-radio-button>
          <el-radio-button value="READ">已读</el-radio-button>
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
              <el-tag v-if="item.notificationType === 'SYSTEM'" type="danger">系统</el-tag>
              <el-tag v-else-if="item.notificationType === 'INTERNAL'" type="primary">内部</el-tag>
              <el-tag v-else-if="item.notificationType === 'EMAIL'" type="success">邮件</el-tag>
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
            <el-button link type="primary" size="small" @click="handleView(item)"
              >查看详情</el-button
            >
            <el-button
              v-if="item.status === 'UNREAD'"
              link
              type="primary"
              size="small"
              @click="handleMarkRead(item)"
              >标为已读</el-button
            >
            <el-button link type="danger" size="small" @click="handleDelete(item)">删除</el-button>
          </div>
        </div>

        <el-empty v-if="notificationList.length === 0" description="暂无通知" />
      </div>

      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.page_size"
        :total="pagination.total"
        layout="total, prev, pager, next, jumper"
        @current-change="fetchNotifications"
      />
    </el-card>

    <!-- 详情对话框 -->
    <el-dialog v-model="detailDialogVisible" title="通知详情" width="600px">
      <div v-if="currentNotification" class="notification-detail">
        <div class="detail-info">
          <div class="info-item">
            <span class="label">标题：</span>
            <span class="value">{{ currentNotification.title }}</span>
          </div>
          <div class="info-item">
            <span class="label">类型：</span>
            <el-tag v-if="currentNotification.notificationType === 'SYSTEM'" type="danger"
              >系统</el-tag
            >
            <el-tag v-else-if="currentNotification.notificationType === 'INTERNAL'" type="primary"
              >内部</el-tag
            >
            <el-tag v-else-if="currentNotification.notificationType === 'EMAIL'" type="success"
              >邮件</el-tag
            >
            <el-tag v-else type="warning">{{ currentNotification.notificationType }}</el-tag>
          </div>
          <div class="info-item">
            <span class="label">创建时间：</span>
            <span class="value">{{ currentNotification.createdAt }}</span>
          </div>
        </div>
        <div class="detail-content">
          <div class="content-label">内容：</div>
          <div class="content-text">{{ currentNotification.content }}</div>
        </div>
      </div>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listNotifications,
  getNotification,
  markAsRead,
  markAllAsRead,
  deleteNotification,
  getUnreadCount,
  type Notification,
} from '@/api/notification'

const notificationList = ref<Notification[]>([])
const unreadCount = ref(0)
const statusFilter = ref('')

const pagination = reactive({
  page: 1,
  page_size: 20,
  total: 0,
})

const detailDialogVisible = ref(false)
const currentNotification = ref<Notification | null>(null)

const fetchNotifications = async () => {
  try {
    const res: any = await listNotifications({
      page: pagination.page,
      page_size: pagination.page_size,
      status: statusFilter.value || undefined,
    } as any)
    if (res.data) {
      notificationList.value = res.data!.list || res.data! || []
      pagination.total = res.data!.total || res.data?.length || 0
    }
  } catch (e) {
    ElMessage.error('获取通知列表失败')
  }
}

const fetchUnreadCount = async () => {
  try {
    const res: any = await getUnreadCount()
    if (res.data !== undefined) {
      unreadCount.value = res.data!
    }
  } catch (e) {
    // 忽略错误
  }
}

const handleView = async (item: Notification) => {
  if (!item.id) return

  try {
    const res: any = await getNotification(item.id)
    if (res.data) {
      currentNotification.value = res.data!
      detailDialogVisible.value = true
      fetchNotifications()
      fetchUnreadCount()
    }
  } catch (e) {
    ElMessage.error('获取详情失败')
  }
}

const handleMarkRead = async (item: Notification) => {
  if (!item.id) return

  try {
    await markAsRead(item.id)
    ElMessage.success('已标为已读')
    fetchNotifications()
    fetchUnreadCount()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  }
}

const handleMarkAllRead = async () => {
  try {
    await markAllAsRead()
    ElMessage.success('操作成功')
    fetchNotifications()
    fetchUnreadCount()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  }
}

const handleDelete = async (item: Notification) => {
  if (!item.id) return

  try {
    await ElMessageBox.confirm('确认删除该通知？', '提示', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning',
    })

    await deleteNotification(item.id)
    ElMessage.success('删除成功')
    fetchNotifications()
    fetchUnreadCount()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchNotifications()
  loadIfNot('unreadCount', fetchUnreadCount, hasLoaded)
})
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
