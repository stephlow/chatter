use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tower_service::Service;
use wasm_bindgen_futures::wasm_bindgen::JsValue;
use worker::*;

#[derive(Clone)]
pub struct AppState {
    env: Arc<Env>,
}

#[derive(Serialize, Deserialize)]
struct MessageList {
    messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    content: String,
}

fn router(env: Env) -> axum::Router {
    let state = AppState { env: Arc::new(env) };
    axum::Router::new()
        .route("/", axum::routing::get(root))
        .nest(
            "/api",
            axum::Router::new().route(
                "/messages",
                axum::routing::get(get_messages).post(post_messages),
            ),
        )
        .with_state(state)
}

#[event(fetch)]
async fn fetch(
    req: HttpRequest,
    env: Env,
    _ctx: Context,
) -> Result<axum::http::Response<axum::body::Body>> {
    console_error_panic_hook::set_once();
    Ok(router(env).call(req).await?)
}

#[worker::send]
pub async fn get_messages(
    axum::extract::State(AppState { env }): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    let response = fetch_chatroom(
        env,
        worker::Request::new("http://fake-host/messages", worker::Method::Get).unwrap(),
    )
    .await
    .unwrap();

    let parsed = serde_json::from_str::<MessageList>(&response).unwrap();

    axum::Json(parsed)
}

#[worker::send]
#[axum::debug_handler]
pub async fn post_messages(
    axum::extract::State(AppState { env }): axum::extract::State<AppState>,
    axum::extract::Json(payload): axum::extract::Json<Message>,
) -> impl axum::response::IntoResponse {
    let json = serde_json::to_string(&payload).unwrap();

    let response = fetch_chatroom(
        env,
        worker::Request::new_with_init(
            "http://fake-host/messages",
            &RequestInit {
                body: Some(JsValue::from_str(json.as_str())),
                headers: worker::Headers::new(),
                cf: CfProperties {
                    apps: None,
                    cache_everything: None,
                    cache_key: None,
                    cache_ttl: None,
                    cache_ttl_by_status: None,
                    minify: None,
                    mirage: None,
                    polish: None,
                    resolve_override: None,
                    scrape_shield: None,
                },
                method: worker::Method::Post,
                redirect: RequestRedirect::Manual,
            },
        )
        .unwrap(),
    )
    .await
    .unwrap();

    let parsed = serde_json::from_str::<MessageList>(&response).unwrap();

    axum::Json(parsed)
}

#[worker::send]
pub async fn root(
    axum::extract::State(AppState { env }): axum::extract::State<AppState>,
) -> impl axum::response::IntoResponse {
    let response = fetch_chatroom(
        env,
        worker::Request::new("http://fake-host/", worker::Method::Get).unwrap(),
    )
    .await
    .unwrap();

    axum::Json(response)
}

#[allow(dead_code)]
#[durable_object]
pub struct Chatroom {
    messages: Vec<Message>,
    env: Env,
    state: State,
}

#[durable_object]
impl DurableObject for Chatroom {
    fn new(state: State, env: Env) -> Self {
        Self {
            messages: vec![],
            env,
            state,
        }
    }

    async fn fetch(&mut self, mut req: worker::Request) -> Result<Response> {
        let path = req.path();

        if path == "/" {
            Response::from_json(&None::<()>)
        } else if path == "/messages" {
            match req.method() {
                worker::Method::Head => todo!(),
                worker::Method::Get => Response::from_json(&MessageList {
                    messages: self.messages.clone(),
                }),
                worker::Method::Post => {
                    let body = req.text().await.unwrap();
                    let new_message = serde_json::from_str::<Message>(body.as_str()).unwrap();
                    self.messages.push(new_message);
                    Response::from_json(&MessageList {
                        messages: self.messages.clone(),
                    })
                }
                worker::Method::Put => todo!(),
                worker::Method::Patch => todo!(),
                worker::Method::Delete => todo!(),
                worker::Method::Options => todo!(),
                worker::Method::Connect => todo!(),
                worker::Method::Trace => todo!(),
            }
        } else {
            todo!()
        }
    }
}

async fn fetch_chatroom(env: Arc<Env>, req: worker::Request) -> Result<String> {
    let chatroom = env.durable_object("CHATROOM")?;
    let stub = chatroom.id_from_name("CHATROOM")?.get_stub()?;

    let response = stub.fetch_with_request(req).await?.text().await?;

    Ok(response)
}
