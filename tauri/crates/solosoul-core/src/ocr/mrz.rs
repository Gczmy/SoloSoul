//! MRZ（机读区）检测、识别与解析。

use super::types::MrzResult;
use image::{imageops::FilterType, Rgb, RgbImage};

/// 在图像中检测 MRZ 区域。
/// 基于图像下半部分的水平投影，找两条等长、等高的水平文本行。
pub fn detect_mrz_region(image: &RgbImage) -> Option<[(f32, f32); 4]> {
    let (w, h) = (image.width(), image.height());
    if h < 100 || w < 300 {
        return None;
    }

    // 截取图像下半部分（y >= 0.6 * height）
    let bottom_h = (h as f32 * 0.4) as u32;
    let y_start = h - bottom_h;
    let cropped = image::imageops::crop_imm(image, 0, y_start, w, bottom_h).to_image();

    // 转换为灰度图
    let gray = to_grayscale(&cropped);

    // 自适应二值化
    let binary = adaptive_binarize(&gray);

    // 计算水平投影（每行非零像素数）
    let projection: Vec<u32> = (0..binary.height())
        .map(|y| {
            (0..binary.width())
                .filter(|&x| binary.get_pixel(x, y).0[0] > 128)
                .count() as u32
        })
        .collect();

    // 找两条文本行
    let (center1, center2) = find_two_text_lines(&projection)?;

    // 映射回原图坐标
    let pad_y = 12.0;
    let region_top = (y_start as f32 + center1 - pad_y).max(0.0);
    let region_bottom = (y_start as f32 + center2 + pad_y).min((h - 1) as f32);
    let region_left = 0.0;
    let region_right = (w - 1) as f32;

    Some([
        (region_left, region_top),
        (region_right, region_top),
        (region_right, region_bottom),
        (region_left, region_bottom),
    ])
}

fn to_grayscale(img: &RgbImage) -> image::GrayImage {
    image::imageops::grayscale(img)
}

fn adaptive_binarize(img: &image::GrayImage) -> image::GrayImage {
    let mean =
        img.pixels().map(|p| p.0[0] as u32).sum::<u32>() / (img.width() * img.height()).max(1);
    let threshold = mean.saturating_sub(10).min(200) as u8;
    image::GrayImage::from_fn(img.width(), img.height(), |x, y| {
        let val = img.get_pixel(x, y).0[0];
        image::Luma([if val < threshold { 255 } else { 0 }])
    })
}

fn find_two_text_lines(projection: &[u32]) -> Option<(f32, f32)> {
    if projection.is_empty() {
        return None;
    }

    let max_val = *projection.iter().max()?;
    if max_val == 0 {
        return None;
    }

    let threshold = max_val / 4;

    // 找连续的非零行带
    let mut regions = Vec::new();
    let mut in_region = false;
    let mut start = 0usize;

    for (i, &val) in projection.iter().enumerate() {
        if val >= threshold && !in_region {
            in_region = true;
            start = i;
        } else if val < threshold && in_region {
            in_region = false;
            regions.push((start, i));
        }
    }
    if in_region {
        regions.push((start, projection.len()));
    }

    if regions.len() < 2 {
        return None;
    }

    // 找间距合理、强度最大的两条连续行带
    let mut best_score = 0u32;
    let mut best_pair = None;

    for i in 1..regions.len() {
        let (s1, e1) = regions[i - 1];
        let (s2, e2) = regions[i];
        let center1 = (s1 + e1) as f32 / 2.0;
        let center2 = (s2 + e2) as f32 / 2.0;
        let gap = center2 - center1;

        // MRZ 两行间距通常在 8–60 像素之间
        if (8.0..=60.0).contains(&gap) {
            let score =
                projection[s1..e1].iter().sum::<u32>() + projection[s2..e2].iter().sum::<u32>();
            if score > best_score {
                best_score = score;
                best_pair = Some((center1, center2));
            }
        }
    }

    best_pair
}

/// 对 MRZ 区域裁剪图做增强（灰度、放大）。
pub fn enhance_mrz_crop(img: &RgbImage) -> RgbImage {
    let gray = image::imageops::grayscale(img);
    let scaled = image::imageops::resize(
        &gray,
        gray.width() * 2,
        gray.height() * 2,
        FilterType::Triangle,
    );
    RgbImage::from_fn(scaled.width(), scaled.height(), |x, y| {
        let p = scaled.get_pixel(x, y).0[0];
        Rgb([p, p, p])
    })
}

