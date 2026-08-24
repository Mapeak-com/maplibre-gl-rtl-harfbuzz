//! Just enough of a protocol buffer reader to read a glyph server's reply.
//!
//! The writer this workspace ships is not used here on purpose: a test that reads with the same
//! code it writes with would agree with itself about a mistake.

/// A glyph as a glyph server sent it.
pub struct ReferenceGlyph {
    pub id: u32,
    pub bitmap: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub left: i32,
    pub top: i32,
    pub advance: u32,
}

pub fn read(data: &[u8]) -> Vec<ReferenceGlyph> {
    let mut glyphs = Vec::new();
    for (tag, field) in fields(data) {
        if tag == 1 {
            if let Field::Bytes(stack) = field {
                read_fontstack(stack, &mut glyphs);
            }
        }
    }
    glyphs
}

fn read_fontstack(data: &[u8], glyphs: &mut Vec<ReferenceGlyph>) {
    for (tag, field) in fields(data) {
        if tag != 3 {
            continue;
        }
        if let Field::Bytes(glyph) = field {
            glyphs.push(read_glyph(glyph));
        }
    }
}

fn read_glyph(data: &[u8]) -> ReferenceGlyph {
    let mut glyph = ReferenceGlyph {
        id: 0,
        bitmap: Vec::new(),
        width: 0,
        height: 0,
        left: 0,
        top: 0,
        advance: 0,
    };

    for (tag, field) in fields(data) {
        match (tag, field) {
            (1, Field::Varint(value)) => glyph.id = value as u32,
            (2, Field::Bytes(value)) => glyph.bitmap = value.to_vec(),
            (3, Field::Varint(value)) => glyph.width = value as u32,
            (4, Field::Varint(value)) => glyph.height = value as u32,
            (5, Field::Varint(value)) => glyph.left = unzigzag(value),
            (6, Field::Varint(value)) => glyph.top = unzigzag(value),
            (7, Field::Varint(value)) => glyph.advance = value as u32,
            _ => {}
        }
    }

    glyph
}

enum Field<'a> {
    Varint(u64),
    Bytes(&'a [u8]),
}

fn fields(data: &[u8]) -> impl Iterator<Item = (u32, Field<'_>)> {
    let mut at = 0;
    core::iter::from_fn(move || {
        let key = varint(data, &mut at)?;
        let (tag, wire_type) = ((key >> 3) as u32, key & 7);
        match wire_type {
            0 => Some((tag, Field::Varint(varint(data, &mut at)?))),
            2 => {
                let length = varint(data, &mut at)? as usize;
                let bytes = data.get(at..at + length)?;
                at += length;
                Some((tag, Field::Bytes(bytes)))
            }
            _ => None,
        }
    })
}

fn varint(data: &[u8], at: &mut usize) -> Option<u64> {
    let mut value = 0u64;
    let mut shift = 0;
    loop {
        let byte = *data.get(*at)?;
        *at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(value);
        }
        shift += 7;
    }
}

fn unzigzag(value: u64) -> i32 {
    ((value >> 1) as i32) ^ -((value & 1) as i32)
}
