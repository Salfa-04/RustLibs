//!
//! 一个基于 `PushPlus` 的微信信息推送方案
//!

use std::io::{BufRead as _, Write as _};
use std::io::{BufReader, BufWriter};
use std::{fmt, net::TcpStream};

pub use std::io::{Error, ErrorKind, Result};
const HOST: &str = "www.pushplus.plus:80";

///
/// Notice 通知数据结构体
///
/// 用于储存 ***PushPlus 的配置信息 (token, template, channel)***
///
/// **Example:**
/// ```
/// mod sal_notice;
/// use sal_notice::Notice;
/// ```
///
pub struct Notice<'a> {
    token: &'a str,
    template: Template,
    channel: Channel,
}

///
/// Response 数据结构体
///
/// 用于储存 请求返回的***数据 (code, msg, data)***
///
///     code: String
///     msg: String
///     data: String
///
/// > 数据来源于服务器的返回内容,
/// 具体信息请查询 `PushPlus` 官方文档
/// http://pushplus.plus/doc/guide/code.html
///
pub struct Response {
    pub code: String,
    pub msg: String,
    pub data: String,
}

///
/// Template 模板数据枚举
///
/// 用于储存 ***推送模板*** 信息
///
#[allow(dead_code)]
pub enum Template {
    HTML,
    TXT,
    JSON,
    MD,
}

///
/// Channel 渠道数据枚举
///
/// 用于储存 ***推送方式*** 信息
///
#[allow(dead_code)]
pub enum Channel {
    Wechat,
    Email,
}

impl<'a> Notice<'a> {
    ///
    /// 创建一个新的 `Notice` 实例
    ///
    /// 参数：
    /// - token: &str, PushPlus 的 token
    /// - template: Template, 模板枚举
    /// - channel: Channel， 渠道枚举
    ///
    /// 返回一个 `Notice` 结构体
    ///
    /// **Example:**
    /// ```
    /// mod sal_notice;
    /// use sal_notice::{Channel, Notice, Template};
    ///
    /// const TOKEN: &str = "dd1c8a......";
    ///
    /// let content = "Some Json Data......";
    ///
    /// let noter = Notice::new(
    ///     TOKEN,
    ///     Template::JSON,
    ///     Channel::Wechat,
    /// );
    ///
    /// let res = noter.send("Newest Data!!! 🤤", content.into()).unwrap();
    ///
    /// let client = HTTP::new(&head, Some(body));
    /// ```
    ///
    pub fn new(token: &'a str, template: Template, channel: Channel) -> Notice<'a> {
        Self {
            token,
            template,
            channel,
        }
    }

    ///
    /// 在构建完成之后发送数据
    ///
    /// 参数：
    /// - title: 所发送的标题
    /// - content: 所发送的内容
    ///
    /// 返回一个 `io::Result<Response>` 枚举
    /// - 成功：
    ///     - Ok(Response): Response
    /// - 失败：
    ///     - Err(io::Error): io::ErrorKind 枚举
    ///
    /// **Example:**
    /// ```
    /// mod sal_notice;
    /// use sal_notice::{Channel, Notice, Template};
    ///
    /// const TOKEN: &str = "dd1c8a......";
    ///
    /// let content = "Some Json Data......";
    ///
    /// let noter = Notice::new(
    ///     TOKEN,
    ///     Template::JSON,
    ///     Channel::Wechat,
    /// );
    ///
    /// let res = noter.send("Newest Data!!! 🤤", content.into()).unwrap();
    ///
    /// ```
    ///
    /// *请注意：该方法会阻塞运行！*
    ///
    pub fn send<'f>(&self, title: &'f str, content: String) -> Result<Response> {
        let stream = TcpStream::connect(HOST)?;
        let mut reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);
        let _ = writer.write(self.structen(title, content).as_bytes())?;
        let _ = writer.flush()?;

        let buffer = reader.fill_buf()?.to_vec();

        let _ = drop(reader);
        let _ = drop(writer);
        let _ = drop(stream);

        let buffer = String::from_utf8_lossy(&buffer);
        let Some(fron) = buffer.find('{') else {
            return Err(Error::from(ErrorKind::InvalidData));
        };
        let Some(back) = buffer.find('}') else {
            return Err(Error::from(ErrorKind::InvalidData));
        };

        Self::handler(&buffer[fron + 1..back])
    }

    fn structen<'s>(&self, title: &'s str, content: String) -> String {
        let content = content.replace('\"', "\\\"");

        let data_body_json = format!(
            r#"{{"token":"{}","template":"{}","channel":"{}","title":"{}","content":"{}"}}"#,
            self.token, self.template, self.channel, title, content
        );

        format!(
            "POST /send HTTP/1.1\r\n\
            Host: www.pushplus.plus\r\n\
            User-Agent: Mozilla Curl Saloxy\r\n\
            Content-Type: application/json\r\n\
            Content-Length: {1}\r\n\r\n{0}",
            data_body_json,
            data_body_json.len()
        )
    }

    fn handler(buff: &str) -> Result<Response> {
        if buff.contains("code") && buff.contains("data") && buff.contains("msg") {
            let buff = buff.replace(' ', "");
            let mut code = String::new();
            let mut msg = String::new();
            let mut data = String::new();

            for buff in buff.split(",\"") {
                let buff = buff.replace('\"', "");
                let Some((key, val)) = buff.split_once(':') else {
                    return Err(Error::from(ErrorKind::InvalidData));
                };
                match key {
                    "code" => code = val.to_string(),
                    "msg" => msg = val.to_string(),
                    "data" => data = val.to_string(),
                    _ => {}
                };
            }

            Ok(Response { code, msg, data })
        } else {
            Err(Error::from(ErrorKind::InvalidData))
        }
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Response {{ code: {}, msg: {}, data: {} }}",
            self.code, self.msg, self.data
        )
    }
}

impl fmt::Display for Template {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            &Self::HTML => "html",
            &Self::TXT => "txt",
            &Self::JSON => "json",
            &Self::MD => "markdown",
        })
    }
}

impl fmt::Display for Channel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            &Self::Wechat => "wechat",
            &Self::Email => "mail",
        })
    }
}
