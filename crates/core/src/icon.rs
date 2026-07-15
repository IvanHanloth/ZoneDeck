//! 进程图标提取：从可执行文件取图标，编码为 PNG data URI 供配置界面显示。
//!
//! PNG/base64 为手写最小实现（stored deflate 块，无压缩）：图标只有 32×32，
//! 未压缩也只有约 4KB，省掉外部依赖，保持核心二进制极小。

use std::ffi::c_void;

use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAP, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, DeleteObject, GetDC, GetDIBits,
    GetObjectW, ReleaseDC,
};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::UI::Shell::{SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGetFileInfoW};
use windows::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};
use windows::core::PCWSTR;

use crate::util::to_wide_null;

/// 顶到底、每像素 RGBA 四字节的位图。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconRgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

/// 提取可执行文件图标并编码为 `data:image/png;base64,...`。
/// 文件不存在或没有可用图标时返回 None。
pub fn icon_data_uri(path: &str) -> Option<String> {
    let icon = extract_icon_rgba(path)?;
    let png = encode_png(icon.width, icon.height, &icon.pixels)?;
    Some(format!("data:image/png;base64,{}", base64(&png)))
}

/// 通过 SHGetFileInfoW 取文件的大图标（32×32）并转为 RGBA。
pub fn extract_icon_rgba(path: &str) -> Option<IconRgba> {
    if path.is_empty() {
        return None;
    }
    let wide = to_wide_null(path);
    let mut info = SHFILEINFOW::default();
    let ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info),
            std::mem::size_of::<SHFILEINFOW>() as u32,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if ok == 0 || info.hIcon.is_invalid() {
        return None;
    }
    let rgba = hicon_to_rgba(info.hIcon);
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    rgba
}

fn hicon_to_rgba(hicon: HICON) -> Option<IconRgba> {
    unsafe {
        let mut icon_info = ICONINFO::default();
        GetIconInfo(hicon, &mut icon_info).ok()?;
        let color = icon_info.hbmColor;
        let mask = icon_info.hbmMask;

        let result = (|| {
            if color.is_invalid() {
                return None; // 单色图标，忽略
            }
            let mut bmp = BITMAP::default();
            if GetObjectW(
                color.into(),
                std::mem::size_of::<BITMAP>() as i32,
                Some(&mut bmp as *mut BITMAP as *mut c_void),
            ) == 0
            {
                return None;
            }
            let width = bmp.bmWidth as u32;
            let height = bmp.bmHeight as u32;
            if width == 0 || height == 0 || width > 256 || height > 256 {
                return None;
            }

            let hdc = GetDC(None);
            let header = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // 负值 = 顶到底行序
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            };
            let mut bgra = vec![0u8; (width * height * 4) as usize];
            let mut bmi = BITMAPINFO {
                bmiHeader: header,
                ..Default::default()
            };
            let lines = GetDIBits(
                hdc,
                color,
                0,
                height,
                Some(bgra.as_mut_ptr() as *mut c_void),
                &mut bmi,
                DIB_RGB_COLORS,
            );

            // 32bpp 图标不含 alpha 时（老式图标），用掩码位图补 alpha。
            let mut mask_bgra = None;
            if lines != 0 && bgra.chunks_exact(4).all(|px| px[3] == 0) && !mask.is_invalid() {
                let mut buf = vec![0u8; (width * height * 4) as usize];
                let mut mask_bmi = BITMAPINFO {
                    bmiHeader: header,
                    ..Default::default()
                };
                if GetDIBits(
                    hdc,
                    mask,
                    0,
                    height,
                    Some(buf.as_mut_ptr() as *mut c_void),
                    &mut mask_bmi,
                    DIB_RGB_COLORS,
                ) != 0
                {
                    mask_bgra = Some(buf);
                }
            }
            ReleaseDC(None, hdc);
            if lines == 0 {
                return None;
            }

            Some(IconRgba {
                width,
                height,
                pixels: bgra_to_rgba(&bgra, mask_bgra.as_deref()),
            })
        })();

        if !color.is_invalid() {
            let _ = DeleteObject(color.into());
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(mask.into());
        }
        result
    }
}

