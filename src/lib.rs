mod client;

use std::collections::HashMap;

use url::Url;
use worker::{Env, Headers, Request, RequestInit, Response};

use client::Client;

const BUCKET: &'static str = "test-bucket-the-first/";

#[worker::event(fetch)]
async fn main(req: Request, env: Env, _ctx: worker::Context) -> worker::Result<Response> {
    let router = worker::Router::new();

    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    router
        // Returns the index page
        .get_async("/", |_req, _ctx| async {
            Response::from_html(include_str!("index.html"))
        })
        // Takes `name` and `content_type` as query parameters and returns a google storage resumable
        // upload session url
        .get_async("/get_session_url", |req, ctx| async move {
            let client = ctx
                .durable_object(Client::BINDING)?
                .id_from_name(Client::NAME)?
                .get_stub()?;

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

            let resp = client
                .fetch_with_request(Request::new_with_init(url.as_str(), &init)?)
                .await?;

            Response::ok(
                resp.headers()
                    .get("location")?
                    .expect("No location header was found"),
            )
        })
        .run(req, env)
        .await
}
