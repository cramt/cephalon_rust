use std::time::Duration;

use tokio::{
    sync::{OnceCell, Semaphore},
    time::sleep,
};

use http::{Extensions, StatusCode};
use reqwest::{Client, Request, Response};
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware, Middleware, Next, Result};

static PERMITS: Semaphore = Semaphore::const_new(10);

struct RateLimittingMiddleware;

#[async_trait::async_trait]
impl Middleware for RateLimittingMiddleware {
    async fn handle(
        &self,
        req: Request,
        extensions: &mut Extensions,
        next: Next<'_>,
    ) -> Result<Response> {
        let permit = PERMITS.acquire().await.unwrap();
        let res = loop {
            let res = next
                .clone()
                .run(req.try_clone().unwrap(), extensions)
                .await?;
            if res.status() != StatusCode::TOO_MANY_REQUESTS {
                break res;
            }
            sleep(Duration::new(1, 0)).await;
        };
        drop(permit);
        Ok(res)
    }
}

pub async fn client() -> &'static ClientWithMiddleware {
    static ONCE: OnceCell<ClientWithMiddleware> = OnceCell::const_new();
    let static_ref = ONCE
        .get_or_init(|| async {
            let reqwest_client = Client::builder().build().unwrap();
            let client = ClientBuilder::new(reqwest_client)
                .with(RateLimittingMiddleware)
                .build();
            client
        })
        .await;
    static_ref
}
