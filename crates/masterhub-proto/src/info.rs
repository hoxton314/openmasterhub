/// Parse a firmware-version feature report: `<id> 0f <crc:4> "vv.vv.vv.vv"`.
pub fn parse_firmware_version(report: &[u8]) -> Option<String> {
    let text = report.get(6..)?;
    let end = text.iter().position(|&b| b == 0).unwrap_or(text.len());
    let s = std::str::from_utf8(&text[..end]).ok()?;
    (!s.is_empty()).then(|| s.to_owned())
}

/// Display geometry reported by the keypad (`0x3B`) and dial (`0x4B`) device-info query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayInfo {
    pub rows: u8,
    pub cols: u8,
    pub keys: u8,
    pub screen_width: u16,
    pub screen_height: u16,
    pub image_width: u8,
    pub image_height: u8,
}

/// Parse `<id> 0b 14 rows cols keys w:u16le h:u16le ?? img_w img_h`.
pub fn parse_display_info(report: &[u8]) -> Option<DisplayInfo> {
    if report.len() < 13 || report[1] != 0x0b {
        return None;
    }
    Some(DisplayInfo {
        rows: report[3],
        cols: report[4],
        keys: report[5],
        screen_width: u16::from_le_bytes([report[6], report[7]]),
        screen_height: u16::from_le_bytes([report[8], report[9]]),
        image_width: report[11],
        image_height: report[12],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(hex: &str) -> Vec<u8> {
        hex.split_whitespace().map(|b| u8::from_str_radix(b, 16).unwrap()).collect()
    }

    #[test]
    fn firmware() {
        let r = bytes("2e 0f 54 e2 61 51 30 31 2e 30 30 2e 30 35 2e 30 30 00 00");
        assert_eq!(parse_firmware_version(&r).as_deref(), Some("01.00.05.00"));
        assert_eq!(parse_firmware_version(&[0x38]), None);
    }

    #[test]
    fn display_info() {
        let k = parse_display_info(&bytes("3b 0b 14 03 05 0f e0 01 10 01 01 48 48")).unwrap();
        assert_eq!((k.rows, k.cols, k.keys), (3, 5, 15));
        assert_eq!((k.screen_width, k.screen_height), (480, 272));
        assert_eq!((k.image_width, k.image_height), (72, 72));
        let d = parse_display_info(&bytes("4b 0b 14 00 00 00 f0 00 f0 00 01 f0 f0")).unwrap();
        assert_eq!((d.keys, d.image_width, d.screen_height), (0, 240, 240));
    }
}
