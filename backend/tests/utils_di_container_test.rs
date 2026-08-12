#[cfg(test)]
mod tests {
use bingxi_backend::utils::di_container::*;


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