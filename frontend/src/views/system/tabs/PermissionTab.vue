<!--
  PermissionTab.vue - 权限管理 Tab
  来源：原 system/index.vue 中 权限管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="permission-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('system.permission.title') }}</h2>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="permissionListLoading" :data="permissionList" stripe :aria-label="t('system.permission.aria.list')">
        <el-table-column prop="resource_type" :label="t('system.permission.column.resourceType')" width="150" />
        <el-table-column prop="action" :label="t('system.permission.column.action')" width="120" />
        <el-table-column prop="allowed" :label="t('system.permission.column.status')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.allowed ? 'success' : 'danger'" size="small">
              {{ row.allowed ? t('system.permission.status.allowed') : t('system.permission.status.denied') }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="resource_id" :label="t('system.permission.column.resourceId')" width="100" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { request } from '@/api/request'

const { t } = useI18n({ useScope: 'global' })

interface PermissionRow {
  resource_type: string
  action: string
  allowed: boolean
  resource_id: string
}

const permissionList = ref<PermissionRow[]>([])
const permissionListLoading = ref(false)

const fetchPermissionList = async () => {
  permissionListLoading.value = true
  try {
    const res = await request.get<PermissionRow[]>('/permissions')
    permissionList.value = res || []
  } catch (_e) {
    // 接口失败时静默处理，避免向用户暴露内部错误
    permissionList.value = []
  } finally {
    permissionListLoading.value = false
  }
}

defineExpose({ refresh: fetchPermissionList })

onMounted(() => {
  fetchPermissionList()
})
</script>
