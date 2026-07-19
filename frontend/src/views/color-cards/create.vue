<template>
  <div class="color-card-create">
    <el-card>
      <template #header>
        <span>新建色卡</span>
      </template>

      <el-form :model="form" :rules="rules" ref="formRef" label-width="100px" style="max-width: 800px" aria-label="色卡创建表单">
        <el-form-item label="色卡编号" prop="card_no">
          <el-input v-model="form.card_no" placeholder="例如: PANTONE-TPX-2024-SS" />
        </el-form-item>
        <el-form-item label="色卡名称" prop="card_name">
          <el-input v-model="form.card_name" placeholder="例如: 2024 春夏 PANTONE 色卡" />
        </el-form-item>
        <el-form-item label="色卡类型" prop="card_type">
          <el-select v-model="form.card_type" placeholder="请选择" style="width: 100%">
            <el-option v-for="(label, value) in COLOR_CARD_TYPE_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item label="季节">
          <el-select v-model="form.season" placeholder="可选" clearable style="width: 100%">
            <el-option v-for="(label, value) in SEASON_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item label="品牌">
          <el-input v-model="form.brand" placeholder="例如: 自有品牌 / 客户名称" />
        </el-form-item>
        <el-form-item label="封面图 URL">
          <el-input v-model="form.cover_image_url" placeholder="可选: 封面图 URL" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="3" placeholder="可选: 色卡描述" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">立即创建</el-button>
          <el-button @click="$router.back()">取消</el-button>
        </el-form-item>
      </el-form>

      <el-alert
        v-if="createdCardId"
        type="success"
        :closable="false"
        style="margin-top: 24px"
      >
        <template #title>
          色卡创建成功 (ID: {{ createdCardId }})！
        </template>
        <div style="margin-top: 8px">
          <el-button type="primary" size="small" @click="goAddItems">立即添加色号</el-button>
          <el-button size="small" @click="resetForm">继续创建</el-button>
        </div>
      </el-alert>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useRouter } from 'vue-router'
import { ElMessage, FormInstance, FormRules } from 'element-plus'
import { createColorCard, COLOR_CARD_TYPE_LABELS, SEASON_LABELS } from '@/api/color-card'

const router = useRouter()
const formRef = ref<FormInstance>()
const submitting = ref(false)
const createdCardId = ref<number | null>(null)

const form = reactive({
  card_no: '',
  card_name: '',
  card_type: 'CUSTOM',
  season: '',
  brand: '',
  description: '',
  cover_image_url: '',
})

const rules: FormRules = {
  card_no: [{ required: true, message: '请输入色卡编号', trigger: 'blur' }],
  card_name: [{ required: true, message: '请输入色卡名称', trigger: 'blur' }],
  card_type: [{ required: true, message: '请选择色卡类型', trigger: 'change' }],
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async (valid) => {
    if (!valid) return
    submitting.value = true
    try {
      const res: Awaited<ReturnType<typeof createColorCard>> = await createColorCard({
        card_no: form.card_no,
        card_name: form.card_name,
        card_type: form.card_type,
        season: form.season || undefined,
        brand: form.brand || undefined,
        description: form.description || undefined,
        cover_image_url: form.cover_image_url || undefined,
      })
      createdCardId.value = res.data?.id
      ElMessage.success('色卡创建成功')
    } finally {
      submitting.value = false
    }
  })
}

const goAddItems = () => {
  if (createdCardId.value) {
    router.push(`/color-cards/detail/${createdCardId.value}?tab=items`)
  }
}

const resetForm = () => {
  form.card_no = ''
  form.card_name = ''
  form.card_type = 'CUSTOM'
  form.season = ''
  form.brand = ''
  form.description = ''
  form.cover_image_url = ''
  createdCardId.value = null
}
</script>

<style scoped>
.color-card-create { padding: 16px; }
</style>
