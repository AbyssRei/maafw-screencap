// Copyright (C) 2026 AbyssRei (Rust migration)
// Configuration semantics adapted from meojson, Copyright (c) 2020 MistEO.
// Its MIT permission notice is retained in THIRD_PARTY_NOTICES.
// SPDX-License-Identifier: LGPL-3.0-or-later
//! Private fixed Linux Config.h boundary, including observed meojson quirks.
//! This is not a general JSON API or a proposed SDK configuration contract.
//! Byte strings, lexical integers, duplicate rejection and legacy trailing '/'
//! differ from a serde_json::Value conversion. The reader validates discarded
//! values too, with a fallible explicit container stack instead of recursion.
use std::{
    borrow::Cow,
    collections::{HashSet, TryReserveError},
};

#[derive(Debug)]
pub(crate) enum Error {
    Invalid,
    Allocation(TryReserveError),
}
impl From<TryReserveError> for Error {
    fn from(value: TryReserveError) -> Self {
        Self::Allocation(value)
    }
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub(crate) struct Config {
    pub method: u64,
    pub input_method: u64,
    pub wlr_socket: Vec<u8>,
    pub pipewire_fd: i32,
    pub pipewire_node: u32,
    // Retain configuration/metadata, without implementing input actions.
    pub uinput_width: i32,
    pub uinput_height: i32,
    pub uinput_path: Cow<'static, [u8]>,
    pub eis_socket: Vec<u8>,
    pub use_win32_vk_code: bool,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            method: 0,
            input_method: 0,
            wlr_socket: Vec::new(),
            pipewire_fd: -1,
            pipewire_node: 0,
            uinput_width: 0,
            uinput_height: 0,
            uinput_path: Cow::Borrowed(b"/dev/uinput"),
            eis_socket: Vec::new(),
            use_win32_vk_code: false,
        }
    }
}
impl Config {
    pub fn parse_c_string(input: Option<&[u8]>) -> Result<Self> {
        let input = input.ok_or(Error::Invalid)?;
        let end = input.iter().position(|b| *b == 0).unwrap_or(input.len());
        let mut reader = Reader {
            input: &input[..end],
            position: 0,
        };
        reader.take(b'{')?;
        let mut keys = HashSet::new();
        let mut config = Self::default();
        let (mut method, mut input_method) = (false, false);
        if !reader.consume(b'}') {
            loop {
                let key = reader.string()?;
                reader.take(b':')?;
                let value = reader.value()?;
                match key.as_slice() {
                    b"screencap_method" => {
                        config.method = value.integer()?;
                        method = true;
                    }
                    b"input_method" => {
                        config.input_method = value.integer()?;
                        input_method = true;
                    }
                    b"wlr_socket_path" => config.wlr_socket = value.string()?,
                    b"pw_socket_fd" => config.pipewire_fd = value.integer()?,
                    b"pw_node_id" => config.pipewire_node = value.integer()?,
                    b"uinput_screen_width" => config.uinput_width = value.integer()?,
                    b"uinput_screen_height" => config.uinput_height = value.integer()?,
                    b"uinput_path" => config.uinput_path = Cow::Owned(value.string()?),
                    b"eis_socket_path" => config.eis_socket = value.string()?,
                    b"use_win32_vk_code" => config.use_win32_vk_code = value.boolean()?,
                    _ => {}
                }
                unique(&mut keys, key)?;
                if reader.consume(b'}') {
                    break;
                }
                reader.take(b',')?;
            }
        }
        reader.whitespace();
        // Current source parse() uses false from skip_whitespace as EOF, even
        // when strict JSON mode encounters '/'. Preserve that observed boundary.
        if !method || !input_method || !matches!(reader.peek(), None | Some(b'/')) {
            return Err(Error::Invalid);
        }
        Ok(config)
    }
    pub fn info(&self) -> Result<Vec<u8>> {
        // Match meojson's byte-sorted object keys and compact serialization.
        let mut out = Writer(Vec::new());
        out.raw(b"{\"eis_socket_path\":")?;
        out.string(&self.eis_socket)?;
        out.raw(b",\"input_method\":")?;
        out.unsigned(self.input_method)?;
        out.raw(b",\"pw_node_id\":")?;
        out.unsigned(u64::from(self.pipewire_node))?;
        out.raw(b",\"pw_socket_fd\":")?;
        out.signed(self.pipewire_fd)?;
        out.raw(b",\"screencap_method\":")?;
        out.unsigned(self.method)?;
        out.raw(b",\"type\":\"linux\",\"uinput_path\":")?;
        out.string(&self.uinput_path)?;
        out.raw(b",\"uinput_screen_height\":")?;
        out.signed(self.uinput_height)?;
        out.raw(b",\"uinput_screen_width\":")?;
        out.signed(self.uinput_width)?;
        out.raw(b",\"use_win32_vk_code\":")?;
        out.raw(if self.use_win32_vk_code {
            b"true"
        } else {
            b"false"
        })?;
        out.raw(b",\"wlr_socket_path\":")?;
        out.string(&self.wlr_socket)?;
        out.raw(b"}")?;
        Ok(out.0)
    }
    pub fn uuid(&self) -> Result<Vec<u8>> {
        let mut out = Writer(Vec::new());
        match self.method {
            1 | 2 => out.raw(&self.wlr_socket)?,
            4 => {
                out.signed(self.pipewire_fd)?;
                out.raw(b"_")?;
                out.unsigned(u64::from(self.pipewire_node))?;
            }
            _ => out.raw(b"-1")?,
        }
        out.raw(b"_")?;
        match self.input_method {
            1 => out.raw(&self.wlr_socket)?,
            2 => out.raw(&self.uinput_path)?,
            4 => out.raw(&self.eis_socket)?,
            _ => out.raw(b"-1")?,
        }
        Ok(out.0)
    }
}

