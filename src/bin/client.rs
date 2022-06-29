use std::{io, thread};
use awc::{ ws::{Frame,Codec}, BoxedSocket};
use actix_http::ws::Item;
use tokio::{select, sync::mpsc};
use futures_util::{SinkExt as _};
use bytes::Bytes;
use futures::stream::{StreamExt};
use std::convert::TryFrom;
use uuid::Uuid;
use actix_codec::{Framed};

// cargo run --bin client

#[actix_web::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("starting echo WebSocket client");

     
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let mut cmd_rx = tokio_stream::wrappers::UnboundedReceiverStream::new(cmd_rx);

    // Также можно запустить в отдельном потоке прослушивание команд консоли и передавать их серверу
    // run blocking terminal input reader on separate thread
    let input_thread = thread::spawn(move || loop {
        let mut cmd = String::with_capacity(32);

        if io::stdin().read_line(&mut cmd).is_err() {
            log::error!("error reading line");
            return;
        }
        if cmd == "start\n".to_string(){ 
            cmd_tx.send("ready send".to_string()).unwrap();
           // addr.do_send(ClientCommand( Command::ReadySend("ready send".to_string()) ));   
        } 
    }); 
     
    let room = Uuid::new_v4();
    let url = format!("ws://127.0.0.1:4011/{}",room);
    log::info!("Connect {}",url);

    let (_res, mut server_addr_framed):(_, Framed<BoxedSocket, Codec>) = awc::Client::new()
        .ws(url)
        .connect()
        .await
        .map_err(|e| {
            log::error!("Error: {}", e);
        })
        .unwrap();
 
  
        log::info!("connected; server will echo messages sent");
 
        loop {
        select! {
            Some(msg) = server_addr_framed.next() => {
                match msg {
                    Ok(awc::ws::Frame::Text(txt)) => {
                        // log echoed messages from server
                        log::info!("Client:Type msg Text: {:?}", txt)
                    }

                    Ok(awc::ws::Frame::Ping(_)) => {
                        //log::info!("Client:Type msg Ping"); 
                        // respond to ping probes
                        server_addr_framed.send(awc::ws::Message::Pong(Bytes::new())).await.unwrap();
                    }

                    Ok(Frame::Pong(_)) => {
                        log::info!("Client:Type msg Pong");  
                    }
 
                    Ok(Frame::Binary(bin)) => {
                        log::info!("Client:Type msg Binary: {:?}", bin);
                         
     
                         let size_byte = 4096;//65536-1 4096 16384 32768   (max=65536 2 byte/ 16bit)
                         let buff = (0..=size_byte).map(|_|78).collect::<Vec<u8>>();
     
                         if let Ok(n) = Number::try_from(bin){
                             match n.0 {
                                 0 => {
                                    server_addr_framed.send( awc::ws::Message::Continuation( Item::FirstBinary( Bytes::from_iter(buff.clone())))); 
                                 },
                                 e @ 1..=8 => {
                                    server_addr_framed.send( awc::ws::Message::Continuation( Item::Continue( Bytes::from_iter(buff.clone())))); 
                                 },
                                 e @ 9  => {
                                    server_addr_framed.send( awc::ws::Message::Continuation( Item::Last( Bytes::from_iter(buff.clone()))));
                                 },
                                 _ => {
                                     println!("Client:send end");
                                 }
                             }
                         }
                     }

                    _ => {
                        log::info!("Other message type");
                    }
                }
            }

            Some(cmd) = cmd_rx.next() => {
                if cmd.is_empty() {
                    continue;
                }

                server_addr_framed.send(awc::ws::Message::Text(cmd.into())).await.unwrap();
            }

            else => break
        }
    }
     input_thread.join().unwrap();  
}


#[derive(Debug)]
struct Number(usize);

impl TryFrom<Bytes> for Number {
    type Error = String;
    fn try_from(item: Bytes) -> Result<Self, Self::Error> {
        let v = item.into_iter().collect::<Vec<u8>>();
        match v.try_into() {
            Ok(arr) =>{ Ok(Number(usize::from_be_bytes(arr))) },
            Err(e) => { Err(format!("{:?}",e)) }
        }
    }
}