use std::io;
use std::io::prelude::*;
use std::u8;

pub fn mask(x: u8, z: usize) -> u8 {
    x & ((1 << z) - 1)
}

pub fn is_suffix(x: u8, y: u8, z: usize) -> bool {
    mask(x, z) == y
}

pub fn read_u8<R: Read>(r: &mut R) -> io::Result<Option<u8>> {
    let mut buf = [0; 1];
    let n = r.read(&mut buf)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf[0]))
    }
}

pub fn floor(x: usize, y: usize) -> usize {
    (x + y - 1) / y
}

pub fn reverse_bits(byte: u8) -> u8 {
    let mut result = 0;
    for i in 0..8 {
        result = result | ((byte >> i) & 1) << (8 - 1 - i);
    }
    result
}
