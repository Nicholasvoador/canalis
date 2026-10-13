//! Wire format spoken by the TRYX Panorama screen over its USB serial port.
//!
//! Frame:    0x5A | escaped(length:u16be + payload + checksum:u8) | 0x5A
//! length:   unescaped size of the whole frame, both flags included
//! checksum: sum of the length bytes and payload, modulo 256
//! escape:   0x5A -> 0x5B 0x01, 0x5B -> 0x5B 0x02
//!
//! The payload is an HTTP-like text message: a start line, CRLF headers,
//! a blank line, then an optional JSON body.

use serde_json::Value;

const FRAME_FLAG: u8 = 0x5A;
const ESCAPE_BYTE: u8 = 0x5B;
const ESCAPED_FLAG: u8 = 0x01;
const ESCAPED_ESCAPE: u8 = 0x02;
const FRAME_OVERHEAD: usize = 5;
const PROTOCOL_VERSION: &str = "1";

#[derive(Debug, PartialEq)]
pub struct FrameError(pub String);

impl std::fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid frame: {}", self.0)
    }
}

pub fn encode_frame(payload: &[u8]) -> Vec<u8> {
    let length = ((payload.len() + FRAME_OVERHEAD) as u16).to_be_bytes();
    let mut body = Vec::with_capacity(payload.len() + 3);
    body.extend_from_slice(&length);
    body.extend_from_slice(payload);
    body.push(checksum(&body));
    let mut frame = vec![FRAME_FLAG];
    frame.extend(escape(&body));
    frame.push(FRAME_FLAG);
    frame
}

pub fn decode_frame(frame: &[u8]) -> Result<Vec<u8>, FrameError> {
    let delimited = frame.len() >= FRAME_OVERHEAD && frame[0] == FRAME_FLAG && frame[frame.len() - 1] == FRAME_FLAG;
    if !delimited {
        return Err(FrameError("not delimited by 0x5A flags".into()));
    }
    let body = unescape(&frame[1..frame.len() - 1]);
    verify_length(&body)?;
    verify_checksum(&body)?;
    Ok(body[2..body.len() - 1].to_vec())
}

fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))
}

fn escape(bytes: &[u8]) -> Vec<u8> {
    let mut escaped = Vec::with_capacity(bytes.len() + 8);
    for &byte in bytes {
        match byte {
            FRAME_FLAG => escaped.extend_from_slice(&[ESCAPE_BYTE, ESCAPED_FLAG]),
            ESCAPE_BYTE => escaped.extend_from_slice(&[ESCAPE_BYTE, ESCAPED_ESCAPE]),
            _ => escaped.push(byte),
        }
    }
    escaped
}

fn unescape(bytes: &[u8]) -> Vec<u8> {
    let mut unescaped = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let replacement = match (bytes[index], bytes.get(index + 1)) {
            (ESCAPE_BYTE, Some(&ESCAPED_FLAG)) => Some(FRAME_FLAG),
            (ESCAPE_BYTE, Some(&ESCAPED_ESCAPE)) => Some(ESCAPE_BYTE),
            _ => None,
        };
        match replacement {
            Some(byte) => {
                unescaped.push(byte);
                index += 2;
            }
            None => {
                unescaped.push(bytes[index]);
                index += 1;
            }
        }
    }
    unescaped
}

fn verify_length(body: &[u8]) -> Result<(), FrameError> {
    if body.len() < 3 {
        return Err(FrameError("too short".into()));
    }
    let declared = u16::from_be_bytes([body[0], body[1]]) as usize;
    let actual = body.len() + 2;
    if declared == actual {
        Ok(())
    } else {
        Err(FrameError(format!("length field {declared} != frame size {actual}")))
    }
}

fn verify_checksum(body: &[u8]) -> Result<(), FrameError> {
    let (data, received) = body.split_at(body.len() - 1);
    let expected = checksum(data);
    if received[0] == expected {
        Ok(())
    } else {
        Err(FrameError(format!("checksum {:#04x} != computed {expected:#04x}", received[0])))
    }
}

/// Reassembles complete frame payloads from arbitrary serial reads.
#[derive(Default)]
pub struct FrameReader {
    buffer: Vec<u8>,
}

impl FrameReader {
    pub fn feed(&mut self, data: &[u8]) -> Vec<Vec<u8>> {
        self.buffer.extend_from_slice(data);
        std::iter::from_fn(|| self.next_payload()).collect()
    }

