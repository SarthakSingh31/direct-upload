mod client;

use std::collections::HashMap;

use url::Url;
use worker::{Env, Headers, Request, RequestInit, Response};

const BUCKET: &'static str = "test-bucket-the-first";

#[worker::event(fetch)]
async fn main(req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let router = worker::Router::new();

    router
        .get_async("/", |_req, _ctx| async {
            Response::from_html(include_str!("index.html"))
        })
        .get_async("/get_session_url", |req, ctx| async move {
            let mut client = client::Client::new(&ctx.env)?;

            let req_url = req.url()?;
            let query: HashMap<String, String> = req_url
                .query_pairs()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

            let url = Url::parse("https://storage.googleapis.com/")
                .expect("Failed to parse google storage url")
                .join(BUCKET)
                .expect("Failed to join bucket to url")
                .join(query[&"name".to_string()].as_ref())
                .expect("Failed to join file name");

            let mut headers = Headers::default();
            headers.append("Content-Length", "0")?;
            headers.append("Content-Type", query[&"content_type".to_string()].as_ref())?;
            headers.append("x-goog-resumable", "start")?;

            let init = RequestInit {
                body: None,
                headers,
                cf: worker::CfProperties::default(),
                method: worker::Method::Post,
                redirect: worker::RequestRedirect::default(),
            };

            client.request(url.as_str(), init).await
        })
        .run(req, env)
        .await
}
