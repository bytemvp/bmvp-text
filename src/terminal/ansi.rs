#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MultiByteToWideChar(
        CodePage: u32,
        dwFlags: u32,
        lpMultiByteStr: *const u8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;
}

pub fn decode_oem(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(target_os = "windows")]
    {
        let len = bytes.len() as i32;
        let wide_len =
            unsafe { MultiByteToWideChar(1, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0) };
        if wide_len > 0 {
            let mut wide_buf = vec![0u16; wide_len as usize];
            unsafe {
                MultiByteToWideChar(1, 0, bytes.as_ptr(), len, wide_buf.as_mut_ptr(), wide_len);
            }
            return String::from_utf16_lossy(&wide_buf);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn decode_base64_utf8(input: &str) -> Option<String> {
    fn val(b: u8) -> Option<u32> {
        match b {
            b'A'..=b'Z' => Some((b - b'A') as u32),
            b'a'..=b'z' => Some((b - b'a' + 26) as u32),
            b'0'..=b'9' => Some((b - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for chunk in bytes.as_chunks::<4>().0 {
        let v0 = val(chunk[0])?;
        let v1 = val(chunk[1])?;
        let v2 = if chunk[2] == b'=' {
            None
        } else {
            Some(val(chunk[2])?)
        };
        let v3 = if chunk[3] == b'=' {
            None
        } else {
            Some(val(chunk[3])?)
        };

        let n = (v0 << 18) | (v1 << 12) | (v2.unwrap_or(0) << 6) | v3.unwrap_or(0);
        out.push(((n >> 16) & 0xff) as u8);
        if v2.is_some() {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if v3.is_some() {
            out.push((n & 0xff) as u8);
        }
    }

    String::from_utf8(out).ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnsiSpan {
    pub text: String,
    pub fg_rgb: Option<u32>,
    pub bg_rgb: Option<u32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Default, Clone, Copy)]
struct AnsiStyleState {
    fg_rgb: Option<u32>,
    bg_rgb: Option<u32>,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
}

fn ansi_16_fg(code: u32, is_light: bool) -> Option<u32> {
    if is_light {
        match code {
            30 => Some(0x24292e),
            31 => Some(0xcf222e),
            32 => Some(0x1a7f37),
            33 => Some(0x9a6700),
            34 => Some(0x0969da),
            35 => Some(0x8250df),
            36 => Some(0x0e7490),
            37 => Some(0x57606a),
            90 => Some(0x6e7781),
            91 => Some(0xa40e26),
            92 => Some(0x116329),
            93 => Some(0x7d4e00),
            94 => Some(0x0550ae),
            95 => Some(0x6639ba),
            96 => Some(0x155e75),
            97 => Some(0x24292e),
            _ => None,
        }
    } else {
        match code {
            30 => Some(0x4b5563),
            31 => Some(0xf87171),
            32 => Some(0x4ade80),
            33 => Some(0xfacc15),
            34 => Some(0x60a5fa),
            35 => Some(0xc084fc),
            36 => Some(0x22d3ee),
            37 => Some(0xe5e7eb),
            90 => Some(0x9ca3af),
            91 => Some(0xfca5a5),
            92 => Some(0x86efac),
            93 => Some(0xfde047),
            94 => Some(0x93c5fd),
            95 => Some(0xd8b4fe),
            96 => Some(0x67e8f9),
            97 => Some(0xffffff),
            _ => None,
        }
    }
}

fn ansi_16_bg(code: u32) -> Option<u32> {
    match code {
        40 => Some(0x1f2328),
        41 => Some(0xcf222e),
        42 => Some(0x1a7f37),
        43 => Some(0xbf8700),
        44 => Some(0x0969da),
        45 => Some(0x8250df),
        46 => Some(0x0e7490),
        47 => Some(0xd0d7de),
        100 => Some(0x57606a),
        101 => Some(0xfa4549),
        102 => Some(0x2da44e),
        103 => Some(0xd4a72c),
        104 => Some(0x218bff),
        105 => Some(0xa475f9),
        106 => Some(0x1b7c83),
        107 => Some(0xf6f8fa),
        _ => None,
    }
}

fn ansi_256_color(idx: u32) -> u32 {
    if idx < 16 {
        let base = [
            0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
            0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
        ];
        return base[idx as usize];
    }
    if idx < 232 {
        let n = idx - 16;
        let r = n / 36;
        let g = (n % 36) / 6;
        let b = n % 6;
        let map = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
        return (map(r) << 16) | (map(g) << 8) | map(b);
    }
    let gray = 8 + (idx - 232) * 10;
    (gray << 16) | (gray << 8) | gray
}

/// 单遍快速剥离 ANSI CSI/OSC 序列与 `\r`，避免构造临时 Vec<AnsiSpan>
pub fn strip_ansi(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == 0x1b {
            if i + 1 < len && bytes[i + 1] == b'[' {
                let mut j = i + 2;
                while j < len && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                i = if j < len { j + 1 } else { len };
                continue;
            }
            if i + 1 < len && bytes[i + 1] == b']' {
                let mut j = i + 2;
                while j < len {
                    if bytes[j] == 0x07 {
                        j += 1;
                        break;
                    }
                    if bytes[j] == 0x1b && j + 1 < len && bytes[j + 1] == b'\\' {
                        j += 2;
                        break;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            i += 1;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            if ch != '\r' {
                out.push(ch);
            }
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    out
}

pub fn parse_ansi_spans(input: &str, is_light: bool) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut state = AnsiStyleState::default();
    let mut buf = String::new();

    let flush_buf = |spans: &mut Vec<AnsiSpan>, buf: &mut String, st: AnsiStyleState| {
        if !buf.is_empty() {
            spans.push(AnsiSpan {
                text: std::mem::take(buf),
                fg_rgb: st.fg_rgb,
                bg_rgb: st.bg_rgb,
                bold: st.bold,
                italic: st.italic,
                underline: st.underline,
                inverse: st.inverse,
            });
        }
    };

    while i < len {
        if bytes[i] == 0x1b {
            flush_buf(&mut spans, &mut buf, state);
            if i + 1 < len && bytes[i + 1] == b'[' {
                // CSI sequence: ESC [ ... <final byte 0x40..=0x7E>
                let mut j = i + 2;
                while j < len && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                if j < len {
                    if bytes[j] == b'm' {
                        let params_str = std::str::from_utf8(&bytes[i + 2..j]).unwrap_or("");
                        let nums: Vec<u32> = if params_str.is_empty() {
                            vec![0]
                        } else {
                            params_str
                                .split(';')
                                .filter_map(|p| p.parse::<u32>().ok())
                                .collect()
                        };
                        let mut k = 0;
                        while k < nums.len() {
                            match nums[k] {
                                0 => state = AnsiStyleState::default(),
                                1 => state.bold = true,
                                3 => state.italic = true,
                                4 => state.underline = true,
                                7 => state.inverse = true,
                                22 => state.bold = false,
                                23 => state.italic = false,
                                24 => state.underline = false,
                                27 => state.inverse = false,
                                39 => state.fg_rgb = None,
                                49 => state.bg_rgb = None,
                                30..=37 | 90..=97 => {
                                    state.fg_rgb = ansi_16_fg(nums[k], is_light);
                                }
                                40..=47 | 100..=107 => {
                                    state.bg_rgb = ansi_16_bg(nums[k]);
                                }
                                38 if k + 2 < nums.len() && nums[k + 1] == 5 => {
                                    state.fg_rgb = Some(ansi_256_color(nums[k + 2]));
                                    k += 2;
                                }
                                48 if k + 2 < nums.len() && nums[k + 1] == 5 => {
                                    state.bg_rgb = Some(ansi_256_color(nums[k + 2]));
                                    k += 2;
                                }
                                38 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                    let r = nums[k + 2].min(255);
                                    let g = nums[k + 3].min(255);
                                    let b = nums[k + 4].min(255);
                                    state.fg_rgb = Some((r << 16) | (g << 8) | b);
                                    k += 4;
                                }
                                48 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                    let r = nums[k + 2].min(255);
                                    let g = nums[k + 3].min(255);
                                    let b = nums[k + 4].min(255);
                                    state.bg_rgb = Some((r << 16) | (g << 8) | b);
                                    k += 4;
                                }
                                _ => {}
                            }
                            k += 1;
                        }
                    }
                    i = j + 1;
                    continue;
                }
            } else if i + 1 < len && bytes[i + 1] == b']' {
                // OSC sequence: ESC ] ... BEL or ST
                let mut j = i + 2;
                while j < len {
                    if bytes[j] == 0x07 {
                        j += 1;
                        break;
                    }
                    if bytes[j] == 0x1b && j + 1 < len && bytes[j + 1] == b'\\' {
                        j += 2;
                        break;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            i += 1;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            if ch != '\r' {
                buf.push(ch);
            }
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    flush_buf(&mut spans, &mut buf, state);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ansi_parse_styles_and_strip() {
        let raw =
            "\x1b[1;3;4;32mhello\x1b[22;23;24m \x1b[7;48;2;10;20;30;38;2;255;0;0mworld\x1b[0m\r";
        let spans = parse_ansi_spans(raw, true);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "hello");
        assert!(spans[0].bold && spans[0].italic && spans[0].underline);
        assert!(spans[0].fg_rgb.is_some());

        assert_eq!(spans[1].text, " ");
        assert!(!spans[1].bold && !spans[1].italic && !spans[1].underline);

        assert_eq!(spans[2].text, "world");
        assert!(spans[2].inverse);
        assert_eq!(spans[2].fg_rgb, Some(0xff0000));
        assert_eq!(spans[2].bg_rgb, Some(0x0a141e));

        assert_eq!(strip_ansi(raw), "hello world");
        assert_eq!(strip_ansi("\x1b]0;title\x07ok"), "ok");
    }

    #[test]
    fn test_base64_decode() {
        assert_eq!(decode_base64_utf8("aGVsbG8=").as_deref(), Some("hello"));
        assert_eq!(decode_base64_utf8("5Lit5paH").as_deref(), Some("中文"));
    }
}
