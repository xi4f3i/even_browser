#include "url.h"

#include <arpa/inet.h>
#include <fmt/base.h>
#include <fmt/format.h>
#include <netdb.h>
#include <openssl/bio.h>
#include <openssl/err.h>
#include <openssl/ssl.h>
#include <sys/socket.h>
#include <unistd.h>
#include <algorithm>
#include <map>
#include <memory>
#include <sstream>

constexpr auto HTTP = "http";
constexpr auto HTTPS = "https";
constexpr auto SLASH = "/";
constexpr auto COLON = ":";

void error(std::string_view msg) {
    fmt::println(stderr, "Error: {}", msg);
    exit(1);
}

URL::URL(std::string url) {
    try {
        auto scheme_end = url.find("://");
        if (scheme_end == std::string::npos) throw std::runtime_error("Invalid url");

        this->scheme = url.substr(0, scheme_end);
        if (this->scheme != HTTP && this->scheme != HTTPS)
            throw std::runtime_error("Unsupported scheme");

        url = url.substr(scheme_end + 3);
        auto slash_pos = url.find(SLASH);
        if (slash_pos == std::string::npos) {
            url = url + SLASH;
            slash_pos = url.find(SLASH);
        }

        this->host = url.substr(0, slash_pos);
        this->path = url.substr(slash_pos);

        if (this->scheme == HTTP) {
            this->port = 80;
        } else if (this->scheme == HTTPS) {
            this->port = 443;
        }

        auto colon_pos = this->host.find(COLON);
        if (colon_pos != std::string::npos) {
            this->port = std::stoi(host.substr(colon_pos + 1));
            this->host = host.substr(0, colon_pos);
        }

    } catch (std::runtime_error& err) {
        fmt::println("Malformed URL found, falling back to the WBE home page.");
        fmt::println("  URL was: {}", url);
        fmt::println("  Warn: {}", err.what());
        *this = URL("https://browser.engineering");
    }
}

std::string to_lower(const std::string& str) {
    std::string lower = str;
    std::transform(lower.begin(), lower.end(), lower.begin(), ::tolower);
    return lower;
}

using SSL_ptr = std::unique_ptr<SSL, decltype(&SSL_free)>;
using SSL_CTX_ptr = std::unique_ptr<SSL_CTX, decltype(&SSL_CTX_free)>;

struct Socket {
    int fd = -1;
    Socket() = default;
    explicit Socket(int f) : fd(f) {}
    ~Socket() {
        if (fd != -1) close(fd);
    }
    Socket(const Socket&) = delete;
    Socket& operator=(const Socket&) = delete;
    Socket(Socket&& other) noexcept : fd(other.fd) { other.fd = -1; }
    Socket& operator=(Socket&& other) noexcept {
        if (this != &other) {
            if (fd != -1) close(fd);
            fd = other.fd;
            other.fd = -1;
        }
        return *this;
    }
    operator int() const { return fd; }
};

std::string URL::request() {
    struct addrinfo hints{}, *res;
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;

    if (getaddrinfo(host.c_str(), std::to_string(port).c_str(), &hints, &res) != 0) {
        throw std::runtime_error("DNS resolution failed");
    }
    std::unique_ptr<addrinfo, decltype(&freeaddrinfo)> res_ptr(res, freeaddrinfo);

    Socket sock;
    struct addrinfo* ptr = res;
    for (; ptr != nullptr; ptr = ptr->ai_next) {
        int temp_fd = socket(ptr->ai_family, ptr->ai_socktype, ptr->ai_protocol);
        if (temp_fd < 0) continue;

        if (connect(temp_fd, ptr->ai_addr, ptr->ai_addrlen) == 0) {
            sock = Socket(temp_fd);
            break;
        }
        close(temp_fd);
    }

    if (sock.fd == -1) {
        throw std::runtime_error("Connection failed (all addresses attempted)");
    }

    SSL_CTX_ptr ctx(nullptr, SSL_CTX_free);
    SSL_ptr ssl(nullptr, SSL_free);

    if (scheme == "https") {
        ctx.reset(SSL_CTX_new(TLS_client_method()));
        if (!ctx) throw std::runtime_error("Failed to create SSL Context");

        SSL_CTX_set_default_verify_paths(ctx.get());
        SSL_CTX_set_verify(ctx.get(), SSL_VERIFY_PEER, nullptr);

        ssl.reset(SSL_new(ctx.get()));
        if (!ssl) throw std::runtime_error("Failed to create SSL object");

        if (SSL_set_fd(ssl.get(), sock.fd) != 1) {
            throw std::runtime_error("Failed to bind SSL to socket");
        }

        SSL_set_tlsext_host_name(ssl.get(), host.c_str());

        if (SSL_set1_host(ssl.get(), host.c_str()) != 1) {
            throw std::runtime_error("Certificate hostname verification setup failed");
        }

        if (SSL_connect(ssl.get()) <= 0) {
            throw std::runtime_error("SSL Handshake failed");
        }

        if (SSL_get_verify_result(ssl.get()) != X509_V_OK) {
            throw std::runtime_error("Certificate verification failed");
        }
    }

    std::string request = fmt::format("GET {} HTTP/1.0\r\nHost: {}\r\n\r\n", path, host);
    if (scheme == HTTPS) {
        if (SSL_write(ssl.get(), request.c_str(), request.length()) <= 0) {
            throw std::runtime_error("SSL write failed");
        }
    } else {
        if (send(sock.fd, request.c_str(), request.length(), 0) < 0) {
            throw std::runtime_error("Socket send failed");
        }
    }

    std::stringstream response_stream;
    char buffer[4096];
    int bytes_read;

    while (true) {
        if (scheme == "https") {
            bytes_read = SSL_read(ssl.get(), buffer, sizeof(buffer));
        } else {
            bytes_read = recv(sock.fd, buffer, sizeof(buffer), 0);
        }

        if (bytes_read <= 0) break;
        response_stream.write(buffer, bytes_read);
    }

    std::string full_response = response_stream.str();
    std::istringstream reader(full_response);
    std::string line;

    if (!std::getline(reader, line)) return "";
    if (!line.empty() && line.back() == '\r') line.pop_back();

    size_t first_space = line.find(' ');
    size_t second_space = line.find(' ', first_space + 1);
    std::string version = line.substr(0, first_space);
    std::string status = line.substr(first_space + 1, second_space - (first_space + 1));
    std::string explanation = line.substr(second_space + 1);
    fmt::println("version: {}, status: {}, explanation: {}", version, status, explanation);

    std::map<std::string, std::string> response_headers;
    while (std::getline(reader, line)) {
        if (!line.empty() && line.back() == '\r') line.pop_back();
        if (line.empty()) break;

        size_t colon_pos = line.find(":");
        if (colon_pos != std::string::npos) {
            std::string key = to_lower(line.substr(0, colon_pos));
            std::string value = line.substr(colon_pos + 1);

            size_t val_start = value.find_first_not_of(" \t");
            if (val_start != std::string::npos) value = value.substr(val_start);

            response_headers[key] = value;
        }
    }

    if (response_headers.find("transfer-encoding") != response_headers.end()) {
        throw std::runtime_error("Unsupported response header: transfer-encoding");
    }
    if (response_headers.find("content-encoding") != response_headers.end()) {
        throw std::runtime_error("Unsupported response header: content-encoding");
    }

    std::string content((std::istreambuf_iterator<char>(reader)), std::istreambuf_iterator<char>());

    return content;
}