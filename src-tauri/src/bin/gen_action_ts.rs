fn main() {
    let ts = workflow_engine::engine::action_def::generate_ts_metadata();
    let path = "../src/types/action-metadata.ts";
    std::fs::write(path, ts).expect("Failed to write TS metadata");
    eprintln!("Generated {}", path);
}
