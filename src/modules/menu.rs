use super::{
    bets::opposing::opposing_bets, deposit::deposit_to_accounts, registration::register_accounts,
    stats_check::check_and_display_stats,
};
use crate::db::constants::{ADDRESS_FILE_PATH, PROXY_ADDRESS_FILE_PATH};
use crate::modules::stats_check::{check_and_display_stats_from_db, check_and_display_stats_from_text, get_proxy_address_from_txt};
use crate::{
    config::Config,
    db::database::Database,
    modules::{sell::sell_all::sell_all_open_positions, withdraw::withdraw_for_all},
};
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Password, Select};

const LOGO: &str = r#"

██████╗  ██████╗ ██╗  ██╗   ██╗███╗   ███╗ █████╗ ██████╗ ██╗  ██╗███████╗████████╗    ██████╗  ██████╗ ████████╗
██╔══██╗██╔═══██╗██║  ╚██╗ ██╔╝████╗ ████║██╔══██╗██╔══██╗██║ ██╔╝██╔════╝╚══██╔══╝    ██╔══██╗██╔═══██╗╚══██╔══╝
██████╔╝██║   ██║██║   ╚████╔╝ ██╔████╔██║███████║██████╔╝█████╔╝ █████╗     ██║       ██████╔╝██║   ██║   ██║
██╔═══╝ ██║   ██║██║    ╚██╔╝  ██║╚██╔╝██║██╔══██║██╔══██╗██╔═██╗ ██╔══╝     ██║       ██╔══██╗██║   ██║   ██║
██║     ╚██████╔╝███████╗██║   ██║ ╚═╝ ██║██║  ██║██║  ██║██║  ██╗███████╗   ██║       ██████╔╝╚██████╔╝   ██║
╚═╝      ╚═════╝ ╚══════╝╚═╝   ╚═╝     ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝   ╚═╝       ╚═════╝  ╚═════╝    ╚═╝

                                         Author:[𝕏] @0xNaiXi
                                         Author:[𝕏] @0xNaiXi
                                         Author:[𝕏] @0xNaiXi
"#;

pub async fn menu() -> eyre::Result<()> {
    async fn read_or_create_db(password: Option<&str>) -> eyre::Result<Database> {
        match Database::read(password).await {
            Ok(db) => Ok(db),
            Err(_) => Database::new(password).await,
        }
    }

    async fn read_db(password: Option<&str>) -> eyre::Result<Database> {
        match Database::read(password).await {
            Ok(db) => Ok(db),
            Err(e) => {
                println!(
                    "{}",
                    "✘ Failed to read database! (password is wrong?)".red()
                );
                Err(e) // 直接返回错误，不创建新数据库
            }
        }
    }

    // 通过txt 读取数据（地址 一行一个）
    async fn read_data_from_txt(file_path: &str) -> eyre::Result<Vec<String>> {
        let content = tokio::fs::read_to_string(file_path).await?;
        let lines: Vec<String> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(String::from)
            .collect();
        Ok(lines)
    }

    let config = Config::read_default().await;
    let logo = LOGO.blue();

    println!("{logo}");

    let aes_key = Password::with_theme(&ColorfulTheme::default())
        .allow_empty_password(true)
        .with_prompt("Please enter password")
        .interact()
        .unwrap();
    let aes_key = if aes_key.is_empty() {
        None
    } else {
        Some(aes_key.as_str())
    };

    loop {
        let options = vec![
            "Accounts registration",
            "Proxy wallets stats check from txt",
            "Proxy wallets stats check",
            "USDC deposit",
            "Opposing bets",
            "Sell all open positions",
            "Withdraw",
            "Get proxy address from txt",
            "Exit",
        ];

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choice:")
            .items(&options)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => {
                let db = Database::new(aes_key).await?;
                register_accounts(db, &config).await?;
            }
            1 => {
                let file_path: String = dialoguer::Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Please enter file path")
                    .default(PROXY_ADDRESS_FILE_PATH.to_string())
                    .interact()
                    .unwrap();
                let data = read_data_from_txt(&file_path).await?;
                check_and_display_stats_from_text(data, &config).await?;
            }
            2 => {
                let db = read_db(aes_key).await?;
                check_and_display_stats_from_db(db, &config).await?;
            }
            3 => {
                let mut db = read_db(aes_key).await?;
                db.shuffle();
                deposit_to_accounts(db, &config).await?;
            }
            4 => {
                let mut db = read_db(aes_key).await?;
                db.shuffle();
                opposing_bets(db, &config).await?;
            }
            5 => {
                let db = read_db(aes_key).await?;
                sell_all_open_positions(db, &config).await?;
            }
            6 => {
                let mut db = read_db(aes_key).await?;
                withdraw_for_all(&mut db, &config).await?;
            }
            7 => {
                let file_path: String = dialoguer::Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Please enter file path")
                    .default(ADDRESS_FILE_PATH.to_string())
                    .interact()
                    .unwrap();
                let data = read_data_from_txt(&file_path).await?;
                get_proxy_address_from_txt(data).await?;
            }
            8 => {
                return Ok(());
            }
            _ => tracing::error!("Invalid selection"),
        }
    }
}
