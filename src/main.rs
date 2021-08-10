use cotton::prelude::*;
use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr;
use qr2term::print_qr;
use async_std::task;
use tide::{Request, Response};
use http_types::{mime, StatusCode};
use serde::Deserialize;
use tide_rustls::TlsListener;
use rustls::{ServerConfig, NoClientAuth, Certificate, PrivateKey};
use std::sync::Arc;
use rcgen::generate_simple_self_signed;

// for application/x-www-form-urlencoded
#[derive(Debug, Deserialize)]
struct Form {
    message: String,
}

const UPLOAD_FORM: &str = r##" </head>
<body>
    <form id="uploadbanner" enctype="multipart/form-data" method="post" action="#">
        <textarea rows="8" cols="60" name="message"></textarea>
        <br/>
        <input type="submit" value="submit" formenctype="application/x-www-form-urlencoded" />
    </form>
</body>
"##;

//TODO: try https://docs.rs/formdata/0.13.0/formdata/ to parse for file support

// <input name="file" type="file" />
// <br/>

const THANK_YOU: &str = r##"
</head>
<body>
    Thank you!
</body>
"##;

// https://docs.rs/structopt/0.3.2/structopt/index.html#how-to-derivestructopt
/// Does stuff
#[derive(Debug, StructOpt)]
struct Cli {
    #[structopt(flatten)]
    logging: LoggingOpt,

    #[structopt(flatten)]
    dry_run: DryRunOpt,

    /// Does not use TLS
    #[structopt(short = "i", long)]
    insecure: bool,

    /// Listen on this port
    #[structopt(short, long, default_value = "16333")]
    port: u16,
}

async fn error_not_found(_req: Request<()>) -> tide::Result {
    Ok(Response::builder(StatusCode::NotFound)
        .body("Not found!")
        .content_type(mime::PLAIN)
        .build())
}

async fn upload_form(_req: Request<()>) -> tide::Result {
    Ok(Response::builder(200)
        .body(UPLOAD_FORM)
        .content_type(mime::HTML)
        .build())
}

async fn handle_upload(mut req: Request<()>) -> tide::Result {
    //dbg![&req.body_string().await];

    let form: Form = req.body_form().await?;
    println!("{}", form.message);

    Ok(Response::builder(200)
        .body(THANK_YOU)
        .content_type(mime::HTML)
        .build())
}

fn main() -> FinalResult {
    let args = Cli::from_args();
    init_logger(&args.logging, vec![module_path!()]);

    let inet_addresses = getifaddrs()?
        .flat_map(|ifaddr| ifaddr.address)
        .flat_map(|addr| if let SockAddr::Inet(inet_addr) = addr {
            Some(inet_addr)
        } else {
            None
        })
        .map(|inet_addr| inet_addr.to_std().ip())
        .filter(|inet_addr| !inet_addr.is_loopback())
        .filter(|inet_addr| inet_addr.is_ipv4()); // bind does not work for v6

    let mut thread = None;
    let port = args.port;
    let insecure = args.insecure;

    for addr in inet_addresses {
        info!("Found address: {:?}", addr);

        let scheme = if insecure {
            "http"
        } else {
            "https"
        };

        let url = format!("{}://{}:{}/", scheme, addr, port);

        eprintln!("URL: {}", url);

        //TODO: this prints to stdout and I need stderr so stdout can be redirected
        print_qr(&url)?;
        eprintln!();

        thread = Some(task::spawn(async move {
            let mut app = tide::new();

            app.at("/").get(upload_form);
            app.at("/").post(handle_upload);
            app.at("*").all(error_not_found);

            if insecure {
                app.listen((addr.to_string(), port)).await
            } else {
                let addr = addr.to_string();
                let cert = generate_simple_self_signed(vec![addr.clone()]).or_failed_to("Generate self-signed certificate");
                let cert_der = Certificate(cert.serialize_der().unwrap());
                let key_pair_der = PrivateKey(cert.get_key_pair().serialize_der());
                let mut config = ServerConfig::new(Arc::new(NoClientAuth));
                config.set_single_cert(vec![cert_der], key_pair_der).unwrap();

                app.listen(
                    TlsListener::build()
                    .addrs((addr, port))
                    .config(config)
                    // .cert(std::env::var("TIDE_CERT_PATH").unwrap())
                    // .key(std::env::var("TIDE_KEY_PATH").unwrap()),
                ).await
            }
        }));
    }

    thread.map(|t| task::block_on(t).or_failed_to("bind server"));

    error!("No non-local IPs to bind to found!");

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