fn unique(keys: &mut HashSet<Vec<u8>>, key: Vec<u8>) -> Result<()> {
    keys.try_reserve(1)?;
    if !keys.insert(key) {
        return Err(Error::Invalid);
    }
    Ok(())
}
struct Writer(Vec<u8>);
impl Writer {
    fn raw(&mut self, bytes: &[u8]) -> Result<()> {
        self.0.try_reserve(bytes.len())?;
        self.0.extend_from_slice(bytes);
        Ok(())
    }
    fn unsigned(&mut self, mut value: u64) -> Result<()> {
        let mut digits = [b'0'; 20];
        let mut offset = digits.len() - 1;
        loop {
            digits[offset] =
                b'0' + u8::try_from(value % 10).expect("decimal digit is at most nine");
            value /= 10;
            if value == 0 {
                break;
            }
            offset -= 1;
        }
        self.raw(&digits[offset..])
    }
    fn signed(&mut self, value: i32) -> Result<()> {
        if value < 0 {
            self.raw(b"-")?;
        }
        self.unsigned(i64::from(value).unsigned_abs())
    }
    fn string(&mut self, value: &[u8]) -> Result<()> {
        self.raw(b"\"")?;
        for &byte in value {
            match byte {
                b'"' => self.raw(b"\\\"")?,
                b'\\' => self.raw(b"\\\\")?,
                8 => self.raw(b"\\b")?,
                12 => self.raw(b"\\f")?,
                b'\n' => self.raw(b"\\n")?,
                b'\r' => self.raw(b"\\r")?,
                b'\t' => self.raw(b"\\t")?,
                0..32 => {
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    self.raw(&[
                        b'\\',
                        b'u',
                        b'0',
                        b'0',
                        HEX[usize::from(byte >> 4)],
                        HEX[usize::from(byte & 15)],
                    ])?;
                }
                _ => self.raw(&[byte])?,
            }
        }
        self.raw(b"\"")
    }
}

