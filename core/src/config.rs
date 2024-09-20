use std::{ops::Deref, path::PathBuf, time::Duration};

use tokio::{
    fs::create_dir_all,
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
            sleep(Duration::new(0, 100)).await;
        };
        drop(permit);
        Ok(res)
    }
}

pub async fn client() -> &'static ClientWithMiddleware {
    static ONCE: OnceCell<ClientWithMiddleware> = OnceCell::const_new();

    (ONCE
        .get_or_init(|| async {
            let reqwest_client = Client::builder().build().unwrap();

            ClientBuilder::new(reqwest_client)
                .with(RateLimittingMiddleware)
                .build()
        })
        .await) as _
}

#[derive(serde::Deserialize)]
pub struct Settings {
    pub tesseract_path: String,
    pub cache_path: PathBuf,
}

pub async fn settings() -> &'static Settings {
    static ONCE: OnceCell<Settings> = OnceCell::const_new();

    let result = ONCE
        .get_or_init(|| async {
            config::Config::builder()
                .add_source(config::Environment::default())
                .build()
                .unwrap()
                .try_deserialize()
                .unwrap()
        })
        .await;
    create_dir_all(result.cache_path.deref()).await.unwrap();

    result
}
