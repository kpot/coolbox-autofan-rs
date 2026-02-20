use std::{
    io::{self},
    time::Duration,
};

use actix_web::{
    HttpResponse, Responder, get, post,
    web::{self},
};
use bytes::Bytes;
use serde_json::json;
use utoipa::ToSchema;

use super::autofan::CoolboxAutofan;

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
struct PlainMessage {
    #[schema(example = "service_mode=0")]
    text: String,
}

#[derive(serde::Serialize, serde::Deserialize, ToSchema)]
/// Returns current device information (firwmare, PCB version, etc.)
struct TempUpdate {
    /// JSON array of GPU core temperatures, one per GPU
    core_temp: Vec<i32>,
    /// JSON array of GPU VRAM temperatures, one per GPU
    mem_temp: Vec<i32>,

    /// Target GPU core temperature.
    target_core_temp: i32,
    /// Target GPU VRAM temperature.
    target_mem_temp: i32,

    /// Watchdog reset interval, in minutes. If not provided or 0, the watchdog will be turned off.
    watchdog_interval: Option<u32>,

    /// Manual fan speed. If not given, the speed will be chosen automatically.
    fan_speed: Option<u8>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
enum ApiReply {
    DeviceReply(String),
    Error(String),
}

fn command_reply_to_response(reply: io::Result<String>) -> HttpResponse {
    match reply {
        Ok(text) => HttpResponse::Ok().json(ApiReply::DeviceReply(text)),
        Err(e) => HttpResponse::InternalServerError().json(ApiReply::Error(e.to_string())),
    }
}

#[utoipa::path(
    responses(
        (status = 200, description = "Device's reply", body = ApiReply)
    )
)]
#[post("/fan-check")]
async fn fan_check(autofan: web::Data<CoolboxAutofan>) -> impl Responder {
    const FAN_CHECK_CMD: &[u8] = b"{\"fan_check\":1}";
    command_reply_to_response(autofan.send_command(FAN_CHECK_CMD))
}

#[utoipa::path(
    responses(
        (status = 200, description = "Returns diagnostic information", body = ApiReply)
    )
)]
#[post("/diagnostic")]
async fn diagnostic(autofan: web::Data<CoolboxAutofan>) -> impl Responder {
    const FAN_CHECK_CMD: &[u8] = b"{\"diagnostic\":1}";
    command_reply_to_response(autofan.send_command(FAN_CHECK_CMD))
}

#[utoipa::path(
    request_body(content = PlainMessage, examples(
        ("Service Mode ON" = (value=json!({"text": "service_mode=1" }))),
        ("Service Mode OFF" = (value=json!({"text": "service_mode=0" }))),
        ("Show current config" = (value=json!({"text": "show_config" }))),
        ("Reset all settings to default" = (value=json!({"text": "default"}))),
    )),
    responses(
        (status = 200, description = "Device's reply", body = ApiReply)
    )
)]
#[post("/message")]
async fn plain_message(
    message: web::Json<PlainMessage>,
    autofan: web::Data<CoolboxAutofan>,
) -> impl Responder {
    let message = json!({"message": message.text});
    let cmd = serde_json::to_string(&message).expect("The message must be valid");
    command_reply_to_response(autofan.send_command(cmd.as_bytes()))
}

