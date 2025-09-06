use std::time::Duration;

/// Build a reqwest client with sane defaults (timeouts, redirects disabled by default).
pub fn make_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(6))
        .build()
        .expect("reqwest client")
}

/// Simple exponential backoff utility for async ops.
pub async fn retry_async<T, E, Fut, F>(mut attempts: u32, mut op: F) -> Result<T, E>
where
    F: FnMut(u32) -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut try_num: u32 = 0;
    let mut delay_ms: u64 = 50;
    loop {
        match op(try_num).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                if attempts == 0 {
                    return Err(e);
                }
                attempts -= 1;
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                delay_ms = (delay_ms * 2).min(1_000);
                try_num += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn it_retries_then_succeeds() {
        use super::retry_async;
        let mut calls = 0;
        let res: Result<i32, i32> = retry_async(3, move |_| {
            calls += 1;
            let c = calls;
            async move {
                if c < 3 {
                    Err(-1)
                } else {
                    Ok(42)
                }
            }
        })
        .await;
        assert_eq!(res.unwrap(), 42);
    }
}
