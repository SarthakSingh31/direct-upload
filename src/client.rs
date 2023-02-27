use jwt_compact::{
    alg::{Rsa, RsaPrivateKey},
    prelude::*,
};
use rsa::pkcs8::DecodePrivateKey;
use worker::{Env, Request, Response};

#[derive(Debug, Clone)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub expiry: Option<u64>,
}

pub struct Client {
    private_key: String,
    service_email_id: String,

    token: Option<Token>,
}

impl Client {
    const TOKEN_URL: &'static str = "https://oauth2.googleapis.com/token";

    pub fn new(env: &Env) -> worker::Result<Self> {
        Ok(Client {
            private_key: env.secret("GCP_PRIVATE_KEY")?.to_string(),
            service_email_id: env.secret("GCP_SERVICE_EMAIL_ID")?.to_string(),
            token: None,
        })
    }

    async fn get_token(&mut self) -> worker::Result<Token> {
        if let Some(token) = self.token.clone() {
            if let Some(expiry) = token.expiry {
                if expiry < worker::Date::now().as_millis() {
                    self.token = None;
                } else {
                    return Ok(token);
                }
            }
        }

        let key = RsaPrivateKey::from_pkcs8_pem(&self.private_key).unwrap();
        let header = Header::default().with_token_type("jwt");

        #[derive(serde::Serialize, serde::Deserialize)]
        struct CustomClaims {
            iss: String,
            scope: String,
            aud: String,
        }
        let claims = CustomClaims {
            iss: self.service_email_id.clone(),
            scope: "https://www.googleapis.com/auth/devstorage.full_control".into(),
            aud: "https://oauth2.googleapis.com/token".into(),
        };
        let claims = Claims::new(claims)
            .set_duration_and_issuance(&TimeOptions::default(), chrono::Duration::hours(1));

        let token = Rsa::rs256()
            .token(header, &claims, &key)
            .expect("Failed to create jwt");

        let mut url = url::Url::parse(Self::TOKEN_URL).expect("Failed to parse google token url");
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer");
            pairs.append_pair("assertion", token.as_str());
        }

        let mut headers = worker::Headers::default();
        headers.append("Content-Type", "application/x-www-form-urlencoded")?;

        let req = Request::new_with_init(
            url.as_str(),
            &worker::RequestInit {
                body: None,
                headers,
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
                let now = worker::Date::now();

                Token {
                    access_token: internal.access_token,
                    token_type: internal.token_type,
                    expiry: internal
                        .expires_in
                        .map(|expires_in| now.as_millis() + expires_in * 1000),
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
