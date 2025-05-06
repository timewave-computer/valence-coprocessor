use r2d2::Pool;
use redis::{Client, Commands as _, IntoConnectionInfo};
use valence_coprocessor::{Blake3Hasher, DataBackend, Hash, Hasher as _};

#[derive(Debug, Clone)]
pub struct RedisBackend {
    pool: Pool<Client>,
    namespace: String,
}

impl RedisBackend {
    /// Opens a new connection into redis.
    pub fn open<T: IntoConnectionInfo>(params: T) -> anyhow::Result<Self> {
        let client = Client::open(params)?;

        // test the connection
        client.get_connection()?;

        let pool = Pool::builder().build(client)?;
        let namespace = Default::default();

        Ok(Self { pool, namespace })
    }

    /// Uses a pre-defined redis client.
    pub fn with_redis(mut self, client: Client) -> anyhow::Result<Self> {
        self.pool = Pool::builder().build(client)?;
        Ok(self)
    }

    /// Associate this dataset with a namespace.
    pub fn with_namespace(mut self, namespace: String) -> Self {
        self.namespace = namespace;
        self
    }

    /// Computes the prefix key.
    pub fn prefix(&self, bytes: &[u8]) -> Hash {
        Blake3Hasher::digest([self.namespace.as_bytes(), bytes])
    }

    /// Computes an internal redis key
    pub fn key(&self, prefix: &[u8], key: &[u8]) -> Vec<u8> {
        let prefix = self.prefix(prefix);

        [&prefix, key].concat()
    }
}

impl DataBackend for RedisBackend {
    fn get(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = self.key(prefix, key);

        Ok(self.pool.get()?.get(&key)?)
    }

    fn has(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<bool> {
        self.get(prefix, key).map(|v| v.is_some())
    }

    fn remove(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = self.key(prefix, key);

        Ok(self.pool.get()?.get_del(&key)?)
    }

    fn set(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key = self.key(prefix, key);
        let mut conn = self.pool.get()?;

        let old = conn.get_del(&key)?;
        let _: () = conn.set(key, data)?;

        Ok(old)
    }

    // TODO split the storage
    fn get_bulk(&self, prefix: &[u8], key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        self.get(prefix, key)
    }

    fn set_bulk(&self, prefix: &[u8], key: &[u8], data: &[u8]) -> anyhow::Result<()> {
        self.set(prefix, key, data)?;

        Ok(())
    }
}

#[test]
#[ignore = "depends on running docker"]
fn test_redis_connection() {
    // depends on `docker run --rm -p 56379:6379 redis`

    let client = RedisBackend::open("redis://127.0.0.1:56379/").unwrap();
    let prf = b"prefix";
    let key = b"key";
    let val = b"val";
    let vxl = b"vxl";

    assert!(client.get(prf, key).unwrap().is_none());
    assert!(!client.has(prf, key).unwrap());
    assert!(client.remove(prf, key).unwrap().is_none());

    assert!(client.set(prf, key, val).unwrap().is_none());
    assert!(client.has(prf, key).unwrap());
    assert_eq!(client.get(prf, key).unwrap(), Some(val.to_vec()));

    assert_eq!(client.remove(prf, key).unwrap(), Some(val.to_vec()));

    assert!(client.set(prf, key, val).unwrap().is_none());
    assert_eq!(client.set(prf, key, vxl).unwrap(), Some(val.to_vec()));

    assert_eq!(client.remove(prf, key).unwrap(), Some(vxl.to_vec()));
    assert!(!client.has(prf, key).unwrap());
    assert!(client.remove(prf, key).unwrap().is_none());
    assert!(client.get(prf, key).unwrap().is_none());
}
