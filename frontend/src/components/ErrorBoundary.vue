<template>
  <div v-if="error" class="error-boundary" role="alert" aria-live="assertive">
    <div class="error-boundary__content">
      <el-result icon="error" :title="errorTitle" :sub-title="errorSubTitle">
        <template #extra>
          <el-button type="primary" @click="handleRetry">
            <el-icon><Refresh /></el-icon>
            {{ t('components.errorBoundary.retry') }}
          </el-button>
          <el-button @click="handleReset">
            <el-icon><House /></el-icon>
            {{ t('components.errorBoundary.backHome') }}
          </el-button>
          <el-button text @click="showDetail = !showDetail">
            {{ showDetail ? t('components.errorBoundary.hideDetail') : t('components.errorBoundary.showDetail') }}
          </el-button>
        </template>
      </el-result>
      <el-collapse-transition>
        <pre v-if="showDetail" class="error-boundary__detail">{{ errorStack }}</pre>
      </el-collapse-transition>
    </div>
  </div>
  <slot v-else />
</template>

<script setup lang="ts">
import { ref, onErrorCaptured, computed } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { Refresh, House } from '@element-plus/icons-vue'
import { logger } from '@/utils/logger'

interface Props {
  /** 自定义错误标题 */
  title?: string
  /** 自定义错误副标题 */
  subTitle?: string
  /** 是否上报错误到监控服务（默认 true） */
  report?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  title: '',
  subTitle: '',
  report: true,
})

const { t } = useI18n({ useScope: 'global' })
const router = useRouter()

const error = ref<Error | null>(null)
const showDetail = ref(false)

const errorTitle = computed(() => props.title || t('components.errorBoundary.defaultTitle'))
const errorSubTitle = computed(
  () => props.subTitle || t('components.errorBoundary.defaultSubTitle')
)
const errorStack = computed(() => {
  if (!error.value) return ''
  return `${error.value.name}: ${error.value.message}\n\nStack:\n${error.value.stack || 'N/A'}`
})

/// 捕获子组件抛出的错误，阻止错误冒泡导致整页白屏
onErrorCaptured((err: Error, _instance, info) => {
  error.value = err
  // V15 P1-20-10 前端错误监控上报（通过 logger 统一上报）
  if (props.report) {
    logger.error('[ErrorBoundary] component error captured', {
      name: err.name,
      message: err.message,
      stack: err.stack,
      info,
      url: window.location.href,
      timestamp: new Date().toISOString(),
    })
  }
  // 返回 false 阻止错误继续向上冒泡
  return false
})

const handleRetry = () => {
  error.value = null
  showDetail.value = false
}

const handleReset = () => {
  error.value = null
  showDetail.value = false
  router.push('/').catch(() => {
    window.location.href = '/'
  })
}

defineExpose({
  hasError: () => error.value !== null,
  clearError: () => {
    error.value = null
    showDetail.value = false
  },
})
</script>

<style scoped>
.error-boundary {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
  padding: 24px;
}

.error-boundary__content {
  width: 100%;
  max-width: 640px;
}

.error-boundary__detail {
  margin-top: 16px;
  padding: 16px;
  background: var(--el-fill-color-light, #f5f7fa);
  border: 1px solid var(--el-border-color, #e4e7ed);
  border-radius: 4px;
  font-family: 'Courier New', Consolas, monospace;
  font-size: 12px;
  line-height: 1.6;
  color: var(--el-color-danger, #f56c6c);
  white-space: pre-wrap;
  word-break: break-all;
  overflow-x: auto;
  max-height: 300px;
  overflow-y: auto;
}
</style>
