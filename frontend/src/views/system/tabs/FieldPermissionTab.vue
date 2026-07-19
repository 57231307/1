<!--
  FieldPermissionTab.vue - 字段权限 Tab
  来源：原 system/index.vue 中 字段权限 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="field-permission-tab">
    <div class="page-header">
      <h2 class="page-title">字段权限</h2>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="fieldPermLoading" :data="fieldPermissionList" stripe aria-label="字段权限列表">
        <el-table-column prop="role_name" label="角色" width="120" />
        <el-table-column prop="resource_type" label="资源" width="120" />
        <el-table-column prop="field_name" label="字段名" width="150" />
        <el-table-column prop="visible" label="可见" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.visible ? 'success' : 'danger'" size="small">
              {{ row.visible ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="editable" label="可编辑" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.editable ? 'success' : 'info'" size="small">
              {{ row.editable ? '是' : '否' }}
            </el-tag>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { request } from '@/api/request'

interface FieldPermissionRow {
  role_name: string
  resource_type: string
  field_name: string
  visible: boolean
  editable: boolean
}

const fieldPermissionList = ref<FieldPermissionRow[]>([])
const fieldPermLoading = ref(false)

const fetchFieldPermissions = async () => {
  fieldPermLoading.value = true
  try {
    const res = await request.get<{ items?: FieldPermissionRow[] } | FieldPermissionRow[]>(
      '/permissions/fields'
    )
    const d = res
    if (d && typeof d === 'object' && 'items' in d) {
      fieldPermissionList.value = d.items || []
    } else {
      fieldPermissionList.value = (d as FieldPermissionRow[]) || []
    }
  } catch (_e) {
    fieldPermissionList.value = []
  } finally {
    fieldPermLoading.value = false
  }
}

defineExpose({ refresh: fetchFieldPermissions })

onMounted(() => {
  fetchFieldPermissions()
})
</script>
