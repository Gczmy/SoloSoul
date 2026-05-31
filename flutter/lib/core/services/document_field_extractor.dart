import 'dart:math';

import 'package:solosoul_flutter/core/models/ocr_result.dart';

// =============================================================================
// Document Field Extractor — 0MB 规则引擎
// =============================================================================
//
// 基于 OCR 输出的文本块位置信息（bounding box），通过纯规则推断文档字段。
// 无神经网络、无模型文件、完全离线，推理延迟 < 10ms。
//
// 架构：可插拔提取器链，按优先级匹配文档类型，命中即返回。

/// 单个提取字段，包含值和原文中的位置信息
class ExtractedField {
  final String value;
  final BoundingBox bbox;

  const ExtractedField({required this.value, required this.bbox});
}

/// 提取结果
class ExtractionResult {
  /// 推断的文档类型：'business_card', 'invoice', 'resume', 'generic'
  final String documentType;

  /// 字段名 → 提取结果
  final Map<String, ExtractedField> fields;

  /// 原始文本（用于回退展示）
  final String rawText;

  const ExtractionResult({
    required this.documentType,
    required this.fields,
    required this.rawText,
  });

  bool get hasFields => fields.isNotEmpty;
}

// ---------------------------------------------------------------------------
// 提取器接口
// ---------------------------------------------------------------------------

abstract class FieldExtractor {
  /// 文档类型标识
  String get documentType;

  /// 判断是否能处理当前 OCR 结果
  bool canHandle(String rawText, List<OcrBlock> blocks);

  /// 执行字段提取
  Map<String, ExtractedField> extract(List<OcrBlock> blocks);
}

// ---------------------------------------------------------------------------
// 提取器链
// ---------------------------------------------------------------------------

class FieldExtractorPipeline {
  static final List<FieldExtractor> _extractors = [
    BusinessCardExtractor(),
    InvoiceExtractor(),
    ResumeExtractor(),
    GenericFieldExtractor(),
  ];

  /// 运行提取器链，返回第一个命中的结果
  static ExtractionResult extract(String rawText, List<OcrBlock> blocks) {
    for (final extractor in _extractors) {
      if (extractor.canHandle(rawText, blocks)) {
        final fields = extractor.extract(blocks);
        return ExtractionResult(
          documentType: extractor.documentType,
          fields: fields,
          rawText: rawText,
        );
      }
    }
    // GenericFieldExtractor 作为保底总会命中
    return ExtractionResult(
      documentType: 'generic',
      fields: const {},
      rawText: rawText,
    );
  }
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

BoundingBox _mergeBboxes(List<BoundingBox> boxes) {
  if (boxes.isEmpty) {
    return const BoundingBox(x: 0, y: 0, width: 0, height: 0);
  }
  double minX = boxes.first.x;
  double minY = boxes.first.y;
  double maxX = boxes.first.x + boxes.first.width;
  double maxY = boxes.first.y + boxes.first.height;
  for (final b in boxes.skip(1)) {
    minX = min(minX, b.x);
    minY = min(minY, b.y);
    maxX = max(maxX, b.x + b.width);
    maxY = max(maxY, b.y + b.height);
  }
  return BoundingBox(
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  );
}

/// 按 y 坐标将文本块聚合成行（容忍 3% 的垂直偏移）
List<List<OcrBlock>> _clusterIntoRows(List<OcrBlock> blocks) {
  if (blocks.isEmpty) return [];
  final sorted = List<OcrBlock>.from(blocks)
    ..sort((a, b) => a.bbox.y.compareTo(b.bbox.y));
  final rows = <List<OcrBlock>>[[]];
  double currentY = sorted.first.bbox.y;
  for (final block in sorted) {
    if ((block.bbox.y - currentY).abs() > 0.03) {
      rows.add([block]);
      currentY = block.bbox.y;
    } else {
      rows.last.add(block);
    }
  }
  // 每行内按 x 排序
  for (final row in rows) {
    row.sort((a, b) => a.bbox.x.compareTo(b.bbox.x));
  }
  return rows.where((r) => r.isNotEmpty).toList();
}

// ---------------------------------------------------------------------------
// 1. 名片提取器
// ---------------------------------------------------------------------------

class BusinessCardExtractor implements FieldExtractor {
  @override
  String get documentType => 'business_card';

  static final _emailPattern = RegExp(
    r'\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
    caseSensitive: false,
  );
  static final _phonePattern = RegExp(
    r'(?:\+\d{1,3}[\s-]?)?\(?\d{1,4}\)?[\s-]?\d{1,4}[\s-]?\d{1,4}[\s-]?\d{0,4}',
  );
  static final _urlPattern = RegExp(
    r'https?://[^\s]+|www\.[^\s]+',
    caseSensitive: false,
  );

