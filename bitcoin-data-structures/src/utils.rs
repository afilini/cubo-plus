pub type Error = Box<dyn std::error::Error>;

pub fn from_hex(s: &str) -> Result<Vec<u8>, Error> {
    fn char_to_u8(c: char) -> Result<u8, Error> {
        Ok(c.to_digit(16).ok_or("Invalid hex digit")? as u8)
    }

    let chars = s.chars().collect::<Vec<_>>();
    let chunks_iter = chars
        .chunks_exact(2);
    if !chunks_iter.remainder().is_empty() {
        return Err("Odd number of chars".into());
    }
    
    chunks_iter.map(|c| Ok(char_to_u8(c[0])? << 4 | char_to_u8(c[1])?)).collect::<Result<Vec<_>, _>>()
}

pub fn to_hex<T: AsRef<[u8]>>(bytes: &T) -> String {
    fn u8_to_char(val: u8) -> char {
        match val & 0x0F {
            v @ 0..=9 => char::from(0x30 + v),
            v @ 0xA..=0xF => char::from(87 + v),
            _ => unreachable!()
        }
    }

    bytes.as_ref().iter().map(|b| [u8_to_char(*b >> 4), u8_to_char(*b)]).flatten().collect()
}

#[cfg(test)]
mod test {
    use crate::utils::{from_hex, to_hex};

    #[test]
    fn test_from_hex() {
        assert_eq!(from_hex("00").map_err(|e| e.to_string()), Ok(vec![0x00]));
        assert_eq!(from_hex("aabb").map_err(|e| e.to_string()), Ok(vec![0xaa, 0xbb]));
        assert_eq!(from_hex("000").map_err(|e| e.to_string()), Err("Odd number of chars".into()));
        assert_eq!(from_hex("0x").map_err(|e| e.to_string()), Err("Invalid hex digit".into()));
    }

    #[test]
    fn test_to_hex() {
        assert_eq!(to_hex(&vec![0xAA, 0xBB]), String::from("aabb"));
        assert_eq!(to_hex(&vec![0x00]), String::from("00"));
        assert_eq!(to_hex(&vec![0x99, 0xFF]), String::from("99ff"));
    }
}