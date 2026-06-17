use super::connection::MessagingProvider;
use crate::config::AppConfig;
use lapin::{Connection, ConnectionProperties};
use std::sync::Arc;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::rabbitmq::RabbitMq;

async fn start_rabbitmq() -> (testcontainers::ContainerAsync<RabbitMq>, String) {
    let container = RabbitMq::default()
        .start()
        .await
        .expect("Failed to start RabbitMQ");
    let host = container.get_host().await.unwrap();
    let port = container.get_host_port_ipv4(5672).await.unwrap();
    let url = format!("amqp://{}:{}", host, port);
    (container, url)
}

static TEST_CONN: std::sync::Mutex<Option<Arc<Connection>>> = std::sync::Mutex::new(None);

struct CloseConnOnDeserialize;

impl<'de> serde::Deserialize<'de> for CloseConnOnDeserialize {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if let Ok(mut guard) = TEST_CONN.lock() {
            if let Some(conn) = guard.take() {
                tokio::spawn(async move {
                    let _ = conn.close(320, "closing").await;
                });
            }
        }
        Err(serde::de::Error::custom(
            "forced deserialize error to trigger ack failure",
        ))
    }
}

#[tokio::test]
async fn test_messaging_provider_disabled() {
    let provider = MessagingProvider {
        connection: None,
        channel: None,
        enabled: false,
    };

    let res = provider.publish("test_queue", &"hello").await;
    assert!(res.is_ok());

    let res = provider
        .subscribe("test_queue", |_msg: String| async {})
        .await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_messaging_provider_enabled_failure_paths() {
    let provider = MessagingProvider {
        connection: None,
        channel: None,
        enabled: true,
    };

    let res = provider.publish("test_queue", &"hello").await;
    assert!(res.is_err());

    let res = provider
        .subscribe("test_queue", |_msg: String| async {})
        .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_messaging_provider_connect_failure() {
    dotenvy::dotenv().ok();
    let mut config = AppConfig::load();
    config.messaging_enabled = true;
    config.rabbit_url = "amqp://127.0.0.1:9999".to_string();

    let res = MessagingProvider::init(&config).await;
    assert!(res.is_err());
}

#[tokio::test]
async fn test_messaging_provider_double_init() {
    dotenvy::dotenv().ok();
    let config = AppConfig::load();
    let res = MessagingProvider::init(&config).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_messaging_provider_get() {
    dotenvy::dotenv().ok();
    let mut config = AppConfig::load();
    config.messaging_enabled = false;
    let _ = MessagingProvider::init(&config).await;
    let _ = MessagingProvider::get();
}

#[tokio::test]
async fn test_messaging_provider_full_flow() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let conn = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let chan = conn.create_channel().await.unwrap();
    let provider = MessagingProvider {
        connection: Some(conn),
        channel: Some(chan),
        enabled: true,
    };

    let res = provider
        .publish("test_queue_unit", &"hello".to_string())
        .await;
    assert!(res.is_ok());

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    let res = provider
        .subscribe("test_queue_unit", move |msg: String| {
            let tx = tx.clone();
            async move {
                let _ = tx.send(msg).await;
            }
        })
        .await;
    assert!(res.is_ok());

    let received = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv()).await;
    assert!(received.is_ok());
    assert_eq!(received.unwrap(), Some("hello".to_string()));

    let res = provider.publish("test_queue_unit", &123).await;
    assert!(res.is_ok());

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let res = provider.disconnect().await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_messaging_provider_init_success() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let mut config = AppConfig::load();
    config.messaging_enabled = true;
    config.rabbit_url = rabbit_url;

    let res = MessagingProvider::init(&config).await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn test_messaging_ack_failures_and_errors() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let conn = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let chan = conn.create_channel().await.unwrap();
    let provider = MessagingProvider {
        connection: Some(conn),
        channel: Some(chan),
        enabled: true,
    };

    let conn_pub = Arc::new(
        Connection::connect(&rabbit_url, ConnectionProperties::default())
            .await
            .unwrap(),
    );
    let provider_pub = MessagingProvider {
        connection: Some(conn_pub.clone()),
        channel: Some(conn_pub.create_channel().await.unwrap()),
        enabled: true,
    };

    let (tx1, mut rx1) = tokio::sync::mpsc::channel(1);
    let p_clone1 = provider.clone();
    provider
        .subscribe("queue_ack_fail", move |msg: String| {
            let tx = tx1.clone();
            let p = p_clone1.clone();
            async move {
                let _ = tx.send(msg).await;
                let _ = p.disconnect().await;
            }
        })
        .await
        .unwrap();

    provider_pub
        .publish("queue_ack_fail", &"hello".to_string())
        .await
        .unwrap();
    let _ = rx1.recv().await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let conn2 = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn2 = Arc::new(conn2);
    let chan2 = conn2.create_channel().await.unwrap();
    let provider2 = MessagingProvider {
        connection: Some(conn2),
        channel: Some(chan2),
        enabled: true,
    };

    provider2
        .subscribe("queue_corrupt_fail", |_msg: String| async {})
        .await
        .unwrap();
    provider2.publish("queue_corrupt_fail", &123).await.unwrap();
    let _ = provider2.disconnect().await;
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
}

#[tokio::test]
async fn test_messaging_provider_channel_failure() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let mut config = AppConfig::load();
    config.messaging_enabled = true;

    let separator = if rabbit_url.contains('?') { "&" } else { "?" };
    config.rabbit_url = format!("{}{}FORCE_CHANNEL_ERR=true", rabbit_url, separator);

    let res = MessagingProvider::init(&config).await;
    assert!(res.is_err());
    assert!(res
        .unwrap_err()
        .message()
        .contains("Failed to create RabbitMQ channel"));
}

