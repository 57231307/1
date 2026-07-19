<!--
  WebhookTab.vue - Webhook 配置 Tab
  来源：原 system/index.vue 中 Webhook tab 内容
  拆分日期：2026-06-15 B3-1
-->
<template>
  <div class="webhook-tab">
    <div class="page-header">
      <h2 class="page-title">Webhook 配置</h2>
      <el-button type="primary" @click="openWebhookDialog()">
        <el-icon><Plus /></el-icon> 新建
      </el-button>
    </div>
    <el-card shadow="hover">
      <el-table v-loading="webhookLoading" :data="webhookList" stripe aria-label="Webhook 列表">
        <el-table-column prop="name" label="名称" width="150" />
        <el-table-column prop="url" label="URL" min-width="250" show-overflow-tooltip />
        <el-table-column prop="event_type" label="事件" width="120" />
        <el-table-column prop="is_active" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.is_active ? 'success' : 'info'" size="small">
              {{ row.is_active ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <!-- P2-17 修复（批次 86 v2 复审）：编辑/删除按钮补齐 v-permission -->
            <el-button v-permission="'webhook:update'" size="small" link @click="openWebhookDialog(row as unknown as WebhookRow)"
              >编辑</el-button
            >
            <el-button
              size="small"
              link
              type="warning"
              @click="testWebhook(row as unknown as WebhookRow)"
              >测试</el-button
            >
            <el-button
              v-permission="'webhook:delete'"
              size="small"
              link
              type="danger"
              @click="deleteWebhook(row as unknown as WebhookRow)"
              >删除</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog
      v-model="webhookDialogVisible"
      :title="webhookForm.id ? '编辑 Webhook' : '新建 Webhook'"
      width="500px"
      aria-label="Webhook 编辑对话框"
    >
      <el-form ref="webhookFormRef" :model="webhookForm" label-width="100px" aria-label="Webhook 信息表单">
        <el-form-item label="名称" prop="name">
          <el-input v-model="webhookForm.name" />
        </el-form-item>
        <el-form-item label="URL" prop="url">
          <el-input v-model="webhookForm.url" placeholder="https://" />
        </el-form-item>
        <el-form-item label="事件类型">
          <el-select v-model="webhookForm.event_type" style="width: 100%">
            <el-option label="订单创建" value="order.created" />
            <el-option label="订单更新" value="order.updated" />
            <el-option label="库存变动" value="inventory.changed" />
            <el-option label="审批完成" value="approval.completed" />
            <el-option label="全部" value="all" />
          </el-select>
        </el-form-item>
        <el-form-item label="密钥">
          <el-input v-model="webhookForm.secret" placeholder="可选" />
        </el-form-item>
        <el-form-item label="状态">
          <el-switch v-model="webhookForm.is_active" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="webhookDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="saveWebhook">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { reactive, ref, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import type { FormInstance } from 'element-plus'
import { request } from '@/api/request'

interface WebhookRow {
  id: number
  name: string
  url: string
  event_type: string
  is_active: boolean
  secret?: string
}

const webhookList = ref<WebhookRow[]>([])
const webhookLoading = ref(false)
const webhookDialogVisible = ref(false)
const webhookFormRef = ref<FormInstance>()
const webhookForm = reactive<WebhookRow>({
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
    const res = await request.get<{ items?: WebhookRow[] } | WebhookRow[]>('/webhooks/integrations')
    const d = res
    if (d && typeof d === 'object' && 'items' in d) {
      webhookList.value = d.items || []
    } else {
      webhookList.value = (d as WebhookRow[]) || []
    }
  } catch (_e) {
    webhookList.value = []
  } finally {
    webhookLoading.value = false
  }
}

const openWebhookDialog = (row?: WebhookRow) => {
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
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '保存失败')
  }
}

const deleteWebhook = async (row: WebhookRow) => {
  try {
    await ElMessageBox.confirm('确定删除?', '确认', { type: 'warning' })
    await request.delete(`/webhooks/integrations/${row.id}`)
    ElMessage.success('删除成功')
    fetchWebhooks()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const testWebhook = async (row: WebhookRow) => {
  try {
    await request.post(`/webhooks/integrations/${row.id}`)
    ElMessage.success('测试请求已发送')
  } catch (e) {
    const err = e as { message?: string }
    ElMessage.error(err.message || '测试失败')
  }
}

defineExpose({ refresh: fetchWebhooks })

onMounted(() => {
  fetchWebhooks()
})
</script>
