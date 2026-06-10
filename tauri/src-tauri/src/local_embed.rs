//! Local embedding inference using ONNX Runtime.
//! Loads sentence-transformer models (ONNX format) and runs CPU inference.

use ndarray::{Array2, Array3, Axis};
use ort::session::Session;
use ort::value::Tensor;
use std::sync::{Arc, Mutex};
use tokenizers::Tokenizer;

/// Global cache for the loaded embedder instance.
static EMBEDDER_CACHE: Mutex<Option<Arc<LocalEmbedder>>> = Mutex::new(None);

/// A local embedding model instance.
pub struct LocalEmbedder {
    session: std::sync::Mutex<Session>,
    tokenizer: Tokenizer,
    model_id: String,
}

impl LocalEmbedder {
    /// Load a model from the local models directory.
    pub fn load(model_dir: &std::path::Path, model_id: &str) -> Result<Self, String> {
        let model_path = model_dir.join("model.onnx");
        let tokenizer_path = model_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(format!("Model not found: {:?}", model_path));
        }
        if !tokenizer_path.exists() {
            return Err(format!("Tokenizer not found: {:?}", tokenizer_path));
        }

        let session = std::sync::Mutex::new(
            Session::builder()
                .map_err(|e| format!("Session builder: {}", e))?
                .commit_from_file(&model_path)
                .map_err(|e| format!("Load ONNX model: {}", e))?,
        );

        let tokenizer =
            Tokenizer::from_file(&tokenizer_path).map_err(|e| format!("Load tokenizer: {}", e))?;

        Ok(Self {
            session,
            tokenizer,
            model_id: model_id.to_string(),
        })
    }

    /// Embed a single text into a normalized f32 vector.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, String> {
        let mut embeddings = self.embed_batch(&[text.to_string()])?;
        if embeddings.is_empty() {
            return Err("Empty embedding result".to_string());
        }
        Ok(embeddings.remove(0))
    }

    /// Embed multiple texts in one batch call.
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let batch_size = texts.len();

        // Tokenize all texts and find max length
        let mut encodings = Vec::with_capacity(batch_size);
        let mut max_len = 0;

        for text in texts {
            let encoding = self
                .tokenizer
                .encode(text.as_str(), true)
                .map_err(|e| format!("Tokenize error: {}", e))?;
            max_len = max_len.max(encoding.len());
            encodings.push(encoding);
        }

        // Build padded input arrays [batch, max_len]
        let mut input_ids = Array2::<i64>::zeros((batch_size, max_len));
        let mut attention_mask = Array2::<i64>::zeros((batch_size, max_len));
        let mut token_type_ids = Array2::<i64>::zeros((batch_size, max_len));

        for (i, encoding) in encodings.iter().enumerate() {
            let ids = encoding.get_ids();
            let mask = encoding.get_attention_mask();
            let types = encoding.get_type_ids();
            for (j, &id) in ids.iter().enumerate() {
                input_ids[[i, j]] = id as i64;
            }
            for (j, &m) in mask.iter().enumerate() {
                attention_mask[[i, j]] = m as i64;
            }
            for (j, &t) in types.iter().enumerate() {
                token_type_ids[[i, j]] = t as i64;
            }
        }

        // Create ONNX tensors from ndarray
        let input_ids_tensor =
            Tensor::from_array(input_ids).map_err(|e| format!("Create input_ids tensor: {}", e))?;
        let attention_mask_tensor = Tensor::from_array(attention_mask.clone())
            .map_err(|e| format!("Create attention_mask tensor: {}", e))?;
        let token_type_ids_tensor = Tensor::from_array(token_type_ids)
            .map_err(|e| format!("Create token_type_ids tensor: {}", e))?;

        // Run inference with named inputs
        let mut session_guard = self.session.lock().map_err(|e| e.to_string())?;
        let outputs = session_guard
            .run(ort::inputs! {
                "input_ids" => input_ids_tensor,
                "attention_mask" => attention_mask_tensor,
                "token_type_ids" => token_type_ids_tensor
            })
            .map_err(|e| format!("ONNX inference: {}", e))?;

        // Extract last_hidden_state: shape [batch, seq_len, hidden_dim]
        let (shape, data) = outputs["last_hidden_state"]
            .try_extract_tensor::<f32>()
            .map_err(|e| format!("Extract tensor: {}", e))?;

        let hidden_dim = shape[2] as usize;
        let data_vec: Vec<f32> = data.to_vec();
        let last_hidden = Array3::from_shape_vec((batch_size, max_len, hidden_dim), data_vec)
            .map_err(|e| format!("Reshape hidden state: {}", e))?;

        // Mean pooling: sum(token_emb * mask) / sum(mask)
        let mask_f32 = attention_mask.mapv(|v| v as f32);
        let mask_expanded = mask_f32.clone().insert_axis(Axis(2)); // [batch, max_len, 1]

        let sum_embeddings = (&last_hidden * &mask_expanded).sum_axis(Axis(1)); // [batch, hidden_dim]
        let sum_mask = mask_f32.sum_axis(Axis(1)).insert_axis(Axis(1)); // [batch, 1]
        let clamped_mask = sum_mask.mapv(|v| if v > 0.0 { v } else { 1e-9 });
        let sentence_embeddings = &sum_embeddings / &clamped_mask; // [batch, hidden_dim]

        // L2 normalize each sentence vector
        let mut results = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let vec = sentence_embeddings.row(i).to_vec();
            let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
            if norm > 0.0 {
                results.push(vec.into_iter().map(|v| v / norm).collect());
            } else {
                results.push(vec);
            }
        }

        Ok(results)
    }
}

/// Get or create a cached embedder for the given model.
/// Returns an Arc so the caller can hold onto it without locking the cache.
pub fn get_embedder(
    models_dir: &std::path::Path,
    model_id: &str,
) -> Result<Arc<LocalEmbedder>, String> {
    {
        let cache = EMBEDDER_CACHE.lock().map_err(|e| e.to_string())?;
        if let Some(ref embedder) = *cache {
            if embedder.model_id == model_id {
                return Ok(Arc::clone(embedder));
            }
        }
    }

    // Not in cache or different model — load and cache
    let model_dir = models_dir.join(model_id);
    let embedder = Arc::new(LocalEmbedder::load(&model_dir, model_id)?);

    let mut cache = EMBEDDER_CACHE.lock().map_err(|e| e.to_string())?;
    *cache = Some(Arc::clone(&embedder));

    Ok(embedder)
}

/// Clear the cached embedder (e.g., when switching models).
pub fn clear_embedder_cache() {
    let mut cache = EMBEDDER_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *cache = None;
}

/// Check if a model is installed locally.
pub fn is_model_installed(models_dir: &std::path::Path, model_id: &str) -> bool {
    let model_dir = models_dir.join(model_id);
    model_dir.join("model.onnx").exists() && model_dir.join("tokenizer.json").exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_exists() {
        // This test assumes the model is downloaded in resources for dev testing
        let model_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources/models/all-MiniLM-L6-v2");
        assert!(is_model_installed(&model_dir, ""));
    }
}
