#![feature(iter_intersperse)]

use std::{collections::HashMap, str::FromStr, time::Duration};

use data::{FCPInfo, FCPStorage};
use telegram_bot::{ChatRef, MessageId};
use utils::FetchConnector;
use worker::*;

use crate::data::SentMsg;

mod data;
mod utils;
mod msg;

const SLEEP_DURATION_MS: u64 = 2000;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    router
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await
}

#[event(scheduled)]
pub async fn update(_: ScheduledEvent, env: Env, _: ScheduleContext) {
    if let Err(e) = update_rfc_list(env).await {
        worker::console_warn!("Error: {}", e);
    }
}

pub async fn update_rfc_list(env: Env) -> Result<()> {
    let api = telegram_bot::Api::with_connector(
        env.secret("TG_BOT_TOKEN")?.to_string(),
        Box::new(FetchConnector),
    );
    let fetched: Vec<data::FcpWithInfo> = Fetch::Url(Url::from_str("https://rfcbot.rs/api/all")?)
        .send()
        .await?
        .json()
        .await?;
    let storage = env.kv("FCP")?;
    for info in fetched {
        // worker::console_log!("Processing: {:#?}", info);
        let mapped: FCPInfo = info.into();
        let id = mapped.id.to_string();
        let saved: Option<FCPStorage> = storage.get(&id).json().await?;

        // worker::console_log!("Saved: {:#?}", saved);

        let messages = saved.map(|s| s.messages).unwrap_or_else(HashMap::new);

        let (formatted, entities) = msg::format_msg(&mapped);
        // worker::console_log!("Formatted:\n{}", formatted);
        // worker::console_log!("Entities:\n{:#?}", entities);

        let mut updated = FCPStorage {
            info: mapped,
            messages,
        };

        let targets = env.var("TARGETS").unwrap().to_string();
        let targets = targets.split(",").map(|e| e.trim());

        for target in targets {
            let cur = updated.messages.get_mut(target);

            let success = if let Some(msg) = cur {
                if msg.version == updated.info.updated_at && msg.format == msg::MSG_FORMAT {
                    worker::console_log!("Already at newest version: {} in {}...", msg.id, target);
                    continue;
                }

                worker::console_log!("Updating {} in {}...", msg.id, target);
                let mut req = telegram_bot::EditMessageText::new(
                    ChatRef::ChannelUsername(target.to_owned()),
                    MessageId::from(msg.id),
                    formatted.clone(),
                );
                req.entities(entities.clone());
                match api.send(req).await {
                    Err(e) => {
                        worker::console_error!("Updating error: {}", e);
                        false
                    }
                    Ok(_) => {
                        worker::console_error!("Updated");
                        msg.version = updated.info.updated_at;
                        msg.format = msg::MSG_FORMAT;
                        true
                    }
                }
            } else {
                worker::console_log!("Sending in {}...", target);
                let mut req = telegram_bot::SendMessage::new(
                    ChatRef::ChannelUsername(target.to_owned()),
                    formatted.clone(),
                );
                req.entities(entities.clone());
                match api.send(req).await {
                    Err(e) => {
                        worker::console_error!("Sending error: {}", e);
                        false
                    },
                    Ok(sent) => {
                        let id = sent.message_id().into();
                        worker::console_log!("Sent, id = {}", id);
                        updated.messages.insert(target.to_owned(), SentMsg {
                            id,
                            version: updated.info.updated_at,
                            format: msg::MSG_FORMAT,
                        });
                        true
                    }
                }
            };
            if success {
                storage.put(&id, &updated)?.execute().await?;
            }
            worker::Delay::from(Duration::from_millis(SLEEP_DURATION_MS)).await;
        }
    }
    Ok(())
}
