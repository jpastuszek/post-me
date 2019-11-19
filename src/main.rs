use cotton::prelude::*;
use canteen::*;
use canteen::utils;
use nix::ifaddrs::getifaddrs;
use nix::sys::socket::SockAddr;
use qrcode::QrCode;
use std::thread::spawn;

const UPLOAD_FORM: &str = r##"
</head>
<body>
    <form id="uploadbanner" enctype="multipart/form-data" method="post" action="#">
        <textarea rows="8" cols="80" name="address" id="address"></textarea>
        <input name="file" type="file" />
        <input type="submit" value="submit" id="submit" />
    </form>
</body>
"##;

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

    /// Listen on this port
    #[structopt(short, long, default_value = "16333")]
    port: u16,
}

fn upload_form(_req: &Request) -> Response {
    let mut res = Response::new();

    res.set_status(200);
    res.set_content_type("text/html");
    res.append(UPLOAD_FORM);

    res
}

fn handle_upload(req: &Request) -> Response {
    let mut res = Response::new();

    let body = String::from_utf8_lossy(&req.payload);

    let marker = body.split("\r\n").next().unwrap();
    let marker_split = format!("{}\r\n", marker);
    let marker_end = format!("{}--\r\n", marker);
    let parts = body
        .trim_end_matches(&marker_end).split(&marker_split).filter(|s| !s.is_empty())
        .map(|p| {
            let mut head_body = p.split("\r\n\r\n");
            let head = head_body.next().unwrap();
            let body = head_body.next().filter(|b| !b.trim().is_empty()).map(|b| b.replace("\r\n", "\n"));
            (head, body)
        })
        .flat_map(|(_head, body)| body);

    for body in parts {
        println!("{}", body);
    }

    res.set_status(200);
    res.set_content_type("text/html");
    res.append(THANK_YOU);

    res
}

fn qr(bytes: &[u8]) -> Result<String> {
    let code = QrCode::new(bytes)?;
    Ok(code.render::<char>()
        //.quiet_zone(false)
        .module_dimensions(2, 1)
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

    for addr in inet_addresses {
        info!("Found address: {:?}", addr);

        let url = format!("http://{}:{}/", addr, port);
        eprintln!("URL: {}", url);

        let code = qr(url.as_bytes())?;
        eprintln!("{}", code);

        thread = Some(spawn(move || {
            let mut cnt = Canteen::new();

            // bind to the listening address
            cnt.bind((addr, port));

            // set the default route handler to show a 404 message
            cnt.set_default(utils::err_404);

            cnt.add_route("/", &[Method::Get], upload_form);
            cnt.add_route("/", &[Method::Post], handle_upload);

            cnt.run();
        }));
    }

    thread.map(|t| t.join());

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