enum Value<'a> {
    Number(&'a [u8]),
    String(Vec<u8>),
    Boolean(bool),
    Other,
}
impl Value<'_> {
    fn integer<T: std::str::FromStr>(&self) -> Result<T> {
        let Self::Number(bytes) = self else {
            return Err(Error::Invalid);
        };
        std::str::from_utf8(bytes)
            .map_err(|_| Error::Invalid)?
            .parse()
            .map_err(|_| Error::Invalid)
    }
    fn string(self) -> Result<Vec<u8>> {
        if let Self::String(value) = self {
            Ok(value)
        } else {
            Err(Error::Invalid)
        }
    }
    fn boolean(&self) -> Result<bool> {
        if let Self::Boolean(value) = self {
            Ok(*value)
        } else {
            Err(Error::Invalid)
        }
    }
}
struct Reader<'a> {
    input: &'a [u8],
    position: usize,
}
struct Container {
    object: bool,
    first: bool,
    keys: HashSet<Vec<u8>>,
}
impl<'a> Reader<'a> {
    fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }
    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.position += 1;
        }
    }
    fn consume(&mut self, byte: u8) -> bool {
        self.whitespace();
        if self.peek() == Some(byte) {
            self.position += 1;
            true
        } else {
            false
        }
    }
    fn take(&mut self, byte: u8) -> Result<()> {
        if self.consume(byte) {
            Ok(())
        } else {
            Err(Error::Invalid)
        }
    }
    fn start_container(&mut self) -> Option<Container> {
        self.whitespace();
        let object = match self.peek() {
            Some(b'{') => true,
            Some(b'[') => false,
            _ => return None,
        };
        self.position += 1;
        Some(Container {
            object,
            first: true,
            keys: HashSet::new(),
        })
    }
    fn value(&mut self) -> Result<Value<'a>> {
        let Some(first) = self.start_container() else {
            return self.scalar();
        };
        let mut stack = Vec::new();
        stack.try_reserve(1)?;
        stack.push(first);
        // No recursive parsing or recursive owned tree drops, including unknown
        // fields. Nested objects still reject duplicate *decoded* byte keys.
        while let Some(mut current) = stack.pop() {
            let close = if current.object { b'}' } else { b']' };
            if self.consume(close) {
                continue;
            }
            if !current.first {
                self.take(b',')?;
            }
            current.first = false;
            if current.object {
                let key = self.string()?;
                unique(&mut current.keys, key)?;
                self.take(b':')?;
            }
            stack.try_reserve(2)?;
            stack.push(current);
            if let Some(child) = self.start_container() {
                stack.push(child);
            } else {
                self.scalar()?;
            }
        }
        Ok(Value::Other)
    }
    fn scalar(&mut self) -> Result<Value<'a>> {
        self.whitespace();
        match self.peek() {
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'-' | b'0'..=b'9') => Ok(Value::Number(self.number()?)),
            Some(b't') => {
                self.literal(b"true")?;
                Ok(Value::Boolean(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(Value::Boolean(false))
            }
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(Value::Other)
            }
            _ => Err(Error::Invalid),
        }
    }
    fn literal(&mut self, value: &[u8]) -> Result<()> {
        if !self.input[self.position..].starts_with(value) {
            return Err(Error::Invalid);
        }
        self.position += value.len();
        Ok(())
    }
    fn digits(&mut self) -> Result<()> {
        let first = self.position;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.position += 1;
        }
        if self.position == first {
            Err(Error::Invalid)
        } else {
            Ok(())
        }
    }
    fn number(&mut self) -> Result<&'a [u8]> {
        let start = self.position;
        if self.peek() == Some(b'-') {
            self.position += 1;
        }
        if self.peek() == Some(b'0') {
            self.position += 1;
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(Error::Invalid);
            }
        } else {
            self.digits()?;
        }
        if self.peek() == Some(b'.') {
            self.position += 1;
            self.digits()?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            self.position += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.position += 1;
            }
            self.digits()?;
        }
        Ok(&self.input[start..self.position])
    }
    fn hex4(&mut self) -> Result<u16> {
        let mut value = 0;
        for _ in 0..4 {
            let nibble = match self.peek() {
                Some(b'0'..=b'9') => self.input[self.position] - b'0',
                Some(b'a'..=b'f') => self.input[self.position] - b'a' + 10,
                Some(b'A'..=b'F') => self.input[self.position] - b'A' + 10,
                _ => return Err(Error::Invalid),
            };
            value = (value << 4) | u16::from(nibble);
            self.position += 1;
        }
        Ok(value)
    }
    fn string(&mut self) -> Result<Vec<u8>> {
        self.take(b'"')?;
        let mut out = Writer(Vec::new());
        let mut high = 0_u16;
        loop {
            // Current acceptance references use the default x64 SSE2 parser
            // trait (16 bytes). Preserve its observable skip boundary, including
            // raw literals skipped while a high surrogate is pending. This is
            // safe byte scanning, not a CPU feature/optimization claim. The
            // explicit source-trait assertion rejects a changed reference.
            while self.input.len() - self.position >= 16 {
                let chunk = &self.input[self.position..self.position + 16];
                let literal = chunk
                    .iter()
                    .position(|b| *b < 32 || *b == b'"' || *b == b'\\')
                    .unwrap_or(16);
                out.raw(&chunk[..literal])?;
                self.position += literal;
                if literal < 16 {
                    break;
                }
            }
            let byte = self.peek().ok_or(Error::Invalid)?;
            self.position += 1;
            match byte {
                b'"' if high == 0 => return Ok(out.0),
                b'\\' => {
                    let escape = self.peek().ok_or(Error::Invalid)?;
                    self.position += 1;
                    if high != 0 && escape != b'u' {
                        return Err(Error::Invalid);
                    }
                    match escape {
                        b'"' | b'\\' | b'/' => out.raw(&[escape])?,
                        b'b' => out.raw(&[8])?,
                        b'f' => out.raw(&[12])?,
                        b'n' => out.raw(b"\n")?,
                        b'r' => out.raw(b"\r")?,
                        b't' => out.raw(b"\t")?,
                        b'u' => {
                            let cp = self.hex4()?;
                            if (0xd800..=0xdbff).contains(&cp) {
                                if high != 0 {
                                    return Err(Error::Invalid);
                                }
                                high = cp;
                                continue;
                            }
                            let scalar = if (0xdc00..=0xdfff).contains(&cp) {
                                if high == 0 {
                                    return Err(Error::Invalid);
                                }
                                let scalar = 0x10000
                                    + ((u32::from(high) - 0xd800) << 10)
                                    + (u32::from(cp) - 0xdc00);
                                high = 0;
                                scalar
                            } else {
                                u32::from(cp)
                            };
                            let mut bytes = [0; 4];
                            let encoded = char::from_u32(scalar)
                                .ok_or(Error::Invalid)?
                                .encode_utf8(&mut bytes);
                            out.raw(encoded.as_bytes())?;
                        }
                        _ => return Err(Error::Invalid),
                    }
                }
                0..32 | b'"' => return Err(Error::Invalid),
                _ if high != 0 => return Err(Error::Invalid),
                _ => out.raw(&[byte])?,
            }
        }
    }
}

#[cfg(test)]
#[path = "manager_config_tests.rs"]
mod tests;
