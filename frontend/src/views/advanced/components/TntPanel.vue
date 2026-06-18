<script setup lang="ts">
/**
 * TntPanel - 多租户管理 tab 视图组件（纯展示）
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 3 个 tab）
 * 包含：租户列表、新建/编辑租户按钮
 * 数据与函数全部由父组件通过 props 传入
 */
import { Plus } from '@element-plus/icons-vue'

interface Props {
  tenants: any[]
  tenantLoading: boolean
  openTenantDialog: (row?: any) => void
  updateTenantStatus: (row: any) => Promise<void>
  deleteTenant: (row: any) => Promise<void>
}

defineProps<Props>()
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">租户管理</h2>
    <el-button type="primary" @click="openTenantDialog()">
      <el-icon><Plus /></el-icon>
      新建租户
    </el-button>
  </div>

  <el-card shadow="hover">
    <el-table v-loading="tenantLoading" :data="tenants" stripe>
      <el-table-column prop="tenant_code" label="租户编码" width="120" />
      <el-table-column prop="tenant_name" label="租户名称" width="180" />
      <el-table-column prop="domain" label="域名" width="180" />
      <el-table-column prop="subscription_plan" label="订阅方案" width="120" />
      <el-table-column prop="current_users" label="当前用户" width="100" align="right" />
      <el-table-column prop="max_users" label="最大用户" width="100" align="right" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag
            :type="
              row.status === 'active'
                ? 'success'
                : row.status === 'suspended'
                  ? 'danger'
                  : 'info'
            "
            size="small"
          >
            {{
              row.status === 'active' ? '正常' : row.status === 'suspended' ? '暂停' : '停用'
            }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column prop="subscription_start_date" label="开始日期" width="120" />
      <el-table-column prop="subscription_end_date" label="结束日期" width="120" />
      <el-table-column label="操作" width="200" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="openTenantDialog(row as any)"
            >编辑</el-button
          >
          <el-button type="warning" link size="small" @click="updateTenantStatus(row as any)"
            >更新状态</el-button
          >
          <el-button type="danger" link size="small" @click="deleteTenant(row as any)"
            >删除</el-button
          >
        </template>
      </el-table-column>
    </el-table>
  </el-card>
</template>
