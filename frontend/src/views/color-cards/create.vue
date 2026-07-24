<template>
  <div class="color-card-create">
    <el-card>
      <template #header>
        <span>{{ $t('colorCards.create.title') }}</span>
      </template>

      <el-form :model="form" :rules="rules" ref="formRef" label-width="100px" style="max-width: 800px" :aria-label="$t('colorCards.create.formAriaLabel')">
        <el-form-item :label="$t('colorCards.create.cardNo')" prop="card_no">
          <el-input v-model="form.card_no" :placeholder="$t('colorCards.create.cardNoPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.create.cardName')" prop="card_name">
          <el-input v-model="form.card_name" :placeholder="$t('colorCards.create.cardNamePlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.create.cardType')" prop="card_type">
          <el-select v-model="form.card_type" :placeholder="$t('colorCards.create.pleaseSelect')" style="width: 100%">
            <el-option v-for="(label, value) in COLOR_CARD_TYPE_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorCards.filter.season')">
          <el-select v-model="form.season" :placeholder="$t('colorCards.create.optional')" clearable style="width: 100%">
            <el-option v-for="(label, value) in SEASON_LABELS" :key="value" :label="label" :value="value" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('colorCards.create.brand')">
          <el-input v-model="form.brand" :placeholder="$t('colorCards.create.brandPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.create.coverImageUrl')">
          <el-input v-model="form.cover_image_url" :placeholder="$t('colorCards.create.coverImageUrlPlaceholder')" />
        </el-form-item>
        <el-form-item :label="$t('colorCards.create.description')">
          <el-input v-model="form.description" type="textarea" :rows="3" :placeholder="$t('colorCards.create.descriptionPlaceholder')" />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" :loading="submitting" @click="handleSubmit">{{ $t('colorCards.create.submit') }}</el-button>
          <el-button @click="$router.back()">{{ $t('colorCards.create.cancel') }}</el-button>
        </el-form-item>
      </el-form>

      <el-alert
        v-if="createdCardId"
        type="success"
        :closable="false"
        style="margin-top: 24px"
      >
        <template #title>
          {{ $t('colorCards.create.successAlert', { id: createdCardId }) }}
        </template>
        <div style="margin-top: 8px">
          <el-button type="primary" size="small" @click="goAddItems">{{ $t('colorCards.create.addItems') }}</el-button>
          <el-button size="small" @click="resetForm">{{ $t('colorCards.create.continueCreate') }}</el-button>
        </div>
      </el-alert>
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 新建色卡页
// 提交逻辑：调用 createColorCard，成功后显示创建成功提示
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
import { ElMessage, FormInstance, FormRules } from 'element-plus'
import { createColorCard, COLOR_CARD_TYPE_LABELS, SEASON_LABELS } from '@/api/color-card'

const { t } = useI18n({ useScope: 'global' })

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

// 表单校验规则（响应式：随语言切换自动更新提示文案）
const rules = computed<FormRules>(() => ({
  card_no: [{ required: true, message: t('colorCards.validation.cardNoRequired'), trigger: 'blur' }],
  card_name: [{ required: true, message: t('colorCards.validation.cardNameRequired'), trigger: 'blur' }],
  card_type: [{ required: true, message: t('colorCards.validation.cardTypeRequired'), trigger: 'change' }],
}))

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
      ElMessage.success(t('colorCards.create.createSuccess'))
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
