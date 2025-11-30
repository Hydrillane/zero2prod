
use std::collections::HashSet;

use serde_json::json;

use crate::helpers::{assert_is_redirect_to, spawn_app};


#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    let app = spawn_app().await;

    let body = serde_json::json!({
        "username" : "test-username",
        "password" : "test-password"
    });


    let response = app.post_login(&body).await;

    let html = app.get_html().await;

    assert!(html.contains(r#"<p><i>Authentication Failed</i></p>"#));
    assert!(response.cookies().find(|c| c.name() == "_flash").is_some());
    assert_is_redirect_to(&response, "/login");

    assert_eq!(response.status().as_u16(),303);

}

#[tokio::test]
async fn redirect_to_admin_dashboard_after_login_success() {
    let app = spawn_app().await;

    let auth = json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password
    });

    let response = app.post_login(&auth).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}",app.test_user.username)));
}

#[tokio::test]
async fn need_authorization_for_admin_dashboard() {
    let app = spawn_app().await;
    let response = app.get_admin_dashboard().await;
    assert_is_redirect_to(&response, "/login");
}
