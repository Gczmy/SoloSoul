/**
 * 文档导出格式的扩展名工具。
 *
 * 用户在「导出为文档」中选择保存路径后切换格式时，已选路径的扩展名需要跟随
 * 新格式更新（否则显示的还是旧格式扩展名，与实际导出的文件不一致）。
 */

/** 导出格式（与 ExportDocumentSection 的 DocFormat 保持一致）。 */
export type DocumentFormat = 'docx' | 'pdf' | 'html' | 'txt' | 'markdown';

/** 各格式的主扩展名（不含点）。 */
const FORMAT_EXTENSIONS: Record<DocumentFormat, string> = {
  docx: 'docx',
  pdf: 'pdf',
  html: 'html',
  txt: 'txt',
  markdown: 'md',
};

/** 路径末尾的扩展名（含点），兼容 Windows 反斜杠路径。 */
const EXT_RE = /\.[^.\\/]+$/;

/** 最后一个路径分隔符（/ 或 \\) 之后的文件名主体。 */
function basename(path: string): string {
  const idx = Math.max(path.lastIndexOf('/'), path.lastIndexOf('\\'));
  return idx >= 0 ? path.slice(idx + 1) : path;
}

/**
 * 把已选保存路径的扩展名替换为指定格式的扩展名（仅换扩展名，保留目录与文件名主体）。
 *
 * - 无扩展名 → 直接追加 `.扩展名`；
 * - 已是目标格式扩展名 → 原样返回（避免无意义更新）；
 * - 隐藏文件（如 `.env`）→ 追加新扩展名，避免吞掉隐藏文件名；
 * - 仅适用于桌面本地路径；移动端 SAF URI（`content://`）原样返回，不处理。
 */
export function swapDocumentExt(savePath: string, format: DocumentFormat): string {
  if (savePath.includes('://')) return savePath;
  const ext = FORMAT_EXTENSIONS[format];
  if (savePath.toLowerCase().endsWith(`.${ext}`)) return savePath;
  // 隐藏文件（如 `.env`）整段是点开头，不应被当作扩展名吞掉
  const name = basename(savePath);
  if (!name.startsWith('.') && EXT_RE.test(savePath)) {
    return savePath.replace(EXT_RE, `.${ext}`);
  }
  return `${savePath}.${ext}`;
}
