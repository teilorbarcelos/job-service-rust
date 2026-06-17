use crate::errors::AppError;
use async_trait::async_trait;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::env;

#[async_trait]
pub trait EmailService: Send + Sync + 'static {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError>;
}

pub struct MockEmailService;

#[async_trait]
impl EmailService for MockEmailService {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        tracing::info!(
            "📧 [EMAIL MOCK] Enviando e-mail...\nDestinatário: {}\nAssunto: {}\nCorpo: {}",
            to,
            subject,
            body
        );
        Ok(())
    }
}

pub struct SmtpEmailService {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from_address: String,
}

impl SmtpEmailService {
    pub fn new() -> Result<Self, AppError> {
        let smtp_host = env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
        let smtp_port = env::var("SMTP_PORT")
            .unwrap_or_else(|_| "1025".to_string())
            .parse::<u16>()
            .unwrap_or(1025);
        let smtp_user = env::var("SMTP_USER").ok();
        let smtp_pass = env::var("SMTP_PASS").ok();
        let from_address =
            env::var("SMTP_FROM").unwrap_or_else(|_| "no-reply@mage.com".to_string());

        let mut transport_builder = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)
            .map_err(|e| AppError::Internal(format!("Erro ao criar SMTP relay: {}", e)))?
            .port(smtp_port);

        if let (Some(user), Some(pass)) = (smtp_user, smtp_pass) {
            let credentials = Credentials::new(user, pass);
            transport_builder = transport_builder.credentials(credentials);
        }

        let transport = transport_builder.build();

        Ok(Self {
            transport,
            from_address,
        })
    }
}

#[async_trait]
impl EmailService for SmtpEmailService {
    async fn send_email(&self, to: &str, subject: &str, body: &str) -> Result<(), AppError> {
        let email = Message::builder()
            .from(self.from_address.parse().map_err(|_| {
                AppError::Internal("Formato de remetente SMTP inválido".to_string())
            })?)
            .to(to.parse().map_err(|_| {
                AppError::BadRequest("Formato de destinatário inválido".to_string())
            })?)
            .subject(subject)
            .body(body.to_string());

        #[cfg(test)]
        let email = if subject == "FORCE_CONSTRUCTION_ERROR" {
            Err(lettre::error::Error::MissingFrom)
        } else {
            email
        };

        let email =
            email.map_err(|e| AppError::Internal(format!("Falha ao construir e-mail: {}", e)))?;

        self.transport
            .send(email)
            .await
            .map_err(|e| AppError::Internal(format!("Falha no envio de e-mail SMTP: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_email() {
        let mock = MockEmailService;
        let res = mock.send_email("test@example.com", "subject", "body").await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn test_smtp_email_creation() {
        std::env::set_var("SMTP_USER", "user");
        std::env::set_var("SMTP_PASS", "pass");
        let service = SmtpEmailService::new();
        assert!(service.is_ok());
        std::env::remove_var("SMTP_USER");
        std::env::remove_var("SMTP_PASS");
    }

    #[tokio::test]
    async fn test_smtp_send_invalid_from() {
        let service = SmtpEmailService {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
                .unwrap()
                .build(),
            from_address: "invalid-email".to_string(),
        };
        let res = service
            .send_email("test@example.com", "subject", "body")
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_smtp_send_invalid_to() {
        let service = SmtpEmailService {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
                .unwrap()
                .build(),
            from_address: "test@example.com".to_string(),
        };
        let res = service.send_email("invalid-email", "subject", "body").await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_smtp_send_fail() {
        let service = SmtpEmailService {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay("127.0.0.1")
                .unwrap()
                .port(9999)
                .build(),
            from_address: "sender@example.com".to_string(),
        };
        let res = service
            .send_email("test@example.com", "subject", "body")
            .await;
        assert!(res.is_err());
    }

    #[tokio::test]
    async fn test_smtp_email_construction_error() {
        let service = SmtpEmailService {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay("localhost")
                .unwrap()
                .build(),
            from_address: "sender@example.com".to_string(),
        };
        let res = service
            .send_email("test@example.com", "FORCE_CONSTRUCTION_ERROR", "body")
            .await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .message()
            .contains("Falha ao construir e-mail:"));
    }
}
