use anyhow::{Result, bail};

pub const TYPE58_HEADER_LEN: usize = 22;

pub fn is_audio_sdu(data: &[u8]) -> bool {
    data.len() >= 8
        && data[0] == 0x04
        && data[2] == 0x04
        && u16::from_le_bytes([data[4], data[5]]) == 0x0058
        && u16::from_le_bytes([data[6], data[7]]) == 0x0001
}

pub fn demux_audio_sdu(data: &[u8]) -> Result<Vec<&[u8]>> {
    if !is_audio_sdu(data) {
        bail!("not an AACP 0x58 audio SDU");
    }
    if data.len() < TYPE58_HEADER_LEN {
        bail!("truncated AACP 0x58 header");
    }

    let mut access_units = Vec::new();
    let mut offset = TYPE58_HEADER_LEN;
    while offset < data.len() {
        if data.len() - offset < 5 {
            bail!("truncated access-unit header at offset {offset}");
        }
        let length = data[offset + 4] as usize;
        let start = offset + 5;
        let end = start + length;
        if end > data.len() {
            bail!("access unit at offset {offset} exceeds packet boundary");
        }
        access_units.push(&data[start..end]);
        offset = end;
    }

    Ok(access_units)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header() -> Vec<u8> {
        let mut data = vec![0; TYPE58_HEADER_LEN];
        data[0] = 0x04;
        data[2] = 0x04;
        data[4] = 0x58;
        data[6] = 0x01;
        data
    }

    fn add_au(data: &mut Vec<u8>, timestamp: u32, au: &[u8]) {
        data.extend_from_slice(&timestamp.to_le_bytes());
        data.push(au.len() as u8);
        data.extend_from_slice(au);
    }

    #[test]
    fn valid_single_au() {
        let mut data = header();
        add_au(&mut data, 1, &[1, 2, 3]);
        assert_eq!(demux_audio_sdu(&data).unwrap(), vec![&[1, 2, 3][..]]);
    }

    #[test]
    fn valid_multiple_aus() {
        let mut data = header();
        add_au(&mut data, 1, &[1, 2]);
        add_au(&mut data, 2, &[3, 4, 5]);
        assert_eq!(
            demux_audio_sdu(&data).unwrap(),
            vec![&[1, 2][..], &[3, 4, 5][..]]
        );
    }

    #[test]
    fn rejects_truncated_header() {
        let mut data = header();
        data.truncate(12);
        assert!(demux_audio_sdu(&data).is_err());
    }

    #[test]
    fn rejects_invalid_au_length() {
        let mut data = header();
        data.extend_from_slice(&[0, 0, 0, 0, 4, 1, 2]);
        assert!(demux_audio_sdu(&data).is_err());
    }

    #[test]
    fn accepts_empty_audio_payload() {
        assert!(demux_audio_sdu(&header()).unwrap().is_empty());
    }

    #[test]
    fn rejects_empty_packet() {
        assert!(demux_audio_sdu(&[]).is_err());
    }

    #[test]
    fn rejects_invalid_type() {
        let mut data = header();
        data[4] = 0x57;
        assert!(demux_audio_sdu(&data).is_err());
    }

    #[test]
    fn rejects_garbage() {
        assert!(demux_audio_sdu(&[0xAA; 64]).is_err());
    }
}
