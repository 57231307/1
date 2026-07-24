<template>
  <div class="error-page">
    <el-result icon="warning" title="403" :sub-title="$t('error.page.forbiddenMessage')">
      <template #extra>
        <el-button type="primary" @click="$router.push('/')">{{ $t('error.page.backHome') }}</el-button>
      </template>
    </el-result>
  </div>
</template>