  @override
  bool canHandle(String rawText, List<OcrBlock> blocks) {
    // 名片判定：同时出现 email + phone，且文本块数量适中（5~20）
    final hasEmail = _emailPattern.hasMatch(rawText);
    final hasPhone = _phonePattern.hasMatch(rawText);
    final blockCount = blocks.length;
    return hasEmail && hasPhone && blockCount >= 5 && blockCount <= 25;
  }

  @override
  Map<String, ExtractedField> extract(List<OcrBlock> blocks) {
    final fields = <String, ExtractedField>{};
    final rows = _clusterIntoRows(blocks);

    // 找最大字号的行（通常是姓名）
    var nameRowIndex = -1;
    double maxHeight = 0;
    for (var i = 0; i < rows.length && i < 4; i++) {
      final h = rows[i].map((b) => b.bbox.height).reduce((a, b) => a + b) / rows[i].length;
      if (h > maxHeight) {
        maxHeight = h;
        nameRowIndex = i;
      }
    }

    if (nameRowIndex >= 0 && nameRowIndex < rows.length) {
      final nameBlocks = rows[nameRowIndex];
      final nameText = nameBlocks.map((b) => b.text).join(' ').trim();
      if (nameText.isNotEmpty && nameText.length < 40) {
        fields['name'] = ExtractedField(
          value: nameText,
          bbox: _mergeBboxes(nameBlocks.map((b) => b.bbox).toList()),
        );
      }
      // 第二大的可能是职位
      if (nameRowIndex + 1 < rows.length) {
        final titleBlocks = rows[nameRowIndex + 1];
        final titleText = titleBlocks.map((b) => b.text).join(' ').trim();
        if (titleText.isNotEmpty && titleText.length < 50) {
          fields['title'] = ExtractedField(
            value: titleText,
            bbox: _mergeBboxes(titleBlocks.map((b) => b.bbox).toList()),
          );
        }
      }
    }

    // Email
    for (final block in blocks) {
      final match = _emailPattern.firstMatch(block.text);
      if (match != null) {
        fields['email'] = ExtractedField(
          value: match.group(0)!,
          bbox: block.bbox,
        );
        break;
      }
    }

    // Phone
    for (final block in blocks) {
      final match = _phonePattern.firstMatch(block.text);
      if (match != null) {
        final phone = match.group(0)?.trim();
        if (phone != null && phone.length >= 7) {
          fields['phone'] = ExtractedField(value: phone, bbox: block.bbox);
          break;
        }
      }
    }

    // Website / URL
    for (final block in blocks) {
      final match = _urlPattern.firstMatch(block.text);
      if (match != null) {
        fields['website'] = ExtractedField(
          value: match.group(0)!,
          bbox: block.bbox,
        );
        break;
      }
    }

    return fields;
  }
}

// ---------------------------------------------------------------------------
// 2. 发票提取器
// ---------------------------------------------------------------------------

class InvoiceExtractor implements FieldExtractor {
  @override
  String get documentType => 'invoice';

  static final _invoiceKeywords = RegExp(
    r'invoice|发票|rechnung|facture',
    caseSensitive: false,
  );
  static final _amountPattern = RegExp(
    r'(?:total|amount|合计|总计|sum)[\s:]*[$€£¥]?\s*([\d,]+\.?\d*)',
    caseSensitive: false,
  );
  static final _standaloneAmount = RegExp(r'[$€£¥]\s*[\d,]+\.\d{2}');
  static final _datePattern = RegExp(
    r'\b(\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{4}[/-]\d{1,2}[/-]\d{1,2})\b',
  );
  static final _invoiceNoPattern = RegExp(
    r'(?:invoice\s*(?:#|no|number|num)[:\s]*|发票号码?[:\s]*)([A-Z0-9\-]+)',
    caseSensitive: false,
  );

  @override
  bool canHandle(String rawText, List<OcrBlock> blocks) {
    final text = rawText.toLowerCase();
    final hasKeyword = _invoiceKeywords.hasMatch(text);
    final hasAmount = _standaloneAmount.hasMatch(rawText) ||
        _amountPattern.hasMatch(rawText);
    return hasKeyword || (hasAmount && text.contains('total'));
  }

