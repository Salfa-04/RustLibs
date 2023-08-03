//!
//! 这是一个简易的略有性能的轻量级服务器
//!

mod thread_limit;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::panic::UnwindSafe;
use self::thread_limit::ThreadLimit;

///
/// 服务器实例结构体
///
/// 用于储存 **线程（thread）** 和 **监听（listener）** 信息
///
/// - thread: ThreadLimit
/// - listener: TcpListener
///
/// **Example:**
/// ```
/// mod salfa_server;
/// use salfa_server::SalServer;
/// ```
///
pub struct SalServer {
    thread: ThreadLimit,
    listener: TcpListener,
}

impl SalServer {

    ///
    /// 创建一个新的 `SalServer` 实例
    ///
    /// 参数：
    /// - bind_path: 绑定地址，如：127.0.0.1:80
    /// - thread: 线程数量，注意不能为0
    ///
    /// 返回一个新的 `SalServer` 结构体
    ///
    /// **Example:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("0.0.0.0:4998", 16);
    /// ```
    ///
    pub fn new(bind_path: &str, thread: usize) -> SalServer {
        let thread = ThreadLimit::new(thread);
        let listener = TcpListener::bind(bind_path).expect("Error: Couldn't bind port!");
        SalServer { thread, listener }
    }

    ///
    /// 为服务提供路由，并提供服务
    ///
    /// 参数：
    /// - route: 路由函数
    ///
    /// 使用该方法，需要定义一个特殊函数：
    /// ```
    /// fn route(http_line: (&str, &str), header: Vec<(&str, &str)>, body: &str) -> Vec<u8> {}
    /// ```
    /// 该函数的 `http_line` `header` `body` 参数由 `route` 方法提供
    ///     - http_line: (method: &str, path: &str)
    ///
    /// **Example1:**
    /// ```
    /// mod salfa_server;
    /// use std::collections::HashMap;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:4998", 16);
    /// serv.route(|http_line: (&str, &str), _header: HashMap<&str, &str>, _body: &str| {
    ///     let (method, _path) = http_line;
    ///
    ///     let buf = Vec::from(
    ///         "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n"
    ///     );
    ///     return buf;
    /// });
    /// ```
    ///
    /// **Example 2:**
    /// ```
    /// mod salfa_server;
    /// use std::collections::HashMap;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:4998", 16);
    /// server.route(route);
    ///
    /// fn route(_http_line: (&str, &str), _headers: HashMap<&str, &str>, body: &str) -> Vec<u8> {
    ///     let resp = format!("HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{}", body);
    ///     Vec::from(resp)
    /// }
    /// ```
    ///
    /// > 注意，常见的HTTP方法有：
    /// `GET POST PUT HEAD DELETE OPTIONS PATCH CONNECT TRACE`
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn route<F: FnOnce((&str, &str), HashMap<&str, &str>, &str) -> Vec<u8> + Send + 'static + UnwindSafe + Copy>(&self, route: F) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(x) => {
                    self.thread.execute(move || Self::handler(x, route));
                },
                Err(x) => {
                    eprintln!("Error: {}", &x);
                    continue;
                },
            };
        }
    }

    fn handler<F: FnOnce((&str, &str), HashMap<&str, &str>, &str) -> Vec<u8>>(stream: TcpStream, route: F) {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let Ok(buffer) = reader.fill_buf() else {
            Self::return_error(&mut writer, "Fail to Fill Buffer!");
            return;
        };

        let buffer = String::from_utf8_lossy(buffer);
        let Some((headers, body)) = buffer.split_once("\r\n\r\n") else {
            Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            return;
        };

        let mut headers = headers.lines();

        let Some(http_line) = headers.next() else {
            Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            return;
        };

        let http_line: Vec<&str> = http_line.split_whitespace().collect();
        let [method, path, _] = http_line[..] else {
            Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            return;
        };

        let mut head = HashMap::new();
        for header in headers {
            if let Some(place) = header.find(':') {
                let key = header[..place].trim();
                let value = header[place+1..].trim();
                head.insert(key, value);
            };
        };

        if buffer.is_empty() { // 判断读取是否成功
            Self::return_error(&mut writer, "Empty Buffer!");
        } else { // 若读取成功
            if let Err(x) = writer.write(&route((method, path), head, body)) {
                Self::return_error(&mut writer, x.to_string().as_str());
            };
        };
    }

    ///
    /// 为服务提供路由，并提供服务
    ///
    /// 参数：
    /// - route: 路由函数
    ///
    /// 使用该方法，需要定义一个特殊函数：
    /// ```
    /// fn route(buffer: &[u8]) -> &[u8] {}
    /// ```
    /// 该函数的 `buffer` 参数由 `route_pro` 方法提供
    ///
    /// **Example1:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:4998", 16);
    /// server.route_pro(|buffer| {
    ///     let mut buf = Vec::from(
    ///         "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n"
    ///     );
    ///     buf.extend_from_slice(buffer);
    ///     return buf;
    /// });
    /// ```
    ///
    /// **Example 2:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:4998", 16);
    /// server.route_pro(route);
    ///
    /// fn route(_buffer: &[u8]) -> Vec<u8> {
    ///     Vec::from("HTTP/1.1 200 OK\r\n\r\n")
    /// };
    /// ```
    ///
    /// > 注意，常见的HTTP方法有：
    /// `GET POST PUT HEAD DELETE OPTIONS PATCH CONNECT TRACE`
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn route_pro<F: FnOnce(&[u8]) -> Vec<u8> + Send + 'static + UnwindSafe + Copy>(&self, route: F) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(x) => {
                    self.thread.execute(move || Self::handler_pro(x, route));
                },
                Err(x) => {
                    eprintln!("Error: {}", &x);
                    continue;
                },
            };
        };
    }

    fn handler_pro<F: FnOnce(&[u8]) -> Vec<u8>>(stream: TcpStream, route: F) {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let Ok(buffer) = reader.fill_buf() else {
            Self::return_error(&mut writer, "Fail to Fill Buffer!");
            return;
        };

        if buffer.is_empty() { // 判断读取是否成功
            Self::return_error(&mut writer, "Empty Buffer!");
        } else { // 若读取成功
            if let Err(x) = writer.write(&route(buffer)) {
                Self::return_error(&mut writer, x.to_string().as_str());
            };
        };
    }

    fn return_error(writer: &mut BufWriter<&TcpStream>, err: &str) {
        let mut res = String::from(
            "HTTP/1.1 520 LOVE YOU\r\n\
            Content-Type: text/plain; charset=utf-8\r\n\
            Connection: close\r\n\r\nPlease Reload:\r\n"
        );
        res.push_str(err); // 构建应答信息
        if let Err(x) = writer.write(res.as_bytes()) {
            eprintln!("Send Failure: {}\r\n\tFOR: {x}", err);
        };
    }

}
