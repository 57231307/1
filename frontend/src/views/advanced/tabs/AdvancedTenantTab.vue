<!--
  AdvancedTenantTab.vue - 高级多租户管理 Tab
  来源：原 advanced/index.vue 中 tenant tab
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <div class="page-header">
      <h2 class="page-title">租户管理</h2>
      <el-button type="primary" @click="emit('new-tenant')">
        <el-icon><Plus /></el-icon>
        新建租户
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="tenants" stripe>
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
                row.status === 'active' ? 'success' : row.status === 'suspended' ? 'danger' : 'info'
              "
              size="small"
            >
              {{ row.status === 'active' ? '正常' : row.status === 'suspended' ? '暂停' : '停用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="subscription_start_date" label="开始日期" width="120" />
        <el-table-column prop="subscription_end_date" label="结束日期" width="120" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('edit-tenant', row)"
              >编辑</el-button
            >
            <el-button type="warning" link size="small" @click="emit('update-status', row)"
              >更新状态</el-button
            >
            <el-button type="danger" link size="small" @click="emit('delete-tenant', row)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { Plus } from '@element-plus/icons-vue'

defineProps<{
  tenants: any[]
  loading: boolean
}>()

defineEmits<{
  'new-tenant': []
  'edit-tenant': [row: any]
  'update-status': [row: any]
  'delete-tenant': [row: any]
}>()
</script>
