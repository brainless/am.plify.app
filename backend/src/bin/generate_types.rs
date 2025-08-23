use amplify_backend::types::*;
use std::fs;
use ts_rs::TS;

fn main() {
    println!("Generating TypeScript types...");

    // Create bindings directory if it doesn't exist
    fs::create_dir_all("../webapp/src/types/generated").unwrap();

    // Export TypeScript types
    let health_ts = HealthResponse::decl();
    let hello_ts = HelloResponse::decl();
    let error_ts = ApiError::decl();

    // Write to files with export statements
    fs::write(
        "../webapp/src/types/generated/HealthResponse.ts",
        format!("export {};", health_ts),
    )
    .unwrap();
    fs::write(
        "../webapp/src/types/generated/HelloResponse.ts",
        format!("export {};", hello_ts),
    )
    .unwrap();
    fs::write(
        "../webapp/src/types/generated/ApiError.ts",
        format!("export {};", error_ts),
    )
    .unwrap();

    // Create index file
    let index_content = r#"export type { HealthResponse } from './HealthResponse';
export type { HelloResponse } from './HelloResponse';
export type { ApiError } from './ApiError';
"#;
    fs::write("../webapp/src/types/generated/index.ts", index_content).unwrap();

    println!("TypeScript types generated successfully!");
}
