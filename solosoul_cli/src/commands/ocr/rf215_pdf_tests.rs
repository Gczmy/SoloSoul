//! RF215：经真实 CLI 受管任务调用 Core PDF 管线；仅替换逐页 ONNX 识别。
//! PDF 是可由 PDFium 解析的两页合成文档，临时目录由生产管线创建和回收。

use super::{start_scan_with, OcrRequest, ScanEngine};
use crate::app::{App, AppPhase};
use crate::tasks::TaskId;
use solosoul_core::ocr::control::OcrCancellation;
use solosoul_core::ocr::engine::scan_pdf_cancellable_with_recognizer;
use solosoul_core::ocr::{MrzResult, OcrModelTier, OcrResult};
use solosoul_core::VaultService;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(30);
const SENTINEL: &[u8] = b"RF215 synthetic parent data must survive";

// 只生成标准 PDF 字节；页面仍由生产 PDFium 提取/渲染，不伪造 PNG 或清理目录。
fn write_two_page_pdf(path: &Path) -> Result<(), String> {
    let streams = [
        "q 0.875 0.125 0.188 rg 0 0 72 72 re f Q\n",
        "q 0.188 0.251 0.875 rg 0 0 72 72 re f Q\n",
    ];
    let objects = [
        "<< /Type /Catalog /Pages 2 0 R >>".to_owned(),
        "<< /Type /Pages /Kids [3 0 R 5 0 R] /Count 2 >>".to_owned(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 72 72] /Resources << >> /Contents 4 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            streams[0].len(),
            streams[0]
        ),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 72 72] /Resources << >> /Contents 6 0 R >>"
            .to_owned(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            streams[1].len(),
            streams[1]
        ),
    ];
    let mut pdf = b"%PDF-1.4\n% RF215 synthetic two-page fixture\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    std::fs::write(path, pdf).map_err(|error| error.to_string())
}

fn png_dimensions(path: &Path) -> Result<(u32, u32), String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return Err("real renderer did not produce a PNG with an IHDR".into());
    }
    Ok((
        u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| "invalid PNG width")?),
        u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| "invalid PNG height")?),
    ))
}

struct HeldPage {
    file: File,
    alive: Arc<AtomicBool>,
}

impl HeldPage {
    fn open(path: &Path, alive: Arc<AtomicBool>) -> Result<Self, String> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // 允许其他读写，拒绝 DELETE；临时 owner 必须等待本回调退出。
            options.share_mode(0x0000_0001 | 0x0000_0002);
        }
        let file = options.open(path).map_err(|error| error.to_string())?;
        alive.store(true, Ordering::SeqCst);
        Ok(Self { file, alive })
    }
}

impl Drop for HeldPage {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::SeqCst);
    }
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);

impl ReleaseOnDrop {
    fn release(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.release();
    }
}

struct PdfObservation {
    workspace: PathBuf,
    first_name: String,
    dimensions: [(u32, u32); 2],
}

struct BlockingPdfEngine {
    temp_root: PathBuf,
    started: mpsc::Sender<PdfObservation>,
    release: mpsc::Receiver<()>,
    alive: Arc<AtomicBool>,
    recognized: Arc<AtomicUsize>,
    pdf_calls: Arc<AtomicUsize>,
    image_calls: Arc<AtomicUsize>,
}

impl ScanEngine for BlockingPdfEngine {
    fn image(&mut self, _: &Path, _: &OcrCancellation) -> Result<OcrResult, String> {
        self.image_calls.fetch_add(1, Ordering::SeqCst);
        Err("PDF request was incorrectly sent to the image backend".into())
    }

    fn pdf(&mut self, path: &Path, cancellation: &OcrCancellation) -> Result<OcrResult, String> {
        self.pdf_calls.fetch_add(1, Ordering::SeqCst);
        scan_pdf_cancellable_with_recognizer(path, &self.temp_root, cancellation, |page| {
            self.recognized.fetch_add(1, Ordering::SeqCst);
            let held = HeldPage::open(page, self.alive.clone())?;
            let workspace = page
                .parent()
                .ok_or("rendered page has no owner directory")?;
            let observation = PdfObservation {
                workspace: workspace.to_path_buf(),
                first_name: page
                    .file_name()
                    .ok_or("rendered page has no name")?
                    .to_string_lossy()
                    .into_owned(),
                dimensions: [
                    png_dimensions(&workspace.join("page_0001.png"))?,
                    png_dimensions(&workspace.join("page_0002.png"))?,
                ],
            };
            self.started
                .send(observation)
                .map_err(|error| error.to_string())?;
            self.release
                .recv_timeout(WAIT)
                .map_err(|error| format!("recognizer release: {error}"))?;
            held.file.metadata().map_err(|error| error.to_string())?;
            // 替身正常返回；取消必须由共用生产 PDF 编排在回调后检查，而非替身伪造。
            Ok(OcrResult {
                text: "RF215 recognized synthetic page".into(),
                confidence: 0.9,
                boxes: Vec::new(),
            })
        })
    }

    fn mrz(&mut self, _: &Path, _: &OcrCancellation) -> Result<Option<MrzResult>, String> {
        Err("ordinary PDF request was incorrectly sent to the MRZ backend".into())
    }
}

