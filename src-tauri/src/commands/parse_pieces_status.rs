use bitvec::prelude::*;

pub fn parse_pieces_status(raw_status_bytes: &[u8]) -> Result<BitVec<u8, Lsb0>, &'static str> {
    // Transform Vec<u8> to BitVec
    let mut pieces_status = BitVec::<u8, Lsb0>::new();
    for byte in raw_status_bytes {
        for bit in 0..8 {
            pieces_status.push(byte & (1 << bit) != 0);
        }
    }
    Ok(pieces_status)
}