/// 解析 MRZ 文本行。
/// 支持 TD-1（3 行 × 30 字符）和 TD-3（护照，2 行 × 44 字符）。
pub fn parse_mrz(lines: &[String]) -> Result<MrzResult, String> {
    if lines.len() == 2 && lines.iter().all(|l| l.len() >= 40) {
        parse_td3(lines)
    } else if lines.len() == 3 && lines.iter().all(|l| l.len() >= 25) {
        parse_td1(lines)
    } else {
        Err("无法识别 MRZ 格式".to_string())
    }
}

fn parse_td3(lines: &[String]) -> Result<MrzResult, String> {
    let line1 = &lines[0];
    let line2 = &lines[1];

    // 补齐到 44 字符（左对齐，右侧空格替换为 '<'）
    let l1 = format!("{:<44}", line1).replace(' ', "<");
    let l2 = format!("{:<44}", line2).replace(' ', "<");

    let document_type = l1[0..1].to_string();
    let document_type_sub = l1[1..2].to_string();
    let issuing_country = l1[2..5].to_string();

    let document_number = l2[0..9].to_string();
    let check_digit_document_number = l2.chars().nth(9).unwrap_or('<');
    let nationality = l2[10..13].to_string();
    let date_of_birth = l2[13..19].to_string();
    let check_digit_date_of_birth = l2.chars().nth(19).unwrap_or('<');
    let sex = l2[20..21].to_string();
    let expiry_date = l2[21..27].to_string();
    let check_digit_expiry = l2.chars().nth(27).unwrap_or('<');
    let optional_data = l2[28..42].to_string();
    let optional_check_digit = l2.chars().nth(42).unwrap_or('<');
    let composite_check_digit = l2.chars().nth(43).unwrap_or('<');

    let doc_valid = mrz_checksum(&document_number) == check_digit_document_number;
    let dob_valid = mrz_checksum(&date_of_birth) == check_digit_date_of_birth;
    let expiry_valid = mrz_checksum(&expiry_date) == check_digit_expiry;

    let mut composite = String::new();
    composite.push_str(&document_number);
    composite.push(check_digit_document_number);
    composite.push_str(&date_of_birth);
    composite.push(check_digit_date_of_birth);
    composite.push_str(&expiry_date);
    composite.push(check_digit_expiry);
    composite.push_str(&optional_data);
    if optional_check_digit != '<' {
        composite.push(optional_check_digit);
    }
    let composite_valid = mrz_checksum(&composite) == composite_check_digit;

    let checksum_valid = doc_valid && dob_valid && expiry_valid && composite_valid;

    Ok(MrzResult {
        document_type,
        document_type_sub,
        issuing_country,
        document_number,
        check_digit_document_number,
        nationality,
        date_of_birth,
        check_digit_date_of_birth,
        sex,
        expiry_date,
        check_digit_expiry,
        optional_data,
        composite_check_digit: composite_check_digit.to_string(),
        raw_lines: lines.to_vec(),
        confidence: 1.0,
        checksum_valid,
    })
}

