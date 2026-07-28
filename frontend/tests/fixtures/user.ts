/**
 * 用户域测试 mock 数据夹具（V15 批次 06 P1-6 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 使用 createXxxMock(overrides?) 工厂函数模式，便于通过 overrides 灵活定制。
 */
import type { User, UserCreateRequest, UserUpdateRequest } from '@/api/user';

/** 创建用户 mock（默认 active 状态，可通过 overrides 覆盖任意字段） */
export function createUserMock(overrides: Partial<User> = {}): User {
  const now = new Date().toISOString();
  return {
    id: 1,
    username: 'admin',
    real_name: '管理员',
    email: 'admin@example.com',
    phone: '13800000000',
    department_id: 1,
    department_name: '信息部',
    role_ids: [1],
    role_names: ['超级管理员'],
    status: 1,
    created_at: now,
    updated_at: now,
    ...overrides,
  };
}

/** 创建禁用用户 mock */
export function createDisabledUserMock(overrides: Partial<User> = {}): User {
  return createUserMock({ status: 0, username: 'disabled_user', ...overrides });
}

/** 创建用户列表 mock（默认 3 个用户） */
export function createUserListMock(count = 3): User[] {
  return Array.from({ length: count }, (_, i) =>
    createUserMock({
      id: i + 1,
      username: `user${i + 1}`,
      real_name: `用户${i + 1}`,
      role_names: [`角色${i + 1}`],
    })
  );
}

/** 创建用户创建请求 mock（用于 POST /users） */
export function createUserCreateRequestMock(
  overrides: Partial<UserCreateRequest> = {}
): UserCreateRequest {
  return {
    username: 'new_user',
    password: 'Test@123456',
    real_name: '新用户',
    email: 'new@example.com',
    phone: '13900000000',
    department_id: 1,
    role_ids: [2],
    ...overrides,
  };
}

/** 创建用户更新请求 mock（用于 PUT /users/:id） */
export function createUserUpdateRequestMock(
  overrides: Partial<UserUpdateRequest> = {}
): UserUpdateRequest {
  return {
    real_name: '更新后的用户',
    email: 'updated@example.com',
    phone: '13700000000',
    department_id: 2,
    role_ids: [2, 3],
    status: 1,
    ...overrides,
  };
}

/** 创建修改密码请求 mock */
export function createChangePasswordRequestMock() {
  return {
    old_password: 'Old@123456',
    new_password: 'New@123456',
  };
}
