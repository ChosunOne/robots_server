use std::hash::Hash;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache as MokaCacheImpl;
use thiserror::Error;

#[async_trait]
pub trait Cache<K: Eq + Hash + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static>:
    Send + Sync + 'static
{
    async fn get(&self, key: &K) -> CacheResult<Option<V>>;
    async fn set(&self, key: K, value: V) -> CacheResult<()>;
    async fn delete(&self, key: &K) -> CacheResult<bool>;
}

pub type CacheResult<T> = Result<T, CacheError>;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Cache backend unavailable")]
    Unavailable,
    #[error("Cache write failed: {0}")]
    WriteFailed(String),
}

pub struct MokaCache<K: Hash + Eq + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static>
{
    cache: MokaCacheImpl<K, V>,
}

impl<K: Hash + Eq + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static>
    MokaCache<K, V>
{
    pub fn new() -> Self {
        Self {
            cache: MokaCacheImpl::builder()
                .time_to_live(Duration::from_hours(24))
                .build(),
        }
    }
}

#[async_trait]
impl<K: Hash + Eq + Clone + Send + Sync + 'static, V: Clone + Send + Sync + 'static> Cache<K, V>
    for MokaCache<K, V>
{
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        Ok(self.cache.get(key).await)
    }

    async fn set(&self, key: K, value: V) -> CacheResult<()> {
        self.cache.insert(key, value).await;
        Ok(())
    }

    async fn delete(&self, key: &K) -> CacheResult<bool> {
        Ok(self.cache.remove(key).await.is_some())
    }
}
