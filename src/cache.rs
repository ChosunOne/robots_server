use std::fmt::Debug;
use std::hash::Hash;
use std::time::Duration;

use async_trait::async_trait;
use moka::future::Cache as MokaCacheImpl;
use thiserror::Error;
use tracing::{debug, instrument};

#[async_trait]
pub trait Cache<
    K: Eq + Hash + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
>: Send + Sync + 'static
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

pub struct MokaCache<
    K: Hash + Eq + Clone + Debug + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
> {
    cache: MokaCacheImpl<K, V>,
}

impl<K: Hash + Eq + Clone + Debug + Send + Sync + 'static, V: Clone + Send + Sync + 'static>
    MokaCache<K, V>
{
    pub fn new() -> Self {
        debug!("Creating new Moka cache with 24h TTL");
        Self {
            cache: MokaCacheImpl::builder()
                .time_to_live(Duration::from_hours(24))
                .build(),
        }
    }
}

#[async_trait]
impl<K: Hash + Eq + Clone + Debug + Send + Sync + 'static, V: Clone + Send + Sync + 'static>
    Cache<K, V> for MokaCache<K, V>
{
    #[instrument(skip(self, key), fields(key = ?key))]
    async fn get(&self, key: &K) -> CacheResult<Option<V>> {
        debug!("Checking cache for key");
        Ok(match self.cache.get(key).await {
            Some(value) => {
                debug!("Cache hit");
                Some(value)
            }
            None => {
                debug!("Cache miss");
                None
            }
        })
    }

    #[instrument(skip(self, key, value), fields(key = ?key))]
    async fn set(&self, key: K, value: V) -> CacheResult<()> {
        debug!("Inserting value into cache");
        self.cache.insert(key, value).await;
        debug!("Value inserted");
        Ok(())
    }

    #[instrument(skip(self, key), fields(key = ?key))]
    async fn delete(&self, key: &K) -> CacheResult<bool> {
        debug!("Removing key from cache");
        Ok(match self.cache.remove(key).await {
            Some(_) => {
                debug!("Removed existing key");
                true
            }
            None => {
                debug!("Key not in cache");
                false
            }
        })
    }
}
