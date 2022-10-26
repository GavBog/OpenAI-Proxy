use actix_web::{get, web, App, HttpServer, Responder};
use dotenvy::dotenv;
use hyper::body::Buf;
use hyper::{header, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize)]
struct OpenAiChoices {
    text: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoices>,
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    prompt: String,
    max_tokens: u32,
    temperature: f32,
    top_p: f32,
    frequency_penalty: f32,
    presence_penalty: f32,
    echo: bool,
}

#[tokio::main]
async fn response(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    dotenv().ok();
    let https = HttpsConnector::new();
    let client = Client::builder().build(https);
    let uri = "https://api.openai.com/v1/completions";

    let openai_key = std::env::var("OpenAI_Key").expect("The Api Key must be set.");
    let auth_header = format!("Bearer {}", openai_key);
    let prelude = "Finish the Following: ";

    let openai_request = OpenAiRequest {
        model: "text-davinci-002".to_string(),
        prompt: format!("{}{}", prelude, prompt),
        max_tokens: 50,
        temperature: 0.6,
        top_p: 1.0,
        frequency_penalty: 0.5,
        presence_penalty: 0.5,

        // Repeat the prompt in the response
        echo: false,
    };

    let body = Body::from(serde_json::to_vec(&openai_request)?);

    let req = Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, &auth_header)
        .body(body)
        .unwrap();

    let res = client.request(req).await?;

    let body = hyper::body::aggregate(res).await?;

    let json: OpenAiResponse = serde_json::from_reader(body.reader())?;

    return Ok(json.choices[0].text.clone());
}

#[get("/{prompt}")]
async fn completion(prompt: web::Path<String>) -> impl Responder {
    tokio::task::spawn_blocking(move || format!("{}", response(&prompt).unwrap()))
        .await
        .expect("Task panicked")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(|| async { "Epic AI Magic" }))
            .service(completion)
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
