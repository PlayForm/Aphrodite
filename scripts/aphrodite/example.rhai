// ~/.hermes/aphrodite/scripts/example.rhai
// Example Rhai script — customizes CCR compression behaviour.
// Copy to ~/.hermes/aphrodite/scripts/ to activate.
// Requires: APHRODITE_SCRIPTING=1 or --scripting flag.

// Called before content is compressed. Return modified content.
fn on_compress(content, content_type, size) {
    // Example: prefix all compressed code with language hint
    if content_type == "code_rust" {
        return content; // pass through unchanged
    }
    if content_type == "code_python" {
        return content; // pass through unchanged
    }
    content // default: no modification
}

// Called when a CCR marker is being built. Return modified preview.
fn on_marker(hash, content_type, metadata, preview) {
    preview // default: no modification
}

// Called when content is retrieved from CCR. Return modified content.
fn on_retrieve(hash, content, content_type) {
    content // default: no modification
}
