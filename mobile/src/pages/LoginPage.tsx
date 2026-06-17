/**
 * 登录页（关键路径 demo）
 *
 * 功能：
 * - 用户名 / 密码输入
 * - 登录按钮（loading 状态）
 * - 错误提示
 * - 登录成功后自动跳转到 HomePage
 */
import React, { useState } from 'react';
import { View, StyleSheet, Alert } from 'react-native';
import { TextInput, Button, Text } from 'react-native-paper';

import { ApiClient } from '../components/ApiClient';
import { useAuthStore } from '../stores/authStore';

export const LoginPage: React.FC = () => {
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const login = useAuthStore((s) => s.login);

  const handleLogin = async () => {
    if (!username || !password) {
      Alert.alert('提示', '请输入用户名和密码');
      return;
    }
    setLoading(true);
    try {
      const response = await ApiClient.auth.login({ username, password });
      await login(response.token, response.user);
    } catch (err: any) {
      Alert.alert('登录失败', err?.message ?? '未知错误');
    } finally {
      setLoading(false);
    }
  };

  return (
    <View style={styles.container}>
      <Text variant="headlineMedium" style={styles.title}>
        冰溪 ERP 移动版
      </Text>
      <Text variant="bodyMedium" style={styles.subtitle}>
        P3-3 React Native 关键路径 demo
      </Text>
      <TextInput
        label="用户名"
        value={username}
        onChangeText={setUsername}
        mode="outlined"
        style={styles.input}
        autoCapitalize="none"
        autoCorrect={false}
      />
      <TextInput
        label="密码"
        value={password}
        onChangeText={setPassword}
        mode="outlined"
        secureTextEntry
        style={styles.input}
      />
      <Button
        mode="contained"
        onPress={handleLogin}
        loading={loading}
        disabled={loading}
        style={styles.button}
        contentStyle={styles.buttonContent}
      >
        登录
      </Button>
    </View>
  );
};

const styles = StyleSheet.create({
  container: { flex: 1, padding: 20, justifyContent: 'center', backgroundColor: '#fff' },
  title: { textAlign: 'center', marginBottom: 8, fontWeight: 'bold' },
  subtitle: { textAlign: 'center', marginBottom: 30, color: '#888' },
  input: { marginBottom: 15 },
  button: { marginTop: 10 },
  buttonContent: { paddingVertical: 6 },
});
