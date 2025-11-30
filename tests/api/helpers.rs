


use argon2::{password_hash::SaltString, Argon2, Params, PasswordHasher};
use once_cell::sync::Lazy;
use reqwest::{redirect::{self, Policy}, Response, Url};
use sqlx::{Connection, PgConnection, PgPool,Executor};
use uuid::Uuid;
use wiremock::{MockServer, Request};
use zero2production::{configuration::{get_configuration, DatabaseSetting}, email_client::EmailClient, issue_delivery_work::{try_execute_task, ExecutionOutcome}, startup::get_connection_pool, telemetry::{get_subscriber, init_subscriber}};
use zero2production::startup::Application;
use argon2::password_hash::rand_core::OsRng;

static TRACING: Lazy<()> = Lazy::new(|| {
    let subscriber = get_subscriber("test".into(), "debug".into());
    init_subscriber(subscriber);
});


pub struct ConfirmationLinks {
    pub html:Url,
    pub plain_text: Url,
}

pub struct TestApp {
    pub address: String,
    pub port:u16,
    pub db_pool:PgPool,
    pub email_server: MockServer,
    pub test_user : TestUser,
    pub api_client: reqwest::Client,
    pub email_client: EmailClient
}

pub struct TestUser {
    user_id: Uuid,
    pub username: String,
    pub password: String
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: "aniesta123".into()
        }
    }

    pub async fn login(&self,app:&TestApp) {
        let auth = serde_json::json!({
            "username":&self.username,
            "password":&self.password,
        });

        app.post_login(&auth).await;

    }


    async fn store(&self, pool:&PgPool) {
        let salt = SaltString::generate(&mut OsRng);

        let hash_password = Argon2::new(
            argon2::Algorithm::Argon2id, 
            argon2::Version::V0x13, 
            Params::new(15000, 2, 1, None).unwrap())
            .hash_password(&self.password.as_bytes(), &salt)
            .unwrap()
            .to_string();
        // dbg!(&hash_password);

        sqlx::query!(
            r#"
            INSERT INTO users (user_id,username,hash_password)
            VALUES ($1, $2, $3)
            "#,
            self.user_id,
            self.username,
            hash_password
        )
            .execute(pool)
            .await
            .expect("Failed to store test user.");
    }
}

impl TestApp {
    pub async fn dispatch_all_pending_email(&self) {
        loop {
            if let ExecutionOutcome::EmptyQueue =
                try_execute_task(&self.db_pool, &self.email_client).await.unwrap() {
                    break;
            }
        }

    }

    pub async fn post_to_logout(&self) -> reqwest::Response {
        let response = self.api_client
            .post(format!("{}/admin/logout",self.address))
            .send()
            .await
            .expect(("Failed to post to logout"));
        response
    }


    pub async fn get_reset_form(&self) -> reqwest::Response {
        let response = self.api_client
            .get(format!("{}/admin/reset",self.address))
            .send()
            .await
            .expect("Failed to get reset form!");
        response
    }

    pub async fn get_reset_form_html(&self) -> String {
        let res = self.get_reset_form().await.text().await.unwrap();
        res
    }

    pub async fn get_login_form(&self) -> reqwest::Response {
        let res = self.api_client
            .get(format!("{}/login",self.address))
            .send()
            .await
            .expect("Failed to get login form");
        res
    }

    pub async fn get_login_form_html(&self) -> String {
        self.get_login_form().await.text().await.unwrap()
    }

    pub async fn post_to_reset<Body>(&self,body:Body) -> reqwest::Response 
    where
        Body: serde::Serialize
    {
        let response = self.api_client
            .post(format!("{}/admin/reset",&self.address))
            .form(&body)
            .send()
            .await
            .expect("Failed to post to /reset");

        response
    }

    pub async fn get_admin_dashboard(&self) -> reqwest::Response {
        let response = self.api_client
            .get(format!("{}/admin/dashboard",&self.address))
            .send()
            .await
            .expect("Failed to retrieve the admin dashboard");
        response
    }

