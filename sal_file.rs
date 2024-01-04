//!
//! 超星云盘利用工具
//!

/* 如何获取 `token`:
 *
 * 浏览器登陆超星学习通帐号之后访问:
 * ```
 * https://pan-yz.chaoxing.com/api/token/uservalid
 * ```
 *
 * 即可取得 `_token`=> _token = b8bd***
 *
 */

use std::{
    io::{BufRead as _, Write as _},
    io::{BufReader, BufWriter},
    net::TcpStream,
};

pub use std::io::{Error, ErrorKind, Result};
const HOST_SCAN: &str = "pan-yz.chaoxing.com:80";
const HOST_LINK: &str = "sharewh.xuexi365.com:80";

///
/// `CloudFile` 实例结构体
///
/// 用于储存 **`puid`**, **`_token`**, **`fldid`**, **`filemap`** 等信息
///
/// - filemap: Vec<(String, String)>,
///     - file: (name, objid)
///
/// **Example 1:**
/// ```
/// mod sal_file;
///
/// use sal_file::CloudFile;
/// use std::fs::{read, write};
///
/// let path = "/root/test.bin";
///
/// let data = read(path)?;
/// let mut cloud = CloudFile::from_raw(&data)?;
///
/// let _filelist = cloud.get_filemap();
///
/// let raw: &[u8] = AsRef::as_ref(&cloud);
/// write(path, raw)?;
/// ```
///
/// **Example 2:**
/// ```
/// mod sal_file;
///
/// use sal_file::CloudFile;
/// use std::fs::{read, write};
///
/// let path = "/root/test.bin";
///
/// let mut cloud = CloudFile::new(
///     "29*******".into(),
///     "b8***391*******d3726f*******d0b2".into(),
///     "94***555*******592".into(),
///     &[127, 97, 112, 128],
/// )?;
///
/// cloud.set_stream(Stream::Scan)?;
/// while let Ok(_) = cloud.scan() {}
///
/// let _filelist = cloud.get_filemap();
///
/// write(path, &cloud)?;
/// ````
///
/// **Example 3:**
/// ```
/// mod sal_file;
///
/// use sal_file::{CloudFile, Stream};
/// use std::fs::{read, write};
/// let mut filer = CloudFile::new(
///     "29*******".into(),
///     "b8***391*******d3726f*******d0b2".into(),
///     "94***555*******592".into(),
///     &[127, 97, 112, 128],
/// )?;
///
/// filer.set_stream(Stream::Scan)?;
/// let mut counter = 0;
/// while let Ok(n) = filer.scan() {
///     counter += n;
/// }
///
/// let path = "/home/salfa/test.bin";
/// let data = read(path)?;
/// filer.extend_from_raw(&data)?;
/// write(path, &filer)?;
////
/// println!("扫描完成，新增{counter:03}项文件");
/// filer.set_stream(Stream::Link)?;
/// for (name, objid) in filer.get_filemap() {
///     let link = filer.get_link(objid)?;
///     println!("文件: {name}\r\n直链: {link}\r\n");
/// }
/// ```
///
#[allow(dead_code)]
pub struct CloudFile {
    inner: Vec<u8>,
    stream: Option<TcpStream>,

    uid: String,   // puid
    token: String, // _token
    dirid: String, // fldid

    filemap: Vec<(String, String)>, // filelist: (name, objid)
}

///
/// `TcpStream` 流控制枚举
///
/// 与 `CloudFile::set_stream` 配合使用
///
/// - Stream::Scan => 与超星云盘服务器连接
/// - Stream::Link => 与超星下载服务器连接
/// - Stream::None => 与服务器断开连接
///
#[allow(dead_code)]
pub enum Stream {
    Scan,
    Link,
    None,
}

impl AsRef<[u8]> for CloudFile {
    fn as_ref(&self) -> &[u8] {
        &self.inner
    }
}

