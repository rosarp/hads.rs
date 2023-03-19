use crate::accounts::Account;
use futures::SinkExt;

pub enum Command<'a> {
    PING,
    PUB(&'a str, &'a str),
    SUB,
}

impl Command<'_> {
    pub fn new(message: &str) -> Result<Command, &str> {
        if message.eq("ping") || message.eq("PING") {
            Ok(Command::PING)
        } else if message.starts_with("pub") || message.starts_with("PUB") {
            Ok(Command::PUB("", ""))
        } else if message.starts_with("sub") || message.starts_with("SUB") {
            //
            Ok(Command::SUB)
        } else {
            Err("Invalid input")
        }
    }

    pub async fn execute(&self, account: &mut Account) {
        match self {
            Command::PING => ping(account).await,
            Command::PUB(message, topic) => publish(message, topic),
            Command::SUB => subscribe(vec![]),
        }
    }
}

async fn ping(account: &mut Account) {
    // print PONG
    // send back PONG
    let _result = account.lines.send("PONG").await;
    // TODO: auth
}

fn publish(message: &str, topic: &str) {
    // validate topic
    // send messages to all subscribers of this topic
}

fn subscribe(topics: Vec<String>) {
    // map topics to this subscriber
}
