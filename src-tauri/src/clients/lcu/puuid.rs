//! Champion-select PUUID deobfuscation, matching Akari-Yessi's puuid-decrypt utility.

const XOR_MASK: [u8; 16] = [
    129, 112, 118, 169, 244, 81, 80, 155, 149, 152, 104, 19, 206, 145, 23, 231,
];

pub fn decrypt_puuid(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || !bytes.iter().enumerate().all(|(i, b)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
        || value == "00000000-0000-0000-0000-000000000000"
    {
        return None;
    }

    let hex: String = value.chars().filter(|c| *c != '-').collect();
    let mut result = String::with_capacity(36);
    for (i, mask) in XOR_MASK.iter().enumerate() {
        if matches!(i, 4 | 6 | 8 | 10) {
            result.push('-');
        }
        let byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()? ^ *mask;
        result.push_str(&format!("{byte:02x}"));
    }
    (result != "00000000-0000-0000-0000-000000000000").then_some(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_akari_reference_vector() {
        for value in [
            "906167b8-d673-63a8-d1dc-3d469bc442b2",
            "906167B8-D673-63A8-D1DC-3D469BC442B2",
        ] {
            assert_eq!(
                decrypt_puuid(value).as_deref(),
                Some("11111111-2222-3333-4444-555555555555")
            );
        }
    }

    #[test]
    fn rejects_invalid_and_empty_identifiers() {
        for value in [
            "",
            "not-a-puuid",
            "906167b8_d673-63a8-d1dc-3d469bc442b2",
            "g06167b8-d673-63a8-d1dc-3d469bc442b2",
            "906167b8-d673-63a8-d1dc-3d469bc442b2extra",
            "中06167b8-d673-63a8-d1dc-3d469bc442b2",
            "00000000-0000-0000-0000-000000000000",
            "817076a9-f451-509b-9598-6813ce9117e7",
        ] {
            assert_eq!(decrypt_puuid(value), None, "{value}");
        }
    }
}
