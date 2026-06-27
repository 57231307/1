//! Dependency Injection Container
//!
//! Provides a simple DI container for managing service lifecycle and dependencies

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

/// P9-1: 互斥锁加锁 helper，把散落的 expect 集中到此处
///
/// 互斥锁中毒（Poisoned）通常意味着有线程 panic，状态已不可信。
/// 安全修复：改为优雅降级（`e.into_inner()` 恢复数据继续运行），避免 panic 导致服务中断。
/// TODO(tech-debt): 未来迁移到 `parking_lot::Mutex`（无中毒概念），彻底消除此问题。
fn lock_or_panic<'a, T>(mutex: &'a Mutex<T>, ctx: &str) -> MutexGuard<'a, T> {
    mutex.lock().unwrap_or_else(|e| {
        tracing::error!(ctx = %ctx, error = %e, "P9-1: 互斥锁中毒，恢复数据继续运行以避免服务中断");
        e.into_inner()
    })
}

/// Service factory function type
pub type ServiceFactory = Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// Simple DI container for service management
pub struct DIContainer {
    services: Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>,
    factories: Mutex<HashMap<TypeId, ServiceFactory>>,
}

impl DIContainer {
    /// Create a new DI container
    pub fn new() -> Self {
        Self {
            services: Mutex::new(HashMap::new()),
            factories: Mutex::new(HashMap::new()),
        }
    }

    /// Register a singleton service
    pub fn register_singleton<T: Any + Send + Sync>(&self, instance: Arc<T>) {
        let mut services = lock_or_panic(&self.services, "DI容器服务锁");
        services.insert(TypeId::of::<T>(), instance);
    }

    /// Register a factory for lazy initialization
    pub fn register_factory<T: Any + Send + Sync>(
        &self,
        factory: Box<dyn Fn() -> Arc<T> + Send + Sync>,
    ) {
        let mut factories = lock_or_panic(&self.factories, "DI容器工厂锁");
        let type_id = TypeId::of::<T>();
        let boxed_factory: ServiceFactory = Box::new(move || {
            let instance: Arc<T> = factory();
            instance as Arc<dyn Any + Send + Sync>
        });
        factories.insert(type_id, boxed_factory);
    }

    /// Get a service instance
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Try to get from singletons first
        {
            let services = lock_or_panic(&self.services, "DI容器服务锁-读");
            if let Some(instance) = services.get(&type_id) {
                return instance.clone().downcast::<T>().ok();
            }
        }

        // Try to create from factory
        {
            let mut factories = lock_or_panic(&self.factories, "DI容器工厂锁-读");
            if let Some(factory) = factories.remove(&type_id) {
                let instance = factory();
                let typed_instance = instance.clone().downcast::<T>().ok()?;

                // Cache as singleton
                let mut services = lock_or_panic(&self.services, "DI容器服务锁-写");
                services.insert(type_id, instance);

                return Some(typed_instance);
            }
        }

        None
    }

    /// Check if a service is registered
    pub fn has<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let services = lock_or_panic(&self.services, "DI容器服务锁-has");
        let factories = lock_or_panic(&self.factories, "DI容器工厂锁-has");
        services.contains_key(&type_id) || factories.contains_key(&type_id)
    }

    /// Remove a service
    pub fn remove<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let mut services = lock_or_panic(&self.services, "DI容器服务锁-remove");
        let mut factories = lock_or_panic(&self.factories, "DI容器工厂锁-remove");
        let removed1 = services.remove(&type_id).is_some();
        let removed2 = factories.remove(&type_id).is_some();
        removed1 || removed2
    }

    /// Clear all services
    pub fn clear(&self) {
        let mut services = lock_or_panic(&self.services, "DI容器服务锁-clear");
        let mut factories = lock_or_panic(&self.factories, "DI容器工厂锁-clear");
        services.clear();
        factories.clear();
    }

    /// Get registered service count
    pub fn count(&self) -> usize {
        let services = lock_or_panic(&self.services, "DI容器服务锁-count");
        let factories = lock_or_panic(&self.factories, "DI容器工厂锁-count");
        services.len() + factories.len()
    }
}

impl Default for DIContainer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestService {
        pub value: String,
    }

    #[test]
    fn test_register_and_get() {
        let container = DIContainer::new();
        let service = Arc::new(TestService {
            value: "test".to_string(),
        });
        container.register_singleton(service);

        let retrieved = container.get::<TestService>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "test");
    }

    #[test]
    fn test_factory() {
        let container = DIContainer::new();
        container.register_factory::<TestService>(Box::new(|| {
            Arc::new(TestService {
                value: "factory".to_string(),
            })
        }));

        let retrieved = container.get::<TestService>();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().value, "factory");
    }

    #[test]
    fn test_not_found() {
        let container = DIContainer::new();
        let retrieved = container.get::<TestService>();
        assert!(retrieved.is_none());
    }
}