#[allow(dead_code)]
impl CloudFile {
    ///
    /// 创建一个新的 `Cloudfile` 实例
    ///
    /// 参数：
    /// - uid: `String` 即 `puid`，用于与服务器交流时认证
    /// - token: `String` 即 `_token`，用于与服务器交流时认证
    /// - dirid: `String` 即 `fldid`，
    ///     - 用于与服务器交流时自定义根目录
    ///     - 若为空，则默认为账号根目录
    /// - passwd: `&[u8; 4]` 本地储存数据时所使用的密码
    ///     - 密码格式为 `&[u8; 4]`
    ///     - 每一位的范围为 `0..=128`
    ///     - 必须保证密码的行列式大于零
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(CloudFile)
    /// - Err(std::io::Error)
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use sal_file::CloudFile;
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    /// ```
    ///
    pub fn new(uid: String, token: String, dirid: String, passwd: &[u8; 4]) -> Result<CloudFile> {
        let mut data = vec![
            uid.as_bytes(),   // puid
            token.as_bytes(), // _token
            dirid.as_bytes(), // fldid
        ]
        .join(&[27u8][..]);
        while data.len() < 64 {
            data.push(0);
        }

        let data = Self::matrix_encode(passwd, &data)?;
        let data = &Self::sixteen_to_eight(&data);

        let mut inner = Vec::new();
        inner.extend_from_slice(&[3, 3, 4, 21, 7, 23, 10, 8]);
        inner.extend_from_slice(passwd);
        inner.extend_from_slice(&[25, 0, 0, 3]);
        inner.extend_from_slice(&data);

        Ok(Self {
            uid,
            token,
            dirid,
            inner,
            stream: None,
            filemap: Vec::new(),
        })
    }

