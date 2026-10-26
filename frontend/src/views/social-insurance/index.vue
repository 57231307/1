<template>
  <div class="page">
    <el-card shadow="never">
      <template #header>
        <div class="card-header">
          <span>社保管理</span>
          <el-button type="primary" @click="dialogVisible = true">新建参保记录</el-button>
        </div>
      </template>
      <el-form inline class="mb">
        <el-form-item label="年度"
          ><el-input v-model="queryYearField.text" @change="onQueryYearChange" @blur="queryYearField.normalize()"
        /></el-form-item>
        <el-form-item label="月份"
          ><el-input v-model="queryMonthField.text" @change="onQueryMonthChange" @blur="queryMonthField.normalize()"
        /></el-form-item>
        <el-form-item label="员工ID"
          ><el-input v-model="queryWorkerField.text" @change="onQueryWorkerChange" @blur="queryWorkerField.normalize()"
        /></el-form-item>
      </el-form>
      <el-table v-loading="loading" :data="records" border>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="worker_id" label="员工ID" width="90" />
        <el-table-column prop="period_year" label="年度" width="80" />
        <el-table-column prop="period_month" label="月份" width="80" />
        <el-table-column prop="base_amount" label="缴费基数" width="110" />
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag
              :type="
                row.status === 'paid' ? 'success' : row.status === 'cancelled' ? 'info' : 'warning'
              "
            >
              {{ INSURANCE_STATUS_LABEL[row.status] ?? row.status }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="payment_date" label="缴费日期" width="120" />
        <el-table-column label="操作" width="170" fixed="right">
          <template #default="{ row }">
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="success"
              @click="onMarkPaid(row)"
              >标记已缴</el-button
            >
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="danger"
              plain
              @click="onCancel(row)"
              >取消</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建参保记录" width="480">
      <el-form :model="form" label-width="100px">
        <el-form-item label="员工ID" required
          ><el-input v-model="formWorkerField.text" class="w-full" @blur="formWorkerField.normalize()"
        /></el-form-item>
        <el-form-item label="年度" required
          ><el-input v-model="formYearField.text" class="w-full" @blur="formYearField.normalize()"
        /></el-form-item>
        <el-form-item label="月份" required
          ><el-input v-model="formMonthField.text" class="w-full" @blur="formMonthField.normalize()"
        /></el-form-item>
        <el-form-item label="缴费基数" required
          ><el-input v-model="formBaseField.text" class="w-full" @blur="formBaseField.normalize()"
        /></el-form-item>
        <el-form-item label="缴费日期"
          ><el-date-picker
            v-model="form.payment_date"
            type="date"
            value-format="YYYY-MM-DD"
            class="w-full"
        /></el-form-item>
        <el-form-item label="备注"
          ><el-input v-model="form.remarks" type="textarea" :rows="2"
        /></el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="saving" @click="onCreate">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { onMounted, reactive, ref, toRef } from 'vue';
import { ElMessage, ElMessageBox } from 'element-plus';
import { logAuxLoadFailure } from '@/utils/logger';
import {
  useNumberField,
  validateNumberValue,
  type NumberFieldSpec,
} from '@/composables/useNumberField';
import {
  cancelInsurance,
  createSocialInsurance,
  getSocialInsuranceList,
  markInsurancePaid,
  INSURANCE_STATUS_LABEL,
  type SocialInsurance,
} from '@/api/social-insurance';

const records = ref<SocialInsurance[]>([]);
const loading = ref(false);
const saving = ref(false);
const dialogVisible = ref(false);
const query = reactive({
  year: undefined as number | undefined,
  month: undefined as number | undefined,
  worker_id: undefined as number | undefined,
});
const form = reactive({
  worker_id: undefined as number | undefined,
  period_year: new Date().getFullYear(),
  period_month: new Date().getMonth() + 1,
  base_amount: undefined as number | undefined,
  payment_date: '',
  remarks: '',
});

// 数值手输桥 范围与精度由单源校验器裁决（全项目禁步进控件）
const queryYearField = useNumberField(
  toRef(query, 'year'),
  { label: '年度', min: 2020, max: 2100, precision: 0 },
  'undefined'
);
const queryMonthField = useNumberField(
  toRef(query, 'month'),
  { label: '月份', min: 1, max: 12, precision: 0 },
  'undefined'
);
const queryWorkerField = useNumberField(
  toRef(query, 'worker_id'),
  { label: '员工ID', min: 1, precision: 0 },
  'undefined'
);
const formWorkerSpec: NumberFieldSpec = {
  label: '员工ID',
  required: true,
  min: 1,
  precision: 0,
};
const formYearSpec: NumberFieldSpec = { label: '年度', min: 2020, max: 2100, precision: 0 };
const formMonthSpec: NumberFieldSpec = { label: '月份', min: 1, max: 12, precision: 0 };
const formBaseSpec: NumberFieldSpec = {
  label: '缴费基数',
  required: true,
  min: 0,
  precision: 2,
};
const formWorkerField = useNumberField(
  toRef(form, 'worker_id'),
  formWorkerSpec,
  'undefined'
);
const formYearField = useNumberField(toRef(form, 'period_year'), formYearSpec);
const formMonthField = useNumberField(toRef(form, 'period_month'), formMonthSpec);
const formBaseField = useNumberField(
  toRef(form, 'base_amount'),
  formBaseSpec,
  'undefined'
);

const onQueryYearChange = () => {
  queryYearField.normalize();
  load();
};
const onQueryMonthChange = () => {
  queryMonthField.normalize();
  load();
};
const onQueryWorkerChange = () => {
  queryWorkerField.normalize();
  load();
};

const unwrapList = <T,>(p: unknown): T[] => (p as { data: { items: T[] } }).data.items;

async function load() {
  loading.value = true;
  try {
    records.value = unwrapList(
      await getSocialInsuranceList({
        period_year: query.year,
        period_month: query.month,
        worker_id: query.worker_id,
      })
    );
  } catch (e: unknown) {
    logAuxLoadFailure('社保参保记录加载失败', e);
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  if (!form.worker_id || !form.base_amount) {
    ElMessage.warning('请填写员工ID与缴费基数');
    return;
  }
  const invalid =
    validateNumberValue(form.worker_id, formWorkerSpec) ??
    validateNumberValue(form.period_year, formYearSpec) ??
    validateNumberValue(form.period_month, formMonthSpec) ??
    validateNumberValue(form.base_amount, formBaseSpec);
  if (invalid) {
    ElMessage.warning(invalid);
    return;
  }
  saving.value = true;
  try {
    await createSocialInsurance({
      worker_id: form.worker_id,
      period_year: form.period_year,
      period_month: form.period_month,
      base_amount: form.base_amount,
      payment_date: form.payment_date || undefined,
      remarks: form.remarks || undefined,
    });
    ElMessage.success('参保记录已创建');
    dialogVisible.value = false;
    await load();
  } finally {
    saving.value = false;
  }
}

async function onMarkPaid(row: SocialInsurance) {
  const { value } = await ElMessageBox.prompt('缴费日期（YYYY-MM-DD）', '标记已缴', {
    inputValue: new Date().toISOString().slice(0, 10),
  });
  if (!value) {
    ElMessage.warning('请填写缴费日期');
    return;
  }
  await markInsurancePaid(row.id, { payment_date: value });
  ElMessage.success('已标记缴费');
  await load();
}

async function onCancel(row: SocialInsurance) {
  await ElMessageBox.confirm('确认取消该参保记录？', '确认');
  await cancelInsurance(row.id);
  ElMessage.success('已取消');
  await load();
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
.mb {
  margin-bottom: 8px;
}
</style>
