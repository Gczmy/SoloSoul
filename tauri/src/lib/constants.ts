// Shared frontend timing/UX constants.

/** Duration (ms) to show "copied" feedback after a copy-to-clipboard action. */
export const COPY_FEEDBACK_DURATION_MS = 1500;

/** Default debounce delay (ms) for search inputs and similar live filters. */
export const DEBOUNCE_DELAY_MS = 300;

/** Current OCR model series displayed in the scan page. Change this when switching models. */
export const OCR_MODEL_SERIES = 'PP-OCRv6';

/** Prefix returned by the Rust backend when the active OCR model is not installed. */
export const OCR_MODEL_NOT_INSTALLED_PREFIX = '__OCR_MODEL_NOT_INSTALLED__';
