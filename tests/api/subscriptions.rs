use crate::helpers::{spawn_app, TestApp};
use reqwest::Client;
use wiremock::{matchers::{any, method, path}, Mock, ResponseTemplate};

#[tokio::test]
async fn subscribe_return_200_on_valid_form() {
    let test_app = spawn_app().await;
    let body = "name=billy%20bongso&email=billybongso2001%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&test_app.email_server)
        .await;

    let response = test_app.post_subscriptions(body.into()).await;

    assert_eq!(200,response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_the_new_subscriber() {
    let app = spawn_app().await;
    let body = "name=billy%20bongso&email=billybongso%40gmail.com";

    Mock::given(path("email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscriptions.");

    assert_eq!(saved.email,"billybongso@gmail.com");
    assert_eq!(saved.name,"billy bongso");
    assert_eq!(saved.status,"pending_confirmations");

}



#[tokio::test]
async fn subscribe_return_400_on_invalid_form() {

    let app = spawn_app().await;

    let body = vec![
        ("name=le%20guin", "missing email"),
        ("email=ursulaguin%40@gmail.com","missing name"),
        ("","missing both email and name")
    ];

    for (invalid_body,error_message) in body {
        let response = app.post_subscriptions(invalid_body.into()).await;

        assert_eq!(
            400,
            response.status().as_u16(),
            "{}",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=ursula_le_guin%40gmail.com", "empty name"),
        ("name=Ursula&email=", "empty email"),
        ("name=Ursula&email=definitely-not-an-email", "invalid email"),
    ];
    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body.into()).await;
        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 400 Bad Request when the payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_a_confirmation_email_with_a_link() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;


    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body : serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s: &str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(), 1);
        links[0].as_str().to_owned()
    };

    let html_link = get_link(&body["HtmlBody"].as_str().unwrap());
    let text_link = get_link(&body["TextBody"].as_str().unwrap());
    assert_eq!(html_link,text_link);
}


#[tokio::test]
async fn subscribe_fail_if_theres_database_error() {
    let app = spawn_app().await;
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    sqlx::query!(
        "ALTER TABLE subscription 
        DROP COLUMN email;"
    )
        .execute(&app.db_pool)
        .await
        .unwrap();
    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(),500);
}

#[tokio::test]
async fn newsletter_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;
    
    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title":"Newsletter title",
        "content":{
            "text":"Newsletter body as plain text",
            "html":"<p> Newsletter body as HTML</p>",
        }
    });

    let response = Client::new()
        .post(&format!("{}/newsletter",&app.address))
        .json(&newsletter_body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status().as_u16(),200);

}

async fn create_unconfirmed_subscriber(app:&TestApp) {
    let body = "name=billy%20bongso&email=billybongso2001%40gmail.com";

    let _mock_guard = Mock::given(path("email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();
}
