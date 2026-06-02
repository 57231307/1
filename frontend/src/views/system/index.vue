<template>
  <div class="system-page">
    <el-tabs v-model="activeTab" tab-position="left" class="system-tabs" @tab-change="handleTabChange">
      <!-- 1. 用户管理 -->
      <el-tab-pane label="用户管理" name="user">
        <div class="page-header">
          <h2 class="page-title">用户管理</h2>
          <el-button type="primary" @click="openUserDialog()">
            <el-icon><Plus /></el-icon> 新建用户
          </el-button>
        </div>
        <el-card shadow="hover" class="filter-card">
          <el-form :inline="true" :model="userQuery">
            <el-form-item label="关键词">
              <el-input
                v-model="userQuery.keyword"
                placeholder="用户名/姓名/手机号"
                clearable
                @keyup.enter="fetchUsers"
              />
            </el-form-item>
            <el-form-item label="状态">
              <el-select v-model="userQuery.status" placeholder="选择状态" clearable>
                <el-option label="启用" :value="1" />
                <el-option label="禁用" :value="0" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchUsers">查询</el-button>
              <el-button @click="resetUserQuery">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>
        <el-card shadow="hover">
          <el-table v-loading="userLoading" :data="users" stripe>
            <el-table-column prop="username" label="用户名" width="120" />
            <el-table-column prop="real_name" label="姓名" width="100" />
            <el-table-column prop="phone" label="手机号" width="130" />
            <el-table-column prop="email" label="邮箱" min-width="180" />
            <el-table-column prop="department_name" label="部门" width="120" />
            <el-table-column label="角色" width="150">
              <template #default="{ row }">
                <template v-if="row.role_names?.length">
                  <el-tag v-for="r in row.role_names" :key="r" size="small" class="mr-1">{{
                    r
                  }}</el-tag>
                </template>
                <span v-else class="text-gray">未分配</span>
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">{{
                  row.status === 1 ? '启用' : '禁用'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openUserDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteUser(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-pagination
            v-model:current-page="userQuery.page"
            v-model:page-size="userQuery.page_size"
            :total="userTotal"
            :page-sizes="[10, 20, 50]"
            layout="total, sizes, prev, pager, next"
            class="mt-4"
            @change="fetchUsers"
          />
        </el-card>
      </el-tab-pane>

      <!-- 2. 角色管理 -->
      <el-tab-pane label="角色管理" name="role">
        <div class="page-header">
          <h2 class="page-title">角色管理</h2>
          <el-button type="primary" @click="openRoleDialog()"
            ><el-icon><Plus /></el-icon> 新建角色</el-button
          >
        </div>
        <el-card shadow="hover">
          <el-table v-loading="roleLoading" :data="roles" stripe>
            <el-table-column prop="name" label="角色名称" width="150" />
            <el-table-column prop="code" label="角色编码" width="150" />
            <el-table-column prop="description" label="描述" min-width="200" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">{{
                  row.status === 1 ? '启用' : '禁用'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="created_at" label="创建时间" width="160" />
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openRoleDialog(row)"
                  >编辑</el-button
                >
                <el-button type="primary" link size="small" @click="openPermissionDialog(row)"
                  >权限</el-button
                >
                <el-button type="danger" link size="small" @click="deleteRole(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 3. 部门管理 -->
      <el-tab-pane label="部门管理" name="department">
        <div class="page-header">
          <h2 class="page-title">部门管理</h2>
          <el-button type="primary" @click="openDeptDialog()"
            ><el-icon><Plus /></el-icon> 新建部门</el-button
          >
        </div>
        <el-card shadow="hover">
          <el-table
            v-loading="deptLoading"
            :data="departments"
            stripe
            row-key="id"
            default-expand-all
          >
            <el-table-column prop="name" label="部门名称" min-width="200" />
            <el-table-column prop="code" label="部门编码" width="120" />
            <el-table-column prop="manager_name" label="负责人" width="100" />
            <el-table-column prop="sort_order" label="排序" width="80" align="center" />
            <el-table-column prop="status" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.status === 1 ? 'success' : 'info'" size="small">{{
                  row.status === 1 ? '启用' : '禁用'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openDeptDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteDept(row)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 4. 权限管理 -->
      <el-tab-pane label="权限管理" name="permission">
        <div class="page-header"><h2 class="page-title">权限管理</h2></div>
        <el-card shadow="hover">
          <el-table v-loading="permissionListLoading" :data="permissionList" stripe>
            <el-table-column prop="resource_type" label="资源类型" width="150" />
            <el-table-column prop="action" label="操作" width="120" />
            <el-table-column prop="allowed" label="状态" width="100" align="center">
              <template #default="{ row }">
                <el-tag :type="row.allowed ? 'success' : 'danger'" size="small">{{
                  row.allowed ? '允许' : '禁止'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="resource_id" label="资源ID" width="100" />
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 5. 数据权限 -->
      <el-tab-pane label="数据权限" name="dataPermission">
        <div class="page-header"><h2 class="page-title">数据权限</h2></div>
        <el-card shadow="hover">
          <el-table v-loading="dataPermLoading" :data="dataPermissionList" stripe>
            <el-table-column prop="role_name" label="角色" width="120" />
            <el-table-column prop="scope_type" label="权限范围" width="120" />
            <el-table-column prop="scope_value" label="范围值" min-width="200" />
            <el-table-column prop="created_at" label="创建时间" width="160" />
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 6. 字段权限 -->
      <el-tab-pane label="字段权限" name="fieldPermission">
        <div class="page-header"><h2 class="page-title">字段权限</h2></div>
        <el-card shadow="hover">
          <el-table v-loading="fieldPermLoading" :data="fieldPermissionList" stripe>
            <el-table-column prop="role_name" label="角色" width="120" />
            <el-table-column prop="resource_type" label="资源" width="120" />
            <el-table-column prop="field_name" label="字段名" width="150" />
            <el-table-column prop="visible" label="可见" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.visible ? 'success' : 'danger'" size="small">{{
                  row.visible ? '是' : '否'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="editable" label="可编辑" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.editable ? 'success' : 'info'" size="small">{{
                  row.editable ? '是' : '否'
                }}</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 7. 通知设置 -->
      <el-tab-pane label="通知设置" name="notification">
        <div class="page-header"><h2 class="page-title">通知设置</h2></div>
        <el-card shadow="hover" style="max-width: 600px">
          <el-form :model="notificationForm" label-width="140px">
            <el-form-item label="邮件通知">
              <el-switch v-model="notificationForm.email_enabled" />
            </el-form-item>
            <el-form-item label="站内通知">
              <el-switch v-model="notificationForm.internal_enabled" />
            </el-form-item>
            <el-divider content-position="left">通知类型</el-divider>
            <el-form-item label="订单通知">
              <el-select v-model="notificationForm.order_notification_type" style="width: 100%">
                <el-option label="仅邮件" value="email" />
                <el-option label="仅站内" value="internal" />
                <el-option label="全部" value="both" />
              </el-select>
            </el-form-item>
            <el-form-item label="审批通知">
              <el-select v-model="notificationForm.approval_notification_type" style="width: 100%">
                <el-option label="仅邮件" value="email" />
                <el-option label="仅站内" value="internal" />
                <el-option label="全部" value="both" />
              </el-select>
            </el-form-item>
            <el-form-item label="库存通知">
              <el-select v-model="notificationForm.inventory_notification_type" style="width: 100%">
                <el-option label="仅邮件" value="email" />
                <el-option label="仅站内" value="internal" />
                <el-option label="全部" value="both" />
              </el-select>
            </el-form-item>
            <el-form-item label="系统通知">
              <el-select v-model="notificationForm.system_notification_type" style="width: 100%">
                <el-option label="仅邮件" value="email" />
                <el-option label="仅站内" value="internal" />
                <el-option label="全部" value="both" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="notifSaving" @click="saveNotificationSetting"
                >保存设置</el-button
              >
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>

      <!-- 8. 审计日志 -->
      <el-tab-pane label="审计日志" name="audit">
        <div class="page-header"><h2 class="page-title">审计日志</h2></div>
        <el-card shadow="hover">
          <el-form :inline="true" :model="auditQuery" class="mb-4">
            <el-form-item label="操作人">
              <el-input v-model="auditQuery.operator" placeholder="用户名" clearable />
            </el-form-item>
            <el-form-item label="模块">
              <el-input v-model="auditQuery.module" placeholder="模块名" clearable />
            </el-form-item>
            <el-form-item label="时间范围">
              <el-date-picker
                v-model="auditQuery.dateRange"
                type="daterange"
                range-separator="至"
                start-placeholder="开始日期"
                end-placeholder="结束日期"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchAuditLogs">查询</el-button>
            </el-form-item>
          </el-form>
          <el-table v-loading="auditLoading" :data="auditLogs" stripe>
            <el-table-column prop="created_at" label="时间" width="180" />
            <el-table-column prop="operator_name" label="操作人" width="120" />
            <el-table-column prop="module" label="模块" width="120" />
            <el-table-column prop="action" label="操作" width="100" />
            <el-table-column prop="resource_type" label="资源" width="120" />
            <el-table-column prop="ip_address" label="IP" width="130" />
            <el-table-column prop="detail" label="详情" min-width="200" show-overflow-tooltip />
          </el-table>
          <el-pagination
            v-model:current-page="auditQuery.page"
            v-model:page-size="auditQuery.page_size"
            :total="auditTotal"
            :page-sizes="[20, 50, 100]"
            layout="total, sizes, prev, pager, next"
            class="mt-4"
            @change="fetchAuditLogs"
          />
        </el-card>
      </el-tab-pane>

      <!-- 9. Webhook 配置 -->
      <el-tab-pane label="Webhook 配置" name="webhook">
        <div class="page-header">
          <h2 class="page-title">Webhook 配置</h2>
          <el-button type="primary" @click="openWebhookDialog()"
            ><el-icon><Plus /></el-icon> 新建</el-button
          >
        </div>
        <el-card shadow="hover">
          <el-table v-loading="webhookLoading" :data="webhookList" stripe>
            <el-table-column prop="name" label="名称" width="150" />
            <el-table-column prop="url" label="URL" min-width="250" show-overflow-tooltip />
            <el-table-column prop="event_type" label="事件" width="120" />
            <el-table-column prop="is_active" label="状态" width="80" align="center">
              <template #default="{ row }">
                <el-tag :type="row.is_active ? 'success' : 'info'" size="small">{{
                  row.is_active ? '启用' : '禁用'
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="200" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openWebhookDialog(row)"
                  >编辑</el-button
                >
                <el-button type="warning" link size="small" @click="testWebhook(row)"
                  >测试</el-button
                >
                <el-button type="danger" link size="small" @click="deleteWebhook(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 10. 系统更新 -->
      <el-tab-pane label="系统更新" name="update">
        <div class="page-header"><h2 class="page-title">系统更新</h2></div>
        <el-card shadow="hover">
          <el-descriptions :column="2" border>
            <el-descriptions-item label="当前版本">{{ systemVersion }}</el-descriptions-item>
            <el-descriptions-item label="最后更新">{{ lastUpdate }}</el-descriptions-item>
          </el-descriptions>
          <div style="margin-top: 20px">
            <el-button type="primary" :loading="checkUpdateLoading" @click="checkUpdate"
              >检查更新</el-button
            >
            <el-button
              v-if="hasUpdate"
              type="success"
              :loading="applyUpdateLoading"
              @click="applyUpdate"
              >应用更新</el-button
            >
          </div>
          <el-alert
            v-if="updateInfo"
            :title="updateInfo"
            type="info"
            show-icon
            style="margin-top: 16px"
          />
        </el-card>
      </el-tab-pane>

      <!-- 11. 租户配置 -->
      <el-tab-pane label="租户配置" name="tenant">
        <div class="page-header"><h2 class="page-title">租户配置</h2></div>
        <el-card shadow="hover">
          <el-form :inline="true" :model="tenantConfigQuery" class="mb-4">
            <el-form-item label="配置键">
              <el-input v-model="tenantConfigQuery.key" placeholder="配置键名" clearable />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" @click="fetchTenantConfigs">查询</el-button>
              <el-button type="success" @click="openTenantConfigDialog">新增配置</el-button>
            </el-form-item>
          </el-form>
          <el-table v-loading="tenantConfigLoading" :data="tenantConfigs" stripe>
            <el-table-column prop="config_key" label="配置键" width="200" />
            <el-table-column
              prop="config_value"
              label="配置值"
              min-width="250"
              show-overflow-tooltip
            />
            <el-table-column prop="config_type" label="类型" width="100" />
            <el-table-column prop="description" label="描述" width="200" />
            <el-table-column label="操作" width="150" fixed="right">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="openTenantConfigDialog(row)"
                  >编辑</el-button
                >
                <el-button type="danger" link size="small" @click="deleteTenantConfig(row)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <!-- 12. 公司信息 -->
      <el-tab-pane label="公司信息" name="company">
        <div class="page-header"><h2 class="page-title">公司信息设置</h2></div>
        <el-card shadow="hover">
          <el-form
            ref="companyFormRef"
            :model="companyForm"
            :rules="companyRules"
            label-width="120px"
            style="max-width: 800px"
          >
            <el-divider content-position="left">基本信息</el-divider>
            <el-row :gutter="20">
              <el-col :span="12"
                ><el-form-item label="公司名称" prop="company_name"
                  ><el-input v-model="companyForm.company_name" /></el-form-item
              ></el-col>
              <el-col :span="12"
                ><el-form-item label="公司简称"
                  ><el-input v-model="companyForm.company_short_name" /></el-form-item
              ></el-col>
            </el-row>
            <el-row :gutter="20">
              <el-col :span="12"
                ><el-form-item label="信用代码"
                  ><el-input v-model="companyForm.credit_code" /></el-form-item
              ></el-col>
              <el-col :span="12"
                ><el-form-item label="法定代表人"
                  ><el-input v-model="companyForm.legal_representative" /></el-form-item
              ></el-col>
            </el-row>
            <el-divider content-position="left">联系方式</el-divider>
            <el-row :gutter="20">
              <el-col :span="12"
                ><el-form-item label="联系电话"
                  ><el-input v-model="companyForm.phone" /></el-form-item
              ></el-col>
              <el-col :span="12"
                ><el-form-item label="邮箱"><el-input v-model="companyForm.email" /></el-form-item
              ></el-col>
            </el-row>
            <el-form-item label="地址"><el-input v-model="companyForm.address" /></el-form-item>
            <el-divider content-position="left">银行信息</el-divider>
            <el-row :gutter="20">
              <el-col :span="12"
                ><el-form-item label="开户银行"
                  ><el-input v-model="companyForm.bank_name" /></el-form-item
              ></el-col>
              <el-col :span="12"
                ><el-form-item label="银行账号"
                  ><el-input v-model="companyForm.bank_account" /></el-form-item
              ></el-col>
            </el-row>
            <el-form-item>
              <el-button type="primary" :loading="companySubmitLoading" @click="saveCompanyInfo"
                >保存</el-button
              >
              <el-button @click="resetCompanyForm">重置</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-tab-pane>
    </el-tabs>

    <!-- 用户对话框 -->
    <el-dialog
      v-model="userDialogVisible"
      :title="userForm.id ? '编辑用户' : '新建用户'"
      width="500px"
    >
      <el-form ref="userFormRef" :model="userForm" :rules="userRules" label-width="80px">
        <el-form-item label="用户名" prop="username"
          ><el-input v-model="userForm.username" :disabled="!!userForm.id"
        /></el-form-item>
        <el-form-item v-if="!userForm.id" label="密码" prop="password"
          ><el-input v-model="userForm.password" type="password" show-password
        /></el-form-item>
        <el-form-item label="姓名" prop="real_name"
          ><el-input v-model="userForm.real_name"
        /></el-form-item>
        <el-form-item label="手机号"><el-input v-model="userForm.phone" /></el-form-item>
        <el-form-item label="邮箱"><el-input v-model="userForm.email" /></el-form-item>
        <el-form-item label="部门">
          <el-tree-select
            v-model="userForm.department_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            placeholder="选择部门"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="状态"
          ><el-switch v-model="userForm.status" :active-value="1" :inactive-value="0"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="userDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="userSubmitLoading" @click="submitUser">确定</el-button>
      </template>
    </el-dialog>

    <!-- 角色对话框 -->
    <el-dialog
      v-model="roleDialogVisible"
      :title="roleForm.id ? '编辑角色' : '新建角色'"
      width="500px"
    >
      <el-form ref="roleFormRef" :model="roleForm" :rules="roleRules" label-width="80px">
        <el-form-item label="角色名称" prop="name"
          ><el-input v-model="roleForm.name"
        /></el-form-item>
        <el-form-item label="角色编码" prop="code"
          ><el-input v-model="roleForm.code" :disabled="!!roleForm.id"
        /></el-form-item>
        <el-form-item label="描述"
          ><el-input v-model="roleForm.description" type="textarea"
        /></el-form-item>
        <el-form-item label="状态"
          ><el-switch v-model="roleForm.status" :active-value="1" :inactive-value="0"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="roleDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="roleSubmitLoading" @click="submitRole">确定</el-button>
      </template>
    </el-dialog>

    <!-- 部门对话框 -->
    <el-dialog
      v-model="deptDialogVisible"
      :title="deptForm.id ? '编辑部门' : '新建部门'"
      width="500px"
    >
      <el-form ref="deptFormRef" :model="deptForm" :rules="deptRules" label-width="80px">
        <el-form-item label="部门名称" prop="name"
          ><el-input v-model="deptForm.name"
        /></el-form-item>
        <el-form-item label="部门编码" prop="code"
          ><el-input v-model="deptForm.code"
        /></el-form-item>
        <el-form-item label="上级部门">
          <el-tree-select
            v-model="deptForm.parent_id"
            :data="deptTreeData"
            :props="{ label: 'name', value: 'id' }"
            clearable
            check-strictly
          />
        </el-form-item>
        <el-form-item label="排序"
          ><el-input-number v-model="deptForm.sort_order" :min="0"
        /></el-form-item>
        <el-form-item label="状态"
          ><el-switch v-model="deptForm.status" :active-value="1" :inactive-value="0"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="deptDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="deptSubmitLoading" @click="submitDept">确定</el-button>
      </template>
    </el-dialog>

    <!-- 权限配置对话框 -->
    <el-dialog
      v-model="permissionDialogVisible"
      :title="`权限配置 - ${currentRoleName}`"
      width="600px"
    >
      <el-tree
        v-loading="permissionLoading"
        :data="permissionTree"
        show-checkbox
        node-key="id"
        :default-checked-keys="checkedPermissions"
        :props="{ label: 'name', children: 'children' }"
        @check="handlePermissionCheck"
      />
      <template #footer>
        <el-button @click="permissionDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="submitPermissions">确定</el-button>
      </template>
    </el-dialog>

    <!-- Webhook 对话框 -->
    <el-dialog
      v-model="webhookDialogVisible"
      :title="webhookForm.id ? '编辑 Webhook' : '新建 Webhook'"
      width="500px"
    >
      <el-form ref="webhookFormRef" :model="webhookForm" label-width="100px">
        <el-form-item label="名称" prop="name"
          ><el-input v-model="webhookForm.name"
        /></el-form-item>
        <el-form-item label="URL" prop="url"
          ><el-input v-model="webhookForm.url" placeholder="https://"
        /></el-form-item>
        <el-form-item label="事件类型">
          <el-select v-model="webhookForm.event_type" style="width: 100%">
            <el-option label="订单创建" value="order.created" />
            <el-option label="订单更新" value="order.updated" />
            <el-option label="库存变动" value="inventory.changed" />
            <el-option label="审批完成" value="approval.completed" />
            <el-option label="全部" value="all" />
          </el-select>
        </el-form-item>
        <el-form-item label="密钥"
          ><el-input v-model="webhookForm.secret" placeholder="可选"
        /></el-form-item>
        <el-form-item label="状态"><el-switch v-model="webhookForm.is_active" /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="webhookDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveWebhook">确定</el-button>
      </template>
    </el-dialog>

    <!-- 租户配置对话框 -->
    <el-dialog
      v-model="tenantConfigDialogVisible"
      :title="tenantConfigForm.id ? '编辑配置' : '新增配置'"
      width="500px"
    >
      <el-form ref="tenantConfigFormRef" :model="tenantConfigForm" label-width="100px">
        <el-form-item label="配置键" prop="config_key"
          ><el-input v-model="tenantConfigForm.config_key" :disabled="!!tenantConfigForm.id"
        /></el-form-item>
        <el-form-item label="配置值" prop="config_value"
          ><el-input v-model="tenantConfigForm.config_value" type="textarea" :rows="3"
        /></el-form-item>
        <el-form-item label="类型">
          <el-select v-model="tenantConfigForm.config_type" style="width: 100%">
            <el-option label="字符串" value="STRING" />
            <el-option label="数字" value="NUMBER" />
            <el-option label="布尔" value="BOOLEAN" />
            <el-option label="JSON" value="JSON" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述"
          ><el-input v-model="tenantConfigForm.description"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tenantConfigDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveTenantConfig">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance, FormRules } from 'element-plus'
import { request } from '@/api/request'
import {
  listUsers,
  createUser,
  updateUser,
  deleteUser as deleteUserApi,
  type User,
} from '@/api/user'
import {
  listRoles,
  createRole,
  updateRole,
  deleteRole as deleteRoleApi,
  getRolePermissions,
  assignPermission,
  listPermissions,
  type Role,
  type Permission,
} from '@/api/role'
import {
  createDepartment,
  updateDepartment,
  deleteDepartment as deleteDeptApi,
  getDepartmentTree,
  type Department,
} from '@/api/department'

const activeTab = ref('user')
const hasLoaded = reactive<Record<string, boolean>>({})

const loadTabData = (tabName: string) => {
  if (hasLoaded[tabName]) return
  hasLoaded[tabName] = true
  
  const loaders: Record<string, () => void> = {
    'user': fetchUsers,
    'role': fetchRoles,
    'department': fetchDepartments,
    'permission': fetchPermissionList,
    'dataPermission': fetchDataPermissions,
    'fieldPermission': fetchFieldPermissions,
    'notification': fetchNotificationSetting,
    'audit': fetchAuditLogs,
    'webhook': fetchWebhooks,
    'update': fetchSystemVersion,
    'tenant': fetchTenantConfigs,
    'company': fetchCompanyInfo,
  }
  
  if (loaders[tabName]) {
    loaders[tabName]()
  }
}

const handleTabChange = (tabName: string) => {
  activeTab.value = tabName
  loadTabData(tabName)
}

// ============ 用户管理 ============
const users = ref<User[]>([])
const userTotal = ref(0)
const userLoading = ref(false)
const userQuery = reactive({
  keyword: '',
  status: undefined as number | undefined,
  page: 1,
  page_size: 10,
})
const fetchUsers = async () => {
  userLoading.value = true
  try {
    const res = await listUsers(userQuery)
    users.value = res.data?.list || []
    userTotal.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取用户列表失败')
  } finally {
    userLoading.value = false
  }
}
const resetUserQuery = () => {
  userQuery.keyword = ''
  userQuery.status = undefined
  userQuery.page = 1
  fetchUsers()
}
const userDialogVisible = ref(false)
const userFormRef = ref<FormInstance>()
const userSubmitLoading = ref(false)
const userForm = reactive({
  id: 0,
  username: '',
  password: '',
  real_name: '',
  phone: '',
  email: '',
  department_id: undefined as number | undefined,
  status: 1,
})
const validateEmail = (_: any, v: string, cb: any) => {
  v && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(v) ? cb(new Error('邮箱格式错误')) : cb()
}
const validatePhone = (_: any, v: string, cb: any) => {
  v && !/^1[3-9]\d{9}$/.test(v) ? cb(new Error('手机号格式错误')) : cb()
}
const validatePassword = (_: any, v: string, cb: any) => {
  // 编辑模式下密码为可选
  if (userForm.id && !v) {
    cb()
    return
  }
  v && v.length < 8
    ? cb(new Error('密码至少8位'))
    : v && !/^(?=.*[a-z])(?=.*[A-Z])(?=.*\d).+$/.test(v)
      ? cb(new Error('需含大小写字母和数字'))
      : cb()
}
const userRules: FormRules = {
  username: [
    { required: true, message: '请输入用户名', trigger: 'blur' },
    { min: 3, max: 20, message: '3-20位', trigger: 'blur' },
  ],
  password: [{ required: true, validator: validatePassword, trigger: 'blur' }],
  real_name: [{ required: true, message: '请输入姓名', trigger: 'blur' }],
  email: [{ validator: validateEmail, trigger: 'blur' }],
  phone: [{ validator: validatePhone, trigger: 'blur' }],
}
const openUserDialog = (row?: User) => {
  userFormRef.value?.resetFields()
  if (row) {
    Object.assign(userForm, {
      id: row.id,
      username: row.username,
      real_name: row.real_name,
      phone: row.phone || '',
      email: row.email || '',
      department_id: row.department_id,
      status: row.status,
    })
  } else {
    Object.assign(userForm, {
      id: 0,
      username: '',
      password: '',
      real_name: '',
      phone: '',
      email: '',
      department_id: undefined,
      status: 1,
    })
  }
  userDialogVisible.value = true
}
const submitUser = async () => {
  const valid = await userFormRef.value?.validate()
  if (!valid) return
  userSubmitLoading.value = true
  try {
    if (userForm.id) {
      await updateUser(userForm.id, {
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
        status: userForm.status,
      })
      ElMessage.success('更新成功')
    } else {
      await createUser({
        username: userForm.username,
        password: userForm.password,
        real_name: userForm.real_name,
        phone: userForm.phone,
        email: userForm.email,
        department_id: userForm.department_id,
      })
      ElMessage.success('创建成功')
    }
    userDialogVisible.value = false
    fetchUsers()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    userSubmitLoading.value = false
  }
}
const deleteUser = async (row: User) => {
  try {
    await ElMessageBox.confirm(`确定删除用户 "${row.username}"?`, '删除确认', { type: 'warning' })
    await deleteUserApi(row.id)
    ElMessage.success('删除成功')
    fetchUsers()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// ============ 角色管理 ============
const roles = ref<Role[]>([])
const roleLoading = ref(false)
const fetchRoles = async () => {
  roleLoading.value = true
  try {
    const res = await listRoles()
    const d = res.data as any
    roles.value = d?.items || d?.data || d || []
  } catch (e: any) {
    ElMessage.error(e.message || '获取角色列表失败')
  } finally {
    roleLoading.value = false
  }
}
const roleDialogVisible = ref(false)
const roleFormRef = ref<FormInstance>()
const roleSubmitLoading = ref(false)
const roleForm = reactive({ id: 0, name: '', code: '', description: '', status: 1 })
const roleRules: FormRules = {
  name: [{ required: true, message: '请输入角色名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入角色编码', trigger: 'blur' }],
}
const openRoleDialog = (row?: Role) => {
  roleFormRef.value?.resetFields()
  if (row) {
    Object.assign(roleForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      description: row.description || '',
      status: row.status,
    })
  } else {
    Object.assign(roleForm, { id: 0, name: '', code: '', description: '', status: 1 })
  }
  roleDialogVisible.value = true
}
const submitRole = async () => {
  const valid = await roleFormRef.value?.validate()
  if (!valid) return
  roleSubmitLoading.value = true
  try {
    if (roleForm.id) {
      await updateRole(roleForm.id, {
        name: roleForm.name,
        description: roleForm.description,
        status: roleForm.status,
      })
      ElMessage.success('更新成功')
    } else {
      await createRole({
        name: roleForm.name,
        code: roleForm.code,
        description: roleForm.description,
      })
      ElMessage.success('创建成功')
    }
    roleDialogVisible.value = false
    fetchRoles()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    roleSubmitLoading.value = false
  }
}
const deleteRole = async (row: Role) => {
  try {
    await ElMessageBox.confirm(`确定删除角色 "${row.name}"?`, '删除确认', { type: 'warning' })
    await deleteRoleApi(row.id)
    ElMessage.success('删除成功')
    fetchRoles()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// ============ 权限配置 ============
const permissionDialogVisible = ref(false)
const currentRoleId = ref(0)
const currentRoleName = ref('')
const permissionTree = ref<any[]>([])
const checkedPermissions = ref<number[]>([])
const permissionLoading = ref(false)
const openPermissionDialog = (row: Role) => {
  currentRoleId.value = row.id
  currentRoleName.value = row.name
  fetchRolePermissions(row.id)
  permissionDialogVisible.value = true
}
const fetchRolePermissions = async (roleId: number) => {
  permissionLoading.value = true
  try {
    const treeRes = await listPermissions()
    permissionTree.value = buildPermissionTree(treeRes.data || [])
    const roleRes = await getRolePermissions(roleId)
    checkedPermissions.value = (roleRes.data || []).map((p: Permission) => p.id)
  } catch (e) {
    console.error('获取权限失败:', e)
  } finally {
    permissionLoading.value = false
  }
}
const buildPermissionTree = (perms: Permission[]): any[] => {
  const map = new Map<number, any>()
  const tree: any[] = []
  perms.forEach((p) => map.set(p.id, { ...p, children: [] }))
  perms.forEach((p) => {
    const node = map.get(p.id)!
    p.parent_id && map.has(p.parent_id)
      ? map.get(p.parent_id)!.children.push(node)
      : tree.push(node)
  })
  return tree
}
const submitPermissions = async () => {
  try {
    await assignPermission(currentRoleId.value, { permission_ids: checkedPermissions.value })
    ElMessage.success('权限配置成功')
    permissionDialogVisible.value = false
  } catch (e: any) {
    ElMessage.error(e.message || '配置失败')
  }
}
const handlePermissionCheck = (_: any, { checkedKeys }: any) => {
  checkedPermissions.value = checkedKeys
}

// ============ 部门管理 ============
const departments = ref<Department[]>([])
const deptLoading = ref(false)
const fetchDepartments = async () => {
  deptLoading.value = true
  try {
    const res = await getDepartmentTree()
    const d = res.data as any
    departments.value = d?.items || d?.data || d || []
  } catch (e: any) {
    ElMessage.error(e.message || '获取部门列表失败')
  } finally {
    deptLoading.value = false
  }
}
const deptTreeData = computed(() => departments.value)
const deptDialogVisible = ref(false)
const deptFormRef = ref<FormInstance>()
const deptSubmitLoading = ref(false)
const deptForm = reactive({
  id: 0,
  name: '',
  code: '',
  parent_id: undefined as number | undefined,
  sort_order: 0,
  status: 1,
})
const deptRules: FormRules = {
  name: [{ required: true, message: '请输入部门名称', trigger: 'blur' }],
  code: [{ required: true, message: '请输入部门编码', trigger: 'blur' }],
}
const openDeptDialog = (row?: Department) => {
  deptFormRef.value?.resetFields()
  if (row) {
    Object.assign(deptForm, {
      id: row.id,
      name: row.name,
      code: row.code,
      parent_id: row.parent_id,
      sort_order: row.sort_order,
      status: row.status,
    })
  } else {
    Object.assign(deptForm, {
      id: 0,
      name: '',
      code: '',
      parent_id: undefined,
      sort_order: 0,
      status: 1,
    })
  }
  deptDialogVisible.value = true
}
const submitDept = async () => {
  const valid = await deptFormRef.value?.validate()
  if (!valid) return
  deptSubmitLoading.value = true
  try {
    if (deptForm.id) {
      await updateDepartment(deptForm.id, {
        name: deptForm.name,
        sort_order: deptForm.sort_order,
        status: deptForm.status,
      })
      ElMessage.success('更新成功')
    } else {
      await createDepartment({
        name: deptForm.name,
        code: deptForm.code,
        parent_id: deptForm.parent_id,
        sort_order: deptForm.sort_order,
      })
      ElMessage.success('创建成功')
    }
    deptDialogVisible.value = false
    fetchDepartments()
  } catch (e: any) {
    ElMessage.error(e.message || '操作失败')
  } finally {
    deptSubmitLoading.value = false
  }
}
const deleteDept = async (row: Department) => {
  try {
    await ElMessageBox.confirm(`确定删除部门 "${row.name}"?`, '删除确认', { type: 'warning' })
    await deleteDeptApi(row.id)
    ElMessage.success('删除成功')
    fetchDepartments()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// ============ 权限管理 ============
const permissionList = ref<any[]>([])
const permissionListLoading = ref(false)
const fetchPermissionList = async () => {
  permissionListLoading.value = true
  try {
    const res: any = await request.get('/permissions')
    permissionList.value = res.data || []
  } catch (e) {
    /* ignore */
  } finally {
    permissionListLoading.value = false
  }
}

// ============ 数据权限 ============
const dataPermissionList = ref<any[]>([])
const dataPermLoading = ref(false)
const fetchDataPermissions = async () => {
  dataPermLoading.value = true
  try {
    const res: any = await request.get('/data-permissions')
    dataPermissionList.value = res.data?.items || res.data || []
  } catch (e) {
    /* ignore */
  } finally {
    dataPermLoading.value = false
  }
}

// ============ 字段权限 ============
const fieldPermissionList = ref<any[]>([])
const fieldPermLoading = ref(false)
const fetchFieldPermissions = async () => {
  fieldPermLoading.value = true
  try {
    const res: any = await request.get('/permissions/fields')
    fieldPermissionList.value = res.data?.items || res.data || []
  } catch (e) {
    /* ignore */
  } finally {
    fieldPermLoading.value = false
  }
}

// ============ 通知设置 ============
const notificationForm = reactive({
  email_enabled: true,
  internal_enabled: true,
  order_notification_type: 'both',
  approval_notification_type: 'both',
  inventory_notification_type: 'both',
  system_notification_type: 'internal',
  purchase_notification_type: 'both',
  finance_notification_type: 'both',
})
const notifSaving = ref(false)
const fetchNotificationSetting = async () => {
  try {
    const res: any = await request.get('/user/notification-setting')
    if (res.data) Object.assign(notificationForm, res.data)
  } catch (e) {
    /* ignore */
  }
}
const saveNotificationSetting = async () => {
  notifSaving.value = true
  try {
    await request.put('/user/notification-setting', notificationForm)
    ElMessage.success('保存成功')
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  } finally {
    notifSaving.value = false
  }
}

// ============ 审计日志 ============
const auditLogs = ref<any[]>([])
const auditTotal = ref(0)
const auditLoading = ref(false)
const auditQuery = reactive({
  operator: '',
  module: '',
  dateRange: null as any,
  page: 1,
  page_size: 20,
})
const fetchAuditLogs = async () => {
  auditLoading.value = true
  try {
    const params: any = { page: auditQuery.page, page_size: auditQuery.page_size }
    if (auditQuery.operator) params.operator = auditQuery.operator
    if (auditQuery.module) params.module = auditQuery.module
    if (auditQuery.dateRange?.length === 2) {
      params.start_date = auditQuery.dateRange[0]
      params.end_date = auditQuery.dateRange[1]
    }
    const res: any = await request.get('/audit/logs', { params })
    auditLogs.value = res.data?.items || res.data || []
    auditTotal.value = res.data?.total || 0
  } catch (e) {
    /* ignore */
  } finally {
    auditLoading.value = false
  }
}

// ============ Webhook 配置 ============
const webhookList = ref<any[]>([])
const webhookLoading = ref(false)
const webhookDialogVisible = ref(false)
const webhookFormRef = ref<FormInstance>()
const webhookForm = reactive({
  id: 0,
  name: '',
  url: '',
  event_type: 'all',
  secret: '',
  is_active: true,
})
const fetchWebhooks = async () => {
  webhookLoading.value = true
  try {
    const res: any = await request.get('/webhooks/integrations')
    webhookList.value = res.data?.items || res.data || []
  } catch (e) {
    /* ignore */
  } finally {
    webhookLoading.value = false
  }
}
const openWebhookDialog = (row?: any) => {
  if (row) {
    Object.assign(webhookForm, row)
  } else {
    Object.assign(webhookForm, {
      id: 0,
      name: '',
      url: '',
      event_type: 'all',
      secret: '',
      is_active: true,
    })
  }
  webhookDialogVisible.value = true
}
const saveWebhook = async () => {
  try {
    if (webhookForm.id) {
      await request.put(`/webhooks/integrations/${webhookForm.id}`, webhookForm)
    } else {
      await request.post('/webhooks/integrations', webhookForm)
    }
    ElMessage.success('保存成功')
    webhookDialogVisible.value = false
    fetchWebhooks()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  }
}
const deleteWebhook = async (row: any) => {
  try {
    await ElMessageBox.confirm('确定删除?', '确认', { type: 'warning' })
    await request.delete(`/webhooks/integrations/${row.id}`)
    ElMessage.success('删除成功')
    fetchWebhooks()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}
const testWebhook = async (row: any) => {
  try {
    await request.post(`/webhooks/integrations/${row.id}`)
    ElMessage.success('测试请求已发送')
  } catch (e: any) {
    ElMessage.error(e.message || '测试失败')
  }
}

// ============ 系统更新 ============
const systemVersion = ref('v2026.x.x')
const lastUpdate = ref('-')
const hasUpdate = ref(false)
const updateInfo = ref('')
const checkUpdateLoading = ref(false)
const applyUpdateLoading = ref(false)
const checkUpdate = async () => {
  checkUpdateLoading.value = true
  try {
    const res: any = await request.get('/system-update/check')
    updateInfo.value = res.data?.message || '已是最新版本'
    hasUpdate.value = res.data?.has_update || false
  } catch (e: any) {
    ElMessage.error(e.message || '检查更新失败')
  } finally {
    checkUpdateLoading.value = false
  }
}
const applyUpdate = async () => {
  applyUpdateLoading.value = true
  try {
    await request.post('/system-update/update')
    ElMessage.success('更新已提交，服务将重启')
  } catch (e: any) {
    ElMessage.error(e.message || '更新失败')
  } finally {
    applyUpdateLoading.value = false
  }
}
const fetchSystemVersion = async () => {
  try {
    const res: any = await request.get('/system-update/version')
    systemVersion.value = res.data?.version || 'unknown'
    lastUpdate.value = res.data?.updated_at || '-'
  } catch (e) {
    /* ignore */
  }
}

// ============ 租户配置 ============
const tenantConfigs = ref<any[]>([])
const tenantConfigLoading = ref(false)
const tenantConfigQuery = reactive({ key: '' })
const tenantConfigDialogVisible = ref(false)
const tenantConfigFormRef = ref<FormInstance>()
const tenantConfigForm = reactive({
  id: 0,
  config_key: '',
  config_value: '',
  config_type: 'STRING',
  description: '',
})
const fetchTenantConfigs = async () => {
  tenantConfigLoading.value = true
  try {
    const params: any = {}
    if (tenantConfigQuery.key) params.key = tenantConfigQuery.key
    const res: any = await request.get('/tenant/config/settings', { params })
    tenantConfigs.value = res.data?.items || res.data || []
  } catch (e) {
    /* ignore */
  } finally {
    tenantConfigLoading.value = false
  }
}
const openTenantConfigDialog = (row?: any) => {
  if (row) {
    Object.assign(tenantConfigForm, row)
  } else {
    Object.assign(tenantConfigForm, {
      id: 0,
      config_key: '',
      config_value: '',
      config_type: 'STRING',
      description: '',
    })
  }
  tenantConfigDialogVisible.value = true
}
const saveTenantConfig = async () => {
  try {
    await request.post('/tenant/config/settings', {
      key: tenantConfigForm.config_key,
      value: tenantConfigForm.config_value,
      config_type: tenantConfigForm.config_type,
      description: tenantConfigForm.description,
    })
    ElMessage.success('保存成功')
    tenantConfigDialogVisible.value = false
    fetchTenantConfigs()
  } catch (e: any) {
    ElMessage.error(e.message || '保存失败')
  }
}
const deleteTenantConfig = async (row: any) => {
  try {
    await ElMessageBox.confirm('确定删除?', '确认', { type: 'warning' })
    await request.delete(`/tenant/config/settings/${row.config_key}`)
    ElMessage.success('删除成功')
    fetchTenantConfigs()
  } catch (e: any) {
    if (e !== 'cancel') ElMessage.error(e.message || '删除失败')
  }
}

// ============ 公司信息 ============
const companyFormRef = ref<FormInstance>()
const companySubmitLoading = ref(false)
const companyForm = reactive({
  company_name: '',
  company_short_name: '',
  credit_code: '',
  legal_representative: '',
  registered_capital: 0,
  establishment_date: '',
  phone: '',
  fax: '',
  email: '',
  website: '',
  address: '',
  bank_name: '',
  bank_account: '',
  taxpayer_type: 'general',
  tax_registration_number: '',
  logo: '',
  remarks: '',
})
const companyRules: FormRules = {
  company_name: [{ required: true, message: '请输入公司名称', trigger: 'blur' }],
}
const fetchCompanyInfo = async () => {
  try {
    const s = localStorage.getItem('company_info')
    if (s) Object.assign(companyForm, JSON.parse(s))
  } catch (e) {
    /* ignore */
  }
}
const saveCompanyInfo = async () => {
  if (!companyFormRef.value) return
  await companyFormRef.value.validate(async (valid) => {
    if (!valid) return
    companySubmitLoading.value = true
    try {
      localStorage.setItem('company_info', JSON.stringify(companyForm))
      ElMessage.success('保存成功')
    } catch (e: any) {
      ElMessage.error(e.message || '保存失败')
    } finally {
      companySubmitLoading.value = false
    }
  })
}
const resetCompanyForm = () => {
  companyFormRef.value?.resetFields()
}

onMounted(() => {
  loadTabData('user')
})
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
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.filter-card {
  margin-bottom: 20px;
}
.mr-1 {
  margin-right: 4px;
}
.mt-4 {
  margin-top: 16px;
}
.mb-4 {
  margin-bottom: 16px;
}
.text-gray {
  color: #909399;
}
</style>
