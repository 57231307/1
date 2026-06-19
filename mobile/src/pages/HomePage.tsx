/**
 * 首页（占位）
 *
 * P3-3 仅做登录 demo，HomePage 是占位
 * 后续 P4+ 阶段会扩展为：
 * - Dashboard（KPI 概览）
 * - Inventory（库存列表）
 * - Sales（销售订单）
 * - Production（生产工单）
 * - Notifications（通知中心，集成 P3-2 WebSocket）
 */
import React from 'react';
import { View, StyleSheet } from 'react-native';
import { Text, Button } from 'react-native-paper';
import { useAuthStore } from '../stores/authStore';

export const HomePage: React.FC = () => {
  const { user, logout } = useAuthStore();

  return (
    <View style={styles.container}>
      <Text variant="headlineSmall" style={styles.greeting}>
        欢迎，{user?.username ?? '用户'}
      </Text>
      <Text variant="bodyMedium" style={styles.subtitle}>
        租户 ID：{user?.tenant_id ?? '-'}
      </Text>
      <Text variant="bodySmall" style={styles.note}>
        （P3-3 demo 仅展示登录流程，业务模块将在 P4+ 阶段实现）
      </Text>
      <Button
        mode="outlined"
        onPress={logout}
        style={styles.button}
      >
        退出登录
      </Button>
    </View>
  );
};

const styles = StyleSheet.create({
  container: { flex: 1, padding: 20, backgroundColor: '#fff' },
  greeting: { marginTop: 20, fontWeight: 'bold' },
  subtitle: { marginTop: 4, color: '#666' },
  note: { marginTop: 30, color: '#999', textAlign: 'center' },
  button: { marginTop: 40 },
});
