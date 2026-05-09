//! 插件系统
//!
//! 提供插件接口定义、加载器、生命周期管理

#![allow(dead_code)]

use std::collections::HashMap;

/// 插件状态
#[derive(Debug, Clone, PartialEq)]
pub enum PluginState {
    /// 已注册但未加载
    Registered,
    /// 已加载
    Loaded,
    /// 已启用
    Enabled,
    /// 已禁用
    Disabled,
    /// 已卸载
    Unloaded,
}

/// 插件元数据
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// 插件ID
    pub id: String,
    /// 插件名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 作者
    pub author: String,
    /// 描述
    pub description: String,
    /// 依赖的其他插件
    pub dependencies: Vec<String>,
    /// 插件入口点
    pub entry_point: String,
}

/// 插件接口
pub trait Plugin: Send + Sync {
    /// 获取插件元数据
    fn metadata(&self) -> &PluginMetadata;

    /// 初始化插件
    fn initialize(&mut self) -> Result<(), String>;

    /// 启动插件
    fn start(&mut self) -> Result<(), String>;

    /// 停止插件
    fn stop(&mut self) -> Result<(), String>;

    /// 卸载插件
    fn unload(&mut self) -> Result<(), String>;

    /// 执行插件功能
    fn execute(&self, action: &str, params: HashMap<String, String>) -> Result<String, String>;
}

/// 插件管理器
pub struct PluginManager {
    /// 已注册的插件
    plugins: HashMap<String, Box<dyn Plugin>>,
    /// 插件状态
    states: HashMap<String, PluginState>,
    /// 插件配置
    configs: HashMap<String, HashMap<String, String>>,
}

impl PluginManager {
    /// 创建新的插件管理器
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
            states: HashMap::new(),
            configs: HashMap::new(),
        }
    }

    /// 注册插件
    pub fn register(&mut self, plugin: Box<dyn Plugin>) -> Result<(), String> {
        let metadata = plugin.metadata();
        let id = metadata.id.clone();

        if self.plugins.contains_key(&id) {
            return Err(format!("插件 {} 已注册", id));
        }

        // 检查依赖
        for dep in &metadata.dependencies {
            if !self.plugins.contains_key(dep) {
                return Err(format!("插件 {} 依赖 {} 但未找到", id, dep));
            }
        }

        self.plugins.insert(id.clone(), plugin);
        self.states.insert(id.clone(), PluginState::Registered);
        self.configs.insert(id, HashMap::new());

        Ok(())
    }

    /// 加载插件
    pub fn load(&mut self, plugin_id: &str) -> Result<(), String> {
        let state = self.states.get(plugin_id).cloned();

        match state {
            Some(PluginState::Registered) | Some(PluginState::Unloaded) => {
                if let Some(plugin) = self.plugins.get_mut(plugin_id) {
                    plugin.initialize()?;
                    self.states.insert(plugin_id.to_string(), PluginState::Loaded);
                    Ok(())
                } else {
                    Err(format!("插件 {} 未注册", plugin_id))
                }
            }
            Some(s) => Err(format!("插件 {} 当前状态 {:?}，无法加载", plugin_id, s)),
            None => Err(format!("插件 {} 未注册", plugin_id)),
        }
    }

    /// 启用插件
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), String> {
        let state = self.states.get(plugin_id).cloned();

        match state {
            Some(PluginState::Loaded) | Some(PluginState::Disabled) => {
                if let Some(plugin) = self.plugins.get_mut(plugin_id) {
                    plugin.start()?;
                    self.states.insert(plugin_id.to_string(), PluginState::Enabled);
                    Ok(())
                } else {
                    Err(format!("插件 {} 未注册", plugin_id))
                }
            }
            Some(s) => Err(format!("插件 {} 当前状态 {:?}，无法启用", plugin_id, s)),
            None => Err(format!("插件 {} 未注册", plugin_id)),
        }
    }

    /// 禁用插件
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), String> {
        let state = self.states.get(plugin_id).cloned();

        match state {
            Some(PluginState::Enabled) => {
                if let Some(plugin) = self.plugins.get_mut(plugin_id) {
                    plugin.stop()?;
                    self.states.insert(plugin_id.to_string(), PluginState::Disabled);
                    Ok(())
                } else {
                    Err(format!("插件 {} 未注册", plugin_id))
                }
            }
            Some(s) => Err(format!("插件 {} 当前状态 {:?}，无法禁用", plugin_id, s)),
            None => Err(format!("插件 {} 未注册", plugin_id)),
        }
    }

    /// 卸载插件
    pub fn unload(&mut self, plugin_id: &str) -> Result<(), String> {
        let state = self.states.get(plugin_id).cloned();

        match state {
            Some(PluginState::Enabled) => {
                self.disable(plugin_id)?;
                self.unload(plugin_id)
            }
            Some(PluginState::Loaded) | Some(PluginState::Disabled) => {
                if let Some(plugin) = self.plugins.get_mut(plugin_id) {
                    plugin.unload()?;
                    self.states.insert(plugin_id.to_string(), PluginState::Unloaded);
                    Ok(())
                } else {
                    Err(format!("插件 {} 未注册", plugin_id))
                }
            }
            Some(s) => Err(format!("插件 {} 当前状态 {:?}，无法卸载", plugin_id, s)),
            None => Err(format!("插件 {} 未注册", plugin_id)),
        }
    }

    /// 执行插件功能
    pub fn execute(
        &self,
        plugin_id: &str,
        action: &str,
        params: HashMap<String, String>,
    ) -> Result<String, String> {
        let state = self.states.get(plugin_id).cloned();

        match state {
            Some(PluginState::Enabled) => {
                if let Some(plugin) = self.plugins.get(plugin_id) {
                    plugin.execute(action, params)
                } else {
                    Err(format!("插件 {} 未注册", plugin_id))
                }
            }
            Some(s) => Err(format!("插件 {} 当前状态 {:?}，无法执行", plugin_id, s)),
            None => Err(format!("插件 {} 未注册", plugin_id)),
        }
    }

    /// 获取插件状态
    pub fn get_state(&self, plugin_id: &str) -> Option<PluginState> {
        self.states.get(plugin_id).cloned()
    }

    /// 获取所有已注册插件
    pub fn list_plugins(&self) -> Vec<(String, PluginState, PluginMetadata)> {
        self.plugins
            .iter()
            .map(|(id, plugin)| {
                let state = self.states.get(id).cloned().unwrap_or(PluginState::Registered);
                let metadata = plugin.metadata().clone();
                (id.clone(), state, metadata)
            })
            .collect()
    }

    /// 设置插件配置
    pub fn set_config(&mut self, plugin_id: &str, key: String, value: String) -> Result<(), String> {
        if let Some(config) = self.configs.get_mut(plugin_id) {
            config.insert(key, value);
            Ok(())
        } else {
            Err(format!("插件 {} 未注册", plugin_id))
        }
    }

    /// 获取插件配置
    pub fn get_config(&self, plugin_id: &str, key: &str) -> Option<String> {
        self.configs
            .get(plugin_id)
            .and_then(|config| config.get(key).cloned())
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 示例插件：数据同步插件
pub struct DataSyncPlugin {
    metadata: PluginMetadata,
}

impl DataSyncPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "data_sync".to_string(),
                name: "数据同步插件".to_string(),
                version: "1.0.0".to_string(),
                author: "Bingxi ERP".to_string(),
                description: "同步数据到外部系统".to_string(),
                dependencies: vec![],
                entry_point: "sync".to_string(),
            },
        }
    }
}

