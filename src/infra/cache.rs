use crate::errors::AppError;
use deadpool_redis::{Config, Connection, Pool, Runtime};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone)]
pub struct Cache {
    pub pool: Pool,
}

impl Cache {
    pub fn new(redis_url: &str) -> Self {
        let cfg = Config::from_url(redis_url.to_string());

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .expect("Falha ao criar o pool do Redis");

        Self { pool }
    }

    async fn get_conn(&self) -> Result<Connection, AppError> {
        self.pool
            .get()
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao obter conexão do Redis: {}", e)))
    }

    pub async fn create_session(
        &self,
        user_id: &str,
        token: &str,
        expires_sec: i64,
    ) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let epoch_key = format!("session:user:{}:version", user_id);
        let token_key = format!("session:user:{}:token:{}", user_id, token);

        let current_epoch: i64 = redis::cmd("GET")
            .arg(&epoch_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        redis::cmd("SET")
            .arg(&token_key)
            .arg(current_epoch)
            .arg("EX")
            .arg(expires_sec)
            .query_async::<_, ()>(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao salvar sessão: {}", e)))?;

        Ok(())
    }

    pub async fn validate_session(&self, user_id: &str, token: &str) -> Result<bool, AppError> {
        let mut conn = self.get_conn().await?;
        let epoch_key = format!("session:user:{}:version", user_id);
        let token_key = format!("session:user:{}:token:{}", user_id, token);

        let result: Vec<Option<i64>> = redis::cmd("MGET")
            .arg(&token_key)
            .arg(&epoch_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(vec![None, None]);

        let token_version = match result.first().unwrap_or(&None) {
            Some(v) => *v,
            None => return Ok(false),
        };

        let current_version = result.get(1).unwrap_or(&None).unwrap_or(0);

        Ok(token_version == current_version)
    }

    pub async fn invalidate_user_sessions(&self, user_id: &str) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let epoch_key = format!("session:user:{}:version", user_id);

        #[cfg(test)]
        let res = if user_id.contains("FORCE_DEL_ERROR") {
            Err(redis::RedisError::from((
                redis::ErrorKind::ResponseError,
                "Forced DEL error",
            )))
        } else {
            redis::cmd("INCR")
                .arg(&epoch_key)
                .query_async::<_, ()>(&mut conn)
                .await
        };
        #[cfg(not(test))]
        let res = redis::cmd("INCR")
            .arg(&epoch_key)
            .query_async::<_, ()>(&mut conn)
            .await;

        let _: () =
            res.map_err(|e| AppError::Internal(format!("Erro ao expirar sessões antigas: {}", e)))?;

        Ok(())
    }

    pub async fn delete_session(&self, user_id: &str, token: &str) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let token_key = format!("session:user:{}:token:{}", user_id, token);
        let _: () = redis::cmd("DEL")
            .arg(&token_key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao deletar sessão: {}", e)))?;
        Ok(())
    }

    pub async fn delete_key(&self, key: &str) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let _: () = redis::cmd("DEL")
            .arg(key)
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro ao deletar chave: {}", e)))?;
        Ok(())
    }

    pub async fn check_rate_limit(
        &self,
        rate_key: &str,
        limit: i64,
        window_sec: i64,
    ) -> Result<(bool, i64, i64), AppError> {
        let mut conn = self.get_conn().await?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        let clear_before = now - (window_sec * 1000);
        let redis_key = format!("ratelimit:{}", rate_key);

        let script = redis::Script::new(
            r#"
            local key = KEYS[1]
            local now = tonumber(ARGV[1])
            local clear_before = tonumber(ARGV[2])
            local limit = tonumber(ARGV[3])
            local window_sec = tonumber(ARGV[4])
            
            redis.call('ZREMRANGEBYSCORE', key, '-inf', clear_before)
            local count = tonumber(redis.call('ZCARD', key) or "0")
            
            if count >= limit then
                return {0, count}
            end
            
            redis.call('ZADD', key, now, now)
            redis.call('EXPIRE', key, window_sec)
            return {1, count + 1}
            "#,
        );

        let result: Vec<i64> = script
            .key(&redis_key)
            .arg(now)
            .arg(clear_before)
            .arg(limit)
            .arg(window_sec)
            .invoke_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Erro no Rate Limiter: {}", e)))?;

        let allowed = result.first().copied().unwrap_or(0) == 1;
        let count = result.get(1).copied().unwrap_or(limit);
        let remaining = if allowed { limit - count } else { 0 };

        Ok((allowed, remaining.max(0), limit))
    }

