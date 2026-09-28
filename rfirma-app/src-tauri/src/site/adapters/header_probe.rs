//! Un servidor HTTP de una sola petición que devuelve el valor de `Authorization` que recibió.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;

/// `alice:s3cr3t` en base64, tal como sale en la cabecera.
pub(in crate::site::adapters) const ALICE_BASIC: &str = "Basic YWxpY2U6czNjcjN0";

/// Sirve una petición en el bucle local; devuelve `host:puerto/servicio` y el receptor de la cabecera.
pub(in crate::site::adapters) fn authorization_probe() -> (String, mpsc::Receiver<Option<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let address = listener.local_addr().expect("tiene direccion");
    let (sender, received) = mpsc::channel();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("llega la peticion");
        let mut reader = BufReader::new(stream);
        let mut authorization = None;
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("se lee la cabecera");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                match name.to_ascii_lowercase().as_str() {
                    "authorization" => authorization = Some(value.trim().to_owned()),
                    "content-length" => length = value.trim().parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
        let mut body = vec![0; length];
        reader.read_exact(&mut body).expect("se lee el cuerpo");
        reader
            .get_mut()
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            .expect("se contesta");
        sender.send(authorization).expect("la prueba escucha");
    });
    (format!("{address}/servicio"), received)
}

/// La URL con `alice:s3cr3t@` delante del host.
pub(in crate::site::adapters) fn with_alice(address_and_path: &str) -> String {
    format!("http://alice:s3cr3t@{address_and_path}")
}

pub(in crate::site::adapters) fn without_credentials(address_and_path: &str) -> String {
    format!("http://{address_and_path}")
}

/// Sirve una petición con `status` y `body`; devuelve la URL completa.
pub(in crate::site::adapters) fn rejecting_server(status: &str, body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("hay un puerto libre");
    let url = format!(
        "http://{}/servicio",
        listener.local_addr().expect("tiene direccion")
    );
    let answer = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("llega la peticion");
        let mut reader = BufReader::new(stream);
        let mut length = 0;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("se lee la cabecera");
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
        }
        let mut request_body = vec![0; length];
        reader
            .read_exact(&mut request_body)
            .expect("se lee el cuerpo");
        reader
            .get_mut()
            .write_all(answer.as_bytes())
            .expect("se contesta");
    });
    url
}
