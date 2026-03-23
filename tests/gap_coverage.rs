use std::fs;
use std::path::PathBuf;

use threatflux_vertex_rust_sdk::get_model_info;

fn manifest_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file)
}

fn latest_gap_doc_path() -> PathBuf {
    let docs_dir = manifest_path("docs");
    fs::read_dir(&docs_dir)
        .expect("docs directory should exist")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("gap_analysis_") && name_str.ends_with(".md") {
                Some((name_str.to_string(), entry.path()))
            } else {
                None
            }
        })
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, path)| path)
        .expect("at least one gap analysis doc should be present")
}

fn load_latest_gap_doc() -> String {
    fs::read_to_string(latest_gap_doc_path()).expect("gap analysis doc should be readable")
}

#[test]
fn gap_analysis_notes_expected_gaps() {
    let doc = load_latest_gap_doc();

    // Model lineage
    assert!(doc.contains("Gemini 3.1 Pro"), "gap analysis should mention Gemini 3.1 Pro");
    assert!(
        doc.contains("Gemini 3.1 Flash"),
        "gap analysis should mention Gemini 3.1 Flash variants"
    );
    assert!(
        doc.contains("migrating from 2.5 to 3.x")
            || doc.contains("migrating from 2.5")
            || doc.contains("migrate from 2.5"),
        "gap analysis should note 2.5 -> 3.x migration guidance"
    );

    // Embeddings
    assert!(
        doc.contains("gemini-embedding-001"),
        "gap analysis should call out the embeddings model"
    );
    assert!(
        doc.contains("Vector/RAG") || doc.contains("vector search"),
        "gap analysis should mention vector/RAG helper gap"
    );
}

#[test]
fn gap_analysis_documents_resolved_items() {
    let doc = load_latest_gap_doc();

    assert!(doc.contains("Resolved gaps"), "gap analysis should have a resolved gaps section");
    assert!(
        doc.contains("Model metadata") && doc.contains("resolved"),
        "model metadata gap should be marked resolved"
    );
    assert!(
        doc.contains("Embeddings client") && doc.contains("resolved"),
        "embeddings gap should be marked resolved"
    );
    assert!(
        doc.contains("Quick start defaults") && doc.contains("resolved"),
        "quickstart gap should be marked resolved"
    );
}

#[test]
fn readme_links_gap_doc_and_uses_supported_model() {
    let readme = fs::read_to_string(manifest_path("README.md")).expect("README should be readable");

    assert!(
        readme.contains(
            latest_gap_doc_path()
                .file_name()
                .expect("gap doc should have a filename")
                .to_string_lossy()
                .as_ref()
        ),
        "README should link to the latest gap analysis doc filename"
    );
    assert!(
        readme.contains("gemini-2.5-flash"),
        "Quickstart should use a supported model identifier"
    );
    assert!(
        !readme.contains("gemini-2.0-flash-001"),
        "README should no longer reference the retired gemini-2.0-flash-001 model"
    );
}

#[test]
fn model_info_includes_gemini_3_1_family() {
    // Gemini 3.1 Pro
    let pro = get_model_info("gemini-3.1-pro").expect("gemini-3.1-pro should be in model_info");
    assert_eq!(pro.canonical_id, "publishers/google/models/gemini-3.1-pro");
    assert_eq!(pro.context_window_tokens, Some(2_000_000));

    // Gemini 3.1 Flash
    let flash =
        get_model_info("gemini-3.1-flash").expect("gemini-3.1-flash should be in model_info");
    assert_eq!(flash.canonical_id, "publishers/google/models/gemini-3.1-flash");

    // Gemini 3.1 Flash Lite
    let lite = get_model_info("gemini-3.1-flash-lite")
        .expect("gemini-3.1-flash-lite should be in model_info");
    assert_eq!(lite.canonical_id, "publishers/google/models/gemini-3.1-flash-lite");
}

#[test]
fn model_info_includes_embedding_model() {
    let info = get_model_info("gemini-embedding-001")
        .expect("gemini-embedding-001 should be in model_info");
    assert_eq!(info.canonical_id, "publishers/google/models/gemini-embedding-001");
    assert!(info.max_output_tokens.is_none(), "embedding model should not have max_output_tokens");
}

#[test]
fn embeddings_api_types_are_exported() {
    // Verify the public re-exports compile and are usable.
    let req = threatflux_vertex_rust_sdk::EmbeddingRequest::new("hello");
    assert_eq!(req.instances.len(), 1);

    let batch = threatflux_vertex_rust_sdk::EmbeddingRequest::batch(vec!["a".into(), "b".into()]);
    assert_eq!(batch.instances.len(), 2);
}