#[tokio::test]
async fn test_messaging_consumer_stream_error() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let conn = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let chan = conn.create_channel().await.unwrap();
    let provider = MessagingProvider {
        connection: Some(conn),
        channel: Some(chan.clone()),
        enabled: true,
    };

    let queue_name = format!("queue_stream_err_{}", uuid::Uuid::new_v4());
    provider
        .subscribe(&queue_name, |_msg: String| async {})
        .await
        .unwrap();

    let _res = chan
        .basic_publish(
            "non_existent_exchange_abc",
            &queue_name,
            lapin::options::BasicPublishOptions::default(),
            b"{}",
            lapin::BasicProperties::default(),
        )
        .await;

    tokio::time::sleep(std::time::Duration::from_millis(150)).await;
}

#[tokio::test]
async fn test_messaging_ack_failure_valid() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let conn = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let chan = conn.create_channel().await.unwrap();
    let provider = MessagingProvider {
        connection: Some(conn.clone()),
        channel: Some(chan),
        enabled: true,
    };

    let queue_name = format!("queue_ack_fail_valid_{}", uuid::Uuid::new_v4());
    let conn_clone = conn.clone();
    provider
        .subscribe(&queue_name, move |_msg: String| {
            let conn_c = conn_clone.clone();
            async move {
                tokio::spawn(async move {
                    let _ = conn_c.close(320, "closing").await;
                });
            }
        })
        .await
        .unwrap();

    provider
        .publish(&queue_name, &"valid_msg".to_string())
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
}

#[tokio::test]
async fn test_messaging_ack_corrupt_message_failure() {
    let (_container, rabbit_url) = start_rabbitmq().await;
    let conn = Connection::connect(&rabbit_url, ConnectionProperties::default())
        .await
        .unwrap();
    let conn = Arc::new(conn);
    let chan = conn.create_channel().await.unwrap();
    let provider = MessagingProvider {
        connection: Some(conn.clone()),
        channel: Some(chan),
        enabled: true,
    };

    *TEST_CONN.lock().unwrap() = Some(conn.clone());

    let queue_name = format!("queue_ack_corrupt_fail_{}", uuid::Uuid::new_v4());
    provider
        .subscribe(&queue_name, |_msg: CloseConnOnDeserialize| async {})
        .await
        .unwrap();

    provider
        .publish(&queue_name, &"dummy_msg".to_string())
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
}
