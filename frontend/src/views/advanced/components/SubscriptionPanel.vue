<template>
  <div class="subscription-panel">
    <el-tabs v-model="innerTab">
      <el-tab-pane label="报表订阅" name="subscriptions">
        <div class="toolbar">
          <el-button type="primary" @click="openCreate">新建订阅</el-button>
          <el-button plain @click="loadSubscriptions">刷新</el-button>
        </div>
        <el-table v-loading="subLoading" :data="subscriptions" border>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="template_name" label="报表模板" min-width="140" />
          <el-table-column prop="schedule" label="周期" width="90">
            <template #default="{ row }">
              <el-tag>{{ scheduleLabel(row.schedule) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="schedule_time" label="发送时间" width="110" />
          <el-table-column label="收件人" min-width="180" show-overflow-tooltip>
            <template #default="{ row }">{{ (row.recipients || []).join('；') || '-' }}</template>
          </el-table-column>
          <el-table-column prop="format" label="格式" width="90" />
          <el-table-column label="状态" width="90">
            <template #default="{ row }">
              <el-tag :type="row.active ? 'success' : 'info'">
                {{ row.active ? '启用' : '停用' }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="last_sent_at" label="最近发送" width="160">
            <template #default="{ row }">{{ row.last_sent_at || '-' }}</template>
          </el-table-column>
          <el-table-column label="操作" width="250" fixed="right">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              <el-button link size="small" @click="handleToggle(row)">{{
                row.active ? '停用' : '启用'
              }}</el-button>
              <el-button link type="success" size="small" @click="handleSendNow(row)"
                >立即发送</el-button
              >
              <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
            </template>
          </el-table-column>
        </el-table>
      </el-tab-pane>

      <el-tab-pane label="字段与预览" name="preview">
        <div class="section-block">
          <h4>可用字段查询</h4>
          <div class="toolbar">
            <el-select v-model="fieldType" style="width: 180px" placeholder="模板类型">
              <el-option label="sales" value="sales" />
              <el-option label="inventory" value="inventory" />
              <el-option label="production" value="production" />
              <el-option label="finance" value="finance" />
              <el-option label="customer" value="customer" />
            </el-select>
            <el-button type="primary" plain :loading="fieldLoading" @click="loadFields">
              查询字段
            </el-button>
          </div>
          <el-table v-if="fields.length" :data="fields" border size="small">
            <el-table-column prop="key" label="字段键" min-width="140" />
            <el-table-column prop="label" label="名称" min-width="140" />
            <el-table-column prop="type" label="类型" width="100" />
            <el-table-column label="可排序" width="90">
              <template #default="{ row }">{{ row.sortable ? '是' : '否' }}</template>
            </el-table-column>
          </el-table>
        </div>

        <div class="section-block">
          <h4>增强报表预览</h4>
          <div class="toolbar">
            <el-input v-model="previewTemplateId" placeholder="模板 ID" style="width: 140px" />
            <el-button type="primary" plain :loading="previewLoading" @click="loadPreview">
              预览
            </el-button>
          </div>
          <template v-if="previewResult">
            <el-table :data="previewRows" border size="small" max-height="360">
              <!-- data 每行是按列序的字符串数组，非以列名为键的对象，故按位置索引渲染 -->
              <el-table-column
                v-for="(col, idx) in previewColumns"
                :key="col"
                :label="col"
                min-width="120"
                show-overflow-tooltip
              >
                <template #default="{ row }">{{ row[idx] }}</template>
              </el-table-column>
            </el-table>
            <p class="preview-total">共 {{ previewResult.total }} 行</p>
          </template>
        </div>
      </el-tab-pane>
    </el-tabs>

    <el-dialog v-model="subDialogVisible" :title="editingId ? '编辑订阅' : '新建订阅'" width="520">
      <el-form :model="subForm" label-width="100px">
        <el-form-item label="模板 ID" required>
          <el-input-number v-model="subForm.template_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="发送周期" required>
          <el-select v-model="subForm.schedule" style="width: 100%">
            <el-option label="每日" value="daily" />
            <el-option label="每周" value="weekly" />
            <el-option label="每月" value="monthly" />
          </el-select>
        </el-form-item>
        <el-form-item label="发送时间" required>
          <el-input v-model="subForm.schedule_time" placeholder="HH:mm，如 08:30" />
        </el-form-item>
        <el-form-item label="收件人" required>
          <el-input
            v-model="subForm.recipientsText"
            placeholder="多个邮箱用英文逗号分隔"
            type="textarea"
            :rows="2"
          />
        </el-form-item>
        <el-form-item label="导出格式" required>
          <el-select v-model="subForm.format" style="width: 100%">
            <el-option label="PDF" value="pdf" />
            <el-option label="Excel" value="excel" />
            <el-option label="PDF + Excel" value="both" />
          </el-select>
        </el-form-item>
        <el-form-item v-if="editingId" label="启用">
          <el-switch v-model="subForm.active" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="subDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="subSaving" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, onMounted } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  getSubscriptionList,
  createSubscription,
  updateSubscription,
  deleteSubscription,
  toggleSubscription,
  sendSubscriptionNow,
  getAvailableFields,
  previewReport,
  type ReportSubscription,
  type ReportField,
  type ReportPreviewResult,
} from '@/api/report-enhanced';

const innerTab = ref('subscriptions');

// ===== 订阅管理 =====
const subscriptions = ref<ReportSubscription[]>([]);
const subLoading = ref(false);
const subDialogVisible = ref(false);
const subSaving = ref(false);
const editingId = ref<number | null>(null);

const subForm = reactive({
  template_id: 1,
  schedule: 'daily' as 'daily' | 'weekly' | 'monthly',
  schedule_time: '08:30',
  recipientsText: '',
  format: 'pdf' as 'pdf' | 'excel' | 'both',
  active: true,
});

const scheduleLabel = (s: string) => ({ daily: '每日', weekly: '每周', monthly: '每月' })[s] ?? s;

async function loadSubscriptions() {
  subLoading.value = true;
  try {
    const res = await getSubscriptionList({ page: 1, page_size: 50 });
    const data = res.data as unknown as {
      items?: ReportSubscription[];
      list?: ReportSubscription[];
    };
    subscriptions.value = data?.items ?? data?.list ?? [];
  } finally {
    subLoading.value = false;
  }
}

const openCreate = () => {
  editingId.value = null;
  Object.assign(subForm, {
    template_id: 1,
    schedule: 'daily',
    schedule_time: '08:30',
    recipientsText: '',
    format: 'pdf',
    active: true,
  });
  subDialogVisible.value = true;
};

const openEdit = (row: ReportSubscription) => {
  editingId.value = row.id;
  Object.assign(subForm, {
    template_id: row.template_id,
    schedule: row.schedule,
    schedule_time: row.schedule_time,
    recipientsText: (row.recipients || []).join(','),
    format: row.format,
    active: row.active,
  });
  subDialogVisible.value = true;
};

const handleSave = async () => {
  const recipients = subForm.recipientsText
    .split(/[,;，；]/)
    .map(s => s.trim())
    .filter(Boolean);
  if (!subForm.template_id || !subForm.schedule_time || recipients.length === 0) {
    ElMessage.warning('请填写模板 ID/发送时间/收件人');
    return;
  }
  subSaving.value = true;
  try {
    if (editingId.value) {
      await updateSubscription(editingId.value, {
        schedule: subForm.schedule,
        schedule_time: subForm.schedule_time,
        recipients,
        format: subForm.format,
        active: subForm.active,
      });
      ElMessage.success('订阅已更新');
    } else {
      await createSubscription({
        template_id: subForm.template_id,
        schedule: subForm.schedule,
        schedule_time: subForm.schedule_time,
        recipients,
        format: subForm.format,
      });
      ElMessage.success('订阅已创建');
    }
    subDialogVisible.value = false;
    await loadSubscriptions();
  } finally {
    subSaving.value = false;
  }
};

const handleToggle = async (row: ReportSubscription) => {
  try {
    const res = await toggleSubscription(row.id);
    ElMessage.success(res.data?.active ? '订阅已启用' : '订阅已停用');
    await loadSubscriptions();
  } catch (e) {
    ElMessage.error((e as Error).message || '操作失败');
  }
};

const handleSendNow = async (row: ReportSubscription) => {
  try {
    await ElMessageBox.confirm(`立即发送订阅 #${row.id}？`, '确认');
  } catch {
    return;
  }
  try {
    await sendSubscriptionNow(row.id);
    ElMessage.success('已触发发送');
    await loadSubscriptions();
  } catch (e) {
    ElMessage.error((e as Error).message || '发送失败');
  }
};

const handleDelete = async (row: ReportSubscription) => {
  try {
    await ElMessageBox.confirm(`确认删除订阅 #${row.id}？`, '确认', { type: 'warning' });
  } catch {
    return;
  }
  try {
    await deleteSubscription(row.id);
    ElMessage.success('订阅已删除');
    await loadSubscriptions();
  } catch (e) {
    if (e !== 'cancel') ElMessage.error((e as Error).message || '删除失败');
  }
};

// ===== 字段查询 =====
const fieldType = ref('sales');
const fieldLoading = ref(false);
const fields = ref<ReportField[]>([]);

async function loadFields() {
  fieldLoading.value = true;
  try {
    const res = await getAvailableFields(fieldType.value);
    // 后端 data 为 { template_type, fields }，列表在 fields 键
    fields.value = res.data.fields;
  } catch (e) {
    ElMessage.error((e as Error).message || '查询字段失败');
  } finally {
    fieldLoading.value = false;
  }
}

// ===== 增强预览 =====
const previewTemplateId = ref('');
const previewLoading = ref(false);
const previewResult = ref<ReportPreviewResult | null>(null);

// 直接绑后端真实键：columns=string[]，data=string[][]（每行是按列序的字符串数组）。
// 未加载时 previewResult 为 null 才回退空数组；已加载载荷的列/行不做 `?? []` 兜底，
// 以免形状异常被静默吞成空集。
const previewColumns = computed<string[]>(() =>
  previewResult.value ? previewResult.value.columns : []
);
const previewRows = computed<string[][]>(() =>
  previewResult.value ? previewResult.value.data : []
);

async function loadPreview() {
  const id = Number(previewTemplateId.value);
  if (!id) {
    ElMessage.warning('请输入模板 ID');
    return;
  }
  previewLoading.value = true;
  try {
    const res = await previewReport(id);
    // 后端成功响应恒含 data，直接赋值；不用 `?? null` 掩盖契约形状问题。
    previewResult.value = res.data;
  } catch (e) {
    ElMessage.error((e as Error).message || '预览失败');
  } finally {
    previewLoading.value = false;
  }
}

onMounted(() => {
  loadSubscriptions();
});
</script>

<style scoped>
.toolbar {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 12px;
}
.section-block {
  margin-bottom: 24px;
}
.section-block h4 {
  margin: 0 0 8px;
}
.w-full {
  width: 100%;
}
.preview-total {
  color: var(--el-text-color-secondary);
  font-size: 12px;
  margin: 8px 0 0;
}
</style>
