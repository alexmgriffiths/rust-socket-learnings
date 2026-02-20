use std::{collections::HashMap, io::Error};

use futures_util::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Command {
    Register { name: String },
    Say { message: String },
}

enum Event {
    Connected {
        client_id: u64,
        out_tx: UnboundedSender<Message>,
    },
    Received {
        client_id: u64,
        command: Command,
    },
    Disconnected {
        client_id: u64,
    },
}

#[derive(Serialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg {
    Chat { from: String, message: String },
    Info { message: String },
    Error { message: String },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (tx, rx) = mpsc::unbounded_channel::<Event>(); // Don't set a fixed size of messages
    // In the future we should really use a bounded channel and handle back pressure... :/

    let try_socket = TcpListener::bind("127.0.0.1:9901").await;
    let listener = try_socket.expect("failed to bind");
    println!("Listening on 127.0.0.1:9901");

    tokio::spawn(handle_router(rx));

    let mut next_id: u64 = 0;
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(tx.clone(), stream, next_id));
        next_id += 1;
    }

    return Ok(());
}

// mut is required because recv takes a &mut self
async fn handle_router(mut received: UnboundedReceiver<Event>) {
    let mut clients: HashMap<u64, UnboundedSender<Message>> = HashMap::new();
    let mut client_names: HashMap<u64, String> = HashMap::new();
    let mut client_rev_lookup: HashMap<String, u64> = HashMap::new(); // Reverse lookup to also help with unique checks

    while let Some(ev) = received.recv().await {
        match ev {
            Event::Connected { client_id, out_tx } => {
                if let Err(e) = out_tx.send(Message::Text(
                    serde_json::to_string(&ServerMsg::Info {
                        message: "Welcome".to_string(),
                    })
                    .unwrap()
                    .into(),
                )) {
                    eprintln!("failed to send welcome message: {e}");
                    continue;
                }
                clients.insert(client_id, out_tx);
            }
            Event::Received { client_id, command } => {
                match command {
                    Command::Register { name } => {
                        // Check name taken
                        if client_rev_lookup.contains_key(&name) {
                            let serialize_response = serde_json::to_string(&ServerMsg::Error {
                                message: "Username taken".to_string(),
                            })
                            .unwrap(); // We unwrap here because this should never fail, in theory
                            let client_sender = clients.get(&client_id);
                            if let Some(client) = client_sender {
                                if let Err(e) =
                                    client.send(Message::Text(serialize_response.into()))
                                {
                                    eprintln!("failed to send error to client: {e}");
                                    continue; // TODO: Drop this dead client
                                }
                            }
                            continue;
                        }

                        // Check duplicate registration
                        if client_names.contains_key(&client_id) {
                            let serialize_response = serde_json::to_string(&ServerMsg::Error {
                                message: "Already registered".to_string(),
                            })
                            .unwrap(); // We unwrap here because this should never fail, in theory
                            let client_sender = clients.get(&client_id);
                            if let Some(client) = client_sender {
                                if let Err(e) =
                                    client.send(Message::Text(serialize_response.into()))
                                {
                                    eprintln!("failed to send error to client: {e}");
                                    continue; // TODO: Drop this dead client
                                }
                            }
                            continue;
                        }

                        if name.len() < 3 || name.len() > 24 {
                            let serialize_response = serde_json::to_string(&ServerMsg::Error {
                                message: "Invalid name".to_string(),
                            })
                            .unwrap(); // We unwrap here because this should never fail, in theory
                            let client_sender = clients.get(&client_id);
                            if let Some(client) = client_sender {
                                if let Err(e) =
                                    client.send(Message::Text(serialize_response.into()))
                                {
                                    eprintln!("failed to send error to client: {e}");
                                    continue; // TODO: Drop this dead client
                                }
                            }
                            continue;
                        }

                        // Add to client name map and reverse lookup
                        client_names.insert(client_id, name.clone());
                        client_rev_lookup.insert(name, client_id);
                        let client_sender = clients.get(&client_id);
                        client_sender
                            .unwrap()
                            .send(Message::Text(
                                serde_json::to_string(&ServerMsg::Info {
                                    message: "Registered succesfully".to_string(),
                                })
                                .unwrap()
                                .into(),
                            ))
                            .unwrap(); // DO THE MONSTER MASH (monster function)
                    }
                    Command::Say { message } => {
                        // Check the user has registered
                        if !client_names.contains_key(&client_id) {
                            let client_sender = clients.get(&client_id).unwrap();
                            client_sender
                                .send(Message::Text(
                                    serde_json::to_string(&ServerMsg::Error {
                                        message: "You must register first".to_string(),
                                    })
                                    .unwrap()
                                    .into(),
                                ))
                                .unwrap();
                            continue;
                        }

                        // We can unwrap since we just checked if they have a name
                        let client_name = client_names.get(&client_id).unwrap();

                        // This one broadcasts
                        let broadcast_recipients =
                            clients.iter().filter(|(id, _)| **id != client_id);
                        let mut clients_to_send_to: Vec<(u64, UnboundedSender<Message>)> = vec![];

                        // Alright funkiness. We don't want to hold clients while we send, so copy it and send later so the original "clients" is free
                        for (recipient_id, recipient_sender) in broadcast_recipients {
                            clients_to_send_to.push((*recipient_id, recipient_sender.clone()));
                        }

                        let outgoing_message = Message::Text(
                            serde_json::to_string(&ServerMsg::Chat {
                                from: client_name.clone(),
                                message: message.clone(),
                            })
                            .unwrap()
                            .into(),
                        );

                        let mut dead_clients: Vec<u64> = vec![];
                        for (client_to_send_id, client_sender) in clients_to_send_to {
                            if let Err(_) = client_sender.send(outgoing_message.clone()) {
                                // We don't want to remove the dead clients here because we don't want the pattern of
                                // mutating while "doing work"
                                dead_clients.push(client_to_send_id);
                                continue;
                            }
                        }

                        // Clear dead connections
                        for dead_client in dead_clients {
                            clients.remove(&dead_client);
                        }
                    }
                }
            }
            Event::Disconnected { client_id } => {
                // Remove from maps
                clients.remove(&client_id);
                if let Some(name) = client_names.remove(&client_id) {
                    client_rev_lookup.remove(&name);
                }
            }
        }
    }
}