    fn next_payload(&mut self) -> Option<Vec<u8>> {
        loop {
            let Some(start) = self.buffer.iter().position(|&byte| byte == FRAME_FLAG) else {
                self.buffer.clear();
                return None;
            };
            let Some(end) = self.buffer[start + 1..].iter().position(|&byte| byte == FRAME_FLAG).map(|offset| start + 1 + offset) else {
                self.buffer.drain(..start);
                return None;
            };
            match decode_frame(&self.buffer[start..=end]) {
                Ok(payload) => {
                    self.buffer.drain(..=end);
                    return Some(payload);
                }
                Err(_) => {
                    self.buffer.drain(..end);
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub start_line: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Message {
    pub fn request(method: &str, command: &str, sequence: u64, date: u64, body: Option<&Value>) -> Self {
        let mut headers = vec![("SeqNumber".to_string(), sequence.to_string()), ("Date".to_string(), date.to_string())];
        let encoded_body = body.map(|value| value.to_string().into_bytes()).unwrap_or_default();
        if body.is_some() {
            headers.push(("ContentType".into(), "json".into()));
            headers.push(("ContentLength".into(), encoded_body.len().to_string()));
        }
        Self { start_line: format!("{method} {command} {PROTOCOL_VERSION}"), headers, body: encoded_body }
    }

    pub fn parse(payload: &[u8]) -> Self {
        let separator = payload.windows(4).position(|window| window == b"\r\n\r\n");
        let (head, body) = match separator {
            Some(index) => (&payload[..index], payload[index + 4..].to_vec()),
            None => (payload, Vec::new()),
        };
        let head = String::from_utf8_lossy(head);
        let mut lines = head.split("\r\n");
        let start_line = lines.next().unwrap_or_default().to_string();
        let headers = lines
            .filter_map(|line| line.split_once('='))
            .map(|(name, value)| (name.to_string(), value.to_string()))
            .collect();
        Self { start_line, headers, body }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let header_lines: String = self.headers.iter().map(|(name, value)| format!("{name}={value}\r\n")).collect();
        let mut bytes = format!("{}\r\n{header_lines}\r\n", self.start_line).into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(header, _)| header == name).map(|(_, value)| value.as_str())
    }

    pub fn json(&self) -> Option<Value> {
        serde_json::from_slice(&self.body).ok()
    }

    pub fn acknowledged_sequence(&self) -> Option<u64> {
        self.header("AckNumber")?.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Frames captured from a real TRYX Panorama (serialserver_log.txt on the device).
    const CONN_REQUEST: &str = "5A0035504F535420636F6E6E20310D0A5365714E756D6265723D300D0A44617465\
        3D313739313930323236313233340D0A0D0A4F5A";
    const STATUS_RESPONSE: &str = "5A00D231203230300D0A41636B4E756D6265723D313132320D0A436F6E74656E74\
        4C656E6774683D3134330D0A436F6E74656E74547970653D6A736F6E0D0A0D0A7B\
        22737461747573223A7B2266616E4C4344223A2232303130222C22747572626F50\
        756D70223A2232383530227D2C227761726E696E67223A225B027B5C2264657363\
        72697074696F6E5C223A5C224E6F204552524F525C222C5C22747970655C223A5C\
        2246616E204C43445C227D5D222C22617661696C61626C6553746F72616765223A\
        333339333734343839367D845A";

    fn hex(text: &str) -> Vec<u8> {
        let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
        digits.chunks(2).map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()).collect()
    }

    fn conn_payload() -> Vec<u8> {
        let frame = hex(CONN_REQUEST);
        frame[3..frame.len() - 2].to_vec()
    }

    #[test]
    fn reproduces_captured_conn_frame() {
        assert_eq!(encode_frame(&conn_payload()), hex(CONN_REQUEST));
    }

    #[test]
    fn reproduces_captured_frame_with_escaped_bracket() {
        let payload = decode_frame(&hex(STATUS_RESPONSE)).unwrap();
        assert_eq!(encode_frame(&payload), hex(STATUS_RESPONSE));
    }

    #[test]
    fn escapes_frame_flag_inside_payload() {
        let frame = encode_frame(b"Z");
        assert!(!frame[1..frame.len() - 1].contains(&FRAME_FLAG));
        assert_eq!(decode_frame(&frame).unwrap(), b"Z");
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut frame = hex(CONN_REQUEST);
        let checksum_index = frame.len() - 2;
        frame[checksum_index] ^= 0xFF;
        assert!(decode_frame(&frame).is_err());
    }

    #[test]
    fn request_text_matches_captured_conn() {
        let message = Message::request("POST", "conn", 0, 1791902261234, None);
        assert_eq!(message.to_bytes(), conn_payload());
    }

    #[test]
    fn request_with_json_body() {
        let body = serde_json::json!({"enable": true});
        let message = Message::request("POST", "waterBlockScreen", 4, 1791902263603, Some(&body));
        assert_eq!(
            message.to_bytes(),
            b"POST waterBlockScreen 1\r\nSeqNumber=4\r\nDate=1791902263603\r\nContentType=json\r\nContentLength=15\r\n\r\n{\"enable\":true}"
        );
    }

    #[test]
    fn parses_device_response() {
        let message = Message::parse(&decode_frame(&hex(STATUS_RESPONSE)).unwrap());
        assert_eq!(message.start_line, "1 200");
        assert_eq!(message.acknowledged_sequence(), Some(1122));
        assert_eq!(message.json().unwrap()["status"]["turboPump"], "2850");
    }

    #[test]
    fn frame_reader_splits_stream_and_skips_garbage() {
        let mut stream = vec![0x00, 0x01];
        stream.extend(hex(CONN_REQUEST));
        stream.extend(hex(STATUS_RESPONSE));
        let mut reader = FrameReader::default();
        let mut payloads = reader.feed(&stream[..40]);
        payloads.extend(reader.feed(&stream[40..]));
        assert_eq!(payloads, vec![conn_payload(), decode_frame(&hex(STATUS_RESPONSE)).unwrap()]);
    }
}
