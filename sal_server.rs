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
    /// - bind_path: 绑定地址，如：127.0.0.1:8888
    /// - thread: 线程数量。注意不能为0，否则将***无限期阻塞***
    ///
    /// 返回一个新的 `SalServer` 结构体
    ///
    /// **Example:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("0.0.0.0:8888", 16);
    /// ```
    ///
    pub fn new(bind_path: &str, thread: usize) -> SalServer {
        let thread = ThreadLimit::new(thread);
        let listener = TcpListener::bind(bind_path).expect("Error: Couldn't bind port!");
        SalServer { thread, listener }
    }

    ///
    /// 为服务提供路由，并提供服务（原始方法）
    ///
    /// 参数：
    /// - route: 路由函数
    ///
    /// 使用该方法，需要定义一个特殊函数：
    /// ```
    /// fn route(buffer: Vec<u8>) -> (Vec<u8>, bool) {}
    /// ```
    /// 参数：
    /// - buffer: 每次请求的原始数据
    ///
    /// 返回一个元组 `(Vec<u8>, bool)`
    /// - Vec<u8>: 写入流数据所需的原始数据
    /// - bool: 是否保持持续连接 (`Keep-Alive`)
    ///
    /// 该函数的 `buffer` 参数由 `route_pro` 方法提供
    ///
    /// **Example1:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:8888", 16);
    /// server.route_pro(|buffer| {
    ///     let mut buf = Vec::from(
    ///         "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n"
    ///     );
    ///     buf.extend_from_slice(buffer);
    ///     return (buf, false);
    /// });
    /// ```
    ///
    /// **Example 2:**
    /// ```
    /// mod salfa_server;
    /// use salfa_server::SalServer;
    ///
    /// let server = SalServer::new("127.0.0.1:8888", 16);
    /// server.route_pro(route);
    ///
    /// fn route(buffer: Vec<u8>) -> (Vec<u8>, bool) {
    ///     (Vec::from("HTTP/1.1 200 OK\r\n\r\n"), true)
    /// };
    /// ```
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn route_pro<F: FnOnce(Vec<u8>) -> (Vec<u8>, bool) + Copy + Send + 'static + UnwindSafe>(&self, route: F) {
        for stream in self.listener.incoming() {
            if let Ok(stream) = stream {
                self.thread.execute(move || Self::handler_pro(stream, route));
            } else { continue; };
        };
    }

    fn handler_pro<F: FnOnce(Vec<u8>) -> (Vec<u8>, bool) + Copy>(stream: TcpStream, route: F) {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        loop {

            let (buffer, lens) = match reader.fill_buf() {
                Ok(x) => (x.to_vec(), x.len()),
                Err(e) => return Self::return_error(&mut writer, e.to_string().as_str()),
            };

            if buffer.is_empty() {
                return Self::return_error(&mut writer, "Empty Input!");
            };

            let (result, keep_alive) = route(buffer);

            if let Err(e) = writer.write(&result) {
                return Self::return_error(&mut writer, e.to_string().as_str());
            }; // 写出处理后的数据

            if keep_alive { // 将数据消耗，防止出现读取重复现象
                reader.consume(lens);
            } else { break; };

            if let Err(e) = writer.flush() {
                return Self::return_error(&mut writer, e.to_string().as_str());
            } // 立即将数据写出，避免出现无输出现象

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
    /// fn route(http_line: (&str, &str), head: HashMap<&str, &str>, body: &str) -> (Vec<u8>, bool) {}
    /// ```
    /// 参数：
    /// - http_line: HTTP请求的头行，包括 `method` `path` `version`
    ///     - method: 请求方法
    ///     - path: 请求路径
    ///     - version: HTTP版本，暂不提供
    /// - head: HTTP请求的头部信息 (Header)
    /// - body: 请求主体部分，承载信息
    ///
    /// 返回一个元组 `(Vec<u8>, bool)`
    /// - Vec<u8>: 写入流数据所需的*原始*数据
    /// - bool: 是否保持持续连接 (`Keep-Alive`)
    ///
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
    ///     (Vec::from("HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n"), false)
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
    /// fn route(http_line: (&str, &str), head: HashMap<&str, &str>, body: &str) -> (Vec<u8>, bool) {
    ///     let mut buf = Vec::from("HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n");
    ///     let buff = Vec::from(format!("Http Line: {:?}\r\nHead: {:#?}\r\nBody: {}\r\n", http_line, head, body));
    ///     buf.extend(buff);
    ///     return (buf, true)
    /// }
    /// ```
    ///
    /// > 注意，常见的HTTP方法有：
    /// `GET POST PUT HEAD DELETE OPTIONS PATCH CONNECT TRACE`
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn route_http<F: FnOnce((&str, &str), HashMap<&str, &str>, &str) -> (Vec<u8>, bool) + Send + 'static + UnwindSafe + Copy>(&self, route: F) {
        for stream in self.listener.incoming() {
            if let Ok(stream) = stream {
                self.thread.execute(move || Self::handler_http(stream, route));
            } else { continue; };
        };
    }

    fn handler_http<F: FnOnce((&str, &str), HashMap<&str, &str>, &str) -> (Vec<u8>, bool) + Copy>(stream: TcpStream, route: F) {
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        loop {

            let (buffer, lens) = match reader.fill_buf() {
                Ok(x) => (x, x.len()),
                Err(e) => return Self::return_error(&mut writer, &*e.to_string()),
            };

            if buffer.is_empty() {
                return Self::return_error(&mut writer, "Empty Input!");
            };

            let buffer = String::from_utf8_lossy(buffer);
            let Some((headers, body)) = buffer.split_once("\r\n\r\n") else {
                return Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            };

            let mut headers = headers.lines();
            let Some(http_line) = headers.next() else {
                return Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            };

            let http_line: Vec<&str> = http_line.split_whitespace().collect();
            let [method, path, _] = http_line[..] else {
                return Self::return_error(&mut writer, "Non-Standard HTTP Structure!");
            };

            let mut head = HashMap::new();
            for header in headers {
                if let Some(place) = header.find(':') {
                    let key = header[..place].trim();
                    let value = header[place+1..].trim();
                    head.insert(key, value);
                };
            };

            let (result, keep_alive) = route((method, path), head, body);

            if let Err(e) = writer.write(&result) {
                return Self::return_error(&mut writer, &*e.to_string());
            }; // 写出处理后的数据

            if keep_alive { // 将数据消耗，防止出现读取重复现象
                reader.consume(lens);
            } else { break; };

            if let Err(e) = writer.flush() {
                return Self::return_error(&mut writer, &*e.to_string());
            } // 立即将数据写出，避免出现无输出现象

        };

    }

    fn return_error(writer: &mut BufWriter<&TcpStream>, err: &str) {
        let mut res = String::from(
            "HTTP/1.1 520 LOVE YOU\r\n\
            Content-Type: text/plain; charset=utf-8\r\n\
            Connection: close\r\n\r\n"
        );
        res.extend([err, "\r\n"]); // 构建应答信息

        if let Err(e) = writer.write(res.as_bytes()) {
            eprintln!("Write Failure: {}\r\n\tFOR: {e}", err);
        };

        if let Err(e) = writer.flush() {
            eprintln!("Flush Failure: {}\r\n\tFOR: {e}", err);
        } // 立即将数据写出，避免出现无输出现象

    }

}