    ///
    /// 读取文件并导入生成实例
    ///
    /// 参数：
    /// - raw_data: `&[u8]` 符合条件的二进制数据数组
    ///     - 必须是一个完整的可解密的实例备份文件
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(CloudFile)
    /// - Err(std::io::Error)
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use std::fs::read;
    /// use sal_file::CloudFile;
    ///
    /// let data = read("/root/test.bin")?;
    /// let cloud = CloudFile::from_raw(&data)?;
    /// ```
    ///
    pub fn from_raw(raw_data: &[u8]) -> Result<CloudFile> {
        if raw_data.len() < 144 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Len of Data to Short: [144..]",
            ));
        }

        let [day_rz, day_yy, passwd, _] = raw_data.chunks(4).take(4).collect::<Vec<&[u8]>>()[..]
        else {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown: {}", line!()),
            ));
        };

        if day_rz != &[3, 3, 4, 21] && day_yy != &[7, 23, 10, 8] {
            return Err(Error::new(
                ErrorKind::Unsupported,
                "Wrong File Type: Unsupported File Type",
            ));
        }

        let passwd: &[u8; 4] = match passwd.try_into() {
            Ok(x) => x,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Wrong Password Type: Unsupported Password Type",
                ))
            }
        };

        let data = Self::eight_to_sixteen(&raw_data[16..]);
        let data = Self::matrix_decode(&passwd, &data)?;
        let (base, list) = data.split_at(64); // len >= 64

        let mut base_data = [""; 3];
        let base = String::from_utf8_lossy(base);
        for (index, value) in base.splitn(3, '\u{1B}').enumerate() {
            base_data[index] = value.trim();
        }

        let mut list_res = Vec::new();
        if !list.is_empty() {
            for val in String::from_utf8_lossy(list).split('\u{1B}') {
                let [name, objid] = val.splitn(2, "\u{1A}").collect::<Vec<&str>>()[..] else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "Wrong File Data: Unsupported File Type",
                    ));
                };
                list_res.push((name.into(), objid.into()))
            }
        }

        Ok(Self {
            inner: raw_data.into(),
            uid: base_data[0].into(),
            token: base_data[1].into(),
            dirid: base_data[2].into(),
            filemap: list_res,
            stream: None,
        })
    }

    ///
    /// 从一个实例获取 `filemap` 并扩展到本实例
    ///
    /// 参数：
    /// - raw_data: `&[u8]`
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(())
    /// - Err(std::io::Error)
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use std::fs::read;
    /// use sal_file::CloudFile;
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    ///
    /// let data = read("/root/test.bin")?;
    ///
    /// cloud.extend_from_raw(&data)?;
    /// ```
    ///
    pub fn extend_from_raw(&mut self, raw_data: &[u8]) -> Result<()> {
        let file = CloudFile::from_raw(&raw_data)?;
        self.filemap.extend_from_slice(&file.filemap);
        self.update_inner()?;

        Ok(())
    }

    ///
    /// 从云服务器扫描新文件并添加到本实例
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(usize): 新扫描到的文件数量
    ///     - 由于传输限制，一次扫描最多4个
    /// - Err(std::io::Error)
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use sal_file::CloudFile;
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    ///
    /// cloud.set_stream(true)?;
    /// while let Ok(_) = cloud.scan() {}
    /// ```
    ///
    /// 注意：该函数会**自动结束**流!!!
    /// ```
    /// pub fn scan(&mut self) -> Result<usize> {
    ///
    ///     // inner code
    ///
    ///     self.set_stream(Stream::None)?;
    /// }
    /// ````
    ///
    pub fn scan(&mut self) -> Result<usize> {
        let Some(stream) = &self.stream else {
            return Err(Error::new(
                ErrorKind::AddrNotAvailable,
                format!("Stream is Unavailable!"),
            ));
        };

        let mut writer = BufWriter::new(stream);
        let mut reader = BufReader::new(stream);

        writer.write_all(
            format!(
                "GET /api/getMyDirAndFiles\
                ?puid={}&_token={}&fldid={}\
                &page=1&size=4 HTTP/1.1\r\n\
                Host: pan-yz.chaoxing.com\r\n\r\n",
                self.uid, self.token, self.dirid
            )
            .as_bytes(),
        )?;
        writer.flush()?;

        let data = reader.fill_buf()?.to_vec();

        let _ = drop(writer);
        let _ = drop(reader);

        let data = String::from_utf8_lossy(&data);
        let Some((_, data)) = data.split_once("\r\n\r\n") else {
            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                "InvalidData Received from Server",
            ));
        };

        let timer = self.filemap.len();
        let mut resid = Vec::new();

        if data.contains("\"result\":true") {
            if !data.contains("\"data\":[],") {
                for file in data[match data.find("[{") {
                    Some(x) => x,
                    None => {
                        return Err(Error::new(
                            ErrorKind::ConnectionReset,
                            "InvalidData Received from Server",
                        ))
                    }
                } + 2..match data.find("}]") {
                    Some(x) => x,
                    None => {
                        return Err(Error::new(
                            ErrorKind::ConnectionReset,
                            "InvalidData Received from Server",
                        ))
                    }
                }]
                    .split("},{")
                {
                    let objid = if let Some(o) = file.find("\"objectId\"") {
                        let file = &file[o + 12..];
                        if let Some(o) = file.find("\",\"") {
                            file[..o].to_string()
                        } else {
                            return Err(Error::new(
                                ErrorKind::ConnectionReset,
                                "InvalidData Received from Server",
                            ));
                        }
                    } else {
                        return Err(Error::new(
                            ErrorKind::ConnectionReset,
                            "InvalidData Received from Server",
                        ));
                    };

                    let name = if let Some(o) = file.find("\"name\"") {
                        let file = &file[o + 8..];
                        if let Some(o) = file.find("\",\"") {
                            file[..o].to_string()
                        } else {
                            return Err(Error::new(
                                ErrorKind::ConnectionReset,
                                "InvalidData Received from Server",
                            ));
                        }
                    } else {
                        return Err(Error::new(
                            ErrorKind::ConnectionReset,
                            "InvalidData Received from Server",
                        ));
                    };

                    self.filemap.push((name, objid));
                    resid.push(if let Some(o) = file.find("\"residstr\"") {
                        let file = &file[o + 12..];
                        if let Some(o) = file.find("\",\"") {
                            file[..o].to_string()
                        } else {
                            return Err(Error::new(
                                ErrorKind::ConnectionReset,
                                "InvalidData Received from Server",
                            ));
                        }
                    } else {
                        return Err(Error::new(
                            ErrorKind::ConnectionReset,
                            "InvalidData Received from Server",
                        ));
                    });
                }
            }
        } else {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!("Error Received: {}", data),
            ));
        }

        self.delete(&stream, &resid)?;
        self.update_inner()?;

        if self.filemap.len() == timer {
            self.set_stream(Stream::None)?;

            return Err(Error::new(
                ErrorKind::WriteZero,
                format!("Scan Finished: Read 0000!"),
            ));
        }

        Ok(self.filemap.len() - timer)
    }

    ///
    /// 通过 `objectid` 获取下载链接
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(String): 对应文件的链接
    /// - Err(std::io::Error)
    ///
    /// 直接请求访问下载链接可能会：
    /// ```
    /// <h1>403 Forbidden</h1>
    /// <p>You don't have permission to access the URL on this server.<hr/>Powered by Tengine
    /// ```
    ///
    /// 需要给链接添加 `Referer` Header:
    /// ```
    /// `Referer: http://sharewh{id}.xuexi365.com/` id: OPTION(1~4)
    /// ```
    ///
    /// 请求成功会以 `attachment` 形式返回数据。
    ///
    /// 另外存在一个分享页面：
    /// ```
    /// http://cloud.ananas.chaoxing.com/view/fileview?objectid={objid}
    /// ```
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use sal_file::CloudFile;
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    ///
    ///
    /// cloud.set_stream(Stream::Scan)?;
    /// while let Ok(_) = cloud.scan() {}
    ///
    /// filer.set_stream(Stream::Link)?;
    /// for (name, objid) in filer.get_filemap() {
    ///     let link = filer.get_link(objid)?;
    ///     println!("文件: {name}\r\n直链: {link}\r\n");
    /// }
    /// cloud.set_stream(false)?;
    /// ```
    ///
    /// 注意：该函数**不会**自动结束流!!!
    ///
    pub fn get_link(&self, object_id: &String) -> Result<String> {
        let Some(stream) = &self.stream else {
            return Err(Error::new(
                ErrorKind::AddrNotAvailable,
                format!("Stream is Unavailable!"),
            ));
        };

        let mut writer = BufWriter::new(stream);
        let mut reader = BufReader::new(stream);

        writer.write_all(
            format!(
                "GET /share/download/{} HTTP/1.1\r\n\
                Host: sharewh.xuexi365.com\r\n\r\n",
                object_id
            )
            .as_bytes(),
        )?;
        writer.flush()?;

        let data = reader.fill_buf()?.to_vec();

        let _ = drop(writer);
        let _ = drop(reader);

        let data = String::from_utf8_lossy(&data).replace(' ', "");

        let mut res = String::new();
        if let Some(x) = data.find("vardownloadUrl='") {
            let Some((data, _)) = data[x + 16..].split_once("';\r\n") else {
                return Err(Error::new(
                    ErrorKind::ConnectionReset,
                    "InvalidData Received from Server",
                ));
            };
            res.push_str(data);
        } else if data.contains("获取下载地址失败") {
            return Err(Error::new(
                ErrorKind::NotFound,
                "Download Link Not Found: Check ObjectID!",
            ));
        } else {
            let Some((_, data)) = data.split_once("\r\n\r\n") else {
                return Err(Error::new(
                    ErrorKind::UnexpectedEof,
                    "InvalidData Received from Server",
                ));
            };

            return Err(Error::new(
                ErrorKind::UnexpectedEof,
                format!("InvalidData Received: {}", data),
            ));
        };

        Ok(res)
    }

    ///
    /// 用于为实例开启流式通道，与服务器连接
    ///
    /// 参数：
    /// - stream: `Stream` 枚举
    ///     - Stream::Scan => 与超星云盘服务器连接
    ///     - Stream::Link => 与超星下载服务器连接
    ///     - Stream::None => 与服务器断开连接
    ///
    /// 返回一个 `Result` 枚举
    /// - Ok(())
    /// - Err(std::io::Error)
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use sal_file::{CloudFile, Stream};
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    ///
    /// cloud.set_stream(Stream::Scan)?;
    /// while let Ok(_) = cloud.scan() {}
    ///
    /// cloud.set_stream(Stream::Link)?;
    /// let _ = cloud.get_link()?;
    /// cloud.set_stream(Stream::None)?;
    /// ```
    ///
    pub fn set_stream(&mut self, stream: Stream) -> Result<()> {
        match stream {
            Stream::Scan => self.stream = Some(TcpStream::connect(HOST_SCAN)?),
            Stream::Link => self.stream = Some(TcpStream::connect(HOST_LINK)?),
            Stream::None => self.stream = None,
        }

        Ok(())
    }

    ///
    /// 用于获取 `filemap` 的引用
    ///
    /// 返回
    /// - &[(String, String)]
    ///     - 文件表：(name, objectid)
    ///     - name: 文件名
    ///     - objectid: 用于从服务器下载文件
    ///
    /// **Example:**
    /// ```
    /// mod sal_file;
    /// use sal_file::CloudFile;
    ///
    /// let mut cloud = CloudFile::new(
    ///     "29*******".into(),
    ///     "b8***391*******d3726f*******d0b2".into(),
    ///     "94***555*******592".into(),
    ///     &[127, 97, 112, 128],
    /// )?;
    ///
    /// cloud.set_stream(Stream::Scan)?;
    /// while let Ok(_) = cloud.scan() {}
    ///
    /// let map = cloud.get_filemap();
    /// ```
    ///
    pub fn get_filemap(&self) -> &[(String, String)] {
        &self.filemap
    }

    fn update_inner(&mut self) -> Result<()> {
        /*  File:
         *  3, 3, 4, 21,   //  [0, 4]    FileHeader
         *  7, 23, 10, 8   //  [4, 8]    FileHeader
         *  2, 5, 1, 3,    //  [8, 12]   Password
         *  25, 0, 0, 3,   //  [12, 16]  ETX
         *  ...........    //  [16, ..]  EnCodedData
         *
         * EnCodedData:
         *  ...........    //  [16, 144]   BaseData
         *  ...........    //  [144, ..]   ListData
         *
         * DeCodedData:
         *  ...........    //  [0, 64]   BaseData
         *  ...........    //  [64, ..]  ListData
         *
         * */

        if self.inner.len() < 144 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Len of Data to Short: [144..]",
            ));
        }

        let inner = self.inner.clone();
        let [_, _, passwd, _] = inner.chunks(4).take(4).collect::<Vec<&[u8]>>()[..] else {
            return Err(Error::new(
                ErrorKind::Other,
                format!("Unknown: {}", line!()),
            ));
        };

        let passwd: &[u8; 4] = match passwd.try_into() {
            Ok(x) => x,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Wrong Password Type: Unsupported Password Type",
                ))
            }
        };

        let mut data = vec![
            self.uid.as_bytes(),
            self.token.as_bytes(),
            self.dirid.as_bytes(),
        ]
        .join(&[27u8][..]); // Sep By \u{1B}
        while data.len() < 64 {
            data.push(0);
        }

        data.extend_from_slice(
            &self
                .filemap
                .iter()
                .map(|(name, objid)| vec![name.as_bytes(), objid.as_bytes()].join(&[26u8][..]))
                .collect::<Vec<Vec<u8>>>()
                .join(&[27u8][..]),
        );

        let data = Self::matrix_encode(passwd, &data)?;
        let data = Self::sixteen_to_eight(&data);

        self.inner = vec![3, 3, 4, 21, 7, 23, 10, 8];
        self.inner.extend_from_slice(passwd);
        self.inner.extend_from_slice(&[25, 0, 0, 3]);
        self.inner.extend_from_slice(&data);

        Ok(())
    }

    fn delete(&self, stream: &TcpStream, resid: &[String]) -> Result<bool> {
        if resid.len() == 0 {
            return Ok(true);
        }

        let mut writer = BufWriter::new(stream);
        let mut reader = BufReader::new(stream);

        writer.write_all(
            format!(
                "GET /api/delete\
                ?puid={}&_token={}\
                &resids={} HTTP/1.1\r\n\
                Host: pan-yz.chaoxing.com\r\n\r\n",
                self.uid,
                self.token,
                resid.join(","),
            )
            .as_bytes(),
        )?;

        let _ = writer.flush()?;
        let data = reader.fill_buf()?.to_vec();

        let _ = drop(writer);
        let _ = drop(reader);

        let data = String::from_utf8_lossy(&data);
        let data = match data.split_once("\r\n\r\n") {
            Some((_, x)) => x,
            None => {
                return Err(Error::new(
                    ErrorKind::ConnectionReset,
                    "InvalidData Received from Server",
                ))
            }
        };

        if data.contains("\"result\":true") {
            if data.contains("\"success\":false") {
                return Ok(false);
            }
        } else {
            return Err(Error::new(
                ErrorKind::PermissionDenied,
                format!("Error Received: {}", data),
            ));
        }

        Ok(true)
    }

    fn matrix_encode(passwd: &[u8; 4], data: &[u8]) -> Result<Vec<u16>> {
        let [a, b, c, d] = passwd.map(|x| x as u16);

        for p in passwd {
            if p > &128 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Passwd Too Big: 0..=128",
                ));
            }
        }

        if a * d <= b * c {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Wrong Passwd: the Val MUST be POSITIVE",
            ));
        }

        let mut res = Vec::new();
        let len = data.len();
        let max = len >> 1;

        let mut i = 0;
        while i < max {
            res.push(a * data[2 * i] as u16 + b * data[2 * i + 1] as u16);
            res.push(c * data[2 * i] as u16 + d * data[2 * i + 1] as u16);

            i += 1;
        }

        if len % 2 == 1 {
            res.push(a as u16 * data[len - 1] as u16);
            res.push(c as u16 * data[len - 1] as u16);
        }

        Ok(res)
    }

    fn matrix_decode(passwd: &[u8; 4], data: &[u16]) -> Result<Vec<u8>> {
        let [a, b, c, d] = passwd.map(|x| x as u32);

        for p in passwd {
            if p > &128 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Passwd Too Big: 0..=128",
                ));
            }
        }

        if a * d <= b * c {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Wrong Passwd: the Val MUST be POSITIVE",
            ));
        }

        if data.len() % 2 == 1 {
            return Err(Error::new(ErrorKind::InvalidInput, "Wrong Len of Data"));
        }

        let mut res = Vec::new();
        let max = data.len() >> 1;
        let val = a * d - b * c;

        let mut i = 0;
        while i < max {
            res.push(((d * data[2 * i] as u32 - b * data[2 * i + 1] as u32) / val) as u8);
            res.push(((a * data[2 * i + 1] as u32 - c * data[2 * i] as u32) / val) as u8);

            i += 1;
        }

        Ok(res)
    }

    fn sixteen_to_eight(from: &[u16]) -> Vec<u8> {
        let mut res = Vec::new();

        let len = from.len();
        let max = len >> 1;

        let mut i = 0;

        while i < max {
            res.push((from[2 * i] / 256) as u8);
            res.push((from[2 * i] % 256) as u8);
            res.push((from[2 * i + 1] / 256) as u8);
            res.push((from[2 * i + 1] % 256) as u8);

            i += 1;
        }

        if len % 2 == 1 {
            res.push((from[len - 1] / 256) as u8);
            res.push((from[len - 1] % 256) as u8);
        }

        res
    }

    fn eight_to_sixteen(from: &[u8]) -> Vec<u16> {
        let mut res = Vec::new();

        let len = from.len();
        let max = len >> 1;

        let mut i = 0;
        while i < max {
            res.push(256 * from[2 * i] as u16 + from[2 * i + 1] as u16);
            i += 1;
        }

        if len % 2 == 1 {
            res.push(from[len - 1] as u16);
        }

        res
    }
}
