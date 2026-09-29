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
            <el-button
              v-if="row.status !== 'closed'"
              size="small"
              type="primary"
              @click="onAdvance(row)"
              >推进下一阶段</el-button
            >
            <el-button
              v-if="row.status === 'd8_recognize'"
              size="small"
              type="success"
              @click="onClose(row)"
              >关闭</el-button
            >
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
          <el-input
            v-model="form.plan"
            type="textarea"
            :rows="3"
            placeholder="准备阶段计划说明（可选）"
          />
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
import { logger } from '@/utils/logger';
import {
  advanceQuality8d,
  closeQuality8d,
  getQuality8dList,
  printQuality8d,
  startQuality8d,
  type Quality8dReport,
} from '@/api/quality-8d';

const list = ref<Quality8dReport[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const form = reactive({ quality_issue_id: undefined as number | undefined, plan: '' });

/**
 * 解包 8D 列表响应：request 返回完整信封 {code,message,data}，
 * 分页体在 data.items（EightDPagedResponse）。原实现直接读顶层 items，
 * 恒为 undefined 后 `?? []` 静默变成空列表——列表有数据也显示"暂无数据"。
 */
const unwrapList = (res: unknown): Quality8dReport[] => {
  const data = (res as { data?: unknown })?.data;
  if (Array.isArray(data)) return data as Quality8dReport[];
  const items = (data as { items?: unknown })?.items;
  if (!Array.isArray(items)) {
    throw new Error(`8D 列表响应缺少 data.items 数组：${JSON.stringify(res).slice(0, 200)}`);
  }
  return items as Quality8dReport[];
};

const STAGE_LABELS: Record<string, string> = {
  not_started: '未启动',
  d0_plan: 'D0 成立团队前计划',
  d1_team: 'D1 团队',
  d2_problem: 'D2 问题描述',
  d3_interim: 'D3 临时遏制',
  d4_root_cause: 'D4 根因分析',
  d5_permanent: 'D5 永久纠正措施',
  d6_verify: 'D6 效果验证',
  d7_prevent: 'D7 预防再发',
  d8_recognize: 'D8 总结表彰',
  closed: '已关闭',
};

const stageLabel = (s: string) => {
  const label = STAGE_LABELS[s];
  if (!label) {
    logger.warn(`未知 8D 阶段状态，后端状态值清单需同步：${s}`);
    return s;
  }
  return label;
};
const stageTag = (s: string) =>
  s === 'closed' ? 'info' : s === 'not_started' ? 'warning' : 'primary';

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

// 推进边按「当前状态」索引，字段名与 step 取值对齐 AdvanceStepPayload
// （models/quality_8d_dto.rs:67-96 的 tagged enum，step 为 snake_case 变体名，
//  且每个 step 的适用前态由后端 quality_8d_service.rs:440-450 规定）
const STAGE_EDGES: Record<
  string,
  {
    step: string;
    fields: Array<{ key: string; label: string; placeholder?: string }>;
  }
> = {
  d0_plan: {
    step: 'd1_team',
    fields: [{ key: 'team_members', label: '团队成员（D1）' }],
  },
  d1_team: {
    step: 'd2_problem',
    fields: [{ key: 'problem_description', label: '问题描述（D2）' }],
  },
  d2_problem: {
    step: 'd3_interim',
    fields: [{ key: 'interim_action', label: '临时遏制措施（D3）' }],
  },
  d3_interim: {
    step: 'd4_root_cause',
    fields: [
      { key: 'method', label: '根因分析方法（5why / fishbone / other）', placeholder: '5why' },
      { key: 'detail', label: '根因分析详细过程（D4）' },
      { key: 'summary', label: '根因总结' },
    ],
  },
  d4_root_cause: {
    step: 'd5_permanent',
    fields: [
      { key: 'permanent_action', label: '永久纠正措施（D5）' },
      { key: 'action_owner', label: '措施责任人' },
      { key: 'due_date', label: '计划完成日期（YYYY-MM-DD）' },
    ],
  },
  d5_permanent: {
    step: 'd6_verify',
    fields: [{ key: 'verification_result', label: '效果验证结果（D6）' }],
  },
  d6_verify: {
    step: 'd7_prevent',
    fields: [{ key: 'prevention_action', label: '预防措施（D7）' }],
  },
  d7_prevent: {
    step: 'd8_recognize',
    fields: [{ key: 'closure_summary', label: 'closure 总结表彰（D8）' }],
  },
};

async function onAdvance(row: Quality8dReport) {
  const edge = STAGE_EDGES[row.status];
  if (!edge) {
    // 终态（d8_recognize 只能关闭、closed）或后端新增了阶段值：明确提示，不静默
    ElMessage.warning(`阶段 ${stageLabel(row.status)}（${row.status}）没有可推进的边`);
    return;
  }
  const values: Record<string, string> = {};
  for (const f of edge.fields) {
    const { value } = await ElMessageBox.prompt(
      f.placeholder ? `${f.label}（如 ${f.placeholder}）` : f.label,
      `推进到 ${stageLabel(edge.step)}`
    );
    values[f.key] = value;
  }
  const payload = { step: edge.step, ...values } as Parameters<typeof advanceQuality8d>[1];
  await advanceQuality8d(row.id, payload);
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