#[utoipa::path(
    description = "Updates current GPU temperatures, their desired targets, controls fans and watchdog settings.",
    request_body(content = TempUpdate, examples(
        ("Set manual fan control" = (value=json!({
            "core_temp": [],
            "mem_temp": [],
            "target_core_temp": 60,
            "target_mem_temp": 70,
            "fan_speed": 70
        }))),
        ("Regular update for automatic temperature control" = (value=json!({
            "core_temp": [65, 66],
            "mem_temp": [80, 76],
            "target_core_temp": 70,
            "target_mem_temp": 90
        }))),
        ("Update temperatures for automatic fan control with the watchdog" = (value=json!({
            "core_temp": [63, 68],
            "mem_temp": [80, 85],
            "target_core_temp": 80,
            "target_mem_temp": 90,
            "watchdog_interval": 5,
        }))),
    )),
    responses(
        (status = 200, description = "Device's reply", body = ApiReply),
        (status = 422, description = "Something is wrong with the temperature arrays", body = ApiReply)
    )
)]
#[post("/update")]
async fn update(
    update: web::Json<TempUpdate>,
    autofan: web::Data<CoolboxAutofan>,
) -> impl Responder {
    let update = update.into_inner();
    const AUTO_FAN_MODE: i32 = 2;
    const MANUAL_FAN_MODE: i32 = 1;
    if update.core_temp.len() != update.mem_temp.len()
        && !update.core_temp.is_empty()
        && !update.mem_temp.is_empty()
    {
        return HttpResponse::UnprocessableEntity().json(ApiReply::Error(
            "Both arrays of core and VRAM temps, if provided, must be of the same size".into(),
        ));
    }
    let mut result_json_chunks = serde_json::Map::<String, serde_json::Value>::new();
    if !update.core_temp.is_empty() {
        result_json_chunks.insert("gpu_temp".into(), update.core_temp.into());
        if update.fan_speed.is_none() {
            result_json_chunks.insert("fan_mode".into(), AUTO_FAN_MODE.into());
        }
    }
    if !update.mem_temp.is_empty() {
        result_json_chunks.insert("gpu_mem".into(), update.mem_temp.into());
        if update.fan_speed.is_none() {
            result_json_chunks.insert("fan_mode".into(), AUTO_FAN_MODE.into());
        }
    }
    result_json_chunks.insert("target_temp".into(), update.target_core_temp.into());
    result_json_chunks.insert("target_mem".into(), update.target_mem_temp.into());
    match update.watchdog_interval {
        None | Some(0) => {
            result_json_chunks.insert("watchdog".into(), 0.into());
        }
        Some(..) => {
            result_json_chunks.insert("watchdog".into(), 1.into());
        }
    }
    result_json_chunks.insert("wd_reset_interval".into(), update.watchdog_interval.into());
    if let Some(fan_speed) = update.fan_speed {
        result_json_chunks.insert("fan_mode".into(), MANUAL_FAN_MODE.into());
        result_json_chunks.insert("manual_fan_speed".into(), fan_speed.into());
    } else {
        if !result_json_chunks.contains_key("fan_mode") {
            result_json_chunks.insert("fan_mode".into(), AUTO_FAN_MODE.into());
        }
        result_json_chunks.insert("manual_fan_speed".into(), 0.into());
    }
    let command_string = serde_json::to_string(&serde_json::Value::from(result_json_chunks))
        .expect("The object must be serializable");
    command_reply_to_response(autofan.send_command(command_string.as_bytes()))
}

#[utoipa::path(
    description = "Checks whether the service is up and running",
    responses(
        (status = 200, description = "Everything is OK."),
        (status = 500, description = "Unable to interact with the device."),
    )
)]
#[get("/health")]
async fn health(autofan: web::Data<CoolboxAutofan>) -> impl Responder {
    if autofan.is_listener_alive() {
        HttpResponse::Ok().json(json!({
            "device": autofan.device_path(),
            "status": "OK",
        }))
    } else {
        HttpResponse::InternalServerError().json(json!({
            "device": autofan.device_path(),
            "status": "ERROR",
            "error": "Unable to interact with the device",
        }))
    }
}

#[utoipa::path(
    description = "Streams raw, unfiltered device's output.",
    responses(
        (status = 200, description = "Constantly appending text streamed from the device.")
    )
)]
#[get("/watch")]
async fn watch(autofan: web::Data<CoolboxAutofan>) -> impl Responder {
    // This code receives broadcasts from the device listener and re-transmits it into
    // a channel, that acts as an asynchronous response stream.
    let (mut response_sender, output_stream) =
        futures::channel::mpsc::channel::<io::Result<Bytes>>(5);
    // Because the broadcasts come from a synchronous channel,
    // a dedicated thread is neccessary for the re-transmission.
    let mut receiver = autofan.subscribe();
    std::thread::spawn(move || {
        loop {
            // Receiving broadcasts from the device listener/reader,
            // regularly checking whether the client is still ready for the events.
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(msg) => {
                    let bytes: io::Result<Bytes> = Ok(Bytes::from(msg));
                    let result = response_sender.try_send(bytes);
                    if result.is_err() {
                        // Ther could be two different reasons for this error:
                        // either the connection has been closed (the client has disconnected),
                        // or the channel's queue is full (the client is too slow to receive).
                        // In both cases it's reasonable to just stop.
                        break;
                    }
                }
                Err(_timeout_error) => {
                    if response_sender.is_closed() {
                        // The connection is closed
                        break;
                    }
                }
            }
        }
    });

    HttpResponse::Ok()
        .content_type(actix_web::mime::APPLICATION_OCTET_STREAM)
        .insert_header(("X-Content-Type-Options", "nosniff"))
        .insert_header(actix_web::http::header::ContentEncoding::Identity)
        .streaming(output_stream)
}
