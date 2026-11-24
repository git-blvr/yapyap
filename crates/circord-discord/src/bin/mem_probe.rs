//! Temporary A/B measurement probe for the retained-memory work: starts the
//! Discord service against the fake server, opens a channel's member list,
//! and prints stage markers while it idles so its private bytes can be
//! sampled per stage. Not part of Circord.

use std::time::{Duration, Instant};

use anyhow::{Context, Result, ensure};
use circord_discord::{
    client,
    service::{Discord, Status},
    token,
};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let guild = args
        .first()
        .context("usage: mem_probe GUILD CHANNEL")?
        .clone();
    let channel = args
        .get(1)
        .context("usage: mem_probe GUILD CHANNEL")?
        .clone();
    client::test_server().context("set CIRCORD_DISCORD_TEST_SERVER")?;
    token::store(client::TEST_TOKEN)?;
    let discord = Discord::start(token::load()?)?;
    let mut snapshots = discord.snapshots();
    let deadline = Instant::now() + Duration::from_secs(120);
    while snapshots.borrow_and_update().status != Status::Online {
        ensure!(Instant::now() < deadline, "never went online");
        snapshots.changed().await?;
    }
    println!("stage=online pid={}", std::process::id());
    tokio::time::sleep(Duration::from_secs(15)).await;
    println!("stage=settled");
    discord.view(&guild, Some(&channel));
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let snapshot = snapshots.borrow_and_update().clone();
        if let Some((listed, count)) = &snapshot.listed {
            println!("stage=listed channel={listed} count={count}");
            break;
        }
        ensure!(Instant::now() < deadline, "member list never synced");
        snapshots.changed().await?;
    }
    println!("stage=done");
    loop {
        tokio::time::sleep(Duration::from_secs(30)).await;
        println!("tick");
    }
}
