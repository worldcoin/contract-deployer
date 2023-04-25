use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{Mutex, Notify};

#[derive(Debug)]
pub struct NotifyCell {
    notify: Arc<Notify>,
    value: Option<Arc<dyn Any + Send + Sync>>,
}

#[derive(Debug)]
pub struct DependencyMap {
    map: Mutex<HashMap<TypeId, NotifyCell>>,
}

impl DependencyMap {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }

    pub async fn set<T: 'static + Send + Sync>(&self, value: T) {
        let mut map = self.map.lock().await;

        let key = TypeId::of::<T>();

        let entry = map.entry(key).or_insert_with(|| {
            let notify = Arc::new(Notify::new());
            NotifyCell {
                notify,
                value: None,
            }
        });

        entry.value = Some(Arc::new(value));
        entry.notify.notify_one();
    }

    pub async fn get<T: 'static + Send + Sync>(&self) -> Arc<T> {
        let key = TypeId::of::<T>();

        let mut map = self.map.lock().await;

        let entry = map.entry(key).or_insert_with(|| {
            let notify = Arc::new(Notify::new());
            NotifyCell {
                notify,
                value: None,
            }
        });

        if let Some(value) = entry.value.as_ref() {
            return value.clone().downcast().unwrap();
        }

        let notify = entry.notify.clone();

        // Drop the lock and wait for a notification
        drop(map);

        notify.notified().await;

        let map = self.map.lock().await;
        map.get(&key)
            .expect("Missing entry")
            .value
            .as_ref()
            .expect("Missing value")
            .clone()
            .downcast()
            .expect("Failed to downcast")
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

    #[tokio::test]
    async fn test_typed_cache() {
        let cache = DependencyMap::new();

        cache.set(TypedU32(23)).await;
        cache.set(AnotherTypedU32(32)).await;

        assert_eq!(cache.get::<TypedU32>().await, Arc::new(TypedU32(23)));
        assert_eq!(
            cache.get::<AnotherTypedU32>().await,
            Arc::new(AnotherTypedU32(32))
        );
    }

    #[tokio::test]
    async fn complex_send_and_sync_type() {
        let cache = DependencyMap::new();

        cache
            .set(ComplexType {
                _mutexed: Arc::new(Mutex::new(vec![1, 2, 3])),
            })
            .await;

        cache.get::<ComplexType>().await;
    }

    #[tokio::test]
    async fn is_send_and_sync() -> eyre::Result<()> {
        let cache = DependencyMap::new();

        cache.set(TypedU32(23)).await;
        cache.set(AnotherTypedU32(32)).await;

        let cache = Arc::new(cache);

        let handle_1 = {
            let cache = cache.clone();
            tokio::spawn(async move {
                assert_eq!(
                    cache.get::<TypedU32>().await,
                    Arc::new(TypedU32(23))
                );
            })
        };

        let handle_2 = {
            let cache = cache.clone();

            tokio::spawn(async move {
                assert_eq!(
                    cache.get::<AnotherTypedU32>().await,
                    Arc::new(AnotherTypedU32(32))
                );
            })
        };

        let (a, b) = tokio::join!(handle_1, handle_2);
        a?;
        b?;

        Ok(())
    }
}
