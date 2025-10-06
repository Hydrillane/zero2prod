

use crate::helpers::spawn_app;
use actix_web::{HttpResponse,web};
use reqwest::Url;
use wiremock::{matchers::{method, path}, Mock, ResponseTemplate};

#[derive(serde::Deserialize)]
pub struct Parameters {
    subscription_token: String,
}

#[tracing::instrument(
    name = " Confirm a pending subscriber",
    skip(_parameters)
)]
pub async fn confirm(_parameters:web::Query<Parameters>) -> HttpResponse {
    HttpResponse::Ok().finish()
}


#[tokio::test]
async fn confirmations_without_token_are_rejeceted_with_a_400(){
    let app = spawn_app().await;
    let response = reqwest::get(&format!("{}/subscriptions/confirm",app.address))
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(),400);
}

#[tokio::test]
async fn the_link_treturned_by_subscriber_returns_a_200_if_called() {
    let app = spawn_app().await;
    let body = "name=billy%20bongso&email=billybongso2001%40gmail.com";

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

    let get_link = |s:&str| {
        let links: Vec<_> = linkify::LinkFinder::new()
            .links(s)
            .filter(|l| *l.kind() == linkify::LinkKind::Url)
            .collect();
        assert_eq!(links.len(),1);
        links[0].as_str().to_owned()
    };

    let raw_confirmation_link = &get_link(&body["HtmlBody"].as_str().unwrap());
    let confirmation_link = Url::parse(&raw_confirmation_link).unwrap();

    assert_eq!(confirmation_link.host_str().unwrap(),"127.0.0.1");

    let response = reqwest::get(confirmation_link)
        .await
        .unwrap();

    assert_eq!(response.status().as_u16(),200);
}