/// BGRA → RGBA。可选掩码（AND 掩码：非零 = 透明）用于补全缺失的 alpha 通道。
fn bgra_to_rgba(bgra: &[u8], mask: Option<&[u8]>) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for (i, px) in bgra.chunks_exact(4).enumerate() {
        let alpha = match mask {
            Some(m) => {
                // 掩码位图像素非零（白）表示透明，零（黑）表示不透明。
                if m[i * 4] == 0 { 255 } else { 0 }
            }
            None => px[3],
        };
        rgba.extend_from_slice(&[px[2], px[1], px[0], alpha]);
    }
    rgba
}

const PNG_SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];

/// 将顶到底 RGBA 像素编码为 PNG。`rgba` 长度必须等于 `width*height*4`。
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || rgba.len() != (width * height * 4) as usize {
        return None;
    }

    // IHDR：8 位深、颜色类型 6（真彩 + alpha）。
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);

    // 原始扫描线：每行前缀过滤器字节 0（None）。
    let stride = (width * 4) as usize;
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for row in rgba.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut png = Vec::with_capacity(raw.len() + 128);
    png.extend_from_slice(&PNG_SIGNATURE);
    png.extend_from_slice(&chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&chunk(b"IDAT", &zlib_stored(&raw)));
    png.extend_from_slice(&chunk(b"IEND", &[]));
    Some(png)
}

/// PNG 块：长度(BE) + 类型 + 数据 + CRC32(类型+数据)。
fn chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 12);
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(data.len() + 4);
    crc_input.extend_from_slice(name);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    out
}

/// zlib 流：2 字节头 + 未压缩 deflate stored 块 + Adler-32 校验。
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    const MAX_BLOCK: usize = 65535;
    let mut out = Vec::with_capacity(data.len() + data.len() / MAX_BLOCK * 5 + 16);
    out.extend_from_slice(&[0x78, 0x01]); // CMF/FLG：32K 窗口，无字典

    let mut blocks = data.chunks(MAX_BLOCK).peekable();
    loop {
        let Some(block) = blocks.next() else {
            // data 为空时也要输出一个空的最终块
            out.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
            break;
        };
        let is_final = blocks.peek().is_none();
        out.push(if is_final { 0x01 } else { 0x00 });
        let len = block.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(block);
        if is_final {
            break;
        }
    }

    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