  @override
  Map<String, ExtractedField> extract(List<OcrBlock> blocks) {
    final fields = <String, ExtractedField>{};
    final rawText = blocks.map((b) => b.text).join(' ');

    // Invoice Number
    final invMatch = _invoiceNoPattern.firstMatch(rawText);
    if (invMatch != null) {
      final group0 = invMatch.group(0);
      final group1 = invMatch.group(1);
      if (group0 != null && group1 != null) {
        final block = _findBlockContaining(blocks, group0);
        fields['invoice_number'] = ExtractedField(
          value: group1.trim(),
          bbox: block?.bbox ?? const BoundingBox(x: 0, y: 0, width: 0, height: 0),
        );
      }
    }

    // Date
    final dateMatch = _datePattern.firstMatch(rawText);
    if (dateMatch != null) {
      final dateGroup = dateMatch.group(0);
      if (dateGroup != null) {
        final block = _findBlockContaining(blocks, dateGroup);
        fields['date'] = ExtractedField(
          value: dateGroup,
          bbox: block?.bbox ?? const BoundingBox(x: 0, y: 0, width: 0, height: 0),
        );
      }
    }

    // Total Amount — 优先匹配 "Total: $xxx" 模式，否则找最大金额
    final amountMatch = _amountPattern.firstMatch(rawText);
    if (amountMatch != null) {
      final group0 = amountMatch.group(0);
      final group1 = amountMatch.group(1);
      if (group0 != null && group1 != null) {
        final block = _findBlockContaining(blocks, group0);
        fields['total'] = ExtractedField(
          value: group1.trim(),
          bbox: block?.bbox ?? const BoundingBox(x: 0, y: 0, width: 0, height: 0),
        );
      }
    } else {
      // 找最大的独立金额（最可能是总计）
      double maxAmount = 0;
      OcrBlock? maxBlock;
      for (final block in blocks) {
        final m = _standaloneAmount.firstMatch(block.text);
        if (m != null) {
          final group0 = m.group(0);
          if (group0 == null) continue;
          final val = double.tryParse(
            group0.replaceAll(RegExp(r'[^\d.]'), ''),
          );
          if (val != null && val > maxAmount) {
            maxAmount = val;
            maxBlock = block;
          }
        }
      }
      if (maxBlock != null) {
        fields['total'] = ExtractedField(
          value: maxBlock.text.trim(),
          bbox: maxBlock.bbox,
        );
      }
    }

    return fields;
  }

  OcrBlock? _findBlockContaining(List<OcrBlock> blocks, String text) {
    for (final block in blocks) {
      if (block.text.contains(text)) return block;
    }
    return null;
  }
}

// ---------------------------------------------------------------------------
// 3. 简历提取器
// ---------------------------------------------------------------------------

class ResumeExtractor implements FieldExtractor {
  @override
  String get documentType => 'resume';

  /// 更宽泛的 section 关键字匹配（支持多词形式及常见变体）
  static final _sectionPattern = RegExp(
    r'^\s*(?:\d+\.?\s*)?(EDUCATION|ACADEMIC\s+BACKGROUND|WORK\s+EXPERIENCE|PROFESSIONAL\s+EXPERIENCE|'
    r'EMPLOYMENT|RESEARCH\s+INTERESTS|RESEARCH|MAJOR\s+ACHIEVEMENTS|PUBLICATIONS|'
    r'PROJECTS|SELECTED\s+PROJECTS|SKILLS|TECHNICAL\s+SKILLS|CORE\s+COMPETENCIES|'
    r'AWARDS|HONORS|LANGUAGES|INTERESTS|CERTIFICATIONS|SUMMARY|OBJECTIVE|PROFILE)\s*(?:\(.*\))?\s*$',
    caseSensitive: false,
  );

  static final _sectionKeywords = RegExp(
    r'\b(education|experience|skills|projects|publications|research|awards|languages|interests)\b',
    caseSensitive: false,
  );
  static final _emailPattern = RegExp(
    r'\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
    caseSensitive: false,
  );
  static final _phonePattern = RegExp(
    r'(?:\+\d{1,3}[\s-]?)?\(?\d{1,4}\)?[\s-]?\d{1,4}[\s-]?\d{1,4}',
  );
  static final _linkedinPattern = RegExp(
    r'linkedin\.com/in/[^\s]+',
    caseSensitive: false,
  );

  @override
  bool canHandle(String rawText, List<OcrBlock> blocks) {
    final text = rawText.toLowerCase();
    final sectionMatches = _sectionKeywords.allMatches(text).length;
    return sectionMatches >= 2 ||
        (text.contains('education') && text.contains('experience'));
  }

