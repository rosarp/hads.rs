use accounts::{Account, Shared};
use commands::Command;
use futures::SinkExt;
use std::{env, error::Error, net::SocketAddr, sync::Arc};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_stream::StreamExt;
use tokio_util::codec::{Framed, LinesCodec};
use tracing::{debug, error, info};

mod accounts;
mod commands;

#[tokio::main]
async fn main() {
    use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
    // Configure a `tracing` subscriber that logs traces emitted by the chat
    // server.
    tracing_subscriber::fmt()
        // Filter what traces are displayed based on the RUST_LOG environment
        // variable.
        //
        // Traces emitted by the example code will always be displayed. You
        // can set `RUST_LOG=tokio=trace` to enable additional traces emitted by
        // Tokio itself.
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("chat=info".parse().expect("issue with parsing directive")),
        )
        // Log events when `tracing` spans are created, entered, exited, or
        // closed. When Tokio's internal tracing support is enabled (as
        // described above), this can be used to track the lifecycle of spawned
        // tasks on the Tokio runtime.
        .with_span_events(FmtSpan::FULL)
        // Set this subscriber as the default, to collect all traces emitted by
        // the program.
        .init();

    // Create the shared state. This is how all the peers communicate.
    //
    // The server task will hold a handle to this. For every new client, the
    // `state` handle is cloned and passed into the task that processes the
    // client connection.
    let state = Arc::new(Mutex::new(Shared::new()));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:4222".to_string());

    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to start server");

    info!("Starting hads-server on {}", addr);

    loop {
        // Asynchronously wait for an inbound TcpStream.
        let (stream, addr) = listener.accept().await.expect("Issue with listener accept");

        // Clone a handle to the `Shared` state for the new connection.
        let state = Arc::clone(&state);

        // Spawn our handler to be run asynchronously.
        tokio::spawn(async move {
            debug!("accepted connection");
            if let Err(e) = process(state, stream, addr).await {
                info!("an error occurred; error = {:?}", e);
            }
        });
    }
}

/// Process an individual chat client
async fn process(
    state: Arc<Mutex<Shared>>,
    stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn Error>> {
    let lines = Framed::new(stream, LinesCodec::new());

    // Register our account with state which internally sets up some channels.
    let mut account = Account::new(state.clone(), lines).await?;

    // Process incoming messages until our stream is exhausted by a disconnect.
    loop {
        tokio::select! {
            // A message was received from a account. Send it to the current user.
            Some(message) = account.rx.recv() => {
                account.lines.send(&message).await?;
            }
            result = account.lines.next() => match result {
                Some(Ok(message)) => {
                    // create command and execute
                    // TODO: handle unwrap()
                    Command::new(&message).unwrap().execute(&mut account).await;
                    let mut state = state.lock().await;
                    let message = format!("MSG {}", message);

                    state.broadcast(addr, &message).await;
                }
                // An error occurred.
                Some(Err(e)) => {
                    error!(
                        "an error occurred while processing messages for {}; error = {:?}",
                        addr.to_string(),
                        e
                    );
                }
                // The stream has been exhausted.
                None => break,
            },
        }
    }

    // If this section is reached it means that the client was disconnected!
    // Let's clean up
    {
        let mut state = state.lock().await;
        state.peers.remove(&addr);
    }

    Ok(())
}
