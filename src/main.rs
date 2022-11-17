use actix_web::{get, web, App, HttpRequest, HttpServer};
use dotenvy::dotenv;
use hyper::body::Buf;
use hyper::{header, Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_derive::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

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

#[get("/{prompt}")]
async fn index(
    request: HttpRequest,
    prompt: web::Path<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    dotenv().ok();
    let req_headers = request.headers();
    if req_headers.get(header::AUTHORIZATION).is_none() {
        return Ok(format!("Auth token has not been set. Try logging in."));
    }
    let auth_header = req_headers
        .get(header::AUTHORIZATION)
        .unwrap()
        .to_str()
        .unwrap();
    let id = auth_header.replace("Bearer ", "");

    let database_url = std::env::var("DATABASE_URL").expect("The Database Url must be set.");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let userdata = sqlx::query("SELECT * FROM users WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await;
    if userdata.is_err() {
        return Ok(format!("User not found! Try logging in again."));
    }

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

    return Ok(format!("{}", json.choices[0].text.clone()));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(|| async { "Epic AI Magic" }))
            .service(index)
    })
    .bind(("0.0.0.0", 80))?
    .run()
    .await
}
