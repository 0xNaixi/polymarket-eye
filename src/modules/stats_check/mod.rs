use std::{fs::File, sync::Arc};

use alloy::{
    primitives::{utils::format_units, Address},
    providers::ProviderBuilder,
};
use chrono::{FixedOffset, LocalResult, TimeZone, Utc};
use csv::WriterBuilder;
use itertools::Itertools;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use reqwest::{Proxy, Url};
use scraping::{
    scrape_open_positions, scrape_users_open_pos_value, scrape_users_pnl, scrape_users_trade_count,
    scrape_users_volume,
};
use serde::Serialize;
use tabled::{settings::Style, Table, Tabled};

use crate::db::constants::PROXIES_FILE_PATH;
use crate::modules::registration::{check_if_proxy_wallet_activated, retry_check_activation};
use crate::modules::stats_check::scraping::scrape_users_last_activity_time;
use crate::utils::files::read_file_lines;
use crate::utils::poly::get_proxy_wallet_address_from_address;
use crate::{
    config::Config,
    db::database::Database,
    onchain::{multicall::multicall_balance_of, types::token::Token},
};

mod scraping;

const EXPORT_FILE_PATH: &str = "data/stats.csv";
const EXPORT_ADDRESS_FILE_PATH: &str = "data/address_info.csv";
#[derive(Tabled, Serialize)]
struct UserStats {
    #[tabled(rename = "Proxy Address")]
    #[serde(rename = "Proxy Address")]
    address: String,

    #[tabled(rename = "USDC.e Balance")]
    #[serde(rename = "USDC.e Balance")]
    balance: String,

    #[tabled(rename = "Open positions count")]
    #[serde(rename = "Open positions count")]
    open_positions_count: usize,

    #[tabled(rename = "Open positions value")]
    #[serde(rename = "Open positions value")]
    open_positions_value: f64,

    #[tabled(rename = "Volume")]
    #[serde(rename = "Volume")]
    volume: f64,

    #[tabled(rename = "P&L")]
    #[serde(rename = "P&L")]
    pnl: f64,

    #[tabled(rename = "Trade count")]
    #[serde(rename = "Trade count")]
    trade_count: u64,

    #[tabled(rename = "Is Registered")]
    #[serde(rename = "Is Registered")]
    is_registered: bool,

    //最后活动时间
    #[tabled(rename = "Last Activity Time")]
    #[serde(rename = "Last Activity Time")]
    last_activity_time: String,
}


#[derive(Tabled, Serialize)]
struct AddressInfo {
    #[tabled(rename = "Address")]
    #[serde(rename = "Address")]
    address: String,

    #[tabled(rename = "Proxy Address")]
    #[serde(rename = "Proxy Address")]
    proxy_address: String,
}


pub async fn check_and_display_stats_from_db(db: Database, config: &Config) -> eyre::Result<()> {
    let (addresses, proxies): (Vec<Address>, Vec<Option<Proxy>>) =
        db.0.iter()
            .map(|account| (account.get_proxy_address(), account.proxy()))
            .unzip();

    check_and_display_stats(addresses, proxies, config).await?;
    Ok(())
}


pub async fn check_and_display_stats_from_text(proxy_addresses: Vec<String>, config: &Config) -> eyre::Result<()> {
    tracing::info!("Processing {} addresses", proxy_addresses.len());
    let proxy_addresses: Vec<Address> = proxy_addresses
        .into_iter()
        .map(|address| address.parse().unwrap())
        .collect();
    let proxies = read_file_lines(PROXIES_FILE_PATH).await.unwrap();
    tracing::info!("Loaded {} proxies from file", proxies.len());
    // 确保有可用的代理
    if proxies.is_empty() {
        return Err(eyre::eyre!("No proxies available in file"));
    }
    //遍历 proxy_addresses 为每一个地址生成一个随机的 proxy
    let mut use_proxies = Vec::with_capacity(proxy_addresses.len());
    for _ in 0..proxy_addresses.len() {
        // 随机 获取
        let random_proxy = proxies.choose(&mut thread_rng()).unwrap();
        let proxy = Proxy::all(random_proxy)?;
        use_proxies.push(Some(proxy));
    }
    check_and_display_stats(proxy_addresses, use_proxies, config).await?;
    Ok(())
}


