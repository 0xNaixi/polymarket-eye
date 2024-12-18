use super::{
    bets::opposing::opposing_bets, deposit::deposit_to_accounts, registration::register_accounts,
    stats_check::check_and_display_stats,
};
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
                println!("{}", "✘ Failed to read database! (password is wrong?)".red());
                Err(e)  // 直接返回错误，不创建新数据库
            }
        }
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
            "USDC deposit",
            "Opposing bets",
            "Proxy wallets stats check",
            "Sell all open positions",
            "Withdraw",
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
                let mut db = read_db(aes_key).await?;
                db.shuffle();
                deposit_to_accounts(db, &config).await?;
            }
            2 => {
                let mut db = read_db(aes_key).await?;
                db.shuffle();

                opposing_bets(db, &config).await?;
            }
            3 => {
                let db = read_db(aes_key).await?;
                check_and_display_stats(db, &config).await?;
            }
            4 => {
                let db = read_db(aes_key).await?;
                sell_all_open_positions(db, &config).await?;
            }
            5 => {
                let mut db = read_db(aes_key).await?;
                withdraw_for_all(&mut db, &config).await?;
            }
            6 => {
                return Ok(());
            }
            _ => tracing::error!("Invalid selection"),
        }
    }
}