fn drain_until_joined(app: &mut App, task_id: TaskId) -> Result<(), String> {
    let deadline = Instant::now() + WAIT;
    loop {
        app.drain_task_events(32)
            .map_err(|error| error.to_string())?;
        if !app.ocr_tasks.contains_key(&task_id) && app.tasks.is_empty() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("OCR task did not finish after its recognizer was released".into());
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn rf215_cancel_real_multipage_pdf_keeps_pages_until_worker_join_then_cleans() -> Result<(), String>
{
    let _serial = crate::VAULT_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let directory = tempfile::tempdir().map_err(|error| error.to_string())?;
    let input = directory.path().join("synthetic-input.PDF");
    write_two_page_pdf(&input)?;
    let input_before = std::fs::read(&input).map_err(|error| error.to_string())?;
    let temp_root = directory.path().join("scans");
    std::fs::create_dir(&temp_root).map_err(|error| error.to_string())?;
    std::fs::write(temp_root.join("keep.txt"), SENTINEL).map_err(|error| error.to_string())?;
    std::fs::write(directory.path().join("parent-sentinel"), SENTINEL)
        .map_err(|error| error.to_string())?;
    let service = VaultService::with_base_path(directory.path().join("vault"));
    let account = service.create_account("RF215 synthetic", crate::TEST_PASSWORD, None)?;
    let account_id = account["id"]
        .as_str()
        .ok_or("synthetic account has no ID")?
        .to_owned();
    let mut app = App::new(Arc::new(service)).map_err(|error| error.to_string())?;
    app.phase = AppPhase::Home { account_id };
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let mut release = ReleaseOnDrop(Some(release_tx));
    let alive = Arc::new(AtomicBool::new(false));
    let recognized = Arc::new(AtomicUsize::new(0));
    let pdf_calls = Arc::new(AtomicUsize::new(0));
    let image_calls = Arc::new(AtomicUsize::new(0));
    let engine = BlockingPdfEngine {
        temp_root: temp_root.clone(),
        started: started_tx,
        release: release_rx,
        alive: alive.clone(),
        recognized: recognized.clone(),
        pdf_calls: pdf_calls.clone(),
        image_calls: image_calls.clone(),
    };
    let task_id = start_scan_with(
        &mut app,
        OcrRequest {
            path: input.clone(),
            models_dir: directory.path().join("synthetic-models"),
            tier: OcrModelTier::Small,
            mrz: false,
        },
        move |_, _| Ok(engine),
    )?;

    let observed = started_rx.recv_timeout(WAIT);
    let accepted = app.tasks.request_cancel(task_id);
    let drain_before_release = app.drain_task_events(32);
    let pending_before_release = app.ocr_tasks.contains_key(&task_id) && !app.tasks.is_empty();
    let held_before_release = alive.load(Ordering::SeqCst);
    let pages_before_release = observed.as_ref().ok().is_some_and(|seen| {
        seen.workspace.join("page_0001.png").is_file()
            && seen.workspace.join("page_0002.png").is_file()
    });
    // 无论 ready 是否超时，都先释放及真实 join；任何断言失败不遗留后台 worker。
    release.release();
    let joined = drain_until_joined(&mut app, task_id);
    // 退出清理也会清空正文，故必须在 shutdown 前记录真实 join 时的状态。
    let published_after_join =
        app.last_ocr_result.is_some() || matches!(app.phase, AppPhase::OcrResult { .. });
    let workspace_after_join = observed
        .as_ref()
        .ok()
        .is_some_and(|seen| seen.workspace.exists());
    let held_after_join = alive.load(Ordering::SeqCst);
    let shutdown = app.shutdown_tasks();
    drain_before_release.map_err(|error| error.to_string())?;
    joined?;
    shutdown.map_err(|error| error.to_string())?;
    let observed = observed.map_err(|error| {
        format!(
            "real PDF pipeline did not reach recognition; PDFIUM_LIBRARY_PATH is required: {error}"
        )
    })?;

    assert!(
        accepted,
        "cancel must be accepted while the real recognizer is still running"
    );
    assert!(
        pending_before_release,
        "cancel must not announce terminal or release the task before join"
    );
    assert!(held_before_release && pages_before_release);
    assert_eq!(pdf_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        image_calls.load(Ordering::SeqCst),
        0,
        ".PDF must use the PDF branch"
    );
    assert_eq!(
        recognized.load(Ordering::SeqCst),
        1,
        "second page must not be recognized after cancellation"
    );
    assert_eq!(observed.first_name, "page_0001.png");
    assert_eq!(observed.dimensions, [(150, 150), (150, 150)]);
    assert_eq!(observed.workspace.parent(), Some(temp_root.as_path()));
    assert!(
        !held_after_join,
        "the page handle must be released before the real join is observed"
    );
    assert!(
        !workspace_after_join,
        "the production PDF owner must clean all rendered pages before shutdown"
    );
    assert!(!observed.workspace.exists());
    assert!(
        !published_after_join,
        "cancelled plaintext must not enter the result cache or page"
    );
    assert_eq!(
        std::fs::read(&input).map_err(|error| error.to_string())?,
        input_before
    );
    assert_eq!(
        std::fs::read(directory.path().join("parent-sentinel"))
            .map_err(|error| error.to_string())?,
        SENTINEL
    );
    assert_eq!(
        std::fs::read(temp_root.join("keep.txt")).map_err(|error| error.to_string())?,
        SENTINEL
    );
    assert_eq!(
        std::fs::read_dir(&temp_root)
            .map_err(|error| error.to_string())?
            .count(),
        1
    );
    Ok(())
}