pub fn base64(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for group in data.chunks(3) {
        let b0 = group[0] as u32;
        let b1 = *group.get(1).unwrap_or(&0) as u32;
        let b2 = *group.get(2).unwrap_or(&0) as u32;
        let bits = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(bits >> 18) as usize & 63] as char);
        out.push(TABLE[(bits >> 12) as usize & 63] as char);
        out.push(if group.len() > 1 {
            TABLE[(bits >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if group.len() > 2 {
            TABLE[bits as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vectors() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b"IEND"), 0xAE42_6082, "PNG IEND 块的经典 CRC");
    }

    #[test]
    fn adler32_matches_known_vectors() {
        assert_eq!(adler32(b""), 1);
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn encode_png_rejects_mismatched_dimensions() {
        assert!(encode_png(2, 2, &[0u8; 15]).is_none());
        assert!(encode_png(0, 2, &[]).is_none());
    }

    /// 测试用最小 PNG 解析：遍历块并校验 CRC，返回 (名称, 数据) 列表。
    fn parse_chunks(png: &[u8]) -> Vec<(String, Vec<u8>)> {
        assert_eq!(&png[..8], &PNG_SIGNATURE, "PNG 签名不正确");
        let mut chunks = Vec::new();
        let mut pos = 8;
        while pos < png.len() {
            let len = u32::from_be_bytes(png[pos..pos + 4].try_into().unwrap()) as usize;
            let name = String::from_utf8(png[pos + 4..pos + 8].to_vec()).unwrap();
            let data = png[pos + 8..pos + 8 + len].to_vec();
            let expected_crc =
                u32::from_be_bytes(png[pos + 8 + len..pos + 12 + len].try_into().unwrap());
            assert_eq!(
                crc32(&png[pos + 4..pos + 8 + len]),
                expected_crc,
                "{name} 块 CRC 校验失败"
            );
            chunks.push((name, data));
            pos += 12 + len;
        }
        chunks
    }

    /// 测试用 stored-deflate 解压（只支持我们自己产生的未压缩块）。
    fn inflate_stored(zlib: &[u8]) -> Vec<u8> {
        assert_eq!(zlib[0], 0x78, "zlib CMF 头");
        let mut out = Vec::new();
        let mut pos = 2;
        loop {
            let header = zlib[pos];
            assert_eq!(header & 0xFE, 0, "只应产生 stored（BTYPE=00）块");
            let len = u16::from_le_bytes([zlib[pos + 1], zlib[pos + 2]]) as usize;
            let nlen = u16::from_le_bytes([zlib[pos + 3], zlib[pos + 4]]);
            assert_eq!(!(len as u16), nlen, "LEN/NLEN 应互补");
            out.extend_from_slice(&zlib[pos + 5..pos + 5 + len]);
            pos += 5 + len;
            if header & 1 == 1 {
                break;
            }
        }
        let checksum = u32::from_be_bytes(zlib[pos..pos + 4].try_into().unwrap());
        assert_eq!(adler32(&out), checksum, "Adler-32 校验失败");
        out
    }

    #[test]
    fn encoded_png_has_valid_structure_and_round_trips_pixels() {
        // 2×2：红、绿、蓝、半透明白。
        #[rustfmt::skip]
        let pixels: Vec<u8> = vec![
            255, 0, 0, 255,   0, 255, 0, 255,
            0, 0, 255, 255,   255, 255, 255, 128,
        ];
        let png = encode_png(2, 2, &pixels).expect("编码应成功");

        let chunks = parse_chunks(&png);
        let names: Vec<&str> = chunks.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["IHDR", "IDAT", "IEND"]);

        let ihdr = &chunks[0].1;
        assert_eq!(u32::from_be_bytes(ihdr[0..4].try_into().unwrap()), 2);
        assert_eq!(u32::from_be_bytes(ihdr[4..8].try_into().unwrap()), 2);
        assert_eq!(ihdr[8], 8, "位深应为 8");
        assert_eq!(ihdr[9], 6, "颜色类型应为 RGBA");

        let raw = inflate_stored(&chunks[1].1);
        // 每行：过滤器字节 0 + 8 字节像素。
        assert_eq!(raw.len(), 2 * (1 + 8));
        assert_eq!(raw[0], 0);
        assert_eq!(&raw[1..9], &pixels[0..8]);
        assert_eq!(raw[9], 0);
        assert_eq!(&raw[10..18], &pixels[8..16]);
    }

    #[test]
    fn large_image_splits_into_multiple_stored_blocks() {
        // 150×150 RGBA = 90000 字节原始数据 > 单个 stored 块上限 65535。
        let size = 150u32;
        let pixels = vec![0xABu8; (size * size * 4) as usize];
        let png = encode_png(size, size, &pixels).expect("编码应成功");
        let chunks = parse_chunks(&png);
        let raw = inflate_stored(&chunks[1].1);
        assert_eq!(raw.len(), (size as usize) * (1 + size as usize * 4));
    }

    #[test]
    fn bgra_conversion_uses_mask_when_alpha_channel_is_empty() {
        // 两个像素：BGRA (1,2,3,0) 与 (4,5,6,0)，alpha 全 0。
        let bgra = [1, 2, 3, 0, 4, 5, 6, 0];
        // 掩码：第一个像素不透明（0），第二个透明（255）。
        let mask = [0, 0, 0, 0, 255, 255, 255, 0];
        let rgba = bgra_to_rgba(&bgra, Some(&mask));
        assert_eq!(rgba, vec![3, 2, 1, 255, 6, 5, 4, 0]);

        // 无掩码时保留原 alpha。
        let with_alpha = [1, 2, 3, 200];
        assert_eq!(bgra_to_rgba(&with_alpha, None), vec![3, 2, 1, 200]);
    }

    #[test]
    fn extracts_icon_from_a_real_executable() {
        let icon =
            extract_icon_rgba("C:\\Windows\\explorer.exe").expect("explorer.exe 应有可提取的图标");
        assert!(icon.width >= 16 && icon.height >= 16);
        assert_eq!(icon.pixels.len(), (icon.width * icon.height * 4) as usize);
        assert!(
            icon.pixels.chunks_exact(4).any(|px| px[3] > 0),
            "图标应至少有一个不透明像素"
        );

        let uri = icon_data_uri("C:\\Windows\\explorer.exe").expect("应生成 data URI");
        assert!(
            uri.starts_with("data:image/png;base64,iVBOR"),
            "base64 前缀应是 PNG 签名"
        );
    }

    #[test]
    fn missing_file_or_empty_path_yields_none() {
        assert!(extract_icon_rgba("").is_none());
        assert!(icon_data_uri("Z:\\no\\such\\file_12345.exe").is_none());
    }
}