    pub async fn get_admin_dashboard_html(&self) -> String {
        self.get_admin_dashboard().await.text().await.unwrap()
    }

    pub async fn post_subscriptions(&self,body:String) -> reqwest::Response {
        self.api_client
            .post(format!("{}/subscriptions",&self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute")
    }

    pub async fn get_html(&self) -> String {
        self.api_client
            .get(format!("{}/login",self.address))
            .send()
            .await
            .expect("Failed to extract html")
            .text()
            .await
            .unwrap()
    }


    pub async fn post_newsletter<Body>(&self,body:&Body) -> reqwest::Response
    where 
        Body: serde::Serialize
    {
        let response = self.api_client
            .post(format!("{}/admin/newsletter",self.address))
            .form(&body)
            .send()
            .await
            .expect("Failed to execute to newsletter");
        response
    }
    pub async fn get_newsletter(&self) -> reqwest::Response {
        let response = self.api_client
            .get(format!("{}/admin/newsletter",&self.address))
            .send()
            .await
            .expect("Failed to get newsletter");
        response
    }
    pub async fn get_newsletter_html(&self) -> String {
        self.get_newsletter()
            .await
            .text()
            .await
            .unwrap()
    }

    pub async fn post_login<Body>(&self,
        body:&Body) -> reqwest::Response
    where 
        Body: serde::Serialize {
            self.api_client
                .post(format!("{}/login",self.address))
                .form(body)
                .send()
                .await
                .expect("Failed to send post to /login on test")
    }

    pub fn get_confirmations_link(
        &self,
        email_request:&Request,
    ) -> ConfirmationLinks {
        let body : serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        let get_links = |s:&str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(),1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = Url::parse(&raw_link).unwrap();
            assert_eq!(confirmation_link.host_str().unwrap(),"127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_links(&body["HtmlBody"].as_str().unwrap());
        let text = get_links(&body["TextBody"].as_str().unwrap());

        ConfirmationLinks {
            html,
            plain_text:text
        }

    }

    pub async fn test_user(&self) -> (String,String) {
        let row = sqlx::query!(
            "SELECT username, hash_password FROM users LIMIT 1",
        )
            .fetch_one(&self.db_pool)
            .await
            .expect("Failed to create test users");
        (row.username,row.hash_password)
    }
}

pub async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;
    let configuration = {
        let mut c = get_configuration().expect("Failed to get Configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.email_client.base_url = email_server.uri();
        c.application.port = 0;
        c
    };


    configure_database(&configuration.database).await;

    let application = Application::build
        (configuration.clone())
        .await
        .expect("Failed to build app!");
    let port = application.port();

    let _ = tokio::spawn(application.run_untill_stopped());

    let api_client = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(Policy::none())
        .build()
        .unwrap();


    let test_app = TestApp {
        address:format!("http://127.0.0.1:{}",port),
        port,
        db_pool: get_connection_pool(&configuration.database),
        email_server,
        test_user: TestUser::generate(),
        api_client,
        email_client:configuration.email_client.client()
    };
    // add_test_users(&test_app.db_pool).await;
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}


async fn configure_database(config:&DatabaseSetting) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.connection_without_dbname())
        .await
        .expect("Failed to connect!");

    connection.execute(
        format!(r#"CREATE DATABASE "{}";"#,&config.database_name).as_str())
        .await
        .expect("Failed to create database!");

    let connection_pool = PgPool::connect_with(config.connection_string())
        .await
        .expect("Failed to Connect");
    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to do migrations");

    connection_pool
}

async fn add_test_users(pool:&PgPool) {
    sqlx::query!(
        r#"INSERT INTO users (user_id, username, hash_password)
        VALUES ($1, $2, $3)
        "#,
        Uuid::new_v4(),
        Uuid::new_v4().to_string(),
        Uuid::new_v4().to_string()
    )
        .execute(pool)
        .await
        .expect("Failed to create new user in function add_test_users");
}

pub fn assert_is_redirect_to(response:&Response,location:&str) {
    assert_eq!(response.status().as_u16(),303);
    assert_eq!(response.headers()["LOCATION"],location);
}
