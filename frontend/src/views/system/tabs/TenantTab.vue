<!--
  TenantTab.vue - 租户配置 Tab
  来源：原 system/index.vue 中 租户配置 tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="tenant-tab">
    <div class="page-header">
      <h2 class="page-title">租户配置</h2>
    </div>
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
        <el-table-column prop="config_value" label="配置值" min-width="250" show-overflow-tooltip />
        <el-table-column prop="config_type" label="类型" width="100" />
        <el-table-column prop="description" label="描述" width="200" />
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button
              size="small"
              link
              @click="openTenantConfigDialog(row as unknown as TenantConfigRow)"
              >编辑</el-button
            >
            <el-button
              size="small"
              link
              type="danger"
              @click="deleteTenantConfig(row as unknown as TenantConfigRow)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="tenantConfigDialogVisible"
      :title="tenantConfigForm.id ? '编辑配置' : '新增配置'"
      width="500px"
    >
      <el-form ref="tenantConfigFormRef" :model="tenantConfigForm" label-width="100px">
        <el-form-item label="配置键" prop="config_key">
          <el-input v-model="tenantConfigForm.config_key" :disabled="!!tenantConfigForm.id" />
        </el-form-item>
        <el-form-item label="配置值" prop="config_value">
          <el-input v-model="tenantConfigForm.config_value" type="textarea" :rows="3" />
        </el-form-item>
        <el-form-item label="类型">
          <el-select v-model="tenantConfigForm.config_type" style="width: 100%">
            <el-option label="字符串" value="STRING" />
            <el-option label="数字" value="NUMBER" />
            <el-option label="布尔" value="BOOLEAN" />
            <el-option label="JSON" value="JSON" />
          </el-select>
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="tenantConfigForm.description" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="tenantConfigDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveTenantConfig">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance } from 'element-plus'
import { request } from '@/api/request'

interface TenantConfigRow {
  id: number
  config_key: string
  config_value: string
  config_type: 'STRING' | 'NUMBER' | 'BOOLEAN' | 'JSON'
  description: string
}

const tenantConfigs = ref<TenantConfigRow[]>([])
const tenantConfigLoading = ref(false)
const tenantConfigQuery = reactive<{ key: string }>({ key: '' })
const tenantConfigDialogVisible = ref(false)
const tenantConfigFormRef = ref<FormInstance>()
const tenantConfigForm = reactive<TenantConfigRow>({
  id: 0,
  config_key: '',
  config_value: '',
  config_type: 'STRING',
  description: '',
})

const fetchTenantConfigs = async () => {
  tenantConfigLoading.value = true
  try {
    const params: Record<string, string> = {}
    if (tenantConfigQuery.key) params.key = tenantConfigQuery.key
    const res = await request.get<{ items?: TenantConfigRow[] } | TenantConfigRow[]>(
      '/tenant/config/settings',
      { params }
    )
    const d = res
    if (d && typeof d === 'object' && 'items' in d) {
      tenantConfigs.value = d.items || []
    } else {
      tenantConfigs.value = (d as TenantConfigRow[]) || []
    }
  } catch (_e) {
    tenantConfigs.value = []
  } finally {
    tenantConfigLoading.value = false
  }
}

const openTenantConfigDialog = (row?: TenantConfigRow) => {
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
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '保存失败')
  }
}

const deleteTenantConfig = async (row: TenantConfigRow) => {
  try {
    await ElMessageBox.confirm('确定删除?', '确认', { type: 'warning' })
    await request.delete(`/tenant/config/settings/${row.config_key}`)
    ElMessage.success('删除成功')
    fetchTenantConfigs()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '删除失败')
    }
  }
}

defineExpose({ refresh: fetchTenantConfigs })

onMounted(() => {
  fetchTenantConfigs()
})
</script>
