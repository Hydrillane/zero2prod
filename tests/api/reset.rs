use uuid::Uuid;

use crate::helpers::{spawn_app,assert_is_redirect_to};


#[tokio::test]
async fn not_authenticated_users_are_redirected() {
    let app = spawn_app().await;

    let response = app.get_reset_form().await;
    assert_is_redirect_to(&response, "/login");
}

#[tokio::test]
async fn reset_is_failed_if_form_not_valid() {
    let app = spawn_app().await;

    let auth = serde_json::json!({
        "username":&app.test_user.username,
        "password":&app.test_user.password
    });

    let res = app.post_login(&auth).await;
    assert_is_redirect_to(&res, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();

    let reset_body = serde_json::json!({
        "old_password":&app.test_user.password,
        "new_password":new_password,
        "confirm_new_password":Uuid::new_v4().to_string()
    });

    let response = app.post_to_reset(reset_body).await;
    assert_eq!(response.status().as_u16(),303);

    let response_html = app.get_reset_form_html().await;
}

#[tokio::test]
async fn reset_is_failed_if_old_password_doesnt_match() {
    let app = spawn_app().await;

    let auth = serde_json::json!({
        "username": &app.test_user.username,
        "password":&app.test_user.password
    });

    let response = app.post_login(&auth).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();

    let reset_body = serde_json::json!({
        "old_password":Uuid::new_v4().to_string(),
        "new_password":new_password,
        "confirm_new_password":Uuid::new_v4().to_string()
    });

    let response_reset = app.post_to_reset(reset_body).await;
    assert_eq!(response.status().as_u16(),303);
    assert!(response.cookies().find(|c| c.name() == "_flash").is_some());

    let response_html = app.get_reset_form_html().await;
    assert!(response_html.contains(r#"<p><i>Wrong Password!</i></p>"#));
}

#[tokio::test]
async fn reset_is_failed_if_password_not_strong() {
    let app = spawn_app().await;

    let auth = serde_json::json!({
        "username": &app.test_user.username,
        "password":&app.test_user.password
    });

    let response = app.post_login(&auth).await;
    assert_is_redirect_to(&response, "/admin/dashboard");

}

#[tokio::test]
async fn changing_password_works() {

    let app = spawn_app().await;
    
    let auth = serde_json::json!({
        "username": &app.test_user.username,
        "password":&app.test_user.password
    });

    // Login 
    let response_login = app.post_login(&auth).await;
    assert_is_redirect_to(&response_login, "/admin/dashboard");

    let new_password = Uuid::new_v4().to_string();

    let reset_auth = serde_json::json!({
        "old_password":&app.test_user.password,
        "new_password":&new_password,
        "confirm_new_password":&new_password
    });

    let response_reset = app.post_to_reset(&reset_auth).await;
    let reset_body = app.get_admin_dashboard_html().await;
    assert_is_redirect_to(&response_reset, "/admin/dashboard");
    assert!(reset_body.contains("<p><i>Your password has been changed</i></p>"));

    let response_logout = app.post_to_logout().await;
    let logout_body = app.get_login_form_html().await;
    assert_is_redirect_to(&response_logout, "/login");
    // assert!(logout_body.contains("<p></i>You have successfully logged out.</i></p>"));

    // Relogin with new password
    
    let new_auth = serde_json::json!({
        "username":app.test_user.username,
        "password":new_password
    });

    let new_login = app.post_login(&new_auth).await;
    assert_is_redirect_to(&new_login, "/admin/dashboard");

}
