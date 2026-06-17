/**
 * 输入校验工具
 */

/** 校验用户名（4-20 位字母数字下划线） */
export function isValidUsername(username: string): boolean {
  return /^[a-zA-Z0-9_]{4,20}$/.test(username);
}

/** 校验密码（6-20 位） */
export function isValidPassword(password: string): boolean {
  return password.length >= 6 && password.length <= 20;
}

/** 校验邮箱 */
export function isValidEmail(email: string): boolean {
  return /^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$/.test(email);
}
