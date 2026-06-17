use crate::errors::AppError;
use std::sync::atomic::{AtomicI64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct PdfProvider;

static FAILURES: AtomicUsize = AtomicUsize::new(0);
static OPEN_UNTIL: AtomicI64 = AtomicI64::new(0);

const MAX_FAILURES: usize = 3;
const OPEN_DURATION_SEC: i64 = 10;

impl PdfProvider {
    pub async fn generate_pdf(
        pdf_service_url: &str,
        template: &str,
        data: serde_json::Value,
    ) -> Result<Vec<u8>, AppError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let open_until = OPEN_UNTIL.load(Ordering::Relaxed);

        if open_until > now {
            tracing::warn!("Circuit Breaker OPEN. Ativando fallback local.");
            return Ok(Self::get_fallback_pdf_bytes());
        }

        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "template": template,
            "data": data,
        });

        let url = format!("{}/v1/pdf/generate", pdf_service_url);
        let resp = client.post(&url).json(&payload).send().await;

        match resp {
            Ok(res) if res.status().is_success() => {
                FAILURES.store(0, Ordering::Relaxed);
                OPEN_UNTIL.store(0, Ordering::Relaxed);

                let bytes = res
                    .bytes()
                    .await
                    .map_err(|e| AppError::Internal(format!("Falha ao ler bytes do PDF: {}", e)))?;
                Ok(bytes.to_vec())
            }
            Ok(res) => {
                Self::record_failure(now);
                let status = res.status();
                let err_body = res.text().await.unwrap_or_default();
                tracing::warn!(
                    "Erro ao gerar PDF no serviço (Status: {}). Resposta: {}. Ativando fallback local.",
                    status,
                    err_body
                );
                Ok(Self::get_fallback_pdf_bytes())
            }
            Err(e) => {
                Self::record_failure(now);
                tracing::warn!(
                    "Falha ao conectar ao serviço de PDF ({}). Ativando fallback local.",
                    e
                );
                Ok(Self::get_fallback_pdf_bytes())
            }
        }
    }

    fn record_failure(now: i64) {
        let failures = FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
        if failures >= MAX_FAILURES {
            OPEN_UNTIL.store(now + OPEN_DURATION_SEC, Ordering::Relaxed);
            tracing::error!(
                "Serviço PDF falhou {} vezes consecutivas. Circuit Breaker ABERTO por {} segundos.",
                failures,
                OPEN_DURATION_SEC
            );
            FAILURES.store(0, Ordering::Relaxed);
        }
    }

    pub fn get_fallback_pdf_bytes() -> Vec<u8> {
        let mock_pdf = "%PDF-1.4\n\
1 0 obj\n\
<< /Type /Catalog /Pages 2 0 R >>\n\
endobj\n\
2 0 obj\n\
<< /Type /Pages /Kids [3 0 R] /Count 1 >>\n\
endobj\n\
3 0 obj\n\
<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << >> /Contents 4 0 R >>\n\
endobj\n\
4 0 obj\n\
<< /Length 51 >>\n\
stream\n\
BT\n\
/F1 12 Tf\n\
72 712 Td\n\
(Mock PDF Content) Tj\n\
ET\n\
endstream\n\
endobj\n\
xref\n\
0 5\n\
0000000000 65535 f \n\
0000000009 00000 n \n\
0000000056 00000 n \n\
0000000111 00000 n \n\
0000000212 00000 n \n\
trailer\n\
<< /Size 5 /Root 1 0 R >>\n\
startxref\n\
311\n\
%%EOF";
        mock_pdf.as_bytes().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    static PDF_TEST_MUTEX: once_cell::sync::Lazy<tokio::sync::Mutex<()>> =
        once_cell::sync::Lazy::new(|| tokio::sync::Mutex::new(()));

    async fn reset_cb() {
        FAILURES.store(0, Ordering::Relaxed);
        OPEN_UNTIL.store(0, Ordering::Relaxed);
    }

    async fn setup_mock_server(app: axum::Router) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{}", addr)
    }

    #[tokio::test]
    async fn test_generate_pdf_success() {
        use axum::{routing::post, Json, Router};

        let app = Router::new().route(
            "/v1/pdf/generate",
            post(|Json(payload): Json<serde_json::Value>| async move {
                assert_eq!(payload["template"], "test-template");
                "custom-pdf-bytes"
            }),
        );
        let url = setup_mock_server(app).await;
        let _guard = PDF_TEST_MUTEX.lock().await;
        reset_cb().await;
        let res = PdfProvider::generate_pdf(&url, "test-template", serde_json::json!({})).await;
        assert!(res.is_ok());
        let bytes = res.unwrap();
        assert_eq!(bytes, b"custom-pdf-bytes");
    }

    #[tokio::test]
    async fn test_generate_pdf_server_error() {
        use axum::{http::StatusCode, routing::post, Router};

        let app = Router::new().route(
            "/v1/pdf/generate",
            post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "error message") }),
        );
        let url = setup_mock_server(app).await;
        let _guard = PDF_TEST_MUTEX.lock().await;
        reset_cb().await;
        let res = PdfProvider::generate_pdf(&url, "test-template", serde_json::json!({})).await;
        assert!(res.is_ok());
        let bytes = res.unwrap();
        assert_eq!(bytes, PdfProvider::get_fallback_pdf_bytes());
    }

    #[tokio::test]
    async fn test_generate_pdf_connection_failure() {
        let url = "http://127.0.0.1:1";
        let _guard = PDF_TEST_MUTEX.lock().await;
        reset_cb().await;
        let res = PdfProvider::generate_pdf(url, "test-template", serde_json::json!({})).await;
        assert!(res.is_ok());
        let bytes = res.unwrap();
        assert_eq!(bytes, PdfProvider::get_fallback_pdf_bytes());
    }

    #[tokio::test]
    async fn test_generate_pdf_byte_reading_failure() {
        use tokio::io::AsyncWriteExt;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buf = [0; 1024];
                let _ = tokio::io::AsyncReadExt::read(&mut socket, &mut buf).await;

                let response = "HTTP/1.1 200 OK\r\n\
                                Transfer-Encoding: chunked\r\n\
                                Content-Type: application/pdf\r\n\
                                \r\n\
                                5\r\n\
                                hello\r\n";
                let _ = socket.write_all(response.as_bytes()).await;
                let _ = socket.shutdown().await;
            }
        });

        let url = format!("http://{}", addr);
        let _guard = PDF_TEST_MUTEX.lock().await;
        reset_cb().await;
        let res = PdfProvider::generate_pdf(&url, "test-template", serde_json::json!({})).await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message().contains("Falha ao ler bytes do PDF"));
    }

    #[tokio::test]
    async fn test_generate_pdf_circuit_breaker() {
        let url = "http://127.0.0.1:2";
        let _guard = PDF_TEST_MUTEX.lock().await;
        reset_cb().await;

        for _ in 0..MAX_FAILURES {
            let res = PdfProvider::generate_pdf(url, "test-template", serde_json::json!({})).await;
            assert!(res.is_ok());
        }

        let res = PdfProvider::generate_pdf(url, "test-template", serde_json::json!({})).await;
        assert!(res.is_ok());

        assert!(OPEN_UNTIL.load(Ordering::Relaxed) > 0);
        reset_cb().await;
    }
}
