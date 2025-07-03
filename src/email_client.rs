use std::time::Duration;

use secrecy::{ExposeSecret, SecretString};
use serde::Serialize;

use crate::domain::SubscriberEmail;

#[derive(Debug)]
pub struct EmailClient {
    sender: SubscriberEmail,
    base_url: String,
    http_client: reqwest::Client,
    server_api_token: SecretString,
    timeout: Duration,
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    html_body: &'a str,
    text_body: &'a str,
}

impl EmailClient {
    pub fn new(
        base_url: String,
        sender: SubscriberEmail,
        server_api_token: SecretString,
        timeout: Duration,
    ) -> EmailClient {
        EmailClient {
            sender,
            base_url,
            http_client: reqwest::Client::new(),
            server_api_token,
            timeout,
        }
    }
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = format!("{}/email", self.base_url);
        let body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            html_body: html_content,
            text_body: text_content,
        };
        self.http_client
            .post(url)
            .json(&body)
            .header(
                "X-Postmark-Server-Token",
                self.server_api_token.expose_secret(),
            )
            .timeout(self.timeout)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, time::Duration};

    use assertor::*;
    use fake::{
        Fake,
        faker::{
            internet::en::{Password, SafeEmail},
            lorem::en::{Paragraph, Sentence},
        },
    };
    use secrecy::SecretString;
    use wiremock::{Match, Mock, MockServer, ResponseTemplate, matchers};

    use crate::{domain::SubscriberEmail, email_client::EmailClient};

    struct SendEmailBodyMatcher;
    impl Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            match request.body_json::<HashMap<String, String>>() {
                Ok(body) => {
                    body.contains_key("From")
                        && body.contains_key("To")
                        && body.contains_key("Subject")
                        && body.contains_key("HtmlBody")
                        && body.contains_key("TextBody")
                }
                _ => false,
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }
    fn content() -> String {
        Paragraph(1..10).fake()
    }
    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).expect("cannot parse email")
    }
    fn email_client(base_url: String) -> EmailClient {
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        EmailClient::new(
            base_url,
            sender,
            SecretString::from(Password(0..10).fake::<String>()),
            Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::header_exists("X-Postmark-Server-Token"))
            .and(matchers::header("Content-Type", "application/json"))
            .and(matchers::path("/email"))
            .and(matchers::method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_that!(resp).is_ok();
    }

    #[tokio::test]
    async fn send_email_succeeds_if_the_server_return_200() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_that!(resp).is_ok();
    }

    #[tokio::test]
    async fn send_email_fails_if_the_server_return_500() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_that!(resp).is_err();
    }

    #[tokio::test]
    async fn send_email_times_out_if_the_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let email_client = email_client(mock_server.uri());

        Mock::given(matchers::any())
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(1000)))
            .expect(1)
            .mount(&mock_server)
            .await;

        let resp = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_that!(resp).is_err();
    }
}
