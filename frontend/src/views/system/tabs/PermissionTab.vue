<!--
  PermissionTab.vue - 权限管理 Tab
  来源：原 system/index.vue 中 权限管理 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="permission-tab">
    <div class="page-header">
      <h2 class="page-title">权限管理</h2>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="permissionListLoading" :data="permissionList" stripe>
        <el-table-column prop="resource_type" label="资源类型" width="150" />
        <el-table-column prop="action" label="操作" width="120" />
        <el-table-column prop="allowed" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="row.allowed ? 'success' : 'danger'" size="small">
              {{ row.allowed ? '允许' : '禁止' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="resource_id" label="资源ID" width="100" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { request } from '@/api/request'

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
