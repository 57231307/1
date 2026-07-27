import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'
import { resolve } from 'path'

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 3000,
    allowedHosts: ['.monkeycode-ai.online'],
    proxy: {
      '/api': {
        target: 'http://localhost:8082',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'static',
    sourcemap: false,
    // V15 P1-20-3 chunk 分割策略：将大依赖拆分为独立 chunk，优化首屏加载
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        manualChunks: {
          // Vue 核心
          'vue-vendor': ['vue', 'vue-router', 'vue-i18n', 'pinia'],
          // Element Plus UI 库
          'element-plus': ['element-plus', '@element-plus/icons-vue'],
          // ECharts 图表库
          'echarts-vendor': ['echarts', 'echarts/core', 'echarts/charts', 'echarts/components', 'echarts/renderers'],
          // 工具库（仅引用 package.json 实际存在的依赖，避免 rollup 解析失败）
          'utils-vendor': ['axios'],
        },
      },
    },
  },
  // V15 P1-20-3 预构建依赖优化（减少冷启动时间）
  optimizeDeps: {
    include: [
      'vue',
      'vue-router',
      'vue-i18n',
      'pinia',
      'element-plus',
      '@element-plus/icons-vue',
      'axios',
      'dayjs',
      'echarts/core',
      'echarts/charts',
      'echarts/components',
      'echarts/renderers',
    ],
    exclude: ['@playwright/test'],
  },
})
