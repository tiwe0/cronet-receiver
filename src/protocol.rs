use bytes::{Buf, BytesMut};
use futures::io;
use tokio_util::codec::Decoder;

pub struct ProtocolCodec {
    pub magic_key: Vec<u8>,
}

impl ProtocolCodec {
    pub fn new(magic_key: &String) -> Self {
        let mut key_bytes = magic_key.as_bytes().to_vec();
        // 确保 magic_key 正好是 4 字节
        key_bytes.truncate(4);  // 截取前4字节
        while key_bytes.len() < 4 {
            key_bytes.push(0);  // 不足4字节则填充0
        }
        ProtocolCodec { magic_key: key_bytes }
    }
}

pub struct Packet {
    pub tag: String,
    pub tag_app: String,
    pub tag_url: String,
    pub end_flag: bool,
    pub payload: Vec<u8>,
}


impl Packet {
    pub fn new(tag_app: String, tag_url: String, end_flag: bool, payload: Vec<u8>) -> Self {
        Packet {
            tag: format!("{}|{}", tag_app, tag_url),
            tag_app,
            tag_url,
            end_flag,
            payload,
        }
    }
}

// MagicNumber(4B)|TagPackageLen(2B)|TagUrlLen(2B)|PayloadLen(4B)|EndFlag(4B)|TagPackage(M bytes)|TagUrl(N bytes)|Payload(K bytes)
impl Decoder for ProtocolCodec {
    type Item = Packet;
    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // 1. 最小 Header 检查 4 + 2 + 2 + 4 + 4 = 16 bytes
        if src.len() < 16 {return Ok(None);}

        // 2. Magic 校验，防止扫描器
        if &src[0..4] != self.magic_key.as_slice() {
            eprintln!("[!] Magic 校验失败: 期望 {:?}, 收到 {:?}", self.magic_key, &src[0..4]);
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Magic"));
        }

        // 3. 获取 Tag 中 App path 的长度, Tag 中 Url 的长度，Payload 的长度，EndFlag
        let tag_app_len = u16::from_be_bytes(src[4..6].try_into().unwrap()) as usize;
        let tag_url_len = u16::from_be_bytes(src[6..8].try_into().unwrap()) as usize;
        let payload_len = u32::from_be_bytes(src[8..12].try_into().unwrap()) as usize;
        let end_flag = u32::from_be_bytes(src[12..16].try_into().unwrap()) as usize;

        if src.len() < 16 + tag_app_len + tag_url_len + payload_len {
            src.reserve(16 + tag_app_len + tag_url_len + payload_len);
            return Ok(None);
        }

        // 4. 弹出
        src.advance(16);

        // 5. 解析变长字段，零拷贝操作
        let tag_app = src.split_to(tag_app_len);
        let tag_url = src.split_to(tag_url_len);
        let payload = src.split_to(payload_len);

        Ok(Some(Packet::new(
            String::from_utf8_lossy(&tag_app).to_string(),
            String::from_utf8_lossy(&tag_url).to_string(),
            end_flag != 0,
            payload.to_vec()
        )))
    }
}