  @override
  Map<String, ExtractedField> extract(List<OcrBlock> blocks) {
    final fields = <String, ExtractedField>{};
    final rows = _clusterIntoRows(blocks);

    // ── 1. 按 section 分区提取 ──
    final sections = _extractSections(rows);
    for (final entry in sections.entries) {
      final sectionBlocks = entry.value;
      if (sectionBlocks.isEmpty) continue;
      final text = sectionBlocks.map((b) => b.text).join('\n').trim();
      if (text.length > 20) {
        fields[_fieldKeyForSection(entry.key)] = ExtractedField(
          value: text,
          bbox: _mergeBboxes(sectionBlocks.map((b) => b.bbox).toList()),
        );
      }
    }

    // ── 2. 姓名：第一行、字号最大、不含冒号和 @ ──
    if (rows.isNotEmpty) {
      OcrBlock? nameBlock;
      double maxH = 0;
      for (final row in rows.take(4)) {
        for (final block in row) {
          final h = block.bbox.height;
          final text = block.text.trim();
          if (h > maxH &&
              text.isNotEmpty &&
              text.length < 40 &&
              !text.contains(':') &&
              !text.contains('@') &&
              !_sectionPattern.hasMatch(text)) {
            maxH = h;
            nameBlock = block;
          }
        }
      }
      if (nameBlock != null) {
        fields['name'] = ExtractedField(value: nameBlock.text.trim(), bbox: nameBlock.bbox);
      }
    }

    // ── 3. Email / Phone / LinkedIn ──
    for (final block in blocks) {
      final emailMatch = _emailPattern.firstMatch(block.text);
      final emailGroup = emailMatch?.group(0);
      if (emailGroup != null && !fields.containsKey('email')) {
        fields['email'] = ExtractedField(value: emailGroup, bbox: block.bbox);
      }
      final phoneMatch = _phonePattern.firstMatch(block.text);
      final phoneGroup = phoneMatch?.group(0);
      if (phoneGroup != null && !fields.containsKey('phone')) {
        final phone = phoneGroup.trim();
        if (phone.length >= 7) {
          fields['phone'] = ExtractedField(value: phone, bbox: block.bbox);
        }
      }
      final linkedinMatch = _linkedinPattern.firstMatch(block.text);
      final linkedinGroup = linkedinMatch?.group(0);
      if (linkedinGroup != null && !fields.containsKey('linkedin')) {
        fields['linkedin'] = ExtractedField(value: linkedinGroup, bbox: block.bbox);
      }
    }

    return fields;
  }

  /// 将文本块按 section 分区。
  /// 返回：section 名（规范化）→ 该 section 下的所有文本块
  Map<String, List<OcrBlock>> _extractSections(List<List<OcrBlock>> rows) {
    final sections = <String, List<OcrBlock>>{};
    String currentSection = '';

    for (final row in rows) {
      final rowText = row.map((b) => b.text).join(' ').trim();
      final match = _sectionPattern.firstMatch(rowText);
      final group1 = match?.group(1);
      if (group1 != null) {
        currentSection = group1.toLowerCase().trim();
        sections[currentSection] = [];
        continue;
      }
      if (currentSection.isNotEmpty) {
        sections[currentSection]?.addAll(row);
      }
    }

    return sections;
  }

  String _fieldKeyForSection(String sectionName) {
    return switch (sectionName) {
      'education' || 'academic background' => 'education',
      'work experience' || 'professional experience' || 'employment' => 'work_experience',
      'research interests' || 'research' => 'research',
      'major achievements' || 'publications' => 'publications',
      'projects' || 'selected projects' => 'projects',
      'skills' || 'technical skills' || 'core competencies' => 'skills',
      'awards' || 'honors' => 'awards',
      'languages' => 'languages',
      'interests' => 'interests',
      'certifications' => 'certifications',
      'summary' || 'objective' || 'profile' => 'summary',
      _ => sectionName.replaceAll(' ', '_'),
    };
  }
}

// ---------------------------------------------------------------------------
// 4. 通用回退提取器（总是命中）
// ---------------------------------------------------------------------------

class GenericFieldExtractor implements FieldExtractor {
  @override
  String get documentType => 'generic';

  static final _patterns = <String, RegExp>{
    'email': RegExp(
      r'\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b',
      caseSensitive: false,
    ),
    'phone': RegExp(
      r'(?:\+\d{1,3}[\s-]?)?\(?\d{1,4}\)?[\s-]?\d{1,4}[\s-]?\d{1,4}',
    ),
    'url': RegExp(
      r'https?://[^\s]+|www\.[^\s]+',
      caseSensitive: false,
    ),
    'date': RegExp(
      r'\b(\d{1,2}[/-]\d{1,2}[/-]\d{2,4}|\d{4}[/-]\d{1,2}[/-]\d{1,2})\b',
    ),
  };

  @override
  bool canHandle(String rawText, List<OcrBlock> blocks) => true;

  @override
  Map<String, ExtractedField> extract(List<OcrBlock> blocks) {
    final fields = <String, ExtractedField>{};
    for (final entry in _patterns.entries) {
      for (final block in blocks) {
        final match = entry.value.firstMatch(block.text);
        if (match != null) {
          var value = match.group(0)!;
          if (entry.key == 'phone' && value.length < 7) continue;
          fields[entry.key] = ExtractedField(value: value, bbox: block.bbox);
          break;
        }
      }
    }
    return fields;
  }
}
