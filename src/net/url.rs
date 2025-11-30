use native_tls::TlsConnector;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

pub struct URL {
    pub scheme: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl URL {
    pub fn new(url_str: &str) -> Self {
        match URL::parse_url(url_str) {
            Ok(url) => url,
            Err(_) => {
                eprintln!("Malformed URL found, falling back to the WBE home page.");
                eprintln!("  URL was: {}", url_str);
                URL {
                    scheme: "https".to_string(),
                    host: "browser.engineering".to_string(),
                    port: 443,
                    path: "/".to_string(),
                }
            }
        }
    }

    fn parse_url(url_str: &str) -> Result<Self, String> {
        let parts: Vec<&str> = url_str.split("://").collect();
        if parts.len() != 2 {
            return Err("Invalid URL scheme".to_string());
        }

        let scheme = parts[0].to_string();
        if scheme != "http" && scheme != "https" {
            return Err(format!("Unsupported scheme: {}", scheme));
        }

        let mut url = parts[1].to_string();

        if !url.contains('/') {
            url.push('/');
        }

        let host_path: Vec<&str> = url.splitn(2, '/').collect();
        let host_part = host_path[0].to_string();
        let path = "/".to_string() + host_path.get(1).unwrap_or(&"");

        let default_port = match scheme.as_str() {
            "http" => 80,
            "https" => 443,
            _ => 80,
        };

        let (host, port) = if host_part.contains(':') {
            let host_port: Vec<&str> = host_part.split(':').collect();
            if host_port.len() != 2 {
                return Err("Invalid host:port format".to_string());
            }
            let port = host_port[1]
                .parse::<u16>()
                .map_err(|e| format!("Invalid port number: {}", e))?;
            (host_port[0].to_string(), port)
        } else {
            (host_part, default_port)
        };

        Ok(URL {
            scheme,
            host,
            port,
            path,
        })
    }

    pub fn request(&self) -> String {
        let address = format!("{}:{}", self.host, self.port);
        let stream = TcpStream::connect(&address).unwrap();

        if self.scheme == "https" {
            let connector = TlsConnector::new().unwrap();
            let tls_stream = connector.connect(&self.host, stream).unwrap();
            self.handle_https_response(tls_stream)
        } else {
            self.handle_http_response(stream)
        }
    }

    fn handle_http_response(&self, mut stream: TcpStream) -> String {
        let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", self.path, self.host);
        stream.write_all(request.as_bytes()).unwrap();
        stream.flush().unwrap();

        let mut reader = BufReader::new(stream);
        self.parse_response(&mut reader)
    }

    fn handle_https_response(&self, mut tls_stream: native_tls::TlsStream<TcpStream>) -> String {
        let request = format!("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", self.path, self.host);
        use std::io::Write;

        tls_stream.write_all(request.as_bytes()).unwrap();
        tls_stream.flush().unwrap();

        let mut reader = BufReader::new(tls_stream);
        self.parse_response(&mut reader)
    }

    fn parse_response<T: Read>(&self, reader: &mut BufReader<T>) -> String {
        let mut status_line = String::new();
        reader.read_line(&mut status_line).unwrap();

        let parts: Vec<&str> = status_line.trim().splitn(3, ' ').collect();
        if parts.len() < 3 {
            return String::new();
        }

        let _version = parts[0];
        let _status = parts[1];
        let _explanation = parts[2];

        let mut headers = std::collections::HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();

            let trimmed = line.trim_end_matches(&['\r', '\n'][..]);
            if trimmed.is_empty() {
                break;
            }

            if let Some((key, value)) = trimmed.split_once(':') {
                headers.insert(key.trim().to_lowercase(), value.trim().to_string());
            }
        }

        assert!(!headers.contains_key("transfer-encoding"));
        assert!(!headers.contains_key("content-encoding"));

        let mut content = String::new();
        reader.read_to_string(&mut content).unwrap();

        content
    }
}

impl std::fmt::Display for URL {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "URL(scheme={}, host={}, port={}, path={:?})",
            self.scheme, self.host, self.port, self.path
        )
    }
}
