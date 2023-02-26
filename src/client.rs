use std::time;

use worker::{Env, Request, Response};

#[derive(Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub expiry: Option<time::Instant>,
}

pub struct Client {
    client_id: String,
    client_secret: String,
    refresh_token: String,

    token: Option<Token>,
}

impl Client {
    const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";

    pub fn new(env: &Env) -> worker::Result<Self> {
        Ok(Client {
            client_id: env.secret("GCP_CLIENT_ID")?.to_string(),
            client_secret: env.secret("GCP_CLIENT_SECRET")?.to_string(),
            refresh_token: env.secret("GCP_CLIENT_REFRESH_TOKEN")?.to_string(),
            token: None,
        })
    }

    async fn get_token(&mut self) -> worker::Result<Token> {
        if let Some(token) = self.token.clone() {
            if let Some(expiry) = token.expiry {
                if expiry < time::Instant::now() {
                    self.token = None;
                } else {
                    return Ok(token);
                }
            }
        }

        let req = Request::new_with_init(
            Self::TOKEN_URL,
            &worker::RequestInit {
                body: Some(
                    serde_json::json!({
                        "client_id": &self.client_id,
                        "client_secret": &self.client_secret,
                        "grant_type": "refresh_token",
                        "refresh_token": &self.refresh_token
                    })
                    .as_str()
                    .into(),
                ),
                headers: worker::Headers::default(),
                cf: worker::CfProperties::default(),
                method: worker::Method::Post,
                redirect: worker::RequestRedirect::default(),
            },
        )?;

        #[derive(Clone, serde::Deserialize)]
        struct InternalToken {
            pub access_token: String,
            pub token_type: String,
            pub expires_in: Option<u64>,
        }

        impl From<InternalToken> for Token {
            fn from(internal: InternalToken) -> Self {
                let now = time::Instant::now();

                Token {
                    access_token: internal.access_token,
                    token_type: internal.token_type,
                    expiry: internal
                        .expires_in
                        .map(|expires_in| now + time::Duration::from_secs(expires_in)),
                }
            }
        }

        let mut resp = worker::Fetch::Request(req).send().await?;
        let token: Token = resp.json::<InternalToken>().await?.into();
        self.token = Some(token.clone());

        Ok(token)
    }

    pub async fn request(
        &mut self,
        uri: &str,
        mut init: worker::RequestInit,
    ) -> worker::Result<Response> {
        let token = self.get_token().await?;

        init.headers.append(
            "Authorization",
            format!("Bearer {}", token.access_token).as_str(),
        )?;

        worker::Fetch::Request(Request::new_with_init(uri, &init)?)
            .send()
            .await
    }
}