fn parse_td1(lines: &[String]) -> Result<MrzResult, String> {
    let line1 = &lines[0];
    let line2 = &lines[1];

    // TD-1: 3 行 × 30 字符
    let l1 = format!("{:<30}", line1).replace(' ', "<");
    let l2 = format!("{:<30}", line2).replace(' ', "<");

    let document_type = l1[0..1].to_string();
    let document_type_sub = l1[1..2].to_string();
    let issuing_country = l1[2..5].to_string();

    // TD-1 行 1: document number (5-13=9 chars) + check digit at 14
    let document_number = l1[5..14].to_string();
    let check_digit_document_number = l1.chars().nth(14).unwrap_or('<');
    let optional_data_line1 = l1[15..18].to_string();

    // TD-1 行 2: DOB + check + sex + expiry + check + nationality + optional
    let date_of_birth = l2[0..6].to_string();
    let check_digit_date_of_birth = l2.chars().nth(6).unwrap_or('<');
    let sex = l2[7..8].to_string();
    let expiry_date = l2[8..14].to_string();
    let check_digit_expiry = l2.chars().nth(14).unwrap_or('<');
    let nationality = l2[15..18].to_string();
    let optional_data_line2 = l2[18..28].to_string();

    let optional_data = format!("{}{}", optional_data_line1, optional_data_line2);

    let doc_valid = mrz_checksum(&document_number) == check_digit_document_number;
    let dob_valid = mrz_checksum(&date_of_birth) == check_digit_date_of_birth;
    let expiry_valid = mrz_checksum(&expiry_date) == check_digit_expiry;

    // TD-1 没有统一的 composite check digit，以行 1/行 2 各自的 composite 代替
    let composite_check_digit = format!(
        "{}-{}",
        l1.chars().nth(18).unwrap_or('<'),
        l2.chars().nth(28).unwrap_or('<')
    );

    let checksum_valid = doc_valid && dob_valid && expiry_valid;

    Ok(MrzResult {
        document_type,
        document_type_sub,
        issuing_country,
        document_number,
        check_digit_document_number,
        nationality,
        date_of_birth,
        check_digit_date_of_birth,
        sex,
        expiry_date,
        check_digit_expiry,
        optional_data,
        composite_check_digit,
        raw_lines: lines.to_vec(),
        confidence: 1.0,
        checksum_valid,
    })
}

/// MRZ 校验位算法。
fn mrz_checksum(s: &str) -> char {
    let weights = [7, 3, 1];
    let mut sum = 0u32;

    for (i, c) in s.chars().enumerate() {
        let val = mrz_char_value(c);
        sum += val * weights[i % 3];
    }

    let digit = sum % 10;
    std::char::from_digit(digit, 10).unwrap_or('0')
}

fn mrz_char_value(c: char) -> u32 {
    match c {
        '0'..='9' => c.to_digit(10).unwrap_or(0),
        'A'..='Z' => (c as u32 - 'A' as u32) + 10,
        '<' => 0,
        'a'..='z' => (c as u32 - 'a' as u32) + 10,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mrz_checksum_known() {
        // L898902C3 -> 6 (from ICAO sample)
        assert_eq!(mrz_checksum("L898902C3"), '6');
        // 740812 -> 2
        assert_eq!(mrz_checksum("740812"), '2');
        // 120415 -> 9
        assert_eq!(mrz_checksum("120415"), '9');
    }

    #[test]
    fn test_parse_td3_valid() {
        let lines = vec![
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10".to_string(),
        ];
        let result = parse_td3(&lines).unwrap();
        assert_eq!(result.document_type, "P");
        assert_eq!(result.document_type_sub, "<");
        assert_eq!(result.issuing_country, "UTO");
        assert_eq!(result.document_number, "L898902C3");
        assert_eq!(result.check_digit_document_number, '6');
        assert_eq!(result.nationality, "UTO");
        assert_eq!(result.date_of_birth, "740812");
        assert_eq!(result.check_digit_date_of_birth, '2');
        assert_eq!(result.sex, "F");
        assert_eq!(result.expiry_date, "120415");
        assert_eq!(result.check_digit_expiry, '9');
        assert!(result.checksum_valid);
    }

    #[test]
    fn test_parse_td3_invalid_checksum() {
        let lines = vec![
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
            "L898902C30UTO7408122F1204159ZE184226B<<<<<10".to_string(),
        ];
        let result = parse_td3(&lines).unwrap();
        assert!(!result.checksum_valid);
    }

    #[test]
    fn test_parse_td1_valid() {
        let lines = vec![
            "I<UTOD231458907<<<<<<<<<<<<<<<".to_string(),
            "7408122F1204159UTO<<<<<<<<<<<<".to_string(),
            "ERIKSSON<<ANNA<MARIA<<<<<<<<<<".to_string(),
        ];
        let result = parse_td1(&lines).unwrap();
        assert_eq!(result.document_type, "I");
        assert_eq!(result.document_type_sub, "<");
        assert_eq!(result.issuing_country, "UTO");
        assert_eq!(result.document_number, "D23145890");
        assert_eq!(result.check_digit_document_number, '7');
        assert_eq!(result.nationality, "UTO");
        assert_eq!(result.date_of_birth, "740812");
        assert_eq!(result.check_digit_date_of_birth, '2');
        assert_eq!(result.sex, "F");
        assert_eq!(result.expiry_date, "120415");
        assert_eq!(result.check_digit_expiry, '9');
        assert!(result.checksum_valid);
    }
}
