<!--
  system/index.vue - 系统管理主页（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1521 行"上帝组件"已拆分为以下 12 个独立 Tab 子组件，
  位于 views/system/tabs/ 目录：

  | Tab       | 子组件                              | 状态     |
  | --------- | ----------------------------------- | -------- |
  | 用户管理  | tabs/UserTab.vue                    | ✅ 完整  |
  | 角色管理  | tabs/RoleTab.vue                    | ✅ 完整  |
  | 部门管理  | tabs/DepartmentTab.vue              | ✅ 已拆  |
  | 权限管理  | tabs/PermissionTab.vue              | ✅ 已拆  |
  | 数据权限  | tabs/DataPermissionTab.vue          | ✅ 已拆  |
  | 字段权限  | tabs/FieldPermissionTab.vue         | ✅ 已拆  |
  | 通知设置  | tabs/NotificationTab.vue            | ✅ 已拆  |
  | 审计日志  | tabs/AuditTab.vue                   | ✅ 已拆  |
  | Webhook   | tabs/WebhookTab.vue                 | ✅ 已拆  |
  | 系统更新  | tabs/SystemUpdateTab.vue            | ✅ 已拆  |
  | 公司信息  | tabs/CompanyTab.vue                 | ✅ 已拆  |

  本主入口仅承担：Tab 切换 + 公共样式。
  业务逻辑已全部迁入子组件，通过 props/emit 通信。
  拆分计划见：docs/refactoring/frontend-vue-splitting-plan.md
-->
<template>
  <div class="system-page">
    <el-tabs
      v-model="activeTab"
      tab-position="left"
      class="system-tabs"
      @tab-change="handleTabChange"
    >
      <el-tab-pane label="用户管理" name="user">
        <UserTab />
      </el-tab-pane>
      <el-tab-pane label="角色管理" name="role">
        <RoleTab />
      </el-tab-pane>
      <el-tab-pane label="部门管理" name="department">
        <DepartmentTab />
      </el-tab-pane>
      <el-tab-pane label="权限管理" name="permission">
        <PermissionTab />
      </el-tab-pane>
      <el-tab-pane label="数据权限" name="dataPermission">
        <DataPermissionTab />
      </el-tab-pane>
      <el-tab-pane label="字段权限" name="fieldPermission">
        <FieldPermissionTab />
      </el-tab-pane>
      <el-tab-pane label="通知设置" name="notification">
        <NotificationTab />
      </el-tab-pane>
      <el-tab-pane label="审计日志" name="audit">
        <AuditTab />
      </el-tab-pane>
      <el-tab-pane label="Webhook 配置" name="webhook">
        <WebhookTab />
      </el-tab-pane>
      <el-tab-pane label="系统更新" name="update">
        <SystemUpdateTab />
      </el-tab-pane>
      <el-tab-pane label="公司信息" name="company">
        <CompanyTab />
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import UserTab from './tabs/UserTab.vue'
import RoleTab from './tabs/RoleTab.vue'
import DepartmentTab from './tabs/DepartmentTab.vue'
import PermissionTab from './tabs/PermissionTab.vue'
import DataPermissionTab from './tabs/DataPermissionTab.vue'
import FieldPermissionTab from './tabs/FieldPermissionTab.vue'
import NotificationTab from './tabs/NotificationTab.vue'
import AuditTab from './tabs/AuditTab.vue'
import WebhookTab from './tabs/WebhookTab.vue'
import SystemUpdateTab from './tabs/SystemUpdateTab.vue'
import CompanyTab from './tabs/CompanyTab.vue'

// 当前激活的 Tab（懒加载由各子组件 onMounted 内部处理）
const activeTab = ref('user')

// Tab 切换时仅记录当前 Tab 名；
// 真实数据加载由各子组件内部的 onMounted 处理（拆分原则：状态本地化）。
const handleTabChange = (tabName: string | number) => {
  activeTab.value = String(tabName)
}
</script>

<style scoped>
.system-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.system-tabs {
  min-height: calc(100vh - 160px);
}
.system-tabs :deep(.el-tabs__header) {
  min-width: 140px;
}
.system-tabs :deep(.el-tabs__content) {
  padding: 0 0 0 20px;
}
</style>
