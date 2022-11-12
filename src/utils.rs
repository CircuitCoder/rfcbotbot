use std::future::Future;

use cfg_if::cfg_if;
use telegram_bot::{connector::Connector, Error, ErrorKind, HttpResponse, Method};

cfg_if! {
    // https://github.com/rustwasm/console_error_panic_hook#readme
    if #[cfg(feature = "console_error_panic_hook")] {
        extern crate console_error_panic_hook;
        pub use self::console_error_panic_hook::set_once as set_panic_hook;
    } else {
        #[inline]
        pub fn set_panic_hook() {}
    }
}

struct IKnowThisIsBadTMButWeAreSingleThreaded<O, F: Future<Output = O> + 'static>(F);
unsafe impl<O, F: Future<Output = O> + 'static> Send
    for IKnowThisIsBadTMButWeAreSingleThreaded<O, F>
{
}
impl<O, F: Future<Output = O> + 'static> Future for IKnowThisIsBadTMButWeAreSingleThreaded<O, F> {
    type Output = O;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        unsafe { self.map_unchecked_mut(|e| &mut e.0) }.poll(cx)
    }
}

#[derive(Debug)]
pub struct FetchConnector;

impl Connector for FetchConnector {
    fn request(
        &self,
        token: &str,
        req: telegram_bot::HttpRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<telegram_bot::HttpResponse, telegram_bot::Error>,
                > + Send,
        >,
    > {
        let url = req.url.url(token);
        let future = async move {
            let method = match req.method {
                Method::Get => worker::Method::Get,
                Method::Post => worker::Method::Post,
            };

            let mut headers = worker::Headers::new();

            let mut reqinit = worker::RequestInit::new();
            reqinit.with_method(method);

            let body = match req.body {
                telegram_bot::Body::Empty => None,
                telegram_bot::Body::Json(json) => {
                    headers.set("Content-Type", "application/json").unwrap();
                    Some(json.into())
                }
                body => panic!("Unsupported body type {:?}", body),
            };

            reqinit.with_body(body).with_headers(headers);

            worker::console_log!("Start sending...");

            let req = worker::Request::new_with_init(&url, &reqinit).map_err(|e| {
                worker::console_error!("Failed to construct request: {}", e);
                let boxed =
                    <anyhow::Error as Into<Box<dyn std::error::Error + Send + 'static>>>::into(
                        anyhow::anyhow!("Unable to construct request: {}", e),
                    );
                ErrorKind::from_generic_boxed(boxed)
            })?;
            let mut resp = worker::Fetch::Request(req).send().await.map_err(|e| {
                worker::console_error!("Failed to fetch: {}", e);
                let boxed =
                    <anyhow::Error as Into<Box<dyn std::error::Error + Send + 'static>>>::into(
                        anyhow::anyhow!("Unable to fetch: {}", e),
                    );
                ErrorKind::from_generic_boxed(boxed)
            })?;
            let body = resp.bytes().await.map_err(|e| {
                worker::console_error!("Failed to decode: {}", e);
                let boxed =
                    <anyhow::Error as Into<Box<dyn std::error::Error + Send + 'static>>>::into(
                        anyhow::anyhow!("Unable to parse: {}", e),
                    );
                ErrorKind::from_generic_boxed(boxed)
            })?;

            worker::console_log!("Got body: {}", String::from_utf8_lossy(&body));

            Ok::<HttpResponse, Error>(HttpResponse { body: Some(body) })
        };

        Box::pin(IKnowThisIsBadTMButWeAreSingleThreaded(future))
    }
}