impl Plugin for DataSyncPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn unload(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, action: &str, params: HashMap<String, String>) -> Result<String, String> {
        match action {
            "sync" => {
                let target = params.get("target").cloned().unwrap_or_default();
                Ok(format!("数据已同步到 {}", target))
            }
            _ => Err(format!("未知操作: {}", action)),
        }
    }
}

/// 示例插件：通知插件
pub struct NotificationPlugin {
    metadata: PluginMetadata,
}

impl NotificationPlugin {
    pub fn new() -> Self {
        Self {
            metadata: PluginMetadata {
                id: "notification".to_string(),
                name: "通知插件".to_string(),
                version: "1.0.0".to_string(),
                author: "Bingxi ERP".to_string(),
                description: "发送系统通知".to_string(),
                dependencies: vec![],
                entry_point: "notify".to_string(),
            },
        }
    }
}

impl Plugin for NotificationPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn start(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn unload(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn execute(&self, action: &str, params: HashMap<String, String>) -> Result<String, String> {
        match action {
            "send" => {
                let message = params.get("message").cloned().unwrap_or_default();
                let channel = params.get("channel").cloned().unwrap_or_default();
                Ok(format!("通知已发送到 {}: {}", channel, message))
            }
            _ => Err(format!("未知操作: {}", action)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager() {
        let mut manager = PluginManager::new();

        // 注册插件
        let plugin = Box::new(DataSyncPlugin::new());
        manager.register(plugin).unwrap();

        // 加载并启用
        manager.load("data_sync").unwrap();
        manager.enable("data_sync").unwrap();

        // 执行
        let mut params = HashMap::new();
        params.insert("target".to_string(), "external_system".to_string());
        let result = manager.execute("data_sync", "sync", params).unwrap();
        assert_eq!(result, "数据已同步到 external_system");

        // 禁用
        manager.disable("data_sync").unwrap();
        assert_eq!(manager.get_state("data_sync"), Some(PluginState::Disabled));
    }
}
