use std::any::{Any, TypeId};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypedMap {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl TypedMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Send + Sync>(&mut self, value: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(value));
    }

    pub fn get<T: 'static + Send + Sync>(&self) -> &T {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
            .unwrap()
    }

    pub fn try_get<T: 'static + Send + Sync>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct TypedU32(pub u32);

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct AnotherTypedU32(pub u32);

    #[derive(Debug, Clone)]
    struct ComplexType {
        _mutexed: Arc<Mutex<Vec<u32>>>,
    }

    #[test]
    fn test_typed_cache() {
        let mut cache = TypedMap::new();

        cache.insert(TypedU32(23));
        cache.insert(AnotherTypedU32(32));

        assert_eq!(cache.try_get::<TypedU32>(), Some(&TypedU32(23)));
        assert_eq!(
            cache.try_get::<AnotherTypedU32>(),
            Some(&AnotherTypedU32(32))
        );
    }

    #[test]
    fn complex_send_and_sync_type() {
        let mut cache = TypedMap::new();

        cache.insert(ComplexType {
            _mutexed: Arc::new(Mutex::new(vec![1, 2, 3])),
        });

        assert!(cache.try_get::<ComplexType>().is_some());
    }

    #[tokio::test]
    async fn is_send_and_sync() -> eyre::Result<()> {
        let mut cache = TypedMap::new();

        cache.insert(TypedU32(23));
        cache.insert(AnotherTypedU32(32));

        let cache = Arc::new(cache);

        let handle_1 = {
            let cache = cache.clone();
            tokio::spawn(async move {
                assert_eq!(cache.try_get::<TypedU32>(), Some(&TypedU32(23)));
            })
        };

        let handle_2 = {
            let cache = cache.clone();

            tokio::spawn(async move {
                assert_eq!(
                    cache.try_get::<AnotherTypedU32>(),
                    Some(&AnotherTypedU32(32))
                );
            })
        };

        let (a, b) = tokio::join!(handle_1, handle_2);
        a?;
        b?;

        Ok(())
    }
}
