//! Dependency Injection Container
//!
//! Provides a simple DI container for managing service lifecycle and dependencies

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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
        let mut services = self
            .services
            .lock()
            .expect("DI容器服务锁被污染，可能存在线程panic");
        services.insert(TypeId::of::<T>(), instance);
    }

    /// Register a factory for lazy initialization
    pub fn register_factory<T: Any + Send + Sync>(
        &self,
        factory: Box<dyn Fn() -> Arc<T> + Send + Sync>,
    ) {
        let mut factories = self
            .factories
            .lock()
            .expect("DI容器工厂锁被污染，可能存在线程panic");
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
            let services = self
                .services
                .lock()
                .expect("DI容器服务锁被污染，可能存在线程panic");
            if let Some(instance) = services.get(&type_id) {
                return instance.clone().downcast::<T>().ok();
            }
        }

        // Try to create from factory
        {
            let mut factories = self
                .factories
                .lock()
                .expect("DI容器工厂锁被污染，可能存在线程panic");
            if let Some(factory) = factories.remove(&type_id) {
                let instance = factory();
                let typed_instance = instance.clone().downcast::<T>().ok()?;

                // Cache as singleton
                let mut services = self
                    .services
                    .lock()
                    .expect("DI容器服务锁被污染，可能存在线程panic");
                services.insert(type_id, instance);

                return Some(typed_instance);
            }
        }

        None
    }

    /// Check if a service is registered
    pub fn has<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let services = self
            .services
            .lock()
            .expect("DI容器服务锁被污染，可能存在线程panic");
        let factories = self
            .factories
            .lock()
            .expect("DI容器工厂锁被污染，可能存在线程panic");
        services.contains_key(&type_id) || factories.contains_key(&type_id)
    }

    /// Remove a service
    pub fn remove<T: Any + Send + Sync>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let mut services = self
            .services
            .lock()
            .expect("DI容器服务锁被污染，可能存在线程panic");
        let mut factories = self
            .factories
            .lock()
            .expect("DI容器工厂锁被污染，可能存在线程panic");
        let removed1 = services.remove(&type_id).is_some();
        let removed2 = factories.remove(&type_id).is_some();
        removed1 || removed2
    }

    /// Clear all services
    pub fn clear(&self) {
        let mut services = self
            .services
            .lock()
            .expect("DI容器服务锁被污染，可能存在线程panic");
        let mut factories = self
            .factories
            .lock()
            .expect("DI容器工厂锁被污染，可能存在线程panic");
        services.clear();
        factories.clear();
    }

    /// Get registered service count
    pub fn count(&self) -> usize {
        let services = self
            .services
            .lock()
            .expect("DI容器服务锁被污染，可能存在线程panic");
        let factories = self
            .factories
            .lock()
            .expect("DI容器工厂锁被污染，可能存在线程panic");
        services.len() + factories.len()
    }
}

impl Default for DIContainer {
    fn default() -> Self {
        Self::new()
    }
}

/// Global DI container instance
pub static GLOBAL_CONTAINER: std::sync::LazyLock<DIContainer> =
    std::sync::LazyLock::new(DIContainer::new);

/// Register a service in the global container
pub fn register<T: Any + Send + Sync>(instance: Arc<T>) {
    GLOBAL_CONTAINER.register_singleton(instance);
}

/// Get a service from the global container
pub fn resolve<T: Any + Send + Sync>() -> Option<Arc<T>> {
    GLOBAL_CONTAINER.get::<T>()
}

/// Check if a service exists in the global container
pub fn is_registered<T: Any + Send + Sync>() -> bool {
    GLOBAL_CONTAINER.has::<T>()
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
