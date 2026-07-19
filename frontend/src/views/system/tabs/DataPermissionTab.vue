<!--
  DataPermissionTab.vue - 数据权限 Tab
  来源：原 system/index.vue 中 数据权限 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="data-permission-tab">
    <div class="page-header">
      <h2 class="page-title">数据权限</h2>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="dataPermLoading" :data="dataPermissionList" stripe aria-label="数据权限列表">
        <el-table-column prop="role_name" label="角色" width="120" />
        <el-table-column prop="scope_type" label="权限范围" width="120" />
        <el-table-column prop="scope_value" label="范围值" min-width="200" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { request } from '@/api/request'

interface DataPermissionRow {
  role_name: string
  scope_type: string
  scope_value: string
  created_at: string
}

const dataPermissionList = ref<DataPermissionRow[]>([])
const dataPermLoading = ref(false)

const fetchDataPermissions = async () => {
  dataPermLoading.value = true
  try {
    const res = await request.get<{ items?: DataPermissionRow[] } | DataPermissionRow[]>(
      '/data-permissions'
    )
    const d = res
    if (d && typeof d === 'object' && 'items' in d) {
      dataPermissionList.value = d.items || []
    } else {
      dataPermissionList.value = (d as DataPermissionRow[]) || []
    }
  } catch (_e) {
    dataPermissionList.value = []
  } finally {
    dataPermLoading.value = false
  }
}

defineExpose({ refresh: fetchDataPermissions })

onMounted(() => {
  fetchDataPermissions()
})
</script>
