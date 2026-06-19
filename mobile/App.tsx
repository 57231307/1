/**
 * 冰溪 ERP 移动端根组件
 *
 * P3-3 关键路径 demo：
 * - 路由：React Navigation（未登录 → LoginPage / 已登录 → HomePage）
 * - 状态：Zustand（authStore）
 * - 启动时调用 hydrate() 恢复登录状态
 */
import React, { useEffect } from 'react';
import { NavigationContainer } from '@react-navigation/native';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { Provider as PaperProvider } from 'react-native-paper';

import { LoginPage } from './src/pages/LoginPage';
import { HomePage } from './src/pages/HomePage';
import { useAuthStore } from './src/stores/authStore';

const Stack = createNativeStackNavigator();

export default function App() {
  const { token, hydrate } = useAuthStore();

  useEffect(() => {
    // 启动时从 AsyncStorage 恢复登录状态
    hydrate();
  }, [hydrate]);

  return (
    <PaperProvider>
      <NavigationContainer>
        <Stack.Navigator>
          {token ? (
            <Stack.Screen
              name="Home"
              component={HomePage}
              options={{ title: '冰溪 ERP' }}
            />
          ) : (
            <Stack.Screen
              name="Login"
              component={LoginPage}
              options={{ headerShown: false }}
            />
          )}
        </Stack.Navigator>
      </NavigationContainer>
    </PaperProvider>
  );
}
