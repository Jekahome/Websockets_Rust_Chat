use crate::ws::WsConn;
use crate::lobby::Lobby;
use actix::Addr;
use actix_web::{get, web::Data, web::Path, web::Payload, Error, HttpResponse, HttpRequest};
use actix_web_actors::ws;
use uuid::Uuid;

// Random uuid room
// ws://192.168.2.104:4011/c048a89b-4e45-44cf-bf6c-1674e6de9ee8

#[get("/{room_id}")]
pub async fn start_connection(
    req: HttpRequest,
    stream: Payload,
    room: Path<Uuid>,
    srv: Data<Addr<Lobby>>,
) -> Result<HttpResponse, Error> {
    let room_id = room.into_inner();
    let ws = WsConn::new(room_id,srv.get_ref().clone());
    let resp = ws::start(ws, &req, stream)?;
    Ok(resp)
}