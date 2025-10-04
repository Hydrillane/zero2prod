
use fake::{faker::{internet::ar_sa::SafeEmail, lorem::{ar_sa::Sentence, zh_cn::Paragraph}}, Fake, Faker};
use reqwest::Client;
use secrecy::{ExposeSecret, SecretBox, SecretString};
use serde::Serialize;
use wiremock::MockServer;

use crate::domain::SubscriberEmail;

pub struct EmailClient {
    http_client:Client,
    base_url:String,
    sender:SubscriberEmail,
    authorization_token: SecretString,
}

impl EmailClient {
    pub fn new(base_url:String,
        sender:SubscriberEmail,
        authorization_token:SecretString,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();
        Self {
            http_client:http_client,
            base_url,
            authorization_token,
            sender
        }
    }

    pub async fn send_email(
        &self,
        receipent: SubscriberEmail,
        subject: &str,
        html_content:&str,
        text_content:&str
    ) -> Result<(),reqwest::Error> {

        let url = format!("{}/email",self.base_url);
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: receipent.as_ref(),
            subject: subject,
            html_body: html_content,
            text_body:text_content,
        };

        let builder = self
            .http_client
            .post(&url)
            .header("X-Postmark-Server-Token", 
                self.authorization_token.expose_secret()
            )
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())


    }
}

#[derive(Serialize)]
#[serde(rename_all= "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject:&'a str,
    html_body: &'a str,
    text_body:&'a str,
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use claim::assert_err;
    use fake::faker::lorem::ja_jp::Sentence;
    use fake::faker::lorem::pt_br::Paragraph;
    use secrecy::{SecretBox, SecretString};
    use wiremock::matchers::{any, header, header_exists, method, path};
    use wiremock::ResponseTemplate;
    use wiremock::{Mock, MockServer};

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::{Fake, Faker};

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &wiremock::Request) -> bool {
            let result: Result<serde_json::Value,_> = 
                serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                dbg!(&body);
                body.get("From").is_some() 
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }

        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;


        let _ = email_client(mock_server.uri())
            .send_email(email(),
            &subject(),
            &content(),
            &content()
            )
            .await;
    }


    #[tokio::test]
    async fn send_email_succeeds_if_the_server_returns_200() {
        let mock_server = MockServer::start().await;

        let response = ResponseTemplate::new(200).set_delay(Duration::from_secs(300));
        Mock::given(any())
            .respond_with(response)
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client(mock_server.uri())
            .send_email(
                email(),
                &subject(),
                &content(),
                &content()
            )
            .await;

        assert_err!(outcome);

    }
    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(mock_server_uri:String) -> EmailClient {
        EmailClient::new(
            mock_server_uri,
            email(),
            SecretString::new(Faker.fake::<String>().into_boxed_str()),
            Duration::from_millis(200))
    }


}


