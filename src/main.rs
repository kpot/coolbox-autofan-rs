use std::io::{self};

use actix_web::{
    App, HttpServer,
    middleware::{self},
    web::{self},
};
use argh::FromArgs;
use utoipa::OpenApi;
use utoipa_actix_web::AppExt;
use utoipa_swagger_ui::SwaggerUi;

mod api;
mod autofan;
use autofan::CoolboxAutofan;

/// Coolbox Autofan Pro controller with REST API. Tested on firmware 1271 and PCB 1031.
#[derive(FromArgs, Debug)]
struct WebCli {
    /// serial port of the Coolbox Autofan board. Default: "/dev/ttyUSB0"
    #[argh(option, short = 'c', default = "\"/dev/ttyUSB0\".to_string()")]
    coolbox_port: String,

    /// REST API host. Default: 127.0.0.1
    #[argh(option, short = 'h', default = "\"127.0.0.1\".to_string()")]
    api_host: String,

    /// REST API port. Default: 65231
    #[argh(option, short = 'p', default = "65231")]
    api_port: u16,

    /// a dummy mode, when a fake is used instead of a real device
    #[argh(switch, short = 'd')]
    dummy: bool,
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    let cli: WebCli = argh::from_env();
    env_logger::init();

    #[derive(OpenApi)]
    #[openapi(
        tags(
            (name = "coolbox_rs", description="Coolbox autofan REST API")
        )
    )]
    struct ApiDoc;

    let autofan = web::Data::new(if cli.dummy {
        CoolboxAutofan::dummy()?
    } else {
        CoolboxAutofan::try_from(cli.coolbox_port.clone()).map_err(|e| {
            io::Error::other(
                format!(
                    "Unable to open terminal {}: {}",
                    &cli.coolbox_port,
                    e
                ),
            )
        })?
    });
    log::info!("Connected to the coolbox tty port {}", &cli.coolbox_port);

    log::info!(
        "Launching REST API at http://{host}:{port}",
        host = cli.api_host,
        port = cli.api_port
    );
    HttpServer::new(move || {
        let autofan_clone = autofan.clone();
        let api_service = utoipa_actix_web::scope("/api").configure(
            |config: &mut utoipa_actix_web::service_config::ServiceConfig| {
                config
                    .app_data(autofan_clone)
                    .service(api::health)
                    .service(api::fan_check)
                    .service(api::plain_message)
                    .service(api::update)
                    .service(api::diagnostic)
                    .service(api::watch);
            },
        );

        App::new()
            .into_utoipa_app()
            .openapi(ApiDoc::openapi())
            .map(|app| app.wrap(middleware::Logger::default()))
            .service(api_service)
            .openapi_service(|api| SwaggerUi::new("/docs/{_:.*}").url("/openapi.json", api))
            .into_app()
    })
    .bind((cli.api_host, cli.api_port))?
    .workers(2)
    .run()
    .await
}
