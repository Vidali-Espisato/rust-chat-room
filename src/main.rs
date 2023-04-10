#[macro_use] extern crate rocket;

use rocket::{
    tokio::{ select, sync::broadcast::{
        channel, Sender, error::RecvError
    }},
    serde::{ Serialize, Deserialize },
    State, Shutdown,
    form::Form,
    response::stream::{ EventStream, Event },
    fairing::{Fairing, Info, Kind},
    {Request, Response}
};


#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
    pub avatar_style: String
}


#[post("/message", data="<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    // A send fails if there are no active subscribers.
    let _res = queue.send(form.into_inner());
}

#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();

    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}


#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .attach(CorsFairing)
        .mount("/", routes![ post, events ])
}


struct CorsFairing;

#[rocket::async_trait]
impl Fairing for CorsFairing {
    fn info(&self) -> Info {
        Info {
            name: "CORS Fairing",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(rocket::http::Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE"),
        );
        response.set_header(
            rocket::http::Header::new("Access-Control-Allow-Headers", "Content-Type"),
        );
    }
}
