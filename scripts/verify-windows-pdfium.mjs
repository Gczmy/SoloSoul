#!/usr/bin/env node
// Windows 发布前验证 PDFium；只读 PE 头和 section 表，不加载或执行 DLL。
import fs from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

const requireValid = (condition, message) => {
  if (!condition) throw new Error(message);
};

function readAt(fd, length, position) {
  const bytes = Buffer.alloc(length);
  let received = 0;
  while (received < length) {
    const count = fs.readSync(fd, bytes, received, length - received, position + received);
    requireValid(count > 0, 'PDFium 文件截断，无法读取完整 PE 头');
    received += count;
  }
  return bytes;
}

export function verifyWindowsPdfium(filename) {
  const fd = fs.openSync(filename, 'r');
  try {
    const stat = fs.fstatSync(fd);
    requireValid(stat.isFile() && stat.size >= 64, 'PDFium DLL 必须是非空文件且包含完整 DOS 头');
    const dos = readAt(fd, 64, 0);
    requireValid(dos.readUInt16LE(0) === 0x5a4d, 'PDFium DLL 缺少 Windows MZ 签名');
    const peOffset = dos.readUInt32LE(0x3c);
    requireValid(peOffset >= 64 && peOffset + 24 <= stat.size, 'PDFium PE 头位置无效或文件截断');
    const pe = readAt(fd, 24, peOffset);
    requireValid(pe.readUInt32LE(0) === 0x00004550, 'PDFium DLL 缺少 PE 签名');
    requireValid(pe.readUInt16LE(4) === 0x8664, 'PDFium DLL 必须是 Windows x64 / AMD64 架构');
    const sectionCount = pe.readUInt16LE(6);
    const optionalSize = pe.readUInt16LE(20);
    const characteristics = pe.readUInt16LE(22);
    requireValid((characteristics & 0x2002) === 0x2002, 'PDFium PE 文件必须同时标记 DLL 与 executable image');
    requireValid(sectionCount > 0 && optionalSize >= 112, 'PDFium PE32+ 头或 section 表缺失');
    const sectionOffset = peOffset + 24 + optionalSize;
    const headerEnd = sectionOffset + sectionCount * 40;
    requireValid(headerEnd <= stat.size, 'PDFium PE32+ 头或 section 表截断');
    const optional = readAt(fd, 112, peOffset + 24);
    requireValid(optional.readUInt16LE(0) === 0x20b, 'PDFium DLL 必须采用 PE32+ 格式');
    const headerSize = optional.readUInt32LE(60);
    requireValid(headerSize >= headerEnd && headerSize <= stat.size, 'PDFium PE SizeOfHeaders 无效');
    const sections = readAt(fd, sectionCount * 40, sectionOffset);
    let containsData = false;
    for (let index = 0; index < sectionCount; index += 1) {
      const rawSize = sections.readUInt32LE(index * 40 + 16);
      const rawOffset = sections.readUInt32LE(index * 40 + 20);
      if (rawSize === 0) continue;
      requireValid(rawOffset >= headerSize && rawOffset + rawSize <= stat.size,
        'PDFium DLL 的 section 数据越界或文件截断');
      containsData = true;
    }
    requireValid(containsData, 'PDFium DLL 不包含任何 section 数据');
    return { path: path.resolve(filename), size: stat.size, architecture: 'x64', sections: sectionCount };
  } finally {
    fs.closeSync(fd);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    requireValid(process.argv.length === 3, 'Usage: node scripts/verify-windows-pdfium.mjs <pdfium.dll>');
    const result = verifyWindowsPdfium(process.argv[2]);
    console.log(`PASS Windows x64 PDFium DLL: ${result.path} (${result.size} bytes)`);
  } catch (error) {
    console.error(`PDFium 校验失败: ${error.message}`);
    process.exitCode = 1;
  }
}
