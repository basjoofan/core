use crate::Record;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

const META: [(&str, &str); 2] = [
    (
        "avro.schema",
        r#"
{
    "name": "record",
    "type": "record",
    "fields": [
        {"name": "name", "type": "string"},
        {"name": "task", "type": "long"},
        {"name": "number", "type": "long"},
        {"name": "order", "type": "long"},
        {"name": "time_start", "type": "long"},
        {"name": "time_end", "type": "long"},
        {"name": "time_total", "type": "long"},
        {"name": "time_resolve", "type": "long"},
        {"name": "time_connect", "type": "long"},
        {"name": "time_write", "type": "long"},
        {"name": "time_delay", "type": "long"},
        {"name": "time_read", "type": "long"},
        {"name": "request_name", "type": "string"},
        {"name": "request_method", "type": "string"},
        {"name": "request_url", "type": "string"},
        {"name": "request_version", "type": "string"},
        {"name": "request_headers", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "request_body", "type": "string"},
        {"name": "response_version", "type": "string"},
        {"name": "response_status", "type": "int"},
        {"name": "response_reason", "type": "string"},
        {"name": "response_headers", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "response_body", "type": "string"},
        {"name": "asserts", "type":
            {
                "type": "array",
                "items": {
                    "name": "assert",
                    "type": "record",
                    "fields": [
                        {"name": "expr", "type": "string"},
                        {"name": "left", "type": "string"},
                        {"name": "compare", "type": "string"},
                        {"name": "right", "type": "string"},
                        {"name": "result", "type": "boolean"}
                    ]
                }
            }
        },
        {"name": "error", "type": "string"}
    ]
    }
"#,
    ),
    ("avro.codec", "null"),
];
const MAGIC: &[u8; 4] = b"Obj\x01";
const MARKER: &[u8; 16] = b"afilesyncmarker\x02";

pub struct Writer<W> {
    w: W,
}

impl<W: AsyncWrite + Unpin> Writer<W> {
    pub async fn new(mut w: W) -> Self {
        let _ = w.write(&header()).await;
        Writer { w }
    }

    pub async fn write(&mut self, records: &[Record], name: &str, task: u32, number: u32) {
        let mut data = Vec::new();
        for (order, record) in records.iter().enumerate() {
            encode_bytes(name.as_bytes(), &mut data);
            encode_long(task as i64, &mut data);
            encode_long(number as i64, &mut data);
            encode_long(order as i64, &mut data);
            encode_long(record.time.start.as_nanos() as i64, &mut data);
            encode_long(record.time.end.as_nanos() as i64, &mut data);
            encode_long(record.time.total.as_nanos() as i64, &mut data);
            encode_long(record.time.resolve.as_nanos() as i64, &mut data);
            encode_long(record.time.connect.as_nanos() as i64, &mut data);
            encode_long(record.time.write.as_nanos() as i64, &mut data);
            encode_long(record.time.delay.as_nanos() as i64, &mut data);
            encode_long(record.time.read.as_nanos() as i64, &mut data);
            encode_bytes(record.name.as_bytes(), &mut data);
            encode_bytes(record.request.method.as_ref(), &mut data);
            encode_bytes(record.request.url.to_string().as_bytes(), &mut data);
            encode_bytes(record.request.version.as_ref(), &mut data);
            encode_long(record.request.headers.len() as i64, &mut data);
            for header in record.request.headers.iter() {
                encode_long(2, &mut data);
                encode_bytes(header.name.as_bytes(), &mut data);
                encode_bytes(header.value.as_bytes(), &mut data);
            }
            encode_bytes(record.request.body.as_bytes(), &mut data);
            encode_bytes(record.response.version.as_bytes(), &mut data);
            encode_long(record.response.status as i64, &mut data);
            encode_bytes(record.response.reason.as_bytes(), &mut data);
            encode_long(record.response.headers.len() as i64, &mut data);
            for header in record.response.headers.iter() {
                encode_long(2, &mut data);
                encode_bytes(header.name.as_bytes(), &mut data);
                encode_bytes(header.value.as_bytes(), &mut data);
            }
            encode_bytes(record.response.body.as_bytes(), &mut data);
            encode_long(record.asserts.len() as i64, &mut data);
            for assert in record.asserts.iter() {
                encode_bytes(assert.expr.as_bytes(), &mut data);
                encode_bytes(assert.left.as_bytes(), &mut data);
                encode_bytes(assert.compare.as_bytes(), &mut data);
                encode_bytes(assert.right.as_bytes(), &mut data);
                encode_bool(assert.result, &mut data);
            }
            encode_bytes(record.error.as_bytes(), &mut data);
        }
        let mut buffer = Vec::new();
        encode_long(records.len() as i64, &mut buffer);
        encode_long(data.len() as i64, &mut buffer);
        buffer.extend_from_slice(&data);
        buffer.extend_from_slice(MARKER);
        let _ = self.w.write(&buffer).await;
        let _ = self.w.flush().await;
    }
}

fn header() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(MAGIC);
    encode_long(META.len() as i64, &mut header);
    for (key, value) in META {
        encode_bytes(key, &mut header);
        encode_bytes(value, &mut header);
    }
    header.push(0u8);
    header.extend_from_slice(MARKER);
    header
}

fn encode_bool(b: bool, buffer: &mut Vec<u8>) {
    buffer.push(u8::from(b));
}

fn encode_bytes<B: AsRef<[u8]> + ?Sized>(s: &B, buffer: &mut Vec<u8>) {
    let bytes = s.as_ref();
    encode_long(bytes.len() as i64, buffer);
    buffer.extend_from_slice(bytes);
}

fn encode_long(i: i64, buffer: &mut Vec<u8>) {
    encode_variable(((i << 1) ^ (i >> 63)) as u64, buffer)
}

fn encode_variable(mut z: u64, buffer: &mut Vec<u8>) {
    loop {
        if z <= 0x7F {
            buffer.push((z & 0x7F) as u8);
            break;
        } else {
            buffer.push((0x80 | (z & 0x7F)) as u8);
            z >>= 7;
        }
    }
}

#[test]
fn test_encode_long() {
    let mut buffer = Vec::new();
    encode_long(27, &mut buffer);
    assert_eq!(buffer, b"\x36");
}

#[test]
fn test_encode_bytes() {
    let mut buffer = Vec::new();
    encode_bytes("foo", &mut buffer);
    assert_eq!(buffer, b"\x06\x66\x6f\x6f");
}

#[test]
fn test_encode_record() {
    let mut data = Vec::new();
    encode_long(27, &mut data);
    encode_bytes("foo", &mut data);
    let mut buffer = Vec::new();
    encode_long(1, &mut buffer);
    encode_long(data.len() as i64, &mut buffer);
    buffer.extend_from_slice(&data);
    println!("{:?}", buffer);
    assert_eq!(buffer, b"\x02\x0a\x36\x06\x66\x6f\x6f");
}

#[tokio::test]
async fn test_writer() {
    let mut writer = Writer::new(Vec::new()).await;
    let record = Record {
        name: "test".to_string(),
        time: crate::http::Time::default(),
        request: crate::http::Request::default(),
        response: crate::http::Response::default(),
        asserts: Vec::new(),
        error: String::default(),
    };
    writer.write(&vec![record], "test", 0, 0).await;
    let encoded = writer.w;
    let reader = avro::Reader::new(std::io::Cursor::new(encoded)).unwrap();
    println!("schema:{:?}", reader.reader_schema());
    for value in reader {
        println!("{:?}", value.unwrap());
    }
}
