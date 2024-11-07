use dotenv::dotenv;
use ollama_rs::{generation::completion::request::GenerationRequest, Ollama};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::Deserialize;
use std::{env, error::Error, process::Stdio};
use tokio::{process::Command, signal, sync::oneshot};

#[derive(Deserialize)]
struct WeatherResponse {
    current_weather: CurrentWeather,
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature: f32,
    weathercode: i32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().expect("Failed to load .env file");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let mut serve_process = Command::new("ollama")
        .arg("serve")
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Failed to start `ollama serve`");

    tokio::spawn(async move {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");

        let _ = shutdown_tx.send(());
    });

    let run_result = run().await;

    tokio::select! {
        _ = shutdown_rx => {
            println!("Shutdown signal received, terminating `ollama serve`...");
        },
        _ = async { if run_result.is_err() { Err(()) } else { Ok(()) } } => {
            println!("Application terminated, terminating `ollama serve`...");
        }
    }

    if serve_process.id().is_some() {
        let _ = serve_process.kill().await;
    }

    run_result
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let token = env::var("GOOD_MORNING_DISCORD_TOKEN")
        .map_err(env_var_error("GOOD_MORNING_DISCORD_TOKEN"))?;
    let channel_id =
        env::var("GOOD_MORNING_CHANNEL_ID").map_err(env_var_error("GOOD_MORNING_CHANNEL_ID"))?;

    let members = parse_members()?;
    let weather_info = get_weather()
        .await
        .unwrap_or_else(|_| "не удалось получить данные о погоде".to_string());

    let generated_message = generate_greeting(&members, &weather_info).await?;
    let final_message = format_message(&members, &generated_message);

    send_message(&token, &channel_id, &final_message).await
}

fn env_var_error(var: &str) -> impl Fn(env::VarError) -> String + '_ {
    move |e| format!("Failed to find {}: {}", var, e)
}

fn parse_members() -> Result<Vec<(String, u64)>, Box<dyn Error>> {
    env::var("GOOD_MORNING_MEMBERS")
        .map_err(|err| format!("Failed to read 'GOOD_MORNING_MEMBERS': {}", err).into())
        .map(|members_str| {
            members_str
                .split(',')
                .collect::<Vec<_>>()
                .chunks(2)
                .filter_map(|chunk| match chunk {
                    [name, id_str] => id_str.parse::<u64>().ok().map(|id| (name.to_string(), id)),
                    _ => None,
                })
                .collect()
        })
}

async fn get_weather() -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://api.open-meteo.com/v1/forecast?latitude=55.7558&longitude=37.6173&current_weather=true";
    let response: WeatherResponse = reqwest::get(url).await?.json().await?;

    Ok(format!(
        "{}°C, {}",
        response.current_weather.temperature,
        map_weather_code_to_description(response.current_weather.weathercode)
    ))
}

fn map_weather_code_to_description(code: i32) -> &'static str {
    match code {
        0 => "clear sky",
        1..=3 => "partly cloudy",
        45 | 48 => "fog",
        51..=57 => "drizzle",
        61..=67 => "rain",
        71..=77 => "snow",
        80..=82 => "showers",
        95 | 96 | 99 => "thunderstorm",
        _ => "unknown weather",
    }
}

async fn generate_greeting(
    members: &[(String, u64)],
    weather_info: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let ollama = Ollama::default();
    let model = "llama3".to_string();
    let prompt = format!(
        "Create a kawaii, uwu and cute morning greeting in Russian, including information about the weather for the day for: {}. Weather: {}. Include a suggestion on how to dress appropriately for the weather and etc. The response should be a direct greeting, without any explanations or additional details.",
        members.iter().map(|(name, _)| name).cloned().collect::<Vec<_>>().join(", "),
        weather_info
    );

    ollama
        .generate(GenerationRequest::new(model, prompt))
        .await
        .map(|response| response.response.trim().to_string())
        .map_err(|e| e.into())
}

fn format_message(members: &[(String, u64)], generated_message: &str) -> String {
    let mentions = members
        .iter()
        .map(|(_, id)| format!("<@{}>", id))
        .collect::<Vec<_>>()
        .join(" ");

    format!("{}\n{}", generated_message, mentions)
}

async fn send_message(
    token: &str,
    channel_id: &str,
    message: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = format!(
        "https://discord.com/api/v9/channels/{}/messages",
        channel_id
    );

    let headers = HeaderMap::from_iter([
        (AUTHORIZATION, HeaderValue::from_str(token)?),
        (CONTENT_TYPE, HeaderValue::from_static("application/json")),
    ]);

    let body = serde_json::json!({
        "content": message,
        "tts": false
    });

    reqwest::Client::new()
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await?
        .error_for_status()
        .map(|_| ())
        .map_err(|e| format!("Failed to send message: {}", e).into())
}
