import js from '@eslint/js';
import tseslint from 'typescript-eslint';
import pluginVue from 'eslint-plugin-vue';
import prettier from 'eslint-config-prettier';
import globals from 'globals';

export default [
  // e2e/ 与 tests/ 不再全局 ignore：此前它们被排除在 eslint 之外，
  // 于是 no-unused-expressions 对测试代码完全失效——`expect(x).toBe;`（漏括号）、
  // `cond && expect(y)`（条件成立才断言，不成立静默通过）这类"看起来在断言、
  // 实际什么都没验"的写法在测试里比业务代码更常见，却一条都拦不住。
  { ignores: ['node_modules/', 'dist/', 'scripts/', '*.config.*', '*.test.*', 'check_*.js', 'full_test.js', 'comprehensive_test.cjs'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...pluginVue.configs['flat/recommended'],
  prettier,
  {
    files: ['**/*.{js,ts,vue}'],
    languageOptions: {
      globals: { ...globals.browser, ...globals.node },
      parserOptions: {
        parser: tseslint.parser,
        ecmaVersion: 'latest',
        sourceType: 'module',
        ecmaFeatures: { jsx: true },
      },
    },
    rules: {
      'vue/multi-word-component-names': 'off',
      'vue/no-v-html': 'error',
      'vue/require-default-prop': 'off',
      'vue/require-explicit-emits': 'warn',
      'vue/no-deprecated-filter': 'off',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/no-unused-expressions': ['error', { allowTernary: true, allowShortCircuit: true }],
      '@typescript-eslint/consistent-type-assertions': 'error',
      '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
      '@typescript-eslint/ban-ts-comment': 'off',
      '@typescript-eslint/no-non-null-assertion': 'off',
      'no-console': ['warn', { allow: ['warn', 'error'] }],
      'no-debugger': 'warn',
      'no-unused-vars': 'off',
    },
  },
  {
    files: ['**/*.vue'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    files: ['**/*.test.ts', '**/*.spec.ts'],
    rules: {
      '@typescript-eslint/no-explicit-any': 'off',
      // 测试代码里不允许"短路/三元形式的断言"：`ok && expect(x)` 在 ok 为假时
      // 一条断言都不执行却通过，`ok ? expect(x) : expect(y)` 同理会随机失效。
      // 全局配置允许这两种写法（业务代码里 `cond && fn()` 是常规卫语句），
      // 在 spec 里必须禁掉——本仓库多轮"假绿"的直接来源就是这种条件断言。
      '@typescript-eslint/no-unused-expressions': [
        'error',
        { allowTernary: false, allowShortCircuit: false },
      ],
    },
  },
  {
    files: ['scripts/**/*.cjs', 'scripts/**/*.js'],
    languageOptions: {
      sourceType: 'script',
      ecmaVersion: 2022,
      globals: { ...globals.node },
    },
    rules: {
      '@typescript-eslint/no-require-imports': 'off',
      'no-console': 'off',
    },
  },
];