use actix::{fut, ActorContext};
use crate::messages::{Disconnect, Connect, WsMessage, ClientActorMessage}; //We'll be writing this later
use crate::lobby::Lobby; // as well as this
use actix::{Actor, Addr, Running, StreamHandler, WrapFuture, ActorFuture, ContextFutureSpawner};
use actix::{AsyncContext, Handler};
use actix_web_actors::ws;
use std::time::{Duration, Instant};
use uuid::Uuid;
use actix::ActorFutureExt;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

/*
room: каждый сокет существует в «комнате», которая в этой реализации будет
      простой HashMap, которая содержит список сокетов по Uuid идентификатору.
      
addr: это адрес лобби, в котором существует сокет. 
      Он будет использоваться для отправки данных в лобби. поэтому отправка текстового 
      сообщения в лобби может выглядеть так: self.addr.do_send('hi!'). 
      Без этого свойства актер не смог бы найти лобби.      

hb: Хотя веб-сокеты отправляют сообщения при закрытии, иногда веб-сокеты закрываются без 
    предупреждения. вместо того, чтобы этот актор существовал вечно, мы отправляем ему 
    пульс каждые N секунд, и если мы не получаем ответа, мы завершаем сокет. 
    Это свойство представляет собой время, прошедшее с момента получения последнего пульса. 
    Во многих библиотеках это делается автоматически. 

id: это идентификатор, присвоенный нами этому сокету. 
    Это полезно, помимо прочего, для обмена личными сообщениями, 
    поэтому мы можем /whisper <id> hello! шептать этому клиенту.   
*/
pub struct WsConn {
    room: Uuid,
    lobby_addr: Addr<Lobby>,
    hb: Instant,
    id: Uuid,
}

impl WsConn {
    pub fn new(room: Uuid, lobby: Addr<Lobby>) -> WsConn {
        WsConn {
            id: Uuid::new_v4(),
            room,
            hb: Instant::now(),
            lobby_addr: lobby,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                log::info!("Disconnecting failed heartbeat");
                act.lobby_addr.do_send(Disconnect { id: act.id, room_id: act.room });
                ctx.stop();
                return;
            }

            ctx.ping(b"PING");
        });
    }
}

// Реализация актера
impl Actor for WsConn {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {

        // Старт серцебиения
        self.hb(ctx);

        /*
        Это лобби, в которое я хочу попасть, и мой идентификатор, а также адрес моего 
        почтового ящика, по которому вы можете со мной связаться».
        Метод send требует ожидания результата поэтому есть блок then
        */
        let addr = ctx.address();
        self.lobby_addr
            .send(Connect {
                addr: addr.recipient(),
                lobby_id: self.room,
                self_id: self.id,
            })
            .into_actor(self)
            .then(|res, _, ctx| {
                match res {
                    Ok(_res) => {
                        log::info!("Lobby started");
                        ()
                    },
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        log::info!("Lobby stoped");
        self.lobby_addr.do_send(Disconnect { id: self.id, room_id: self.room });
        Running::Stop
    }
}

// Обработчик сообщений WS
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConn {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Binary(bin)) => {
                ctx.binary(bin)
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Ok(ws::Message::Continuation(_)) => {
                ctx.stop();
            }
            Ok(ws::Message::Nop) => {()},
            Ok(ws::Message::Text(s)) => {
                // Лобби будет заниматься посредничеством его там, где это нужно
                    self.lobby_addr.do_send(ClientActorMessage {
                    id: self.id,
                    msg: s.to_string(), 
                    room_id: self.room
                })
           },
            Err(e) => panic!("{}",e),
        }
    }
}

// Обработчик WsMessage 
impl Handler<WsMessage> for WsConn {
    // Обратите внимание, что нам также необходимо определить, как может выглядеть ответ 
    // на это письмо. Если почта размещена как do_send, тип ответа не имеет значения. 
    // Если он размещен как send(), то ожидаемый тип результата будет следующим: Result. 
    // может быть, вы делаете type Result = String или что-то подобное из rtype сообщения.
    type Result = ();

    fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) {
        // Здесь, если сервер помещает WsMessage  почту в наш почтовый ящик, 
        // все, что мы делаем, это отправляем ее прямо клиенту.
        ctx.text(msg.0);
    }
}