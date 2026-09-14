<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>质量 8D 报告</span>
          <el-button type="primary" @click="dialogVisible = true">启动 8D</el-button>
        </div>
      </template>
      <el-table v-loading="loading" :data="list" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="quality_issue_id" label="质量问题ID" width="110" />
        <el-table-column prop="status" label="当前阶段" width="130">
          <template #default="{ row }">
            <el-tag :type="stageTag(row.status)">{{ stageLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="plan" label="D0 计划说明" min-width="180" show-overflow-tooltip />
        <el-table-column prop="created_at" label="创建时间" width="170" />
        <el-table-column label="操作" width="260" fixed="right">
          <template #default="{ row }">
            <el-button v-if="row.status !== 'closed'" size="small" type="primary" @click="onAdvance(row)">推进下一阶段</el-button>
            <el-button v-if="row.status === 'd8'" size="small" type="success" @click="onClose(row)">关闭</el-button>
            <el-button size="small" @click="onPrint(row)">打印</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="启动 8D 报告" width="480">
      <el-form :model="form" label-width="110px">
        <el-form-item label="质量问题ID" required>
          <el-input-number v-model="form.quality_issue_id" :min="1" class="w-full" />
        </el-form-item>
        <el-form-item label="D0 计划说明">
          <el-input v-model="form.plan" type="textarea" :rows="3" placeholder="准备阶段计划说明（可选）" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onStart">启动</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import {
  advanceQuality8d,
  closeQuality8d,
  getQuality8dList,
  printQuality8d,
  startQuality8d,
  QUALITY_8D_STAGES,
  type Quality8dReport,
} from '@/api/quality-8d';

const list = ref<Quality8dReport[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const form = reactive({ quality_issue_id: undefined as number | undefined, plan: '' });

const unwrapList = (p: unknown): Quality8dReport[] =>
  Array.isArray(p) ? p : ((p as { items?: Quality8dReport[] })?.items ?? []);

const stageLabel = (s: string) => (s === 'closed' ? '已关闭' : s.toUpperCase());
const stageTag = (s: string) => (s === 'closed' ? 'info' : s === 'not_started' ? 'warning' : 'primary');

async function load() {
  loading.value = true;
  try {
    list.value = unwrapList(await getQuality8dList());
  } finally {
    loading.value = false;
  }
}

async function onStart() {
  if (!form.quality_issue_id) {
    ElMessage.warning('请填写质量问题ID');
    return;
  }
  saving.value = true;
  try {
    await startQuality8d({
      quality_issue_id: form.quality_issue_id,
      plan: form.plan || undefined,
    });
    ElMessage.success('8D 报告已启动');
    dialogVisible.value = false;
    form.quality_issue_id = undefined;
    form.plan = '';
    await load();
  } finally {
    saving.value = false;
  }
}

async function onAdvance(row: Quality8dReport) {
  const idx = QUALITY_8D_STAGES.indexOf(row.status as (typeof QUALITY_8D_STAGES)[number]);
  const next = QUALITY_8D_STAGES[Math.min(idx + 1, QUALITY_8D_STAGES.length - 2)];
  await ElMessageBox.confirm(`确认推进到下一阶段 ${next.toUpperCase()}？`, '推进确认');
  await advanceQuality8d(row.id, { to_stage: next });
  ElMessage.success('已推进');
  await load();
}

async function onClose(row: Quality8dReport) {
  await ElMessageBox.confirm('确认关闭该 8D 报告？', '关闭确认');
  await closeQuality8d(row.id);
  ElMessage.success('已关闭');
  await load();
}

async function onPrint(row: Quality8dReport) {
  await printQuality8d(row.id);
  ElMessage.success('打印数据已生成');
}

onMounted(load);
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.w-full {
  width: 100%;
}
</style>
