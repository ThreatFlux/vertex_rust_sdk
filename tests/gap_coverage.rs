use std::fs;
use std::path::PathBuf;

use threatflux_vertex_rust_sdk::get_model_info;

fn manifest_path(file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(file)
}

#[test]
fn gap_analysis_notes_expected_gaps() {
    let doc = fs::read_to_string(manifest_path("docs/gap_analysis_mar_2026.md"))
        .expect("gap analysis doc should exist");

    // Model lineage
    assert!(
        doc.contains("Gemini 3.1 Pro"),
        "gap analysis should mention Gemini 3.1 Pro"
    );
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

    // Missing features
    assert!(
        doc.contains("gemini-embedding-001"),
        "gap analysis should call out the embeddings gap"
    );
    assert!(
        doc.contains("Vector/RAG") || doc.contains("vector search"),
        "gap analysis should mention vector/RAG helper gap"
    );
}

#[test]
fn readme_links_gap_doc_and_uses_supported_model() {
    let readme =
        fs::read_to_string(manifest_path("README.md")).expect("README should be readable");

    assert!(
        readme.contains("docs/gap_analysis_mar_2026.md"),
        "README should link to the gap analysis doc"
    );
    assert!(
        readme.contains("gemini-2.5-flash"),
        "Quickstart should use a supported model identifier"
    );
}

#[test]
fn model_info_still_lacks_gemini_3_1_until_added() {
    // Document the current gap so adding 3.1 metadata requires updating this test.
    assert!(
        get_model_info("gemini-3.1-pro").is_none(),
        "Gemini 3.1 models are not yet wired into model_info; update when supported"
    );
}
