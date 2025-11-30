use std::time::Duration;

use crate::helpers::{assert_is_redirect_to, spawn_app, ConfirmationLinks, TestApp};
use fake::{faker::{internet::pt_pt::SafeEmail, name::cy_gb::Name}, Fake};
use uuid::Uuid;
use wiremock::{matchers::{any, method, path}, Mock, ResponseTemplate};



#[tokio::test]
async fn newsletter_are_delivered_to_confirmed_subscriber() {
    let app = spawn_app().await; 
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/newsletter"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_body = serde_json::json!({
        "title":"Newsletter Title",
        "html_content":"<p>WELCOME TO NEWSLETTER</p>",
        "text_content":"WELKOMENN TO NEWSLETTERFUGR",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password,
    });

    let login_resp = app.post_login(&auth).await;
    assert_is_redirect_to(&login_resp, "/admin/dashboard");

    let response = app.post_newsletter(&newsletter_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");

}

#[tokio::test]
async fn newsletter_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    
    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password
    });

    let login_response = app.post_login(&auth).await;

    assert_is_redirect_to(&login_response, "/admin/dashboard");

    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "Newsletter body as plain text",
                    "html": "<p>Newsletter body as HTML</p>",
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({"title": "Newsletter!"}),
            "missing content",
        ),
    ];

    for (invalid_body,error_message) in test_cases {
        let response = app.post_newsletter(&invalid_body).await;
            assert_eq!(400,response.status().as_u16(),
            "The API didnt fail with 400 Bad Request when the payload was {}",
            error_message);

    }
}

// #[tokio::test]
// async fn newsletter_returns_500_for_failed_get_confirmed() {
//     let app = spawn_app().await;
//     let unconfirmed = create_unconfirmed_subscriber(&app).await;
//     let result_pub_api = get_confirmed_subscriber(&app.db_pool).await;
//     assert!(result_pub_api.is_ok());
// }

// #[tokio::test]
// async fn request_missing_authorization_are_jerected(){
//     let app = spawn_app().await;
//
//     let body = serde_json::json!({
//         "title":"Newsletter title",
//             "content":{
//                 "text":"Newsletter body as plain text",
//                 "html":"<p> Newsletter as HTML </p>"
//             }
//         });
//
//     let response = app.post_newsletter(body).await;
//
//         assert_eq!(401,response.status().as_u16());
//     assert_eq!(r#"Basic realm="publish""#,response.headers());
//
// }

async fn create_unconfirmed_subscriber(app:&TestApp) -> ConfirmationLinks {
    let name :String = Name().fake();
    let email:String = SafeEmail().fake();

    let body = serde_urlencoded::to_string(&serde_json::json!({
        "name":name,
        "email":email
    })).unwrap();

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .expect("Failed to post to subscriptions endpoint");

    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmations_link(email_request)

}

async fn create_confirmed_subscriber(app:&TestApp) {
    let confirmation_links = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_links.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}


#[tokio::test]
async fn non_existence_user_is_rejected() {

    let app = spawn_app().await;


    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":Uuid::new_v4().to_string()
    });

    let login_response = app.post_login(&auth).await;

    assert_is_redirect_to(&login_response, "/login");

    let body = serde_json::json!({
        "title":"Newsletter Title",
        "html_content":"<p> Newsletter body as HTML </p>",
        "text_content":"Newsletter body as text",
        "idempotency_key":Uuid::new_v4().to_string()
    });
    let newsletter = app.post_newsletter(&body).await;
    assert_eq!(newsletter.status().as_u16(),303);
}

#[tokio::test]
async fn succesfull_send_newsletter() {
    let app = spawn_app().await;

    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password
    });

    let login_resp = app.post_login(&auth).await;
    assert_is_redirect_to(&login_resp, "/admin/dashboard");

    let body = serde_json::json!({
        "title":"Newsletter Title",
        "html_content":"<p> Newsletter body as HTML </p>",
        "text_content":"Newsletter body as text",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    let newsletter_resp = app.post_newsletter(&body).await;
    assert_is_redirect_to(&newsletter_resp, "/admin/newsletter");
    assert!(app.get_newsletter_html().await.contains("<p><i>Succesfully sent email to"));
}

#[tokio::test]
async fn newsletter_creation_is_idempotent() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password,
    });

    let login_resp = app.post_login(&auth).await;
    assert_is_redirect_to(&login_resp, "/admin/dashboard");

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title":"Newsletter Title",
        "html_content":"<p> Newsletter body as HTML </p>",
        "text_content":"Newsletter body as text",
        "idempotency_key":Uuid::new_v4().to_string()
    });
    let repsonse = app.post_newsletter(&newsletter_request_body).await;
    assert_is_redirect_to(&repsonse, "/admin/newsletter");

    let html_page = app.get_newsletter_html().await;
    assert!(html_page.contains("<p><i>Succesfully sent email to"));

    let response = app.post_newsletter(&newsletter_request_body).await;
    // assert_eq!(response.status().as_u16(),500);
    app.dispatch_all_pending_email().await;
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
        "title":"Newsletter Title",
        "html_content":"<p>WELCOME TO NEWSLETTER</p>",
        "text_content":"WELKOMENN TO NEWSLETTERFUGR",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password,
    });

    let login_resp = app.post_login(&auth).await;
    assert_is_redirect_to(&login_resp, "/admin/dashboard");
    let response = app.post_newsletter(&newsletter_body).await;
    assert_is_redirect_to(&response, "/admin/newsletter");
    app.dispatch_all_pending_email().await;
}

#[tokio::test]
async fn concurrent_form_submission_is_handled_gracefully() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(2)))
        .expect(2)
        .mount(&app.email_server)
        .await;

    let newsletter_request_body = serde_json::json!({
        "title": "Newsletter title",
        "text_content": "Newsletter body as plain text",
        "html_content": "<p>Newsletter body as HTML</p>",
        "idempotency_key": uuid::Uuid::new_v4().to_string()
    });
    let response1 = app.post_newsletter(&newsletter_request_body);
    let response2 = app.post_newsletter(&newsletter_request_body);
    let (response1, response2) = tokio::join!(response1, response2);
    assert_eq!(response1.status(), response2.status());
    assert_eq!(response1.text().await.unwrap(), response2.text().await.unwrap());
    app.dispatch_all_pending_email().await;
}

#[tokio::test]
async fn transient_error_do_not_cause_duplicate_on_delivery() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;
    create_confirmed_subscriber(&app).await;
    app.test_user.login(&app).await;

    let body = serde_json::json!({
        "title":"Newsletter Title",
        "html_content":"<p> HTML Content of the newsletter </p>",
        "text_content":"Hello greetings to you all",
        "idempotency_key":Uuid::new_v4().to_string()
    });

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;
    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(1)
        .expect(1)
        .mount(&app.email_server)
        .await;
    let response = app.post_newsletter(&body).await;
    assert_eq!(response.status().as_u16(),500);

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Delivery retry")
        .expect(1)
        .mount(&app.email_server)
        .await;

    let response = app.post_newsletter(&body).await;
    assert_eq!(response.status().as_u16(),303);
    app.dispatch_all_pending_email().await;
}
