//! A protocol buffer writer with just the field types this crate's schema uses.
//!
//! Small enough not to be worth a dependency: three wire types, one of which is a nested message.

pub(crate) struct PbfWriter {
    buf: Vec<u8>,
}

impl PbfWriter {
    pub(crate) fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.buf
    }

    fn varint(&mut self, mut value: u64) {
        loop {
            let byte = (value & 0x7f) as u8;
            value >>= 7;
            if value == 0 {
                self.buf.push(byte);
                return;
            }
            self.buf.push(byte | 0x80);
        }
    }

    fn key(&mut self, tag: u32, wire_type: u32) {
        self.varint(((tag << 3) | wire_type) as u64);
    }

    pub(crate) fn uint32_field(&mut self, tag: u32, value: u32) {
        self.key(tag, 0);
        self.varint(value as u64);
    }

    /// Writes a zig-zag encoded signed field, which is what MapLibre's `readSVarint` decodes.
    pub(crate) fn sint32_field(&mut self, tag: u32, value: i32) {
        self.key(tag, 0);
        self.varint((((value << 1) ^ (value >> 31)) as u32) as u64);
    }

    pub(crate) fn bytes_field(&mut self, tag: u32, value: &[u8]) {
        self.key(tag, 2);
        self.varint(value.len() as u64);
        self.buf.extend_from_slice(value);
    }

    pub(crate) fn string_field(&mut self, tag: u32, value: &str) {
        self.bytes_field(tag, value.as_bytes());
    }

    /// Writes a length-delimited submessage, whose length is only known once `write` has run.
    pub(crate) fn message_field<F: FnOnce(&mut PbfWriter)>(&mut self, tag: u32, write: F) {
        self.key(tag, 2);
        // Reserve a single byte for the length and grow it afterwards if the message needs more,
        // which keeps the common case free of a second buffer.
        let length_position = self.buf.len();
        self.buf.push(0);
        let start = self.buf.len();
        write(self);
        let length = self.buf.len() - start;

        if length < 0x80 {
            self.buf[length_position] = length as u8;
            return;
        }

        let mut length_bytes = PbfWriter::new();
        length_bytes.varint(length as u64);
        let length_bytes = length_bytes.finish();
        self.buf
            .splice(length_position..length_position + 1, length_bytes);
    }
}