async fn handle_connection(tx: UnboundedSender<Event>, stream: TcpStream, client_id: u64) {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Error during websocket handshake");
    let (out_tx, out_rx) = mpsc::unbounded_channel::<Message>();

    // Tell the main router thread that this user is connected
    if let Err(e) = tx.send(Event::Connected { client_id, out_tx }) {
        eprintln!("error on tx send: {e}");
        return;
    }

    let (writer, mut reader) = ws_stream.split();

    // Handle writer thread
    tokio::spawn(handle_client_writer(writer, out_rx));

    // Handle reader
    while let Some(item) = reader.next().await {
        match item {
            Ok(Message::Text(t)) => {
                let raw_command = t.to_string();
                // Just forward commands to the router
                match serde_json::from_str(&raw_command) {
                    Ok(value) => {
                        if let Err(e) = tx.send(Event::Received {
                            client_id,
                            command: value,
                        }) {
                            eprintln!("failed to send command: {e}");
                            continue;
                        }
                    }
                    Err(e) => {
                        eprintln!("could not parse json: {e}");
                        continue;
                    }
                }
            }
            Ok(Message::Close(_)) => {
                break;
            }
            Err(_) => {
                break;
            }
            _ => {
                println!("unimplemented");
            }
        }
    }

    // If we broke out of the reader, it's because the client disconnected... probably.
    if let Err(e) = tx.send(Event::Disconnected { client_id }) {
        eprintln!("failed to send disconnect event: {e}");
        return;
    }
}

async fn handle_client_writer(
    mut writer: SplitSink<WebSocketStream<TcpStream>, Message>,
    mut out_rx: UnboundedReceiver<Message>,
) {
    while let Some(msg) = out_rx.recv().await {
        // So the router passes out_rx down to handle_connection, which creates this task and passes out_rx again
        // So writer is the writer for the TcpStream, and since we have the out_rx from the original receiver we can catch
        // messages from the router and pipe them to the stream's writer
        if let Err(e) = writer.send(msg).await {
            eprintln!("failed to send message to writer: {e}");
            break;
        }
    }
}
