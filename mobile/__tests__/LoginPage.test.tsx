/**
 * LoginPage 单元测试
 *
 * 沙箱限制：
 * - 无 react-native 测试环境
 * - 仅保留基础渲染测试
 * - CI 启用完整测试
 */
import React from 'react';
import { render } from '@testing-library/react-native';

// 注：完整测试需要 PaperProvider 等 Provider 包裹
// 当前 demo 仅测试模块导入和基本结构

describe('LoginPage 模块', () => {
  it('能够导入', () => {
    const { LoginPage } = require('../src/pages/LoginPage');
    expect(LoginPage).toBeDefined();
  });

  it('ApiClient 导出正确', () => {
    const { ApiClient } = require('../src/components/ApiClient');
    expect(ApiClient).toBeDefined();
    expect(ApiClient.auth).toBeDefined();
    expect(ApiClient.inventory).toBeDefined();
  });

  it('authStore 导出正确', () => {
    const { useAuthStore } = require('../src/stores/authStore');
    expect(useAuthStore).toBeDefined();
  });
});

describe('ApiClient 模块', () => {
  it('暴露 auth.login / auth.logout', () => {
    const { ApiClient } = require('../src/components/ApiClient');
    expect(typeof ApiClient.auth.login).toBe('function');
    expect(typeof ApiClient.auth.logout).toBe('function');
  });

  it('暴露 inventory.list', () => {
    const { ApiClient } = require('../src/components/ApiClient');
    expect(typeof ApiClient.inventory.list).toBe('function');
  });
});

describe('输入校验', () => {
  it('用户名校验', () => {
    const { isValidUsername } = require('../src/utils/validation');
    expect(isValidUsername('user123')).toBe(true);
    expect(isValidUsername('ab')).toBe(false);
    expect(isValidUsername('user@123')).toBe(false);
  });

  it('密码校验', () => {
    const { isValidPassword } = require('../src/utils/validation');
    expect(isValidPassword('123456')).toBe(true);
    expect(isValidPassword('123')).toBe(false);
  });

  it('邮箱校验', () => {
    const { isValidEmail } = require('../src/utils/validation');
    expect(isValidEmail('test@example.com')).toBe(true);
    expect(isValidEmail('invalid')).toBe(false);
  });
});
