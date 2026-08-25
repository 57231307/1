import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import AutoImport from 'unplugin-auto-import/vite';
import Components from 'unplugin-vue-components/vite';
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers';
import { visualizer } from 'rollup-plugin-visualizer';
import { resolve } from 'path';

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
    visualizer({ open: false, gzipSize: true, brotliSize: true }),
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
      '/api/': {
        target: 'http://localhost:8082',
        changeOrigin: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'static',
    sourcemap: false,
    target: 'esnext',
    // V15 P1-20-3 chunk 分割策略：将大依赖拆分为独立 chunk，优化首屏加载
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        // Vite 8 (Rolldown) 不支持对象格式 manualChunks，改为函数
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('vue') || id.includes('vue-router') || id.includes('vue-i18n') || id.includes('pinia')) {
              return 'vue-vendor';
            }
            if (id.includes('element-plus') || id.includes('@element-plus/icons-vue')) {
              return 'element-plus';
            }
            if (id.includes('echarts')) {
              return 'echarts-vendor';
            }
            if (id.includes('axios')) {
              return 'utils-vendor';
            }
          }
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
});
