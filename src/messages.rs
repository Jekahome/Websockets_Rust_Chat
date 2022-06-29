use actix::prelude::{Message, Recipient};
use uuid::Uuid;
// Этот файл будет содержать все «сообщения», которые попадают в почтовые ящики нашего актера

//WsConn отвечает на это, чтобы передать его фактическому клиенту
#[derive(Message)]
#[rtype(result = "()")]
pub struct WsMessage(pub String);

//WsConn отправляет это в лобби, чтобы сказать "включите меня, пожалуйста" 
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Recipient<WsMessage>,
    pub lobby_id: Uuid,
    pub self_id: Uuid,
}

//WsConn отправляет это в лобби, чтобы сказать "выведите меня, пожалуйста" 
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub room_id: Uuid,
    pub id: Uuid,
}

//клиент отправляет это в лобби, чтобы лобби выдало эхо. 
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientActorMessage {
    pub id: Uuid,
    pub msg: String,
    pub room_id: Uuid
}