// 这里的address 实际为 proxy address
pub async fn check_and_display_stats(proxy_addresses: Vec<Address>, proxies: Vec<Option<Proxy>>, config: &Config) -> eyre::Result<()> {
    let provider = Arc::new(
        ProviderBuilder::new()
            .with_recommended_fillers()
            .on_http(Url::parse(&config.polygon_rpc_url)?),
    );

    let balances = multicall_balance_of(&proxy_addresses, Token::USDCE, provider.clone()).await?;

    let addresses = proxy_addresses
        .into_iter()
        .map(|addr| addr.to_string())
        .collect_vec();

    let (
        open_positions_stats,
        users_volume_stats,
        users_pnl_stats,
        users_trade_count_stats,
        users_open_pos_value_stats,
        users_last_activity_time_stats,
    ) = tokio::join!(
        scrape_open_positions(addresses.clone(), proxies.clone()),
        scrape_users_volume(addresses.clone(), proxies.clone()),
        scrape_users_pnl(addresses.clone(), proxies.clone()),
        scrape_users_trade_count(addresses.clone(), proxies.clone()),
        scrape_users_open_pos_value(addresses.clone(), proxies.clone()),
        scrape_users_last_activity_time(addresses.clone(), proxies.clone())
    );

    let mut stats_entries = vec![];

    for (address, balance) in addresses.iter().zip(balances.iter()) {
        let balance_in_usdce = format_units(*balance, 6).unwrap_or_else(|_| "0".to_string());

        let is_registered = retry_check_activation(provider.clone(), address.parse().unwrap(), 60).await?;

        let open_positions_count = open_positions_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|positions| positions.1.len())
            .unwrap_or(0);

        let open_positions_value = users_open_pos_value_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|pos_values| pos_values.1.first().unwrap().value)
            .unwrap_or(0f64);

        let user_volume = users_volume_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.first().map_or(0f64, |v| v.amount))
            .unwrap_or(0f64);

        let user_pnl = users_pnl_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.first().map_or(0f64, |v| v.amount))
            .unwrap_or(0f64);

        let trade_count = users_trade_count_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.traded)
            .unwrap_or(0);

        let last_activity_timestamp = users_last_activity_time_stats
            .iter()
            .find(|res| &res.0 == address)
            .map(|volume| volume.1.first().map_or(0u64, |v| v.timestamp))
            .unwrap_or(0);

        let last_activity_text = if last_activity_timestamp > 0 {
            match Utc.timestamp_opt(last_activity_timestamp as i64, 0) {
                LocalResult::Single(time) => {
                    let beijing_time = time.with_timezone(&FixedOffset::east_opt(8 * 3600).unwrap());
                    let duration = Utc::now().signed_duration_since(time);
                    let duration_text = format!("{}d ago", duration.num_days());
                    format!("{} ({})",
                            beijing_time.format("%Y-%m-%d %H:%M:%S"),
                            duration_text
                    )
                }
                _ => "Invalid time".to_string()
            }
        } else {
            "".to_string()
        };

        let entry = UserStats {
            address: address.to_string(),
            balance: balance_in_usdce,
            open_positions_count,
            open_positions_value,
            volume: user_volume,
            pnl: user_pnl,
            trade_count,
            is_registered,
            last_activity_time: last_activity_text,
        };

        stats_entries.push(entry);
    }

    let total_balance: f64 = stats_entries
        .iter()
        .map(|entry| entry.balance.parse::<f64>().unwrap_or(0.0))
        .sum();

    let total_open_positions_count: usize = stats_entries
        .iter()
        .map(|entry| entry.open_positions_count)
        .sum();

    let total_open_positions_value: f64 = stats_entries
        .iter()
        .map(|entry| entry.open_positions_value)
        .sum();

    let total_volume: f64 = stats_entries.iter().map(|entry| entry.volume).sum();

    let total_pnl: f64 = stats_entries.iter().map(|entry| entry.pnl).sum();

    let total_trade_count: u64 = stats_entries.iter().map(|entry| entry.trade_count).sum();

    let total_registered = stats_entries.iter().filter(|entry| entry.is_registered).count();

    let total_entry = UserStats {
        address: format!("Total (Registered: {}/{})", total_registered, addresses.len()),
        balance: format!("{:.2}", total_balance),
        open_positions_count: total_open_positions_count,
        open_positions_value: total_open_positions_value,
        volume: total_volume,
        pnl: total_pnl,
        trade_count: total_trade_count,
        is_registered: false,
        last_activity_time: "".to_string(),
    };

    stats_entries.push(total_entry);

    let mut table = Table::new(&stats_entries);
    let table = table.with(Style::modern_rounded());

    println!("{table}");

    export_stats_to_csv(&stats_entries, EXPORT_FILE_PATH)?;

    Ok(())
}

fn export_stats_to_csv<T>(entries: &[T], path: &str) -> eyre::Result<()>
where
    T: serde::Serialize,
{
    let export_file = File::create(path)?;

    let mut writer = WriterBuilder::new()
        .has_headers(true)
        .from_writer(export_file);

    for entry in entries {
        writer.serialize(entry)?
    }

    writer.flush()?;

    tracing::info!("Stats exported to {}", EXPORT_FILE_PATH);

    Ok(())
}


pub async fn get_proxy_address_from_txt(addresses: Vec<String>) -> eyre::Result<()> {
    let addresses: Vec<Address> = addresses
        .into_iter()
        .map(|address| address.parse().unwrap())
        .collect();

    let mut address_entries = vec![];
    //遍历地址，获取proxy地址
    for address in addresses {
        let proxy_address = get_proxy_wallet_address_from_address(&address);
        let entry = AddressInfo {
            address: address.to_string(),
            proxy_address: proxy_address.to_string(),
        };
        address_entries.push(entry);
    }

    let mut table = Table::new(&address_entries);
    let table = table.with(Style::modern_rounded());

    println!("{table}");

    export_stats_to_csv(&address_entries, EXPORT_ADDRESS_FILE_PATH)?;

    Ok(())
}
