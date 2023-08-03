//!
//! 一个曲线救国的HTTP请求解决方案
//!

use std::process::Command;
use std::collections::HashMap;

///
/// HTTP数据结构体
///
/// 用于储存 **各种数据（head, body）**
///
/// - head: HashMap<String, String>
/// - body: Option<String>
///
/// **Example:**
/// ```
/// mod sal_http;
/// use sal_http::HTTP;
/// ```
///
pub struct HTTP {
    pub head: HashMap<String, String>,
    pub body: Option<String>,
}

impl HTTP {

    ///
    /// 创建一个新的 `cUrl` 实例
    ///
    /// 参数：
    /// - head: Http Header
    /// - body: Http Body
    ///
    /// 返回一个 `HTTP` 结构体
    ///
    /// **Example:**
    /// ```
    /// mod sal_http;
    /// use sal_http::HTTP;
    ///
    /// let head = [
    ///     ("Connection", "close"),
    ///     ("Content-Type", "application/json"),
    /// ];
    ///
    /// let body = String::from(
    ///   "{\
    ///         \"Hello\": \"World\",\
    ///     }"
    /// );
    ///
    /// let client = HTTP::new(&head, Some(body));
    /// ```
    ///
    pub fn new<T: ToString>(head: &[(T, T)], body: Option<String>) -> HTTP {

        let head = head.iter().map(
            |(k, v)| (k.to_string(), v.to_string())
        ).collect();

        HTTP { head, body }
    }

    ///
    /// 在构建完成之后发送数据
    ///
    /// 参数：
    /// - url: 想要请求的网络地址，***仅支持解析HTTP(s)请求***
    /// - method: 进行请求所需要的请求方式
    ///
    /// 返回一个 `Result` 枚举: `Result<(HTTP, String), (i32, String)>`
    /// - 成功：
    ///     - Ok((http, status_code)):
    ///         - http: `HTTP` 结构体
    ///         - status_code: http请求返回的状态码
    /// - 失败：
    ///     - Err(err_code, err_msg):
    ///         - err_code: 错误代码
    ///         - err_msg: 错误信息
    ///
    /// **Example:**
    /// ```
    /// mod sal_http;
    /// use sal_http::HTTP;
    ///
    /// let head = [
    ///     ("Connection", "close"),
    ///     ("Content-Type", "application/json"),
    /// ];
    ///
    /// let body = String::from(
    ///   "{\
    ///         \"Hello\": \"World\",\
    ///     }"
    /// );
    ///
    /// let url = "https://sal-server.fly.dev";
    ///
    /// let client = HTTP::new(&head, Some(body));
    /// let _ = client.send(url, "POST");
    /// ```
    ///
    /// > 注意，常见的HTTP方法有：
    /// `GET POST PUT HEAD DELETE OPTIONS PATCH CONNECT TRACE`
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn send(&self, url: &str, method: &str) -> Result<(HTTP, String), (i32, String)> {

        let mut args: Vec<String> = vec![String::from("-S")];

        for (key, val) in self.head.iter() {
            let temp = format!("{key}: {val}");
            args.extend([String::from("-H"), temp]);
        };

        if let Some(body) = &self.body {
            args.extend([String::from("--data"), body.clone()]);
        };

        Self::fetch(url, method, Some(args))
    }

    ///
    /// 初级方法，直接调用 `cUrl`
    ///
    /// 参数：
    /// - url: 想要请求的网络地址，***仅支持解析HTTP(s)请求***
    /// - method: 进行请求所需要的请求方式
    /// - args: 其他直接应用于 `cUrl` 的参数，如 `Some(["-S"])`
    ///
    /// 返回一个 `Result` 枚举: `Result<(HTTP, String), (i32, String)>`
    /// - 成功：
    ///     - Ok((http, status_code)):
    ///         - http: `HTTP` 结构体
    ///         - status_code: http请求返回的状态码
    /// - 失败：
    ///     - Err(err_code, err_msg):
    ///         - err_code: 错误代码
    ///         - err_msg: 错误信息
    ///
    ///
    /// **Example:**
    /// ```
    /// mod sal_http;
    /// use sal_http::HTTP;
    ///
    /// let url = "https://sal-server.fly.dev";
    ///
    /// let _ = HTTP::fetch(url, "GET", None::<&[&str]>);;
    ///
    /// ```
    ///
    /// > 注意，常见的HTTP方法有：
    /// `GET POST PUT HEAD DELETE OPTIONS PATCH CONNECT TRACE`
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn fetch<I, S>(url: &str, method: &str, args: Option<I>) -> Result<(HTTP, String), (i32, String)>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err((-1, String::from("Fail to Parse (Input)!")));
        };

        let out = { // Run cUrl...
            let mut curl = Command::new("curl");
            let curl = curl.args(["-SiX", method, url]);
            let curl = curl.args(["-A", "Saloxy Mozilla Curl"]);
            let curl = match args {
                Some(x) => curl.args(x),
                None => curl,
            };

            match curl.output() {
                Ok(x) => x,
                Err(x) => return Err((-4999, x.to_string())),
            }
        };

        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);

        if !out.status.success() {
            return Err((-3, stderr.trim().to_string()));
        }

        let (status_code, head, body) = {
            let Some((head, body)) = stdout.split_once("\r\n\r\n") else {
                return Err((-2, String::from("Fail to Parse (in)!")));
            };

            let mut head = head.lines();
            let Some(http_line) = head.next() else {
                return Err((-2, String::from("Fail to Parse (in)!")));
            };

            let http_line: Vec<&str> = http_line.split_whitespace().collect();
            let [_, status_code, ..] = *http_line else {
                return Err((-2, String::from("Fail to Parse (in)!")));
            };

            let head: HashMap<String, String> = head.map(
                |x| if let Some(place) = x.find(':') {
                    (x[..place].trim().to_string(), x[place+1..].trim().to_string())
                } else {
                    (x.trim().to_string(), String::new())
                }
            ).collect();

            let body = if body.len() != 0 {
                Some(body.to_string())
            } else {
                None
            };

            (status_code, head, body)
        };

        Ok((HTTP {
            body, head,
        }, status_code.to_string()))
    }

}
