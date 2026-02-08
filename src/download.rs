use std::time::{Duration, Instant};

use reqwest::Url;

use crate::config::SpeedLimit;

/// Stream-download from `url`, discarding all data. Applies speed throttling
/// if a speed limit is configured. Returns when the download completes or
/// on error.
pub async fn download_stream(
    client: &reqwest::Client,
    url: &Url,
    speed_limit: &Option<SpeedLimit>,
) -> anyhow::Result<()> {
    tracing::info!(%url, "starting download");

    let mut response = client.get(url.as_str()).send().await?;
    let status = response.status();
    if !status.is_success() {
        anyhow::bail!("HTTP {status} from {url}");
    }

    let mut total_bytes: u64 = 0;
    let mut window_start = Instant::now();
    let mut window_bytes: u64 = 0;

    loop {
        let chunk = match response.chunk().await? {
            Some(c) => c,
            None => break,
        };

        let len = chunk.len() as u64;
        total_bytes += len;
        window_bytes += len;

        if let Some(ref limit) = *speed_limit {
            throttle(limit, &mut window_start, &mut window_bytes).await?;
        }
    }

    tracing::info!(%url, total_bytes, "download finished");
    Ok(())
}

/// Sleep if we have exceeded the current speed limit within the window.
/// Resets the window every second so dynamic limits stay responsive.
async fn throttle(
    limit: &SpeedLimit,
    window_start: &mut Instant,
    window_bytes: &mut u64,
) -> anyhow::Result<()> {
    let limit_bps = limit.resolve()?;
    if limit_bps == 0 {
        // Speed 0 means "no downloading" – sleep and re-check until
        // the dynamic limit becomes non-zero.
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let new_limit = limit.resolve()?;
            if new_limit > 0 {
                *window_start = Instant::now();
                *window_bytes = 0;
                break;
            }
        }
        return Ok(());
    }

    let elapsed = window_start.elapsed();
    let target = Duration::from_secs_f64(*window_bytes as f64 / limit_bps as f64);

    if target > elapsed {
        tokio::time::sleep(target - elapsed).await;
    }

    // Reset window each second so dynamic limit changes take effect promptly.
    if window_start.elapsed() >= Duration::from_secs(1) {
        *window_start = Instant::now();
        *window_bytes = 0;
    }

    Ok(())
}