    pub async fn key_exists(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self.get_conn().await?;
        let exists: bool = redis::cmd("EXISTS")
            .arg(key)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        Ok(exists)
    }

    pub async fn is_set_member(&self, key: &str, member: &str) -> Result<bool, AppError> {
        let mut conn = self.get_conn().await?;
        let is_member: bool = redis::cmd("SISMEMBER")
            .arg(key)
            .arg(member)
            .query_async(&mut conn)
            .await
            .unwrap_or(false);
        Ok(is_member)
    }

    pub async fn add_to_set(
        &self,
        key: &str,
        members: &[String],
        expires_sec: i64,
    ) -> Result<(), AppError> {
        let mut conn = self.get_conn().await?;
        let mut pipe = redis::pipe();

        let mut sadd = redis::cmd("SADD");
        sadd.arg(key);
        for m in members {
            sadd.arg(m);
        }

        pipe.add_command(sadd);
        pipe.cmd("EXPIRE").arg(key).arg(expires_sec);

        pipe.query_async::<_, ()>(&mut conn).await.map_err(|e| {
            AppError::Internal(format!("Erro ao salvar permissões no Redis: {}", e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    #[tokio::test]
    async fn test_rate_limit_exceeded() {
        let redis_container = Redis::default().start().await.unwrap();
        let host = redis_container.get_host().await.unwrap();
        let port = redis_container.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://{}:{}", host, port);
        let cache = Cache::new(&redis_url);

        let key = format!("test_rate_limit_exceeded_key_{}", uuid::Uuid::new_v4());

        let (allowed1, remaining1, limit1) = cache.check_rate_limit(&key, 1, 10).await.unwrap();
        assert!(allowed1);
        assert_eq!(remaining1, 0);
        assert_eq!(limit1, 1);

        let (allowed2, remaining2, limit2) = cache.check_rate_limit(&key, 1, 10).await.unwrap();
        assert!(!allowed2);
        assert_eq!(remaining2, 0);
        assert_eq!(limit2, 1);
    }

    #[tokio::test]
    async fn test_invalidate_user_sessions_error() {
        let dead_cache = Cache::new("redis://127.0.0.1:9999");
        let user_id = format!("test-err-{}", uuid::Uuid::new_v4());
        let res = dead_cache.invalidate_user_sessions(&user_id).await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_invalidate_user_sessions_del_error() {
        let redis_container = Redis::default().start().await.unwrap();
        let host = redis_container.get_host().await.unwrap();
        let port = redis_container.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://{}:{}", host, port);
        let cache = Cache::new(&redis_url);

        let user_id = format!("test-del-err-FORCE_DEL_ERROR-{}", uuid::Uuid::new_v4());
        cache
            .create_session(&user_id, "token123", 10)
            .await
            .unwrap();

        let res = cache.invalidate_user_sessions(&user_id).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .message()
            .contains("Erro ao expirar sessões antigas"));
    }

    #[tokio::test]
    async fn test_cache_set_methods() {
        let redis_container = Redis::default().start().await.unwrap();
        let host = redis_container.get_host().await.unwrap();
        let port = redis_container.get_host_port_ipv4(6379).await.unwrap();
        let redis_url = format!("redis://{}:{}", host, port);
        let cache = Cache::new(&redis_url);

        let key = format!("test_set_methods_key_{}", uuid::Uuid::new_v4());

        let exists = cache.key_exists(&key).await.unwrap();
        assert!(!exists);

        cache
            .add_to_set(&key, &[String::from("perm1"), String::from("perm2")], 60)
            .await
            .unwrap();

        let exists = cache.key_exists(&key).await.unwrap();
        assert!(exists);

        let is_m1 = cache.is_set_member(&key, "perm1").await.unwrap();
        let is_m2 = cache.is_set_member(&key, "perm2").await.unwrap();
        let is_m3 = cache.is_set_member(&key, "perm3").await.unwrap();

        assert!(is_m1);
        assert!(is_m2);
        assert!(!is_m3);

        let res_err = cache.add_to_set(&key, &[], 60).await;
        assert!(res_err.is_err());
        assert!(res_err
            .unwrap_err()
            .message()
            .contains("Erro ao salvar permissões no Redis"));
    }
}
